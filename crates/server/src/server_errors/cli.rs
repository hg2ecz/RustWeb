use super::policy_config::{RatePolicyConfigError, ResourceProfileConfigError};
use super::{
    PublicHostError, SecretFileError, ServerConfigError, StaticPrefixError, TlsConfigError,
};
use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub(crate) enum CliValueError {
    MissingValue {
        flag: String,
    },
    InvalidNumber {
        flag: String,
        source: std::num::ParseIntError,
    },
    MustBePositive {
        flag: String,
    },
}

impl fmt::Display for CliValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingValue { flag } => write!(f, "{flag} requires a number"),
            Self::InvalidNumber { flag, source } => {
                write!(f, "{flag} requires a valid number: {source}")
            }
            Self::MustBePositive { flag } => write!(f, "{flag} must be greater than zero"),
        }
    }
}

impl Error for CliValueError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidNumber { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ReservedPathError {
    value: String,
}

impl ReservedPathError {
    pub(crate) fn new(value: &str) -> Self {
        Self {
            value: value.to_string(),
        }
    }
}

impl fmt::Display for ReservedPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid reserved endpoint path `{}`", self.value)
    }
}
impl Error for ReservedPathError {}

#[derive(Debug)]
pub(crate) enum CliParseError {
    Invalid(String),
    ServerConfig(ServerConfigError),
    SecretFile(SecretFileError),
    PublicHost(PublicHostError),
    StaticPrefix(StaticPrefixError),
    ReservedPath(ReservedPathError),
    Value(CliValueError),
    RatePolicy(RatePolicyConfigError),
    ResourceProfile(ResourceProfileConfigError),
    Compile(compiler::CompileError),
    Tls(TlsConfigError),
    Io(io::Error),
    Address(std::net::AddrParseError),
    IntegerConversion(std::num::TryFromIntError),
}

impl CliParseError {
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }
}

impl fmt::Display for CliParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::ServerConfig(source) => write!(f, "{source}"),
            Self::SecretFile(source) => write!(f, "{source}"),
            Self::PublicHost(source) => write!(f, "{source}"),
            Self::StaticPrefix(source) => write!(f, "{source}"),
            Self::ReservedPath(source) => write!(f, "{source}"),
            Self::Value(source) => write!(f, "{source}"),
            Self::RatePolicy(source) => write!(f, "{source}"),
            Self::ResourceProfile(source) => write!(f, "{source}"),
            Self::Compile(source) => write!(f, "{source}"),
            Self::Tls(source) => write!(f, "{source}"),
            Self::Io(source) => write!(f, "I/O error: {source}"),
            Self::Address(source) => write!(f, "invalid network address: {source}"),
            Self::IntegerConversion(source) => write!(f, "integer value out of range: {source}"),
        }
    }
}

impl Error for CliParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ServerConfig(source) => Some(source),
            Self::SecretFile(source) => Some(source),
            Self::PublicHost(source) => Some(source),
            Self::StaticPrefix(source) => Some(source),
            Self::ReservedPath(source) => Some(source),
            Self::Value(source) => Some(source),
            Self::RatePolicy(source) => Some(source),
            Self::ResourceProfile(source) => Some(source),
            Self::Compile(source) => Some(source),
            Self::Tls(source) => Some(source),
            Self::Io(source) => Some(source),
            Self::Address(source) => Some(source),
            Self::IntegerConversion(source) => Some(source),
            Self::Invalid(_) => None,
        }
    }
}

impl From<&'static str> for CliParseError {
    fn from(value: &'static str) -> Self {
        Self::Invalid(value.to_string())
    }
}
impl From<String> for CliParseError {
    fn from(value: String) -> Self {
        Self::Invalid(value)
    }
}
impl From<ServerConfigError> for CliParseError {
    fn from(value: ServerConfigError) -> Self {
        Self::ServerConfig(value)
    }
}
impl From<SecretFileError> for CliParseError {
    fn from(value: SecretFileError) -> Self {
        Self::SecretFile(value)
    }
}
impl From<PublicHostError> for CliParseError {
    fn from(value: PublicHostError) -> Self {
        Self::PublicHost(value)
    }
}
impl From<StaticPrefixError> for CliParseError {
    fn from(value: StaticPrefixError) -> Self {
        Self::StaticPrefix(value)
    }
}
impl From<ReservedPathError> for CliParseError {
    fn from(value: ReservedPathError) -> Self {
        Self::ReservedPath(value)
    }
}
impl From<CliValueError> for CliParseError {
    fn from(value: CliValueError) -> Self {
        Self::Value(value)
    }
}
impl From<RatePolicyConfigError> for CliParseError {
    fn from(value: RatePolicyConfigError) -> Self {
        Self::RatePolicy(value)
    }
}
impl From<ResourceProfileConfigError> for CliParseError {
    fn from(value: ResourceProfileConfigError) -> Self {
        Self::ResourceProfile(value)
    }
}
impl From<compiler::CompileError> for CliParseError {
    fn from(value: compiler::CompileError) -> Self {
        Self::Compile(value)
    }
}
impl From<TlsConfigError> for CliParseError {
    fn from(value: TlsConfigError) -> Self {
        Self::Tls(value)
    }
}
impl From<io::Error> for CliParseError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<std::net::AddrParseError> for CliParseError {
    fn from(value: std::net::AddrParseError) -> Self {
        Self::Address(value)
    }
}
impl From<std::num::TryFromIntError> for CliParseError {
    fn from(value: std::num::TryFromIntError) -> Self {
        Self::IntegerConversion(value)
    }
}
