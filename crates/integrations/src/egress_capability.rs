use crate::egress::{Target, canonical_host};
use crate::error::IntegrationError;

#[derive(Clone)]
pub struct EgressCapability {
    pub(crate) target: Target,
}

impl EgressCapability {
    pub(crate) fn new(target: Target) -> Self {
        Self { target }
    }

    pub fn endpoint(&self, host: &str, port: u16) -> Result<EgressEndpoint, IntegrationError> {
        let host = canonical_host(host)?;
        if !self.target.hosts.iter().any(|allowed| allowed == &host) {
            return Err(IntegrationError::Policy(
                "host is not allowed by egress capability".into(),
            ));
        }
        if !self.target.ports.contains(&port) {
            return Err(IntegrationError::Policy(
                "port is not allowed by egress capability".into(),
            ));
        }
        if !self.target.tls_required {
            return Err(IntegrationError::Policy("TLS is required".into()));
        }
        Ok(EgressEndpoint {
            target: self.target.clone(),
            host,
            port,
        })
    }
}

#[derive(Clone)]
pub struct EgressEndpoint {
    pub(crate) target: Target,
    pub(crate) host: String,
    pub(crate) port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpsPath(String);

impl HttpsPath {
    pub fn new(value: impl Into<String>) -> Result<Self, IntegrationError> {
        let value = value.into();
        validate_path(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn validate_path(value: &str) -> Result<(), IntegrationError> {
    if !value.starts_with('/')
        || value.starts_with("//")
        || value
            .bytes()
            .any(|byte| matches!(byte, b'\r' | b'\n' | 0 | b' '))
    {
        return Err(IntegrationError::Policy(
            "invalid HTTPS request path".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::HttpsPath;

    #[test]
    fn https_path_is_rooted_and_header_safe() {
        assert!(HttpsPath::new("/v1/items?limit=10").is_ok());
        assert!(HttpsPath::new("//metadata").is_err());
        assert!(HttpsPath::new("https://evil.example/").is_err());
        assert!(HttpsPath::new("/x\r\nInjected: yes").is_err());
    }
}
