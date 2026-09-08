use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObsError {
    #[error("serialization failed")]
    Serialization,
}
