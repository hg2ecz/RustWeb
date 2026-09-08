use crate::server_config_file::DomainRuntime;
use crate::server_errors::StartupError;
use crate::{AuthRuntime, WebSecurityCliConfig};
use language_core::RouteAuth;
use std::sync::Arc;

pub(super) fn validate_replay_guard_requirements(
    domains: &[Arc<DomainRuntime>],
    redis_available: bool,
    web: &WebSecurityCliConfig,
) -> Result<(), StartupError> {
    let uses_idempotency = domains
        .iter()
        .any(|domain| domain.program.routes.iter().any(|route| route.idempotent));
    let uses_webhooks = domains.iter().any(|domain| {
        domain
            .program
            .routes
            .iter()
            .any(|route| matches!(route.auth, RouteAuth::Webhook(_)))
    });
    if (uses_idempotency || uses_webhooks) && !redis_available {
        return Err(StartupError::invalid(
            "application uses replay-protected idempotency/webhook routes but Redis is not configured; replay protection is fail-closed and has no memory fallback",
        ));
    }
    if uses_webhooks {
        let Some(root) = web.webhook_secrets_dir.as_deref() else {
            return Err(StartupError::invalid(
                "application declares verified webhook routes but web.webhook_secrets_dir is not configured",
            ));
        };
        let meta = std::fs::symlink_metadata(root).map_err(|_| {
            StartupError::invalid("web.webhook_secrets_dir is unavailable")
        })?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err(StartupError::invalid(
                "web.webhook_secrets_dir must be a real directory, not a symlink",
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_auth_requirements(
    domains: &[Arc<DomainRuntime>],
    auth_runtime: &AuthRuntime,
) -> Result<(), StartupError> {
    let protected_routes = domains.iter().any(|domain| {
        domain
            .program
            .routes
            .iter()
            .any(|route| !matches!(route.auth, RouteAuth::Public | RouteAuth::Webhook(_)))
    });
    if protected_routes && auth_runtime.ldap.is_none() && auth_runtime.local.is_none() {
        return Err(StartupError::invalid(
            "application declares protected routes, but no authentication backend is configured",
        ));
    }
    Ok(())
}
