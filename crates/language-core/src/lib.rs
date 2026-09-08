mod authorization;
mod ast;
mod builtin;
mod config;
mod credential;
mod critical_operation;
mod error;
mod domain_type;
mod effect;
mod handler_security;
mod integration;
mod http_metadata;
mod permission;
mod public_error;
mod production_policy;
mod security_event;
mod program;
mod program_webhooks;
mod program_integrations;
mod program_security_events;
mod query;
mod routing;
mod schema;
mod values;
mod web_types;
mod webhook;

mod public_projection;
pub use handler_security::HandlerSecurityContract;
pub use integration::Integration;
pub use http_metadata::{ContentDisposition, FileName, MediaType};
pub use ast::{ActionBody, ActionFunction, ActionMatchArm, ActionStatement, BinaryOp, BusinessAudit, ComponentFunction, ComputeStatement, Expr, HtmlAttrKind, HtmlPart, HtmlTemplate, LayoutFunction, PageBody, PageFunction, PageMatchArm, OutboundCall, OutboundMethod, QueryCall, RouteCall, ResourceUse, SourceLocation, Statement, TemplateParam, TemplateParamType, TxStatement};
pub use builtin::{BuiltinExecutionKind, BuiltinFunction, BuiltinMetadata};
pub use authorization::{AuthorizationMode, ObjectAuthorization};
pub use config::ServerConfig;
pub use credential::CredentialPurpose;
pub use critical_operation::CriticalOperation;
pub use error::AppError;
pub use permission::Permission;
pub use public_error::PublicError;
pub use production_policy::ProductionPolicy;
pub use security_event::SecurityEvent;
pub use program::Program;
pub use query::{CredentialLifecycleMode, CredentialLifecycleTarget, MutationTarget, QueryCapability, QueryFunction, QueryReturn, TenantScopeTarget};
pub use routing::{PublicCachePolicy, Route, RouteAuth, RouteSegment, UploadField};
pub use schema::{EnumDef, FormFailure, FormField, FormFieldIssue, FormSchema, Model, ValidationKind, ValidationRule};
pub use domain_type::DomainType;
pub use effect::Effect;
pub use values::{F32Value, FunctionParam, ImageRef, PageParam, Value, ValueType};
pub use values::{TRANSACTION_OUTCOME_ENUM_ID, TRANSACTION_OUTCOME_VARIANTS};
pub use values::DataSensitivity;
pub use web_types::{FlashKind, FlashMessage, Html, HttpMethod, LocalUrl, Redirect, RedirectStatus};
pub use webhook::Webhook;

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
            assert_eq!(BuiltinFunction::from_source_name(metadata.source_name), Some(function));
        }
    }
}

pub use public_projection::{ProjectionSourceKind, PublicProjection};
