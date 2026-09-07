mod ast;
mod authorization;
mod builtin;
mod config;
mod credential;
mod critical_operation;
mod domain_type;
mod error;
mod handler_security;
mod permission;
mod program;
mod query;
mod routing;
mod schema;
mod values;
mod web_types;

mod public_projection;
pub use ast::{
    ActionBody, ActionFunction, ActionStatement, BinaryOp, BusinessAudit, ComponentFunction,
    ComputeStatement, Expr, HtmlAttrKind, HtmlPart, HtmlTemplate, LayoutFunction, PageBody,
    PageFunction, QueryCall, ResourceUse, RouteCall, SourceLocation, Statement, TemplateParam,
    TemplateParamType, TxStatement,
};
pub use authorization::{AuthorizationMode, ObjectAuthorization};
pub use builtin::{BuiltinExecutionKind, BuiltinFunction, BuiltinMetadata};
pub use config::ServerConfig;
pub use credential::CredentialPurpose;
pub use critical_operation::CriticalOperation;
pub use domain_type::DomainType;
pub use error::AppError;
pub use handler_security::HandlerSecurityContract;
pub use permission::Permission;
pub use program::Program;
pub use query::{
    CredentialLifecycleMode, CredentialLifecycleTarget, MutationTarget, QueryCapability,
    QueryFunction, QueryReturn,
};
pub use routing::{PublicCachePolicy, Route, RouteAuth, RouteSegment, UploadField};
pub use schema::{
    EnumDef, FormFailure, FormField, FormFieldIssue, FormSchema, Model, ValidationKind,
    ValidationRule,
};
pub use values::DataSensitivity;
pub use values::{F32Value, FunctionParam, ImageRef, PageParam, Value, ValueType};
pub use web_types::{
    FlashKind, FlashMessage, Html, HttpMethod, LocalUrl, Redirect, RedirectStatus,
};

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

pub use public_projection::{ProjectionSourceKind, PublicProjection};
