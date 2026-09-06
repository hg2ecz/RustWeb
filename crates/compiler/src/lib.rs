mod action_statements;
mod domain_symbols;
mod handler_parser;
mod handler_types;
mod html_template;
mod module_namespace;
mod page_statements;
mod query_parser;
mod schema_declarations;
mod source_loader;
mod source_syntax;
mod sql_syntax;
mod statement_helpers;
mod template_declarations;
mod type_resolution;
use language_core::Program;
use std::path::{Path, PathBuf};

pub use diagnostics::CompileError;

#[derive(Debug)]
pub struct CompiledFile {
    pub program: Program,
    pub source_files: Vec<PathBuf>,
}

pub fn compile_file(path: impl AsRef<Path>) -> Result<Program, CompileError> {
    Ok(compile_file_with_dependencies(path)?.program)
}

pub fn compile_file_with_dependencies(
    path: impl AsRef<Path>,
) -> Result<CompiledFile, CompileError> {
    let units = source_loader::load_application(path.as_ref())?;
    let source_files = units.iter().map(|u| u.path.clone()).collect();
    let program = compile_units(&units)?;
    Ok(CompiledFile {
        program,
        source_files,
    })
}

pub fn compile_source(source: &str) -> Result<Program, CompileError> {
    if !source_loader::parse_mod_declarations(source)?.is_empty() {
        return Err(CompileError::Syntax(
            "`mod` requires file compilation; use compile_file/main.rw".into(),
        ));
    }
    compile_source_named(source, "<memory>")
}

fn compile_source_named(source: &str, source_name: &str) -> Result<Program, CompileError> {
    let units = vec![source_loader::SourceUnit {
        path: PathBuf::from(source_name),
        module_path: Vec::new(),
        source: source.to_string(),
    }];
    compile_units(&units)
}

fn compile_units(units: &[source_loader::SourceUnit]) -> Result<Program, CompileError> {
    let units = domain_objects::prepare_domain_units(units)?;
    let mut p = Program::default();
    for u in &units {
        schema_declarations::parse_enums(&u.source, &u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        schema_declarations::parse_models(&u.source, &u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        query_parser::parse_queries(&u.source, &u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        schema_declarations::parse_form_schemas(&u.source, &u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    // Route signatures are parsed before page bodies so typed @href/@action helpers
    // can resolve route names and parameter types across modules while HTML is compiled.
    for u in &units {
        routes::parse_routes(&u.source, &u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        template_declarations::parse_template_functions(&u.source, &u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    template_declarations::validate_template_cycles(&p)?;
    for u in &units {
        handler_parser::parse_pages(
            &u.source,
            &u.path.display().to_string(),
            &u.namespace(),
            &mut p,
        )
        .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        handler_parser::parse_actions(
            &u.source,
            &u.path.display().to_string(),
            &u.namespace(),
            &mut p,
        )
        .map_err(|e| source_loader::source_error(u, e))?;
    }
    routes::validate_routes(&p)?;
    if p.routes.is_empty() {
        return Err(CompileError::Syntax("no routes declared".into()));
    }
    Ok(p)
}

mod arrays;
mod builtin_registry;
mod builtin_types;
mod cache_safety;
mod control_flow;
mod declarations;
mod diagnostics;
mod dicts;
mod domain_objects;
mod lexer;
mod math_builtin_types;
mod regex_types;
mod routes;
mod string_builtin_types;

mod expression;
mod expression_parser;

#[cfg(test)]
mod tests;
