use crate::{errors::ResourceProfileError, memory};
use language_core::{AppError, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Debug, Clone)]
pub struct ExecutionLimits {
    pub max_instructions: u64,
    pub max_allocated_bytes: u64,
}
impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_instructions: 100_000,
            max_allocated_bytes: 32 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ResourceProfileConfig {
    pub max_instructions: u64,
    pub max_allocated_bytes: u64,
    pub max_concurrent: usize,
}

struct ProfileRuntime {
    config: ResourceProfileConfig,
    semaphore: Arc<Semaphore>,
}
#[derive(Clone)]
pub struct ResourceProfiles {
    default: ResourceProfileConfig,
    profiles: Arc<HashMap<String, ProfileRuntime>>,
}
impl ResourceProfiles {
    pub fn new(
        default: ResourceProfileConfig,
        named: HashMap<String, ResourceProfileConfig>,
    ) -> Result<Self, ResourceProfileError> {
        if default.max_instructions == 0
            || default.max_allocated_bytes == 0
            || default.max_concurrent == 0
        {
            return Err(ResourceProfileError::ZeroDefaultLimit);
        }
        let mut profiles = HashMap::new();
        for (name, config) in named {
            if name == "default"
                || name.is_empty()
                || !name.bytes().all(|b| b == b'_' || b.is_ascii_alphanumeric())
            {
                return Err(ResourceProfileError::InvalidName(name));
            }
            if config.max_instructions == 0
                || config.max_allocated_bytes == 0
                || config.max_concurrent == 0
            {
                return Err(ResourceProfileError::ZeroNamedLimit(name));
            }
            profiles.insert(
                name,
                ProfileRuntime {
                    config,
                    semaphore: Arc::new(Semaphore::new(config.max_concurrent)),
                },
            );
        }
        Ok(Self {
            default,
            profiles: Arc::new(profiles),
        })
    }
    pub fn default_for_limits(limits: &ExecutionLimits) -> Self {
        Self::new(
            ResourceProfileConfig {
                max_instructions: limits.max_instructions,
                max_allocated_bytes: limits.max_allocated_bytes,
                max_concurrent: usize::MAX / 2,
            },
            HashMap::new(),
        )
        .expect("valid default limits")
    }
    pub fn default_config(&self) -> ResourceProfileConfig {
        self.default
    }
    pub fn config(&self, name: &str) -> Option<ResourceProfileConfig> {
        self.profiles.get(name).map(|p| p.config)
    }
    pub(crate) async fn acquire(
        &self,
        name: &str,
    ) -> Result<(ResourceProfileConfig, OwnedSemaphorePermit), AppError> {
        let profile = self.profiles.get(name).ok_or(AppError::Internal)?;
        let permit = Arc::clone(&profile.semaphore)
            .acquire_owned()
            .await
            .map_err(|_| AppError::Internal)?;
        Ok((profile.config, permit))
    }
}

#[derive(Clone, Copy)]
struct ScopeBudget {
    remaining_instructions: u64,
    remaining_alloc_bytes: u64,
}
pub(crate) struct Budget {
    request_instructions: u64,
    request_alloc_bytes: u64,
    scopes: Vec<ScopeBudget>,
}
impl Budget {
    pub(crate) fn new(request: &ExecutionLimits, default: ResourceProfileConfig) -> Self {
        Self {
            request_instructions: request.max_instructions,
            request_alloc_bytes: request.max_allocated_bytes,
            scopes: vec![ScopeBudget {
                remaining_instructions: default.max_instructions,
                remaining_alloc_bytes: default.max_allocated_bytes,
            }],
        }
    }
    #[cfg(test)]
    pub(crate) fn remaining_request_instructions(&self) -> u64 {
        self.request_instructions
    }

    pub(crate) fn push_profile(&mut self, c: ResourceProfileConfig) {
        self.scopes.push(ScopeBudget {
            remaining_instructions: c.max_instructions,
            remaining_alloc_bytes: c.max_allocated_bytes,
        });
    }
    pub(crate) fn pop_profile(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }
    pub(crate) fn charge(&mut self, n: u64) -> Result<(), AppError> {
        self.request_instructions = self
            .request_instructions
            .checked_sub(n)
            .ok_or(AppError::InstructionLimit)?;
        let scope = self.scopes.last_mut().ok_or(AppError::Internal)?;
        scope.remaining_instructions = scope
            .remaining_instructions
            .checked_sub(n)
            .ok_or(AppError::InstructionLimit)?;
        Ok(())
    }
    pub(crate) fn charge_alloc(&mut self, n: u64) -> Result<(), AppError> {
        self.request_alloc_bytes = self
            .request_alloc_bytes
            .checked_sub(n)
            .ok_or(AppError::MemoryLimit)?;
        let scope = self.scopes.last_mut().ok_or(AppError::Internal)?;
        scope.remaining_alloc_bytes = scope
            .remaining_alloc_bytes
            .checked_sub(n)
            .ok_or(AppError::MemoryLimit)?;
        Ok(())
    }
    pub(crate) fn charge_value(&mut self, v: &Value) -> Result<(), AppError> {
        self.charge_alloc(memory::estimate_value_bytes(v))
    }
}
