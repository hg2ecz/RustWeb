use crate::LifecycleCliConfig;
use crate::backend_support::prepare_domain_runtime;
use crate::bootstrap_config::{PublicPageCache, json_log_escape, validate_route_rate_policies};
use crate::rate_limit::RouteRateLimiter;
use crate::server_config_file::{DomainRuntime, HostingRuntime};
use crate::server_errors::SourceReloadError;
use language_core::RouteAuth;
use observability::{server_event, server_log};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SourceFileState {
    pub(super) path: PathBuf,
    pub(super) modified: Option<SystemTime>,
    pub(super) len: Option<u64>,
}

pub(super) fn snapshot_source_files(paths: &[PathBuf]) -> Vec<SourceFileState> {
    paths
        .iter()
        .map(|path| match fs::metadata(path) {
            Ok(meta) => SourceFileState {
                path: path.clone(),
                modified: meta.modified().ok(),
                len: Some(meta.len()),
            },
            Err(_) => SourceFileState {
                path: path.clone(),
                modified: None,
                len: None,
            },
        })
        .collect()
}

fn resnapshot(previous: &[SourceFileState]) -> Vec<SourceFileState> {
    let paths: Vec<PathBuf> = previous.iter().map(|v| v.path.clone()).collect();
    snapshot_source_files(&paths)
}

struct WatchState {
    generation: u64,
    last_check: Instant,
    observed: Vec<SourceFileState>,
    pending_since: Option<Instant>,
    failed_observed: Option<Vec<SourceFileState>>,
    retry_after: Option<Instant>,
    retry_delay_ms: u64,
}

fn domain_key(domain: &DomainRuntime) -> String {
    domain.host.clone().unwrap_or_else(|| "<default>".into())
}

fn unique_domains(hosting: &HostingRuntime) -> Vec<Arc<DomainRuntime>> {
    let mut seen = HashSet::new();
    hosting
        .default
        .iter()
        .cloned()
        .chain(hosting.domains.values().cloned())
        .filter(|domain| {
            domain
                .host
                .as_ref()
                .map(|host| seen.insert(host.clone()))
                .unwrap_or(true)
        })
        .collect()
}

fn validate_candidate(
    domain: &DomainRuntime,
    route_rate_limiter: &RouteRateLimiter,
    cache_max_ttl_secs: u64,
    cache_available: bool,
    auth_enabled: bool,
    database_available: bool,
) -> Result<(), SourceReloadError> {
    validate_route_rate_policies(&domain.program, route_rate_limiter.policies.as_ref())?;
    for route in domain
        .program
        .routes
        .iter()
        .filter(|route| route.public_cache.is_some())
    {
        if route.public_cache.as_ref().unwrap().ttl_secs > cache_max_ttl_secs {
            return Err(SourceReloadError::CacheTtlExceeded {
                domain: domain.host.clone(),
                route: route.name.clone(),
            });
        }
    }
    if domain
        .program
        .routes
        .iter()
        .any(|route| route.public_cache.is_some())
        && !cache_available
    {
        return Err(SourceReloadError::CacheUnavailable);
    }
    if (domain.program.pages.iter().any(|page| page.needs_db)
        || domain.program.actions.iter().any(|action| action.needs_db))
        && !database_available
    {
        return Err(SourceReloadError::DatabaseUnavailable);
    }
    if domain
        .program
        .routes
        .iter()
        .any(|route| !matches!(route.auth, RouteAuth::Public))
        && !auth_enabled
    {
        return Err(SourceReloadError::AuthenticationUnavailable);
    }
    Ok(())
}

fn build_candidate(
    current: &Arc<DomainRuntime>,
    lifecycle: &LifecycleCliConfig,
    route_rate_limiter: &RouteRateLimiter,
    cache_max_ttl_secs: u64,
    cache_available: bool,
    auth_enabled: bool,
    database_available: bool,
) -> Result<Arc<DomainRuntime>, SourceReloadError> {
    let candidate = prepare_domain_runtime(
        current.host.clone(),
        current.workdir.clone(),
        &current.app,
        current.config.clone(),
        current.storage_cli.clone(),
        current.static_cli.clone(),
        current.resource_profiles_file.as_deref(),
        current.max_concurrent_requests,
        current.max_queued_requests,
        current.queue_timeout_ms,
        lifecycle,
        current.reload.clone(),
        current.generation.saturating_add(1),
    )?;
    validate_candidate(
        &candidate,
        route_rate_limiter,
        cache_max_ttl_secs,
        cache_available,
        auth_enabled,
        database_available,
    )?;
    Ok(candidate)
}

fn commit_candidate(
    hosting: &Arc<RwLock<HostingRuntime>>,
    old: &Arc<DomainRuntime>,
    candidate: Arc<DomainRuntime>,
) -> Result<bool, SourceReloadError> {
    let mut guard = hosting
        .write()
        .map_err(|_| SourceReloadError::HostingLockPoisoned)?;
    let mut replaced = false;

    if guard
        .default
        .as_ref()
        .map(|current| Arc::ptr_eq(current, old))
        .unwrap_or(false)
    {
        guard.default = Some(Arc::clone(&candidate));
        replaced = true;
    }

    if guard
        .domains
        .values()
        .any(|current| Arc::ptr_eq(current, old))
    {
        let mut domains = (*guard.domains).clone();
        for current in domains.values_mut() {
            if Arc::ptr_eq(current, old) {
                *current = Arc::clone(&candidate);
                replaced = true;
            }
        }
        guard.domains = Arc::new(domains);
    }
    Ok(replaced)
}

async fn invalidate_domain_cache(
    public_cache: &PublicPageCache,
    old: &DomainRuntime,
    candidate: &DomainRuntime,
) -> Result<(), SourceReloadError> {
    let namespace = candidate.host.as_deref().unwrap_or("__default__");
    let mut routes = HashSet::new();
    for route in old
        .program
        .routes
        .iter()
        .chain(candidate.program.routes.iter())
    {
        if route.public_cache.is_some() {
            routes.insert(route.name.clone());
        }
    }
    for route in routes {
        public_cache
            .invalidate_route(&format!("{namespace}:{route}"))
            .await?;
    }
    Ok(())
}

pub(super) fn spawn_source_reload_supervisor(
    hosting: Arc<RwLock<HostingRuntime>>,
    lifecycle: LifecycleCliConfig,
    route_rate_limiter: Arc<RouteRateLimiter>,
    public_cache: Arc<PublicPageCache>,
    cache_max_ttl_secs: u64,
    cache_available: bool,
    auth_enabled: bool,
    database_available: bool,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut states: HashMap<String, WatchState> = HashMap::new();
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let snapshot = match hosting.read() {
                Ok(value) => value.clone(),
                Err(_) => {
                    server_event(
                        "error",
                        "source_reload_supervisor_failed",
                        "reload",
                        "hosting runtime lock poisoned",
                    );
                    return;
                }
            };
            let domains = unique_domains(&snapshot);
            let live_keys: HashSet<String> =
                domains.iter().map(|domain| domain_key(domain)).collect();
            states.retain(|key, _| live_keys.contains(key));

            for domain in domains {
                if !domain.reload.enabled {
                    states.remove(&domain_key(&domain));
                    continue;
                }
                let key = domain_key(&domain);
                let now = Instant::now();
                let state = states.entry(key.clone()).or_insert_with(|| WatchState {
                    generation: domain.generation,
                    last_check: now,
                    observed: (*domain.source_files).clone(),
                    pending_since: None,
                    failed_observed: None,
                    retry_after: None,
                    retry_delay_ms: 2000,
                });
                if state.generation != domain.generation {
                    *state = WatchState {
                        generation: domain.generation,
                        last_check: now,
                        observed: (*domain.source_files).clone(),
                        pending_since: None,
                        failed_observed: None,
                        retry_after: None,
                        retry_delay_ms: 2000,
                    };
                    continue;
                }
                if now.duration_since(state.last_check)
                    < Duration::from_millis(domain.reload.poll_interval_ms)
                {
                    continue;
                }
                state.last_check = now;
                let current = resnapshot(&domain.source_files);
                if current != state.observed {
                    state.observed = current;
                    state.pending_since = Some(now);
                    state.failed_observed = None;
                    state.retry_after = None;
                    state.retry_delay_ms = 2000;
                    server_log(&format!(
                        "{{\"event\":\"source_change_detected\",\"domain\":\"{}\",\"generation\":{},\"debounce_ms\":{}}}",
                        json_log_escape(&key),
                        domain.generation,
                        domain.reload.debounce_ms
                    ));
                    continue;
                }
                if state.observed == *domain.source_files {
                    state.pending_since = None;
                    continue;
                }
                let failed_same = state.failed_observed.as_ref() == Some(&state.observed);
                let retry_due = failed_same
                    && state
                        .retry_after
                        .map(|retry_at| now >= retry_at)
                        .unwrap_or(false);
                if failed_same && !retry_due {
                    continue;
                }
                if !retry_due {
                    let Some(pending_since) = state.pending_since else {
                        state.pending_since = Some(now);
                        continue;
                    };
                    if now.duration_since(pending_since)
                        < Duration::from_millis(domain.reload.debounce_ms)
                    {
                        continue;
                    }
                }

                let failed_snapshot = state.observed.clone();
                state.pending_since = None;
                let _ = state;

                let old: Arc<DomainRuntime> = Arc::clone(&domain);
                let lifecycle_for_build = lifecycle.clone();
                let limiter_for_build = Arc::clone(&route_rate_limiter);
                let build = tokio::task::spawn_blocking(move || {
                    build_candidate(
                        &old,
                        &lifecycle_for_build,
                        &limiter_for_build,
                        cache_max_ttl_secs,
                        cache_available,
                        auth_enabled,
                        database_available,
                    )
                    .map(|candidate: Arc<DomainRuntime>| -> (Arc<DomainRuntime>, Arc<DomainRuntime>) { (old, candidate) })
                    .map_err(|err| err.to_string())
                })
                .await;

                match build {
                    Ok(Ok((old, candidate))) => {
                        if let Err(err) =
                            invalidate_domain_cache(&public_cache, &old, &candidate).await
                        {
                            server_event(
                                "error",
                                "source_reload_cache_invalidation_failed",
                                "reload",
                                &format!("domain={key} error={err}"),
                            );
                            if let Some(state) = states.get_mut(&key) {
                                state.failed_observed = Some(failed_snapshot.clone());
                                state.pending_since = None;
                                state.retry_after = Some(
                                    Instant::now() + Duration::from_millis(state.retry_delay_ms),
                                );
                                state.retry_delay_ms =
                                    state.retry_delay_ms.saturating_mul(2).min(60_000);
                            }
                            continue;
                        }
                        match commit_candidate(&hosting, &old, Arc::clone(&candidate)) {
                            Ok(true) => {
                                server_log(&format!(
                                    "{{\"event\":\"source_reload_committed\",\"domain\":\"{}\",\"old_generation\":{},\"new_generation\":{},\"source_files\":{}}}",
                                    json_log_escape(&key),
                                    old.generation,
                                    candidate.generation,
                                    candidate.source_files.len()
                                ));
                                states.remove(&key);
                            }
                            Ok(false) => {
                                server_log(&format!(
                                    "{{\"event\":\"source_reload_stale\",\"domain\":\"{}\",\"generation\":{}}}",
                                    json_log_escape(&key),
                                    old.generation
                                ));
                                states.remove(&key);
                            }
                            Err(err) => {
                                server_event(
                                    "error",
                                    "source_reload_commit_failed",
                                    "reload",
                                    &format!("domain={key} error={err}"),
                                );
                                if let Some(state) = states.get_mut(&key) {
                                    state.failed_observed = Some(failed_snapshot.clone());
                                    state.pending_since = None;
                                    state.retry_after = Some(
                                        Instant::now()
                                            + Duration::from_millis(state.retry_delay_ms),
                                    );
                                    state.retry_delay_ms =
                                        state.retry_delay_ms.saturating_mul(2).min(60_000);
                                }
                            }
                        }
                    }
                    Ok(Err(err)) => {
                        server_event(
                            "error",
                            "source_reload_rejected",
                            "reload",
                            &format!("domain={key} generation={} error={err}", domain.generation),
                        );
                        if let Some(state) = states.get_mut(&key) {
                            state.failed_observed = Some(failed_snapshot.clone());
                            state.pending_since = None;
                            state.retry_after =
                                Some(Instant::now() + Duration::from_millis(state.retry_delay_ms));
                            state.retry_delay_ms =
                                state.retry_delay_ms.saturating_mul(2).min(60_000);
                        }
                    }
                    Err(err) => {
                        server_event(
                            "error",
                            "source_reload_task_failed",
                            "reload",
                            &format!("domain={key} error={err}"),
                        );
                        if let Some(state) = states.get_mut(&key) {
                            state.failed_observed = Some(failed_snapshot.clone());
                            state.pending_since = None;
                            state.retry_after =
                                Some(Instant::now() + Duration::from_millis(state.retry_delay_ms));
                            state.retry_delay_ms =
                                state.retry_delay_ms.saturating_mul(2).min(60_000);
                        }
                    }
                }
            }
        }
    })
}
