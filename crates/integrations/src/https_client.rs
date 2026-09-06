use crate::egress::{EgressPolicy, Target, canonical_host, ip_allowed};
use crate::error::IntegrationError;
use crate::secrets::SecretString;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, lookup_host};
use tokio_rustls::TlsConnector;

const MAX_HEADER_BYTES: usize = 32 * 1024;
const MAX_HEADER_COUNT: usize = 128;

#[derive(Debug, Clone)]
pub struct HttpsResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Clone)]
pub struct OutboundHttpsClient {
    policy: EgressPolicy,
    tls: TlsConnector,
}

impl OutboundHttpsClient {
    pub fn new(policy: EgressPolicy) -> Self {
        let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let cfg = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        Self {
            policy,
            tls: TlsConnector::from(Arc::new(cfg)),
        }
    }

    pub async fn get(
        &self,
        target: &str,
        host: &str,
        port: u16,
        path_and_query: &str,
    ) -> Result<HttpsResponse, IntegrationError> {
        self.request(target, host, port, "GET", path_and_query, &[], &[])
            .await
    }

    pub async fn post_json(
        &self,
        target: &str,
        host: &str,
        port: u16,
        path_and_query: &str,
        body: &[u8],
        bearer: Option<&SecretString>,
    ) -> Result<HttpsResponse, IntegrationError> {
        let mut headers = vec![("Content-Type", "application/json")];
        let bearer_header;
        if let Some(secret) = bearer {
            let token = std::str::from_utf8(secret.bytes())
                .map_err(|_| IntegrationError::Secret("bearer secret is not UTF-8".into()))?;
            if token.bytes().any(|b| matches!(b, b'\r' | b'\n' | 0)) {
                return Err(IntegrationError::Secret("invalid bearer secret".into()));
            }
            bearer_header = format!("Bearer {token}");
            headers.push(("Authorization", bearer_header.as_str()));
        }
        self.request(target, host, port, "POST", path_and_query, body, &headers)
            .await
    }

    async fn request(
        &self,
        target_name: &str,
        host: &str,
        port: u16,
        method: &str,
        path: &str,
        body: &[u8],
        headers: &[(&str, &str)],
    ) -> Result<HttpsResponse, IntegrationError> {
        let target = self.policy.target(target_name)?.clone();
        let host = canonical_host(host)?;
        if !target.hosts.iter().any(|h| h == &host) {
            return Err(IntegrationError::Policy(
                "host is not allowed by target".into(),
            ));
        }
        if !target.ports.contains(&port) {
            return Err(IntegrationError::Policy(
                "port is not allowed by target".into(),
            ));
        }
        if !target.tls_required {
            return Err(IntegrationError::Policy("TLS is required".into()));
        }
        validate_request_path(path)?;
        if body.len() > target.max_sent_bytes {
            return Err(IntegrationError::SendTooLarge);
        }
        for (key, value) in headers {
            validate_header(key, value)?;
        }
        tokio::time::timeout(
            target.total_timeout,
            self.request_inner(&target, &host, port, method, path, body, headers),
        )
        .await
        .map_err(|_| IntegrationError::Timeout)?
    }

    async fn request_inner(
        &self,
        target: &Target,
        host: &str,
        port: u16,
        method: &str,
        path: &str,
        body: &[u8],
        headers: &[(&str, &str)],
    ) -> Result<HttpsResponse, IntegrationError> {
        let answers: Vec<SocketAddr> = lookup_host((host, port))
            .await
            .map_err(|_| IntegrationError::Dns)?
            .collect();
        if answers.is_empty() || answers.len() > target.max_dns_answers {
            return Err(IntegrationError::Policy("DNS answer count denied".into()));
        }
        let mut approved = Vec::new();
        for addr in answers {
            if !ip_allowed(&target.cidrs, addr.ip()) {
                return Err(IntegrationError::Policy(
                    "DNS answer outside target CIDR".into(),
                ));
            }
            if !approved.contains(&addr) {
                approved.push(addr);
            }
        }
        let mut last_connect_err = None;
        for addr in approved {
            match tokio::time::timeout(target.connect_timeout, TcpStream::connect(addr)).await {
                Ok(Ok(stream)) => {
                    let peer = stream.peer_addr().map_err(|_| IntegrationError::Connect)?;
                    if peer.ip() != addr.ip() || !ip_allowed(&target.cidrs, peer.ip()) {
                        return Err(IntegrationError::Policy("connected peer IP denied".into()));
                    }
                    return self
                        .tls_http(target, host, port, stream, method, path, body, headers)
                        .await;
                }
                _ => last_connect_err = Some(IntegrationError::Connect),
            }
        }
        Err(last_connect_err.unwrap_or(IntegrationError::Connect))
    }

    async fn tls_http(
        &self,
        target: &Target,
        host: &str,
        port: u16,
        stream: TcpStream,
        method: &str,
        path: &str,
        body: &[u8],
        headers: &[(&str, &str)],
    ) -> Result<HttpsResponse, IntegrationError> {
        let server_name =
            ServerName::try_from(host.to_owned()).map_err(|_| IntegrationError::Tls)?;
        let mut stream = self
            .tls
            .connect(server_name, stream)
            .await
            .map_err(|_| IntegrationError::Tls)?;
        let mut request = Vec::new();
        request.extend_from_slice(
            format!(
                "{method} {path} HTTP/1.1\r\nHost: {}\r\nUser-Agent: rwlang-m14/0.1\r\nAccept: application/json\r\nConnection: close\r\n",
                host_header(host, port)
            )
            .as_bytes(),
        );
        for (key, value) in headers {
            request.extend_from_slice(format!("{key}: {value}\r\n").as_bytes());
        }
        if !body.is_empty() || method == "POST" {
            request.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
        }
        request.extend_from_slice(b"\r\n");
        request.extend_from_slice(body);
        if request.len() > target.max_sent_bytes {
            return Err(IntegrationError::SendTooLarge);
        }
        stream
            .write_all(&request)
            .await
            .map_err(|_| IntegrationError::Connect)?;
        stream
            .flush()
            .await
            .map_err(|_| IntegrationError::Connect)?;
        read_http_response(&mut stream, target.max_received_bytes).await
    }
}

fn host_header(host: &str, port: u16) -> String {
    if port == 443 {
        host.to_string()
    } else {
        format!("{host}:{port}")
    }
}

fn validate_request_path(v: &str) -> Result<(), IntegrationError> {
    if !v.starts_with('/')
        || v.starts_with("//")
        || v.bytes().any(|b| matches!(b, b'\r' | b'\n' | 0 | b' '))
    {
        return Err(IntegrationError::Policy("invalid request path".into()));
    }
    Ok(())
}

fn validate_header(k: &str, v: &str) -> Result<(), IntegrationError> {
    if k.is_empty()
        || !k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        || v.bytes().any(|b| matches!(b, b'\r' | b'\n' | 0))
    {
        return Err(IntegrationError::Policy("invalid HTTP header".into()));
    }
    Ok(())
}

async fn read_http_response<S: tokio::io::AsyncRead + Unpin>(
    stream: &mut S,
    max_body: usize,
) -> Result<HttpsResponse, IntegrationError> {
    let mut data = Vec::new();
    let mut buf = [0u8; 4096];
    let head_end;
    loop {
        let n = stream
            .read(&mut buf)
            .await
            .map_err(|_| IntegrationError::Protocol)?;
        if n == 0 {
            return Err(IntegrationError::Protocol);
        }
        data.extend_from_slice(&buf[..n]);
        if data.len() > MAX_HEADER_BYTES {
            return Err(IntegrationError::Protocol);
        }
        if let Some(pos) = find_double_crlf(&data) {
            head_end = pos + 4;
            break;
        }
    }
    let head = std::str::from_utf8(&data[..head_end]).map_err(|_| IntegrationError::Protocol)?;
    let mut lines = head[..head.len() - 4].split("\r\n");
    let status_line = lines.next().ok_or(IntegrationError::Protocol)?;
    let mut parts = status_line.split_whitespace();
    if parts.next() != Some("HTTP/1.1") {
        return Err(IntegrationError::Protocol);
    }
    let status: u16 = parts
        .next()
        .ok_or(IntegrationError::Protocol)?
        .parse()
        .map_err(|_| IntegrationError::Protocol)?;
    let mut headers = HashMap::new();
    let mut content_length = None;
    let mut count = 0usize;
    for line in lines {
        count += 1;
        if count > MAX_HEADER_COUNT {
            return Err(IntegrationError::Protocol);
        }
        let (name, value) = line.split_once(':').ok_or(IntegrationError::Protocol)?;
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        validate_header(&name, value)?;
        if headers.contains_key(&name) {
            return Err(IntegrationError::Protocol);
        }
        if name == "transfer-encoding" {
            return Err(IntegrationError::Protocol);
        }
        if name == "content-length" {
            content_length = Some(
                value
                    .parse::<usize>()
                    .map_err(|_| IntegrationError::Protocol)?,
            );
        }
        headers.insert(name, value.to_string());
    }
    let expected = content_length.unwrap_or(0);
    if expected > max_body {
        return Err(IntegrationError::ResponseTooLarge);
    }
    let mut body = data[head_end..].to_vec();
    if body.len() > expected {
        return Err(IntegrationError::Protocol);
    }
    while body.len() < expected {
        let want = (expected - body.len()).min(buf.len());
        let n = stream
            .read(&mut buf[..want])
            .await
            .map_err(|_| IntegrationError::Protocol)?;
        if n == 0 {
            return Err(IntegrationError::Protocol);
        }
        body.extend_from_slice(&buf[..n]);
        if body.len() > max_body {
            return Err(IntegrationError::ResponseTooLarge);
        }
    }
    Ok(HttpsResponse {
        status,
        headers,
        body,
    })
}

fn find_double_crlf(v: &[u8]) -> Option<usize> {
    v.windows(4).position(|w| w == b"\r\n\r\n")
}

#[cfg(test)]
mod tests {
    use super::validate_request_path;

    #[test]
    fn request_path_rejects_injection() {
        assert!(validate_request_path("/v1/a?x=1").is_ok());
        assert!(validate_request_path("//evil").is_err());
        assert!(validate_request_path("/x\r\nX:y").is_err());
    }
}
