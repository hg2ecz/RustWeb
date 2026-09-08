mod query_parser;
mod query_mutation_contract;
mod domain_symbols;
mod domain_types;
mod domain_validation;
mod domain_refinement;
mod type_resolution;
mod type_semantics;
mod sql_syntax;
mod source_syntax;
mod source_loader;
mod module_namespace;
mod schema_declarations;
mod permission_declarations;
mod critical_declarations;
mod security_event_declarations;
mod webhook_declarations;
mod integration_declarations;
mod production_declarations;
mod critical_security;
mod budget_security;
mod idempotency_security;
mod webhook_security;
mod upload_security;
mod effect_security;
mod outbound_security;
mod outbound_calls;
mod permission_security;
mod template_declarations;
mod handler_parser;
mod handler_security;
mod generic_type_syntax;
mod handler_types;
mod model_security;
mod scalar_security;
mod secret_usage;
mod input_security;
mod expression_security;
mod response_security;
mod public_projection;
mod public_errors;
mod statement_helpers;
mod authorization;
mod mutation_security;
mod mfa_security;
mod tenant_security;
mod tenant_sql;
mod page_statements;
mod action_statements;
mod sum_match;
mod html_template;
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

pub fn compile_file_with_dependencies(path: impl AsRef<Path>) -> Result<CompiledFile, CompileError> {
    let units = source_loader::load_application(path.as_ref())?;
    let source_files = units.iter().map(|u| u.path.clone()).collect();
    let program = compile_units(&units)?;
    Ok(CompiledFile { program, source_files })
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
        domain_types::parse_domain_types(&u.source, &u.namespace(), &mut p).map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        schema_declarations::parse_enums(&u.source, &u.namespace(), &mut p).map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        schema_declarations::parse_models(&u.source, &u.namespace(), &mut p).map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        permission_declarations::parse_permissions(&u.source, &u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        security_event_declarations::parse_security_events(&u.source, &u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        critical_declarations::parse_critical_operations(&u.source, &u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        webhook_declarations::parse_webhooks(&u.source, &u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        integration_declarations::parse_integrations(&u.source, &u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        production_declarations::parse_production_policy(&u.source, &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        query_parser::parse_queries(&u.source, &u.namespace(), &mut p).map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        schema_declarations::parse_form_schemas(&u.source, &u.namespace(), &mut p).map_err(|e| source_loader::source_error(u, e))?;
    }
    // Route signatures are parsed before page bodies so typed @href/@action helpers
    // can resolve route names and parameter types across modules while HTML is compiled.
    for u in &units {
        routes::parse_routes(&u.source, &u.namespace(), &mut p).map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        template_declarations::parse_template_functions(&u.source, &u.namespace(), &mut p).map_err(|e| source_loader::source_error(u, e))?;
    }
    template_declarations::validate_template_cycles(&p)?;
    for u in &units {
        handler_parser::parse_pages(&u.source, &u.path.display().to_string(), &u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        handler_parser::parse_actions(&u.source, &u.path.display().to_string(), &u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    critical_security::validate_program(&p)?;
    routes::validate_routes(&p)?;
    if p.routes.is_empty() {
        return Err(CompileError::Syntax("no routes declared".into()));
    }
    Ok(p)
}


mod arrays;
mod builtin_types;
mod math_builtin_types;
mod string_builtin_types;
mod credential_builtin_types;
mod builtin_registry;
mod regex_types;
mod cache_safety;
mod diagnostics;
mod domain_objects;
mod control_flow;
mod declarations;
mod dicts;
mod lexer;
mod routes;
mod route_security;
mod route_tenant;
mod validation_rules;
mod route_budget;
mod route_cache;
mod route_calls;

mod expression;
mod expression_parser;


#[cfg(test)]
mod tests;

mod query_resource_security;
