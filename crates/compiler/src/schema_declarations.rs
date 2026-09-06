use crate::declarations;
use crate::diagnostics::CompileError;
use crate::lexer::tokenize;
use crate::module_namespace::qualify;
use crate::routes;
use crate::source_syntax::{is_identifier, matching_brace, read_ident};
use crate::type_resolution::resolve_value_type;
use language_core::{
    FormField, FormSchema, FunctionParam, Model, Program, ValidationKind, ValidationRule, ValueType,
};

pub(super) fn parse_enums(
    source: &str,
    namespace: &str,
    p: &mut Program,
) -> Result<(), CompileError> {
    let mut off = 0usize;
    while let Some(rel) = source[off..].find("enum ") {
        let pos = off + rel;
        if !declarations::is_top_level_declaration_at(source, pos) {
            off = pos + 5;
            continue;
        }
        if pos > 0 {
            let prev = source.as_bytes()[pos - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' {
                off = pos + 5;
                continue;
            }
        }
        let line_start = source[..pos].rfind('\n').map(|v| v + 1).unwrap_or(0);
        if !source[line_start..pos].trim().is_empty() {
            off = pos + 5;
            continue;
        }
        let start = pos + 5;
        let name = read_ident(source, start)
            .ok_or_else(|| CompileError::Syntax("enum name expected".into()))?;
        if !name
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
        {
            return Err(CompileError::Syntax(format!(
                "enum `{name}` must start with an uppercase ASCII letter"
            )));
        }
        let symbol_name = qualify(namespace, &name);
        if p.enum_by_name(&symbol_name).is_some() {
            return Err(CompileError::Syntax(format!("duplicate enum `{name}`")));
        }
        if p.enums.len() >= u16::MAX as usize {
            return Err(CompileError::Syntax("too many enum declarations".into()));
        }
        let after = start + name.len();
        let open = source[after..]
            .find('{')
            .map(|v| after + v)
            .ok_or_else(|| CompileError::Syntax(format!("enum `{name}` has no body")))?;
        let close = matching_brace(source, open)
            .ok_or_else(|| CompileError::Syntax(format!("enum `{name}` body is unclosed")))?;
        let mut variants = Vec::new();
        for line in source[open + 1..close].lines() {
            let clean = line.split_once("//").map(|(a, _)| a).unwrap_or(line);
            for raw in clean.split(|c: char| c == ',' || c.is_whitespace()) {
                let v = raw.trim();
                if v.is_empty() {
                    continue;
                }
                if !is_identifier(v)
                    || !v
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_uppercase())
                        .unwrap_or(false)
                {
                    return Err(CompileError::Syntax(format!(
                        "enum `{name}` invalid variant `{v}`"
                    )));
                }
                if variants.iter().any(|x: &String| x == v) {
                    return Err(CompileError::Syntax(format!(
                        "enum `{name}` duplicate variant `{v}`"
                    )));
                }
                variants.push(v.to_string());
            }
        }
        if variants.is_empty() {
            return Err(CompileError::Syntax(format!(
                "enum `{name}` requires at least one variant"
            )));
        }
        if variants.len() > 256 {
            return Err(CompileError::Syntax(format!(
                "enum `{name}` has too many variants (max 256)"
            )));
        }
        p.enums.push(language_core::EnumDef {
            name: symbol_name,
            variants,
        });
        off = close + 1;
    }
    Ok(())
}
pub(super) fn parse_form_schemas(
    source: &str,
    namespace: &str,
    p: &mut Program,
) -> Result<(), CompileError> {
    let mut off = 0usize;
    while let Some(rel) = source[off..].find("form ") {
        let keyword = off + rel;
        if !declarations::is_top_level_declaration_at(source, keyword) {
            off = keyword + 5;
            continue;
        }
        if keyword > 0 {
            let prev = source.as_bytes()[keyword - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' {
                off = keyword + 5;
                continue;
            }
        }
        let start = keyword + 5;
        let Some(name) = read_ident(source, start) else {
            off = start;
            continue;
        };
        let after = start + name.len();
        let mut j = after;
        while j < source.len() && source.as_bytes()[j].is_ascii_whitespace() {
            j += 1;
        }
        if source.as_bytes().get(j) != Some(&b'{') {
            off = after;
            continue;
        }
        let symbol_name = qualify(namespace, &name);
        if p.forms.iter().any(|f| f.name == symbol_name) {
            return Err(CompileError::Syntax(format!("duplicate form `{name}`")));
        }
        let close = matching_brace(source, j)
            .ok_or_else(|| CompileError::Syntax(format!("form `{name}` unclosed")))?;
        let tokens = tokenize(&source[j + 1..close])?;
        let mut fields = Vec::new();
        let mut validations = Vec::new();
        let mut i = 0usize;
        while i < tokens.len() {
            if tokens[i] == "validate" {
                i += 1;
                while i < tokens.len() {
                    let (rule, next) = parse_validation_rule(&tokens, i)?;
                    validations.push(rule);
                    i = next;
                }
                break;
            }
            let field = routes::parse_typed_binding(&name, &tokens[i], namespace, p)?;
            if fields.iter().any(|f: &FormField| f.name == field.name) {
                return Err(CompileError::Syntax(format!(
                    "form `{name}` duplicate field `{}`",
                    field.name
                )));
            }
            fields.push(field);
            i += 1;
        }
        if fields.is_empty() {
            return Err(CompileError::Syntax(format!(
                "form `{name}` requires at least one field"
            )));
        }
        validate_form_rules(&name, &fields, &validations)?;
        p.forms.push(FormSchema {
            name: symbol_name,
            fields,
            validations,
        });
        off = close + 1;
    }
    Ok(())
}

pub(super) fn parse_validation_rule(
    tokens: &[String],
    i: usize,
) -> Result<(ValidationRule, usize), CompileError> {
    let field = tokens
        .get(i)
        .ok_or_else(|| CompileError::Syntax("validation field expected".into()))?
        .clone();
    let kind = tokens
        .get(i + 1)
        .ok_or_else(|| CompileError::Syntax("validation kind expected".into()))?
        .as_str();
    match kind {
        "length" => {
            let a = tokens
                .get(i + 2)
                .ok_or_else(|| CompileError::Syntax("length minimum expected".into()))?;
            let b = tokens
                .get(i + 3)
                .ok_or_else(|| CompileError::Syntax("length maximum expected".into()))?;
            let min: usize = a
                .parse()
                .map_err(|_| CompileError::Syntax("length min must be integer".into()))?;
            let max: usize = b
                .parse()
                .map_err(|_| CompileError::Syntax("length max must be integer".into()))?;
            if min > max {
                return Err(CompileError::Syntax(
                    "validation min cannot exceed max".into(),
                ));
            }
            Ok((
                ValidationRule {
                    field,
                    kind: ValidationKind::Length { min, max },
                },
                i + 4,
            ))
        }
        "range" => {
            let a = tokens
                .get(i + 2)
                .ok_or_else(|| CompileError::Syntax("range minimum expected".into()))?;
            let b = tokens
                .get(i + 3)
                .ok_or_else(|| CompileError::Syntax("range maximum expected".into()))?;
            let min: i64 = a
                .parse()
                .map_err(|_| CompileError::Syntax("range min must be integer".into()))?;
            let max: i64 = b
                .parse()
                .map_err(|_| CompileError::Syntax("range max must be integer".into()))?;
            if min > max {
                return Err(CompileError::Syntax(
                    "validation min cannot exceed max".into(),
                ));
            }
            Ok((
                ValidationRule {
                    field,
                    kind: ValidationKind::Range { min, max },
                },
                i + 4,
            ))
        }
        "pattern" => {
            let regex = tokens
                .get(i + 2)
                .ok_or_else(|| {
                    CompileError::Syntax(
                        "pattern validation requires a quoted regular expression".into(),
                    )
                })?
                .clone();
            if regex.is_empty() || regex.len() > 256 {
                return Err(CompileError::Syntax(
                    "pattern regular expression must be 1..256 bytes".into(),
                ));
            }
            regex::Regex::new(&regex).map_err(|_| {
                CompileError::Syntax(
                    "pattern validation contains an invalid regular expression".into(),
                )
            })?;
            Ok((
                ValidationRule {
                    field,
                    kind: ValidationKind::Pattern { regex },
                },
                i + 3,
            ))
        }
        "same" => {
            let other = tokens
                .get(i + 2)
                .ok_or_else(|| {
                    CompileError::Syntax("same validation requires another field".into())
                })?
                .clone();
            Ok((
                ValidationRule {
                    field,
                    kind: ValidationKind::SameAs { other },
                },
                i + 3,
            ))
        }
        _ => Err(CompileError::Syntax(format!(
            "unknown validation kind `{kind}`"
        ))),
    }
}
fn validate_form_rules(
    name: &str,
    fields: &[FormField],
    rules: &[ValidationRule],
) -> Result<(), CompileError> {
    for r in rules {
        let f = fields.iter().find(|f| f.name == r.field).ok_or_else(|| {
            CompileError::Syntax(format!(
                "form `{name}` validation references unknown field `{}`",
                r.field
            ))
        })?;
        match &r.kind {
            ValidationKind::Length { .. } if f.ty == ValueType::String => {}
            ValidationKind::Range { .. } if f.ty == ValueType::Int => {}
            ValidationKind::Pattern { .. } if f.ty == ValueType::String => {}
            ValidationKind::SameAs { other } => {
                let o = fields.iter().find(|x| x.name == *other).ok_or_else(|| {
                    CompileError::Syntax(format!(
                        "form `{name}` same validation references unknown field `{other}`"
                    ))
                })?;
                if f.ty != o.ty {
                    return Err(CompileError::Syntax(format!(
                        "form `{name}` same validation requires matching field types `{}` and `{other}`",
                        r.field
                    )));
                }
                if matches!(
                    f.ty,
                    ValueType::Upload
                        | ValueType::Image
                        | ValueType::F32Array
                        | ValueType::StringList
                        | ValueType::StringDict
                ) {
                    return Err(CompileError::Syntax(format!(
                        "form `{name}` same validation does not support Upload/Image field `{}`",
                        r.field
                    )));
                }
            }
            ValidationKind::Length { .. } => {
                return Err(CompileError::Syntax(format!(
                    "form `{name}` length validation requires String field `{}`",
                    r.field
                )));
            }
            ValidationKind::Range { .. } => {
                return Err(CompileError::Syntax(format!(
                    "form `{name}` range validation requires Int field `{}`",
                    r.field
                )));
            }
            ValidationKind::Pattern { .. } => {
                return Err(CompileError::Syntax(format!(
                    "form `{name}` pattern validation requires String field `{}`",
                    r.field
                )));
            }
        }
    }
    Ok(())
}

pub(super) fn parse_models(
    source: &str,
    namespace: &str,
    p: &mut Program,
) -> Result<(), CompileError> {
    let mut off = 0;
    while let Some(rel) = source[off..].find("model ") {
        let keyword = off + rel;
        if !declarations::is_top_level_declaration_at(source, keyword) {
            off = keyword + 6;
            continue;
        }
        let start = keyword + 6;
        let name = read_ident(source, start)
            .ok_or_else(|| CompileError::Syntax("model name expected".into()))?;
        let symbol_name = qualify(namespace, &name);
        if p.models.iter().any(|m| m.name == symbol_name) {
            return Err(CompileError::Syntax(format!("duplicate model `{name}`")));
        }
        let brace = source[start + name.len()..]
            .find('{')
            .map(|v| start + name.len() + v)
            .ok_or_else(|| CompileError::Syntax(format!("model `{name}` has no body")))?;
        let close = matching_brace(source, brace)
            .ok_or_else(|| CompileError::Syntax(format!("model `{name}` body is unclosed")))?;
        let mut fields = Vec::new();
        for raw in source[brace + 1..close]
            .lines()
            .map(str::trim)
            .filter(|v| !v.is_empty() && !v.starts_with("//"))
        {
            let raw = raw.trim_end_matches(',').trim();
            let (field, ty) = raw.split_once(':').ok_or_else(|| {
                CompileError::Syntax(format!(
                    "model `{name}` field `{raw}` expected `name: Type`"
                ))
            })?;
            let field = field.trim();
            let ty = ty.trim();
            if !is_identifier(field) {
                return Err(CompileError::Syntax(format!(
                    "model `{name}` invalid field `{field}`"
                )));
            }
            let ty = resolve_value_type(ty, namespace, p)
                .filter(|t| {
                    !matches!(
                        t,
                        ValueType::Upload
                            | ValueType::F32Array
                            | ValueType::StringList
                            | ValueType::StringDict
                    )
                })
                .ok_or_else(|| {
                    CompileError::Syntax(format!("model `{name}` unsupported field type `{ty}`"))
                })?;
            if fields.iter().any(|f: &FunctionParam| f.name == field) {
                return Err(CompileError::Syntax(format!(
                    "model `{name}` duplicate field `{field}`"
                )));
            }
            fields.push(FunctionParam {
                name: field.into(),
                ty,
            });
        }
        if fields.is_empty() {
            return Err(CompileError::Syntax(format!(
                "model `{name}` must have at least one field"
            )));
        }
        p.models.push(Model {
            name: symbol_name,
            fields,
        });
        off = close + 1;
    }
    Ok(())
}
