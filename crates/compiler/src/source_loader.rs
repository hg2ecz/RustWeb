use crate::declarations;
use crate::diagnostics::CompileError;
use crate::source_syntax::is_identifier;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_MODULES: usize = 512;
const MAX_MODULE_BYTES: usize = 1024 * 1024;
const MAX_APP_SOURCE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct SourceUnit {
    pub(crate) path: PathBuf,
    pub(crate) module_path: Vec<String>,
    pub(crate) source: String,
}

impl SourceUnit {
    pub(crate) fn namespace(&self) -> String {
        self.module_path.join("::")
    }
}

pub(crate) fn source_error(unit: &SourceUnit, err: CompileError) -> CompileError {
    if unit.path == Path::new("<memory>") {
        return err;
    }
    CompileError::Syntax(format!("{}: {}", unit.path.display(), err))
}

pub(crate) fn load_application(entry: &Path) -> Result<Vec<SourceUnit>, CompileError> {
    let meta = fs::symlink_metadata(entry)?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(CompileError::Syntax(
            "application entrypoint must be a regular non-symlink .rw file".into(),
        ));
    }
    if entry.extension().and_then(|v| v.to_str()) != Some("rw") {
        return Err(CompileError::Syntax(
            "application entrypoint must use the .rw extension".into(),
        ));
    }
    let entry = entry.canonicalize()?;
    let app_root = entry
        .parent()
        .ok_or_else(|| {
            CompileError::Syntax("application entrypoint has no parent directory".into())
        })?
        .to_path_buf();
    let mut units = Vec::new();
    let mut seen = HashSet::new();
    let mut active = Vec::new();
    let mut total = 0usize;
    load_source_unit(
        &entry,
        &app_root,
        Vec::new(),
        &mut units,
        &mut seen,
        &mut active,
        &mut total,
    )?;
    Ok(units)
}

pub(crate) fn parse_mod_declarations(source: &str) -> Result<Vec<Vec<String>>, CompileError> {
    let mut out: Vec<Vec<String>> = Vec::new();
    for (idx, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if [
            "enum ",
            "object ",
            "model ",
            "query fn ",
            "form ",
            "component fn ",
            "layout fn ",
            "page fn ",
            "action fn ",
            "route ",
        ]
        .iter()
        .any(|prefix| line.starts_with(prefix))
        {
            break;
        }
        if !line.starts_with("mod ") {
            return Err(CompileError::Syntax(format!(
                "line {}: only comments or `mod path;` declarations may appear before the first declaration",
                idx + 1
            )));
        }
        let rest = line.strip_prefix("mod ").unwrap().trim();
        let Some(raw_path) = rest.strip_suffix(';') else {
            return Err(CompileError::Syntax(format!(
                "line {}: module declaration must be `mod path;`",
                idx + 1
            )));
        };
        let raw_path = raw_path.trim();
        if raw_path.starts_with("self::")
            || raw_path.starts_with("super::")
            || raw_path.starts_with("crate::")
        {
            return Err(CompileError::Syntax(format!(
                "line {}: module path `{raw_path}` must be application-root relative; `self::`, `super::`, and `crate::` are not supported",
                idx + 1
            )));
        }
        if raw_path.contains('/') || raw_path.contains('\\') || raw_path.contains("..") {
            return Err(CompileError::Syntax(format!(
                "line {}: module path `{raw_path}` must use `::` segments and may not contain filesystem traversal",
                idx + 1
            )));
        }
        let segments: Vec<String> = raw_path
            .split("::")
            .map(str::trim)
            .map(str::to_owned)
            .collect();
        if segments.is_empty() || segments.iter().any(|segment| !is_identifier(segment)) {
            return Err(CompileError::Syntax(format!(
                "line {}: invalid module path `{raw_path}`",
                idx + 1
            )));
        }
        if out.iter().any(|value| value == &segments) {
            return Err(CompileError::Syntax(format!(
                "line {}: duplicate module declaration `{raw_path}`",
                idx + 1
            )));
        }
        out.push(segments);
    }
    let top_level_mods = source
        .match_indices("mod ")
        .filter(|(pos, _)| declarations::is_top_level_declaration_at(source, *pos))
        .count();
    if top_level_mods != out.len() {
        return Err(CompileError::Syntax(
            "`mod` declarations must appear before the first top-level declaration".into(),
        ));
    }
    Ok(out)
}

fn resolve_module(app_root: &Path, module_path: &[String]) -> Result<PathBuf, CompileError> {
    let display = module_path.join("::");
    let mut candidate = app_root.to_path_buf();
    for segment in module_path {
        candidate.push(segment);
    }
    candidate.set_extension("rw");
    if !candidate.exists() {
        return Err(CompileError::Syntax(format!(
            "module `{display}` was not found at `{}`; module paths are application-root relative",
            candidate.display()
        )));
    }
    let meta = fs::symlink_metadata(&candidate)?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(CompileError::Syntax(format!(
            "module `{display}` must be a regular non-symlink file"
        )));
    }
    let canonical = candidate.canonicalize()?;
    if !canonical.starts_with(app_root) {
        return Err(CompileError::Syntax(format!(
            "module `{display}` escapes the application root"
        )));
    }
    Ok(canonical)
}

fn load_source_unit(
    path: &Path,
    app_root: &Path,
    module_path: Vec<String>,
    units: &mut Vec<SourceUnit>,
    seen: &mut HashSet<PathBuf>,
    active: &mut Vec<PathBuf>,
    total: &mut usize,
) -> Result<(), CompileError> {
    if units.len() >= MAX_MODULES {
        return Err(CompileError::Syntax(format!(
            "application exceeds module limit ({MAX_MODULES})"
        )));
    }
    if active.iter().any(|v| v == path) {
        let mut chain: Vec<String> = active.iter().map(|v| v.display().to_string()).collect();
        chain.push(path.display().to_string());
        return Err(CompileError::Syntax(format!(
            "module cycle: {}",
            chain.join(" -> ")
        )));
    }
    if !seen.insert(path.to_path_buf()) {
        return Ok(());
    }
    let bytes = fs::read(path)?;
    if bytes.len() > MAX_MODULE_BYTES {
        return Err(CompileError::Syntax(format!(
            "module `{}` exceeds {} bytes",
            path.display(),
            MAX_MODULE_BYTES
        )));
    }
    *total = total
        .checked_add(bytes.len())
        .ok_or_else(|| CompileError::Syntax("application source size overflow".into()))?;
    if *total > MAX_APP_SOURCE_BYTES {
        return Err(CompileError::Syntax(format!(
            "application source exceeds {} bytes",
            MAX_APP_SOURCE_BYTES
        )));
    }
    let source = String::from_utf8(bytes)
        .map_err(|_| CompileError::Syntax(format!("module `{}` is not UTF-8", path.display())))?;
    let mods = parse_mod_declarations(&source)?;
    units.push(SourceUnit {
        path: path.to_path_buf(),
        module_path: module_path.clone(),
        source,
    });
    active.push(path.to_path_buf());
    for child_path in mods {
        let child = resolve_module(app_root, &child_path)?;
        load_source_unit(&child, app_root, child_path, units, seen, active, total)?;
    }
    active.pop();
    Ok(())
}
