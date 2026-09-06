use crate::backend_support::build_hosting_runtime;
use crate::server_errors::StartupError;
use crate::startup_args::StartupArgs;
use crate::tls_support::build_tls_acceptor;
use observability::Metrics;
use std::sync::{Arc, RwLock};

pub(super) async fn run(parsed: StartupArgs) -> Result<(), StartupError> {
    let StartupArgs {
        app,
        config,
        db_config,
        auth: auth_cli,
        tls: tls_cli,
        web: mut web_cli,
        storage: storage_cli,
        resource_limits: _resource_limits,
        resource_profiles_file,
        static_assets: static_cli,
        lifecycle,
        rate_limits_file,
        allow_memory_rate_limit,
        observability: observability_cli,
        cache: cache_cli,
        log_config: _log_config,
        domains: domain_cli,
        unix_socket,
        behind_proxy,
        source_reload,
    } = parsed;
    if behind_proxy && unix_socket.is_some() {
        web_cli.trusted_proxy_cidrs.push("127.0.0.0/8".parse()?);
        web_cli.trusted_proxy_cidrs.push("::1/128".parse()?);
    }
    let metrics = Arc::new(Metrics::default());
    let hosting_value = build_hosting_runtime(
        &app,
        &config,
        &storage_cli,
        &static_cli,
        resource_profiles_file.as_deref(),
        &lifecycle,
        &domain_cli,
        &source_reload,
    )?;
    let hosting = Arc::new(RwLock::new(hosting_value));
    let hosting_snapshot = hosting
        .read()
        .map_err(|_| StartupError::invalid("hosting runtime lock poisoned"))?
        .clone();
    let prepared = crate::startup_services::prepare(crate::startup_services::ServicePreparation {
        hosting: &hosting,
        hosting_snapshot: &hosting_snapshot,
        db_config,
        auth: &auth_cli,
        config: &config,
        rate_limits_file: rate_limits_file.as_deref(),
        allow_memory_rate_limit,
        cache: &cache_cli,
        lifecycle: &lifecycle,
    })
    .await?;
    let crate::startup_services::PreparedServices {
        database,
        sessions,
        auth_runtime,
        route_rate_limiter,
        public_cache,
        source_reload_task,
    } = prepared;

    if !hosting_snapshot.domains.is_empty() && tls_cli.public_host.is_some() {
        return Err(StartupError::invalid(
            "multi-domain mode derives allowed public hosts from [[domains]]; do not configure tls.public_host/--public-host",
        ));
    }
    let tls_acceptor = build_tls_acceptor(&tls_cli, &domain_cli)?;
    if behind_proxy && tls_acceptor.is_some() {
        return Err(StartupError::invalid(
            "server.behind_proxy/--behind-proxy cannot be combined with backend TLS; terminate TLS at the trusted reverse proxy",
        ));
    }
    #[cfg(not(unix))]
    if unix_socket.is_some() {
        return Err(StartupError::invalid(
            "server.unix_socket is only supported on Unix platforms",
        ));
    }
    if unix_socket.is_some() && !behind_proxy {
        return Err(StartupError::invalid(
            "server.unix_socket requires server.behind_proxy=true so forwarding headers have explicit proxy semantics",
        ));
    }
    if behind_proxy
        && unix_socket.is_none()
        && (!config.listen.ip().is_loopback() || web_cli.trusted_proxy_cidrs.is_empty())
    {
        return Err(StartupError::invalid(
            "TCP behind-proxy mode requires a loopback listener and at least one explicit web.trusted_proxy_cidrs entry",
        ));
    }
    let reverse_proxy_https = tls_acceptor.is_none()
        && !config.insecure_dev_cookies
        && behind_proxy
        && (unix_socket.is_some()
            || (config.listen.ip().is_loopback() && !web_cli.trusted_proxy_cidrs.is_empty()))
        && (tls_cli.public_host.is_some() || !hosting_snapshot.domains.is_empty());
    if tls_acceptor.is_none() && !config.insecure_dev_cookies && !reverse_proxy_https {
        return Err(StartupError::invalid(
            "plain HTTP is development-only unless rwlang-server runs in explicit behind-proxy mode with a Unix socket or loopback trusted proxy; otherwise configure TLS or pass --insecure-dev-cookies explicitly",
        ));
    }
    if tls_cli.http_redirect_listen.is_some() && tls_acceptor.is_none() {
        return Err(StartupError::invalid(
            "--http-redirect-listen requires HTTPS/TLS configuration",
        ));
    }
    if tls_cli.http_redirect_listen.is_some() && !hosting_snapshot.domains.is_empty() {
        return Err(StartupError::invalid(
            "multi-domain mode does not use the single-host HTTP redirect listener; redirect at the reverse proxy or run separate redirects",
        ));
    }
    if tls_cli.http_redirect_listen.is_some() && tls_cli.public_host.is_none() {
        return Err(StartupError::invalid(
            "--http-redirect-listen requires --public-host",
        ));
    }
    if tls_acceptor.is_some()
        && !config.insecure_dev_cookies
        && tls_cli.public_host.is_none()
        && hosting_snapshot.domains.is_empty()
    {
        return Err(StartupError::invalid(
            "production HTTPS requires --public-host for Host/Origin pinning",
        ));
    }

    crate::startup_transport::serve(crate::startup_transport::TransportRuntime {
        app,
        config,
        tls: tls_cli,
        web: web_cli,
        lifecycle,
        observability: observability_cli,
        cache: cache_cli,
        domains: domain_cli,
        unix_socket,
        behind_proxy,
        reverse_proxy_https,
        multi_domain: !hosting_snapshot.domains.is_empty(),
        hosting,
        database,
        sessions,
        auth_runtime,
        route_rate_limiter,
        public_cache,
        metrics,
        source_reload_task,
        tls_acceptor,
    })
    .await
}
