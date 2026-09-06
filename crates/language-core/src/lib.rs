mod ast;
mod config;
mod error;
mod program;
mod query;
mod routing;
mod schema;
mod values;
mod web_types;

pub use ast::{
    ActionBody, ActionFunction, ActionStatement, BinaryOp, BuiltinExecutionKind, BuiltinFunction,
    BuiltinMetadata, BusinessAudit, ComponentFunction, ComputeStatement, Expr, HtmlAttrKind,
    HtmlPart, HtmlTemplate, LayoutFunction, ObjectAuthorization, PageBody, PageFunction, QueryCall,
    ResourceUse, SourceLocation, Statement, TemplateParam, TemplateParamType, TxStatement,
};
pub use config::ServerConfig;
pub use error::AppError;
pub use program::Program;
pub use query::{QueryCapability, QueryFunction, QueryReturn};
pub use routing::{PublicCachePolicy, Route, RouteAuth, RouteSegment, UploadField};
pub use schema::{
    EnumDef, FormFailure, FormField, FormFieldIssue, FormSchema, Model, ValidationKind,
    ValidationRule,
};
pub use values::{F32Value, FunctionParam, ImageRef, PageParam, Value, ValueType};
pub use web_types::{FlashKind, FlashMessage, Html, HttpMethod, Redirect, RedirectStatus};

#[cfg(test)]
mod builtin_metadata_tests {
    use crate::BuiltinFunction;
    use std::collections::HashSet;

    #[test]
    fn builtin_metadata_has_unique_public_names_and_valid_arity() {
        let mut names = HashSet::new();
        for function in BuiltinFunction::ALL {
            let metadata = function.metadata();
            assert!(names.insert(metadata.source_name));
            assert!(metadata.min_args <= metadata.max_args);
            assert!(metadata.instruction_cost > 0);
            assert_eq!(
                BuiltinFunction::from_source_name(metadata.source_name),
                Some(function)
            );
        }
    }
}
