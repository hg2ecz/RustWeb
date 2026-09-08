use data::Database;
use language_core::{AppError, HttpMethod, Program, ServerConfig, Value};
use runtime::{AppResponse, ExecutionLimits, OutboundRuntime, ResourceProfiles};

pub(super) async fn execute(
    program: &Program,
    method: HttpMethod,
    path: &str,
    query_pairs: &[(String, String)],
    form_pairs: &[(String, String)],
    config: &ServerConfig,
    resource_profiles: &ResourceProfiles,
    system_values: &[(String, Value)],
    database: Option<&Database>,
    outbound: Option<&dyn OutboundRuntime>,
) -> Result<AppResponse, AppError> {
    runtime::execute_request_with_profiles_and_outbound(
        program,
        method,
        path,
        query_pairs,
        form_pairs,
        &ExecutionLimits {
            max_instructions: config.max_instructions,
            max_allocated_bytes: config.max_runtime_alloc_bytes,
        },
        resource_profiles,
        system_values,
        database,
        outbound,
    )
    .await
}
