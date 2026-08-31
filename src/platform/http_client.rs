//! Deployment-bound outbound HTTP/1.1 with explicit destination, trust, and resource policy.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use super::http::HttpHeader;
use super::kernel::{Name, StructuralTypeField, TypeForm, TypeObjectDigest, TypeObjectInterner};
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::Cursor;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::runtime::Handle;
use tokio_rustls::TlsConnector;

pub const HTTP_CLIENT_ADAPTER_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_HTTP_CLIENT_ENDPOINT_BYTES: usize = 4_096;
pub const MAXIMUM_HTTP_CLIENT_REQUEST_HEADERS: usize = 256;
pub const MAXIMUM_HTTP_CLIENT_REQUEST_HEADER_BYTES: usize = 64 * 1_024;
pub const MAXIMUM_HTTP_CLIENT_RESPONSE_HEADERS: usize = 1_024;
pub const MAXIMUM_HTTP_CLIENT_RESPONSE_HEADER_BYTES: usize = 256 * 1_024;
pub const MAXIMUM_HTTP_CLIENT_RESPONSE_BODY_BYTES: usize = 64 * 1_024 * 1_024;
pub const MAXIMUM_HTTP_CLIENT_DNS_RESULTS: usize = 64;
pub const MAXIMUM_HTTP_CLIENT_CONCURRENT_REQUESTS: usize = 1_024;
pub const MAXIMUM_HTTP_CLIENT_MILLISECONDS: u64 = 3_600_000;
pub const MAXIMUM_HTTP_CLIENT_ROOT_CERTIFICATES: usize = 16;

const CONTROL_POLL_MILLISECONDS: u64 = 5;
const MAXIMUM_CHUNK_LINE_BYTES: usize = 8 * 1_024;

const fn http_client_adapter_contract_version() -> u16 {
    HTTP_CLIENT_ADAPTER_CONTRACT_VERSION
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HttpClientAddressPolicy {
    PublicOnly,
    LoopbackOnly,
}

impl HttpClientAddressPolicy {
    pub const fn name(self) -> &'static str {
        match self {
            Self::PublicOnly => "public_only",
            Self::LoopbackOnly => "loopback_only",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "snake_case")]
pub enum HttpClientTrust {
    WebpkiRoots,
    NamedPemRoot { secret: String },
}

impl HttpClientTrust {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::WebpkiRoots => "webpki_roots",
            Self::NamedPemRoot { .. } => "named_pem_root",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HttpClientLimits {
    #[serde(skip, default = "http_client_adapter_contract_version")]
    pub contract_version: u16,
    pub maximum_request_headers: usize,
    pub maximum_request_header_bytes: usize,
    pub maximum_response_headers: usize,
    pub maximum_response_header_bytes: usize,
    pub maximum_response_body_bytes: usize,
    pub maximum_dns_results: usize,
    pub maximum_concurrent_requests: usize,
    pub connection_timeout_milliseconds: u64,
    pub total_timeout_milliseconds: u64,
    pub cleanup_timeout_milliseconds: u64,
}

impl Default for HttpClientLimits {
    fn default() -> Self {
        Self {
            contract_version: HTTP_CLIENT_ADAPTER_CONTRACT_VERSION,
            maximum_request_headers: 16,
            maximum_request_header_bytes: 8 * 1_024,
            maximum_response_headers: 128,
            maximum_response_header_bytes: 32 * 1_024,
            maximum_response_body_bytes: 1024 * 1024,
            maximum_dns_results: 8,
            maximum_concurrent_requests: 16,
            connection_timeout_milliseconds: 5_000,
            total_timeout_milliseconds: 10_000,
            cleanup_timeout_milliseconds: 5_000,
        }
    }
}

impl HttpClientLimits {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != HTTP_CLIENT_ADAPTER_CONTRACT_VERSION {
            return Err(http_client_diagnostic(
                "http_client_contract",
                "HTTP client limits use a predecessor or foreign contract",
            ));
        }
        validate_positive_limit(
            "maximum_request_headers",
            self.maximum_request_headers,
            MAXIMUM_HTTP_CLIENT_REQUEST_HEADERS,
        )?;
        validate_positive_limit(
            "maximum_request_header_bytes",
            self.maximum_request_header_bytes,
            MAXIMUM_HTTP_CLIENT_REQUEST_HEADER_BYTES,
        )?;
        validate_positive_limit(
            "maximum_response_headers",
            self.maximum_response_headers,
            MAXIMUM_HTTP_CLIENT_RESPONSE_HEADERS,
        )?;
        validate_positive_limit(
            "maximum_response_header_bytes",
            self.maximum_response_header_bytes,
            MAXIMUM_HTTP_CLIENT_RESPONSE_HEADER_BYTES,
        )?;
        validate_positive_limit(
            "maximum_response_body_bytes",
            self.maximum_response_body_bytes,
            MAXIMUM_HTTP_CLIENT_RESPONSE_BODY_BYTES,
        )?;
        validate_positive_limit(
            "maximum_dns_results",
            self.maximum_dns_results,
            MAXIMUM_HTTP_CLIENT_DNS_RESULTS,
        )?;
        validate_positive_limit(
            "maximum_concurrent_requests",
            self.maximum_concurrent_requests,
            MAXIMUM_HTTP_CLIENT_CONCURRENT_REQUESTS,
        )?;
        for (name, value) in [
            (
                "connection_timeout_milliseconds",
                self.connection_timeout_milliseconds,
            ),
            (
                "total_timeout_milliseconds",
                self.total_timeout_milliseconds,
            ),
            (
                "cleanup_timeout_milliseconds",
                self.cleanup_timeout_milliseconds,
            ),
        ] {
            if value == 0 || value > MAXIMUM_HTTP_CLIENT_MILLISECONDS {
                return Err(http_client_diagnostic(
                    "http_client_limit",
                    format!(
                        "{name} must be 1 through {MAXIMUM_HTTP_CLIENT_MILLISECONDS} milliseconds"
                    ),
                ));
            }
        }
        if self.connection_timeout_milliseconds > self.total_timeout_milliseconds {
            return Err(http_client_diagnostic(
                "http_client_limit",
                "HTTP client connection timeout may not exceed its total timeout",
            ));
        }
        Ok(())
    }
}

fn validate_positive_limit(name: &str, value: usize, maximum: usize) -> Result<(), Diagnostic> {
    if value == 0 || value > maximum {
        return Err(http_client_diagnostic(
            "http_client_limit",
            format!("{name} must be 1 through {maximum}"),
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct HttpClientSemanticTypes {
    pub(crate) i64_type: TypeObjectDigest,
    pub(crate) bytes_type: TypeObjectDigest,
    pub(crate) text_type: TypeObjectDigest,
    pub(crate) header_type: TypeObjectDigest,
    pub(crate) header_list_type: TypeObjectDigest,
    pub(crate) response_type: TypeObjectDigest,
}

pub(crate) fn semantic_http_client_types(
    types: &mut TypeObjectInterner,
) -> Result<HttpClientSemanticTypes, Diagnostic> {
    let i64_type = types.intern(TypeForm::I64)?;
    let bytes_type = types.intern(TypeForm::Bytes)?;
    let text_type = types.intern(TypeForm::Text)?;
    let header_type = types.intern(TypeForm::StructuralRecord {
        fields: vec![
            structural_field("name", text_type)?,
            structural_field("value", bytes_type)?,
        ],
    })?;
    let header_list_type = types.intern(TypeForm::List { item: header_type })?;
    let response_type = types.intern(TypeForm::StructuralRecord {
        fields: vec![
            structural_field("body", bytes_type)?,
            structural_field("headers", header_list_type)?,
            structural_field("status", i64_type)?,
        ],
    })?;
    Ok(HttpClientSemanticTypes {
        i64_type,
        bytes_type,
        text_type,
        header_type,
        header_list_type,
        response_type,
    })
}

fn structural_field(name: &str, ty: TypeObjectDigest) -> Result<StructuralTypeField, Diagnostic> {
    Ok(StructuralTypeField {
        name: Name::new(name)?,
        ty,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NormalizedRelayUrl {
    pub(crate) endpoint: String,
    pub(crate) address_policy: HttpClientAddressPolicy,
}

pub(crate) fn normalize_nostr_relay_url(value: &str) -> Result<NormalizedRelayUrl, Diagnostic> {
    let (source_scheme, replacement) = if value.starts_with("wss://") {
        ("wss", "https")
    } else if value.starts_with("ws://") {
        ("ws", "http")
    } else if value.starts_with("https://") {
        ("https", "https")
    } else if value.starts_with("http://") {
        ("http", "http")
    } else {
        return Err(http_client_diagnostic(
            "http_client_endpoint",
            "relay URL must use exact lowercase wss, https, ws, or http scheme",
        ));
    };
    let suffix = value.get(source_scheme.len()..).ok_or_else(|| {
        http_client_diagnostic(
            "http_client_endpoint",
            "relay URL scheme boundary is invalid",
        )
    })?;
    let endpoint = format!("{replacement}{suffix}");
    let parsed = HttpClientEndpoint::parse(&endpoint)?;
    let address_policy = if parsed.host.is_lexical_loopback() {
        HttpClientAddressPolicy::LoopbackOnly
    } else {
        HttpClientAddressPolicy::PublicOnly
    };
    if replacement == "http" && address_policy != HttpClientAddressPolicy::LoopbackOnly {
        return Err(http_client_diagnostic(
            "http_client_endpoint",
            "plaintext relay URL is permitted only for an exact loopback destination",
        ));
    }
    parsed.validate_policy(address_policy)?;
    Ok(NormalizedRelayUrl {
        endpoint: parsed.render(),
        address_policy,
    })
}

pub(crate) fn validate_http_client_descriptor(
    endpoint: &str,
    policy: HttpClientAddressPolicy,
    limits: &HttpClientLimits,
) -> Result<(), Diagnostic> {
    limits.validate()?;
    HttpClientEndpoint::parse(endpoint)?.validate_policy(policy)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HttpClientScheme {
    Http,
    Https,
}

impl HttpClientScheme {
    const fn name(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
        }
    }

    const fn default_port(self) -> u16 {
        match self {
            Self::Http => 80,
            Self::Https => 443,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum HttpClientHost {
    Dns(String),
    Ip(IpAddr),
}

impl HttpClientHost {
    fn is_lexical_loopback(&self) -> bool {
        match self {
            Self::Dns(value) => value == "localhost",
            Self::Ip(value) => normalized_ip(*value).is_loopback(),
        }
    }

    fn render(&self) -> String {
        match self {
            Self::Dns(value) => value.clone(),
            Self::Ip(IpAddr::V4(value)) => value.to_string(),
            Self::Ip(IpAddr::V6(value)) => format!("[{value}]"),
        }
    }

    fn server_name(&self) -> Result<ServerName<'static>, ExecutionError> {
        match self {
            Self::Dns(value) => ServerName::try_from(value.clone()).map_err(|_| {
                capability_error(
                    "http_client_tls",
                    "HTTP client endpoint hostname is not valid for TLS verification",
                )
            }),
            Self::Ip(value) => Ok(ServerName::IpAddress((*value).into())),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HttpClientEndpoint {
    scheme: HttpClientScheme,
    host: HttpClientHost,
    port: u16,
    explicit_port: bool,
    path: String,
}

impl HttpClientEndpoint {
    fn parse(value: &str) -> Result<Self, Diagnostic> {
        if value.is_empty()
            || value.len() > MAXIMUM_HTTP_CLIENT_ENDPOINT_BYTES
            || !value.is_ascii()
            || value
                .bytes()
                .any(|byte| byte.is_ascii_control() || matches!(byte, b' ' | b'\\'))
        {
            return Err(http_client_diagnostic(
                "http_client_endpoint",
                "HTTP client endpoint is empty, excessive, non-ASCII, or contains whitespace/control/backslash bytes",
            ));
        }
        let (scheme, remainder) = if let Some(remainder) = value.strip_prefix("https://") {
            (HttpClientScheme::Https, remainder)
        } else if let Some(remainder) = value.strip_prefix("http://") {
            (HttpClientScheme::Http, remainder)
        } else {
            return Err(http_client_diagnostic(
                "http_client_endpoint",
                "HTTP client endpoint must use exact lowercase http or https scheme",
            ));
        };
        if remainder.contains(['?', '#']) {
            return Err(http_client_diagnostic(
                "http_client_endpoint",
                "HTTP client endpoint may not contain a query or fragment",
            ));
        }
        let (authority, path) = match remainder.find('/') {
            Some(index) => (&remainder[..index], &remainder[index..]),
            None => (remainder, "/"),
        };
        if authority.is_empty() || path.is_empty() || !path.starts_with('/') {
            return Err(http_client_diagnostic(
                "http_client_endpoint",
                "HTTP client endpoint has an empty or ambiguous authority/path",
            ));
        }
        if authority.contains(['@', '%']) {
            return Err(http_client_diagnostic(
                "http_client_endpoint",
                "HTTP client authority may not contain user information or escapes",
            ));
        }
        validate_endpoint_path(path)?;
        let (host, port, explicit_port) = parse_authority(authority, scheme.default_port())?;
        Ok(Self {
            scheme,
            host,
            port,
            explicit_port,
            path: path.to_owned(),
        })
    }

    fn validate_policy(&self, policy: HttpClientAddressPolicy) -> Result<(), Diagnostic> {
        if self.scheme == HttpClientScheme::Http && policy != HttpClientAddressPolicy::LoopbackOnly
        {
            return Err(http_client_diagnostic(
                "http_client_endpoint",
                "plaintext HTTP requires the explicit loopback_only address policy",
            ));
        }
        Ok(())
    }

    fn authority(&self) -> String {
        let host = self.host.render();
        if self.explicit_port {
            format!("{host}:{}", self.port)
        } else {
            host
        }
    }

    fn render(&self) -> String {
        format!("{}://{}{}", self.scheme.name(), self.authority(), self.path)
    }
}

fn validate_endpoint_path(value: &str) -> Result<(), Diagnostic> {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            index = index.saturating_add(1);
            continue;
        }
        let Some(high) = bytes.get(index.saturating_add(1)).copied() else {
            return Err(http_client_diagnostic(
                "http_client_endpoint",
                "HTTP client path ends within a percent escape",
            ));
        };
        let Some(low) = bytes.get(index.saturating_add(2)).copied() else {
            return Err(http_client_diagnostic(
                "http_client_endpoint",
                "HTTP client path ends within a percent escape",
            ));
        };
        let decoded = uppercase_hex(high)
            .zip(uppercase_hex(low))
            .map(|(high, low)| high.saturating_mul(16).saturating_add(low))
            .ok_or_else(|| {
                http_client_diagnostic(
                    "http_client_endpoint",
                    "HTTP client path contains a noncanonical percent escape",
                )
            })?;
        if decoded.is_ascii_alphanumeric() || matches!(decoded, b'-' | b'.' | b'_' | b'~') {
            return Err(http_client_diagnostic(
                "http_client_endpoint",
                "HTTP client path percent-encodes an unreserved byte noncanonically",
            ));
        }
        index = index.saturating_add(3);
    }
    Ok(())
}

fn uppercase_hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn parse_authority(
    authority: &str,
    default_port: u16,
) -> Result<(HttpClientHost, u16, bool), Diagnostic> {
    if let Some(remainder) = authority.strip_prefix('[') {
        let close = remainder.find(']').ok_or_else(|| {
            http_client_diagnostic(
                "http_client_endpoint",
                "HTTP client IPv6 authority is missing its closing bracket",
            )
        })?;
        let address = &remainder[..close];
        let suffix = &remainder[close + 1..];
        let parsed = address.parse::<std::net::Ipv6Addr>().map_err(|_| {
            http_client_diagnostic(
                "http_client_endpoint",
                "HTTP client IPv6 authority is malformed",
            )
        })?;
        if parsed.to_string() != address {
            return Err(http_client_diagnostic(
                "http_client_endpoint",
                "HTTP client IPv6 authority is not canonical lowercase text",
            ));
        }
        let (port, explicit) = parse_port_suffix(suffix, default_port)?;
        return Ok((HttpClientHost::Ip(IpAddr::V6(parsed)), port, explicit));
    }
    if authority.contains(['[', ']']) {
        return Err(http_client_diagnostic(
            "http_client_endpoint",
            "HTTP client authority contains ambiguous address brackets",
        ));
    }
    let (host, port, explicit) = match authority.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => {
            let port = parse_port(port)?;
            (host, port, true)
        }
        Some(_) => {
            return Err(http_client_diagnostic(
                "http_client_endpoint",
                "HTTP client IPv6 addresses must use canonical brackets",
            ));
        }
        None => (authority, default_port, false),
    };
    if host.is_empty() {
        return Err(http_client_diagnostic(
            "http_client_endpoint",
            "HTTP client authority has an empty host",
        ));
    }
    if let Ok(address) = host.parse::<std::net::Ipv4Addr>() {
        if address.to_string() != host {
            return Err(http_client_diagnostic(
                "http_client_endpoint",
                "HTTP client IPv4 authority is not canonical text",
            ));
        }
        return Ok((HttpClientHost::Ip(IpAddr::V4(address)), port, explicit));
    }
    validate_dns_name(host)?;
    Ok((HttpClientHost::Dns(host.to_owned()), port, explicit))
}

fn parse_port_suffix(suffix: &str, default_port: u16) -> Result<(u16, bool), Diagnostic> {
    if suffix.is_empty() {
        return Ok((default_port, false));
    }
    let port = suffix.strip_prefix(':').ok_or_else(|| {
        http_client_diagnostic(
            "http_client_endpoint",
            "HTTP client authority has trailing bytes after an IPv6 address",
        )
    })?;
    Ok((parse_port(port)?, true))
}

fn parse_port(value: &str) -> Result<u16, Diagnostic> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(http_client_diagnostic(
            "http_client_endpoint",
            "HTTP client port is empty, malformed, or noncanonical",
        ));
    }
    value
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(|| {
            http_client_diagnostic(
                "http_client_endpoint",
                "HTTP client port is outside 1 through 65535",
            )
        })
}

fn validate_dns_name(value: &str) -> Result<(), Diagnostic> {
    if value.len() > 253
        || value.ends_with('.')
        || value.bytes().any(|byte| byte.is_ascii_uppercase())
        || value.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
    {
        return Err(http_client_diagnostic(
            "http_client_endpoint",
            "HTTP client host is not a bounded canonical lowercase ASCII DNS name",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HttpClientResponse {
    pub(crate) status: u16,
    pub(crate) headers: Vec<HttpHeader>,
    pub(crate) body: Vec<u8>,
}

trait HttpClientIo: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> HttpClientIo for T {}

#[derive(Clone)]
pub(crate) struct HttpClient {
    endpoint: HttpClientEndpoint,
    policy: HttpClientAddressPolicy,
    limits: HttpClientLimits,
    runtime: Handle,
    tls: Option<Arc<ClientConfig>>,
    active: Arc<AtomicUsize>,
    shutdown: Arc<AtomicBool>,
    resolver: HttpClientResolver,
}

#[derive(Clone)]
enum HttpClientResolver {
    System,
    #[cfg(test)]
    Scripted(Arc<Vec<IpAddr>>),
}

impl HttpClient {
    pub(crate) fn prepare(
        endpoint: &str,
        policy: HttpClientAddressPolicy,
        trust: &HttpClientTrust,
        named_root_pem: Option<&str>,
        limits: HttpClientLimits,
        runtime: Handle,
    ) -> Result<Self, Diagnostic> {
        limits.validate()?;
        let endpoint = HttpClientEndpoint::parse(endpoint)?;
        endpoint.validate_policy(policy)?;
        let tls = match endpoint.scheme {
            HttpClientScheme::Http => None,
            HttpClientScheme::Https => Some(Arc::new(tls_config(trust, named_root_pem)?)),
        };
        Ok(Self {
            endpoint,
            policy,
            limits,
            runtime,
            tls,
            active: Arc::new(AtomicUsize::new(0)),
            shutdown: Arc::new(AtomicBool::new(false)),
            resolver: HttpClientResolver::System,
        })
    }

    #[cfg(test)]
    fn with_scripted_resolver(mut self, addresses: Vec<IpAddr>) -> Self {
        self.resolver = HttpClientResolver::Scripted(Arc::new(addresses));
        self
    }

    pub(crate) fn get(
        &self,
        headers: Vec<HttpHeader>,
        control: &ExecutionControl,
    ) -> Result<HttpClientResponse, ExecutionError> {
        control.check()?;
        let guard = ActiveRequest::acquire(self)?;
        let request = build_request(&self.endpoint, headers, &self.limits)?;
        let total_deadline = Instant::now()
            .checked_add(Duration::from_millis(
                self.limits.total_timeout_milliseconds,
            ))
            .ok_or_else(|| {
                resource_error(
                    "http_client_deadline_overflow",
                    "HTTP client total deadline overflowed",
                )
            })?;
        let effective_deadline = control
            .deadline()
            .map_or(total_deadline, |deadline| deadline.min(total_deadline));
        let client = self.clone();
        let control = control.clone();
        let outcome = self.runtime.block_on(async move {
            tokio::select! {
                biased;
                failure = wait_for_cancellation(&control, effective_deadline) => Err(failure),
                result = client.get_async(request, effective_deadline) => result,
            }
        });
        drop(guard);
        outcome
    }

    async fn get_async(
        &self,
        request: Vec<u8>,
        total_deadline: Instant,
    ) -> Result<HttpClientResponse, ExecutionError> {
        let addresses = self.resolve(total_deadline).await?;
        let connection_deadline = Instant::now()
            .checked_add(Duration::from_millis(
                self.limits.connection_timeout_milliseconds,
            ))
            .map_or(total_deadline, |deadline| deadline.min(total_deadline));
        let mut stream = self.connect(&addresses, connection_deadline).await?;
        stream.write_all(&request).await.map_err(|_| {
            possible_visibility(
                "http_client_request_write",
                "HTTP client request transport failed after visibility became possible",
            )
        })?;
        stream.flush().await.map_err(|_| {
            possible_visibility(
                "http_client_request_write",
                "HTTP client request transport failed after visibility became possible",
            )
        })?;
        read_response(&mut stream, &self.limits).await
    }

    async fn resolve(&self, deadline: Instant) -> Result<Vec<SocketAddr>, ExecutionError> {
        let addresses = match &self.resolver {
            HttpClientResolver::System => match &self.endpoint.host {
                HttpClientHost::Ip(address) => vec![*address],
                HttpClientHost::Dns(host) => {
                    let remaining = remaining(deadline)?;
                    let resolved = tokio::time::timeout(
                        remaining,
                        tokio::net::lookup_host((host.as_str(), self.endpoint.port)),
                    )
                    .await
                    .map_err(|_| timeout_error())?
                    .map_err(|_| {
                        capability_error(
                            "http_client_dns",
                            "HTTP client endpoint resolution failed",
                        )
                    })?;
                    resolved
                        .map(|address| address.ip())
                        .take(self.limits.maximum_dns_results.saturating_add(1))
                        .collect()
                }
            },
            #[cfg(test)]
            HttpClientResolver::Scripted(addresses) => addresses.as_ref().clone(),
        };
        validate_resolved_addresses(
            addresses,
            self.endpoint.port,
            self.policy,
            self.limits.maximum_dns_results,
        )
    }

    async fn connect(
        &self,
        addresses: &[SocketAddr],
        deadline: Instant,
    ) -> Result<Box<dyn HttpClientIo>, ExecutionError> {
        let mut tls_failed = false;
        for address in addresses {
            let connect_remaining = remaining(deadline)?;
            let stream =
                match tokio::time::timeout(connect_remaining, TcpStream::connect(address)).await {
                    Ok(Ok(stream)) => stream,
                    Ok(Err(_)) => continue,
                    Err(_) => return Err(timeout_error()),
                };
            let _ = stream.set_nodelay(true);
            let Some(config) = &self.tls else {
                return Ok(Box::new(stream));
            };
            let server_name = self.endpoint.host.server_name()?;
            let connector = TlsConnector::from(Arc::clone(config));
            let tls_remaining = remaining(deadline)?;
            match tokio::time::timeout(tls_remaining, connector.connect(server_name, stream)).await
            {
                Ok(Ok(stream)) => return Ok(Box::new(stream)),
                Ok(Err(_)) => tls_failed = true,
                Err(_) => return Err(timeout_error()),
            }
        }
        if tls_failed {
            Err(capability_error(
                "http_client_tls",
                "HTTP client TLS authentication or handshake failed",
            ))
        } else {
            Err(capability_error(
                "http_client_connect",
                "HTTP client could not establish the bounded endpoint connection",
            ))
        }
    }

    pub(crate) fn shutdown(&self) -> Result<(), ExecutionError> {
        self.shutdown.store(true, Ordering::Release);
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(
                self.limits.cleanup_timeout_milliseconds,
            ))
            .unwrap_or_else(Instant::now);
        while self.active.load(Ordering::Acquire) != 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        if self.active.load(Ordering::Acquire) != 0 {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "http_client_cleanup",
                "HTTP client resources did not close within the configured cleanup bound",
            ));
        }
        Ok(())
    }
}

struct ActiveRequest {
    active: Arc<AtomicUsize>,
}

impl ActiveRequest {
    fn acquire(client: &HttpClient) -> Result<Self, ExecutionError> {
        if client.shutdown.load(Ordering::Acquire) {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Cancelled,
                "http_client_shutdown",
                "HTTP client adapter has stopped admission",
            ));
        }
        client
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < client.limits.maximum_concurrent_requests)
                    .then(|| active.saturating_add(1))
            })
            .map_err(|_| {
                resource_error(
                    "http_client_concurrency_limit",
                    "HTTP client concurrent request limit is exhausted",
                )
            })?;
        if client.shutdown.load(Ordering::Acquire) {
            client.active.fetch_sub(1, Ordering::AcqRel);
            return Err(ExecutionError::new(
                ExecutionFailureClass::Cancelled,
                "http_client_shutdown",
                "HTTP client adapter has stopped admission",
            ));
        }
        Ok(Self {
            active: Arc::clone(&client.active),
        })
    }
}

impl Drop for ActiveRequest {
    fn drop(&mut self) {
        self.active.fetch_sub(1, Ordering::AcqRel);
    }
}

fn tls_config(
    trust: &HttpClientTrust,
    named_root_pem: Option<&str>,
) -> Result<ClientConfig, Diagnostic> {
    let mut roots = RootCertStore::empty();
    match trust {
        HttpClientTrust::WebpkiRoots => {
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        }
        HttpClientTrust::NamedPemRoot { .. } => {
            let pem = named_root_pem.ok_or_else(|| {
                http_client_diagnostic(
                    "http_client_trust",
                    "HTTP client named PEM root is not bound to a deployment secret",
                )
            })?;
            let mut reader = Cursor::new(pem.as_bytes());
            let items = rustls_pemfile::read_all(&mut reader)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| {
                    http_client_diagnostic(
                        "http_client_trust",
                        "HTTP client named PEM root is malformed",
                    )
                })?;
            if items.is_empty() || items.len() > MAXIMUM_HTTP_CLIENT_ROOT_CERTIFICATES {
                return Err(http_client_diagnostic(
                    "http_client_trust",
                    format!(
                        "HTTP client named PEM root must contain 1 through {MAXIMUM_HTTP_CLIENT_ROOT_CERTIFICATES} certificates"
                    ),
                ));
            }
            for item in items {
                let rustls_pemfile::Item::X509Certificate(certificate) = item else {
                    return Err(http_client_diagnostic(
                        "http_client_trust",
                        "HTTP client named PEM root contains a non-certificate PEM item",
                    ));
                };
                roots.add(certificate).map_err(|_| {
                    http_client_diagnostic(
                        "http_client_trust",
                        "HTTP client named PEM root contains an invalid certificate",
                    )
                })?;
            }
        }
    }
    if roots.is_empty() {
        return Err(http_client_diagnostic(
            "http_client_trust",
            "HTTP client trust store is empty",
        ));
    }
    Ok(ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth())
}

fn build_request(
    endpoint: &HttpClientEndpoint,
    headers: Vec<HttpHeader>,
    limits: &HttpClientLimits,
) -> Result<Vec<u8>, ExecutionError> {
    let total_headers = headers.len().checked_add(3).ok_or_else(|| {
        resource_error(
            "http_client_request_header_limit",
            "HTTP client request header count overflowed",
        )
    })?;
    if total_headers > limits.maximum_request_headers {
        return Err(resource_error(
            "http_client_request_header_limit",
            "HTTP client request header count exceeds its deployment limit",
        ));
    }
    let mut request = Vec::new();
    request.extend_from_slice(b"GET ");
    request.extend_from_slice(endpoint.path.as_bytes());
    request.extend_from_slice(b" HTTP/1.1\r\nHost: ");
    request.extend_from_slice(endpoint.authority().as_bytes());
    request.extend_from_slice(b"\r\nAccept-Encoding: identity\r\nConnection: close\r\n");
    for header in headers {
        validate_request_header(&header)?;
        request.extend_from_slice(header.name.as_bytes());
        request.extend_from_slice(b": ");
        request.extend_from_slice(&header.value);
        request.extend_from_slice(b"\r\n");
    }
    request.extend_from_slice(b"\r\n");
    let header_bytes = request
        .len()
        .checked_sub(endpoint.path.len().saturating_add(14))
        .unwrap_or(request.len());
    if header_bytes > limits.maximum_request_header_bytes {
        return Err(resource_error(
            "http_client_request_header_limit",
            "HTTP client request headers exceed their deployment byte limit",
        ));
    }
    Ok(request)
}

fn validate_request_header(header: &HttpHeader) -> Result<(), ExecutionError> {
    if header.name.is_empty()
        || header.name.len() > 128
        || !header.name.bytes().all(is_header_name_byte)
        || header.name.bytes().any(|byte| byte.is_ascii_uppercase())
        || header.value.len() > MAXIMUM_HTTP_CLIENT_REQUEST_HEADER_BYTES
        || header
            .value
            .iter()
            .any(|byte| matches!(byte, b'\r' | b'\n' | 0))
    {
        return Err(capability_error(
            "http_client_header",
            "HTTP client request header is malformed or noncanonical",
        ));
    }
    if is_forbidden_request_header(&header.name) {
        return Err(capability_error(
            "http_client_header_forbidden",
            "HTTP client graph input names a transport- or credential-owned header",
        ));
    }
    Ok(())
}

fn is_header_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

fn is_forbidden_request_header(name: &str) -> bool {
    matches!(
        name,
        "authorization"
            | "connection"
            | "content-length"
            | "cookie"
            | "expect"
            | "host"
            | "keep-alive"
            | "proxy-authorization"
            | "proxy-connection"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "accept-encoding"
    )
}

async fn read_response(
    stream: &mut Box<dyn HttpClientIo>,
    limits: &HttpClientLimits,
) -> Result<HttpClientResponse, ExecutionError> {
    let mut bytes = Vec::new();
    let header_end = loop {
        if let Some(index) = find_bytes(&bytes, b"\r\n\r\n") {
            break index.saturating_add(4);
        }
        if bytes.len() >= limits.maximum_response_header_bytes {
            return Err(resource_error(
                "http_client_response_header_limit",
                "HTTP client response headers exceed their deployment byte limit",
            ));
        }
        let remaining = limits
            .maximum_response_header_bytes
            .saturating_sub(bytes.len());
        let mut chunk = vec![0_u8; remaining.min(8 * 1_024)];
        let read = stream.read(&mut chunk).await.map_err(|_| {
            capability_error(
                "http_client_protocol",
                "HTTP client response transport failed",
            )
        })?;
        if read == 0 {
            return Err(capability_error(
                "http_client_protocol",
                "HTTP client response ended before a complete header block",
            ));
        }
        bytes.extend_from_slice(&chunk[..read]);
    };
    if header_end > limits.maximum_response_header_bytes {
        return Err(resource_error(
            "http_client_response_header_limit",
            "HTTP client response headers exceed their deployment byte limit",
        ));
    }
    let mut parsed_headers = vec![httparse::EMPTY_HEADER; limits.maximum_response_headers];
    let mut parsed = httparse::Response::new(&mut parsed_headers);
    match parsed.parse(&bytes[..header_end]) {
        Ok(httparse::Status::Complete(consumed)) if consumed == header_end => {}
        Ok(_) => {
            return Err(capability_error(
                "http_client_protocol",
                "HTTP client response header block is incomplete or noncanonical",
            ));
        }
        Err(httparse::Error::TooManyHeaders) => {
            return Err(resource_error(
                "http_client_response_header_limit",
                "HTTP client response header count exceeds its deployment limit",
            ));
        }
        Err(_) => {
            return Err(capability_error(
                "http_client_protocol",
                "HTTP client response header block is malformed",
            ));
        }
    }
    if parsed.version != Some(1) {
        return Err(capability_error(
            "http_client_protocol",
            "HTTP client response is not HTTP/1.1",
        ));
    }
    let status = parsed.code.ok_or_else(|| {
        capability_error(
            "http_client_protocol",
            "HTTP client response omits its status code",
        )
    })?;
    if status < 200 {
        return Err(capability_error(
            "http_client_protocol",
            "HTTP client does not admit informational response sequences",
        ));
    }
    let mut headers = Vec::with_capacity(parsed.headers.len());
    let mut content_length = None;
    let mut transfer_encoding = None;
    for header in parsed.headers.iter() {
        let name = header.name.to_ascii_lowercase();
        if name == "content-length" {
            if content_length.is_some() {
                return Err(capability_error(
                    "http_client_protocol",
                    "HTTP client response repeats Content-Length",
                ));
            }
            content_length = Some(parse_content_length(header.value)?);
        } else if name == "transfer-encoding" {
            if transfer_encoding.is_some() {
                return Err(capability_error(
                    "http_client_protocol",
                    "HTTP client response repeats Transfer-Encoding",
                ));
            }
            transfer_encoding = Some(header.value.to_vec());
        }
        headers.push(HttpHeader {
            name,
            value: header.value.to_vec(),
        });
    }
    if content_length.is_some() && transfer_encoding.is_some() {
        return Err(capability_error(
            "http_client_protocol",
            "HTTP client response has ambiguous body framing",
        ));
    }
    let initial = bytes.split_off(header_end);
    let body = if let Some(encoding) = transfer_encoding {
        if !encoding.eq_ignore_ascii_case(b"chunked") {
            return Err(capability_error(
                "http_client_protocol",
                "HTTP client response uses unsupported transfer coding",
            ));
        }
        read_chunked_body(stream, initial, limits.maximum_response_body_bytes).await?
    } else if let Some(length) = content_length {
        read_sized_body(stream, initial, length, limits.maximum_response_body_bytes).await?
    } else if matches!(status, 204 | 304) {
        if !initial.is_empty() {
            return Err(capability_error(
                "http_client_protocol",
                "HTTP client bodyless response has trailing bytes",
            ));
        }
        Vec::new()
    } else {
        read_close_body(stream, initial, limits.maximum_response_body_bytes).await?
    };
    Ok(HttpClientResponse {
        status,
        headers,
        body,
    })
}

fn parse_content_length(value: &[u8]) -> Result<usize, ExecutionError> {
    if value.is_empty()
        || !value.iter().all(u8::is_ascii_digit)
        || (value.len() > 1 && value.starts_with(b"0"))
    {
        return Err(capability_error(
            "http_client_protocol",
            "HTTP client response Content-Length is noncanonical",
        ));
    }
    std::str::from_utf8(value)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .ok_or_else(|| {
            resource_error(
                "http_client_response_body_limit",
                "HTTP client response Content-Length is not representable",
            )
        })
}

async fn read_sized_body(
    stream: &mut Box<dyn HttpClientIo>,
    mut body: Vec<u8>,
    length: usize,
    maximum: usize,
) -> Result<Vec<u8>, ExecutionError> {
    if length > maximum {
        return Err(resource_error(
            "http_client_response_body_limit",
            "HTTP client response body exceeds its deployment byte limit",
        ));
    }
    if body.len() > length {
        return Err(capability_error(
            "http_client_protocol",
            "HTTP client response has bytes after its declared body",
        ));
    }
    while body.len() < length {
        let previous = body.len();
        body.resize(length.min(previous.saturating_add(8 * 1_024)), 0);
        let read = stream.read(&mut body[previous..]).await.map_err(|_| {
            capability_error(
                "http_client_protocol",
                "HTTP client response body transport failed",
            )
        })?;
        if read == 0 {
            return Err(capability_error(
                "http_client_protocol",
                "HTTP client response ended before its declared body length",
            ));
        }
        body.truncate(previous.saturating_add(read));
    }
    Ok(body)
}

async fn read_close_body(
    stream: &mut Box<dyn HttpClientIo>,
    mut body: Vec<u8>,
    maximum: usize,
) -> Result<Vec<u8>, ExecutionError> {
    if body.len() > maximum {
        return Err(resource_error(
            "http_client_response_body_limit",
            "HTTP client response body exceeds its deployment byte limit",
        ));
    }
    let mut chunk = [0_u8; 8 * 1_024];
    loop {
        let read = stream.read(&mut chunk).await.map_err(|_| {
            capability_error(
                "http_client_protocol",
                "HTTP client response body transport failed",
            )
        })?;
        if read == 0 {
            return Ok(body);
        }
        if body.len().saturating_add(read) > maximum {
            return Err(resource_error(
                "http_client_response_body_limit",
                "HTTP client response body exceeds its deployment byte limit",
            ));
        }
        body.extend_from_slice(&chunk[..read]);
    }
}

async fn read_chunked_body(
    stream: &mut Box<dyn HttpClientIo>,
    mut buffered: Vec<u8>,
    maximum: usize,
) -> Result<Vec<u8>, ExecutionError> {
    let mut body = Vec::new();
    loop {
        let line = read_line(stream, &mut buffered).await?;
        let size_text = line.split(|byte| *byte == b';').next().unwrap_or_default();
        if size_text.is_empty()
            || !size_text.iter().all(u8::is_ascii_hexdigit)
            || (size_text.len() > 1 && size_text.starts_with(b"0"))
        {
            return Err(capability_error(
                "http_client_protocol",
                "HTTP client chunk size is malformed or noncanonical",
            ));
        }
        let size = std::str::from_utf8(size_text)
            .ok()
            .and_then(|value| usize::from_str_radix(value, 16).ok())
            .ok_or_else(|| {
                resource_error(
                    "http_client_response_body_limit",
                    "HTTP client chunk size is not representable",
                )
            })?;
        if size == 0 {
            let trailer = read_line(stream, &mut buffered).await?;
            if !trailer.is_empty() || !buffered.is_empty() {
                return Err(capability_error(
                    "http_client_protocol",
                    "HTTP client response trailers or trailing bytes are unsupported",
                ));
            }
            return Ok(body);
        }
        if body.len().saturating_add(size) > maximum {
            return Err(resource_error(
                "http_client_response_body_limit",
                "HTTP client response body exceeds its deployment byte limit",
            ));
        }
        fill_buffer(stream, &mut buffered, size.saturating_add(2)).await?;
        if buffered.get(size..size.saturating_add(2)) != Some(b"\r\n") {
            return Err(capability_error(
                "http_client_protocol",
                "HTTP client chunk omits its canonical terminator",
            ));
        }
        body.extend_from_slice(&buffered[..size]);
        buffered.drain(..size.saturating_add(2));
    }
}

async fn read_line(
    stream: &mut Box<dyn HttpClientIo>,
    buffered: &mut Vec<u8>,
) -> Result<Vec<u8>, ExecutionError> {
    loop {
        if let Some(index) = find_bytes(buffered, b"\r\n") {
            let remainder = buffered.split_off(index.saturating_add(2));
            let mut line = std::mem::replace(buffered, remainder);
            line.truncate(index);
            return Ok(line);
        }
        if buffered.len() >= MAXIMUM_CHUNK_LINE_BYTES {
            return Err(resource_error(
                "http_client_response_header_limit",
                "HTTP client chunk line exceeds its fixed byte limit",
            ));
        }
        fill_buffer(stream, buffered, buffered.len().saturating_add(1)).await?;
    }
}

async fn fill_buffer(
    stream: &mut Box<dyn HttpClientIo>,
    buffered: &mut Vec<u8>,
    minimum: usize,
) -> Result<(), ExecutionError> {
    while buffered.len() < minimum {
        let mut chunk = [0_u8; 8 * 1_024];
        let read = stream.read(&mut chunk).await.map_err(|_| {
            capability_error(
                "http_client_protocol",
                "HTTP client response body transport failed",
            )
        })?;
        if read == 0 {
            return Err(capability_error(
                "http_client_protocol",
                "HTTP client response ended before its framed body",
            ));
        }
        buffered.extend_from_slice(&chunk[..read]);
    }
    Ok(())
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn validate_resolved_addresses(
    addresses: Vec<IpAddr>,
    port: u16,
    policy: HttpClientAddressPolicy,
    maximum: usize,
) -> Result<Vec<SocketAddr>, ExecutionError> {
    if addresses.is_empty() {
        return Err(capability_error(
            "http_client_dns",
            "HTTP client endpoint resolution returned no addresses",
        ));
    }
    if addresses.len() > maximum {
        return Err(resource_error(
            "http_client_dns_limit",
            "HTTP client endpoint resolution exceeds its result limit",
        ));
    }
    let mut selected = BTreeSet::new();
    for address in addresses {
        let address = normalized_ip(address);
        let admitted = match policy {
            HttpClientAddressPolicy::PublicOnly => is_public_address(address),
            HttpClientAddressPolicy::LoopbackOnly => address.is_loopback(),
        };
        if !admitted {
            return Err(capability_error(
                "http_client_destination",
                "HTTP client endpoint resolution includes a forbidden address class",
            ));
        }
        selected.insert(SocketAddr::new(address, port));
    }
    Ok(selected.into_iter().collect())
}

fn is_public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(value) => {
            let octets = value.octets();
            octets[0] != 0
                && octets[0] != 10
                && !(octets[0] == 100 && (octets[1] & 0xc0) == 0x40)
                && octets[0] != 127
                && !(octets[0] == 169 && octets[1] == 254)
                && !(octets[0] == 172 && (16..=31).contains(&octets[1]))
                && !(octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
                && !(octets[0] == 192 && octets[1] == 0 && octets[2] == 2)
                && !(octets[0] == 192 && octets[1] == 88 && octets[2] == 99)
                && !(octets[0] == 192 && octets[1] == 168)
                && !(octets[0] == 198 && (octets[1] == 18 || octets[1] == 19))
                && !(octets[0] == 198 && octets[1] == 51 && octets[2] == 100)
                && !(octets[0] == 203 && octets[1] == 0 && octets[2] == 113)
                && octets[0] < 224
        }
        IpAddr::V6(value) => {
            let segments = value.segments();
            (segments[0] & 0xe000) == 0x2000
                && !(segments[0] == 0x2001 && segments[1] < 0x0200)
                && !(segments[0] == 0x2001 && segments[1] == 0x0db8)
                && segments[0] != 0x2002
                && (segments[0] & 0xfff0) != 0x3ff0
        }
    }
}

fn normalized_ip(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V6(value) => value
            .to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(value)),
        value => value,
    }
}

async fn wait_for_cancellation(control: &ExecutionControl, deadline: Instant) -> ExecutionError {
    loop {
        if let Err(error) = control.check() {
            return ExecutionError::new(
                ExecutionFailureClass::Cancelled,
                "http_client_cancelled",
                if error.code == "execution_deadline" {
                    "HTTP client request exceeded its owning task deadline"
                } else {
                    "HTTP client request was cancelled by its owning task"
                },
            );
        }
        if Instant::now() >= deadline {
            return timeout_error();
        }
        tokio::time::sleep(Duration::from_millis(CONTROL_POLL_MILLISECONDS)).await;
    }
}

fn remaining(deadline: Instant) -> Result<Duration, ExecutionError> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or_else(timeout_error)
}

fn timeout_error() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Cancelled,
        "http_client_timeout",
        "HTTP client request exceeded its deployment time limit",
    )
}

/// Case-insensitive base media-type comparison with strict optional parameter syntax.
pub(crate) fn media_type_matches(value: &[u8], expected: &str) -> bool {
    if expected.is_empty()
        || !expected.is_ascii()
        || expected.bytes().any(|byte| byte.is_ascii_uppercase())
        || !valid_base_media_type(expected.as_bytes())
    {
        return false;
    }
    let mut parts = value.splitn(2, |byte| *byte == b';');
    let base = trim_ows(parts.next().unwrap_or_default());
    if !valid_base_media_type(base) || !base.eq_ignore_ascii_case(expected.as_bytes()) {
        return false;
    }
    parts.next().is_none_or(valid_media_parameters)
}

fn valid_base_media_type(value: &[u8]) -> bool {
    let mut parts = value.split(|byte| *byte == b'/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(left), Some(right), None)
            if !left.is_empty()
                && !right.is_empty()
                && left.iter().all(|byte| is_token_byte(*byte))
                && right.iter().all(|byte| is_token_byte(*byte))
    )
}

fn valid_media_parameters(mut value: &[u8]) -> bool {
    loop {
        value = trim_ows(value);
        if value.is_empty() {
            return false;
        }
        let Some(equals) = value.iter().position(|byte| *byte == b'=') else {
            return false;
        };
        let name = trim_ows(&value[..equals]);
        if name.is_empty() || !name.iter().all(|byte| is_token_byte(*byte)) {
            return false;
        }
        value = trim_ows(&value[equals + 1..]);
        if value.starts_with(b"\"") {
            let Some(consumed) = quoted_value_length(value) else {
                return false;
            };
            value = &value[consumed..];
        } else {
            let length = value
                .iter()
                .take_while(|byte| is_token_byte(**byte))
                .count();
            if length == 0 {
                return false;
            }
            value = &value[length..];
        }
        value = trim_ows(value);
        if value.is_empty() {
            return true;
        }
        let Some(remainder) = value.strip_prefix(b";") else {
            return false;
        };
        value = remainder;
    }
}

fn quoted_value_length(value: &[u8]) -> Option<usize> {
    let mut escaped = false;
    for (index, byte) in value.iter().copied().enumerate().skip(1) {
        if escaped {
            if !matches!(byte, b'\t' | b' '..=b'~') {
                return None;
            }
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == b'"' {
            return Some(index.saturating_add(1));
        } else if !matches!(byte, b'\t' | b' '..=b'!' | b'#'..=b'~') {
            return None;
        }
    }
    None
}

fn trim_ows(mut value: &[u8]) -> &[u8] {
    while value
        .first()
        .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
    {
        value = &value[1..];
    }
    while value
        .last()
        .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
    {
        value = &value[..value.len().saturating_sub(1)];
    }
    value
}

fn is_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

fn http_client_diagnostic(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Capability, code, message)
}

fn capability_error(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Capability, code, message)
}

fn resource_error(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::resource(code, message)
}

fn possible_visibility(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::PossibleVisibility, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_url_normalization_is_closed_and_loopback_plaintext_is_explicit() {
        assert_eq!(
            normalize_nostr_relay_url("wss://relay.example/path")
                .expect("public relay")
                .endpoint,
            "https://relay.example/path"
        );
        assert_eq!(
            normalize_nostr_relay_url("ws://127.0.0.1:8080/relay").expect("loopback relay"),
            NormalizedRelayUrl {
                endpoint: "http://127.0.0.1:8080/relay".to_owned(),
                address_policy: HttpClientAddressPolicy::LoopbackOnly,
            }
        );
        assert_eq!(
            normalize_nostr_relay_url("wss://relay.example/a@b/%E2%98%83/%2F")
                .expect("canonical escaped path")
                .endpoint,
            "https://relay.example/a@b/%E2%98%83/%2F"
        );
        for invalid in [
            "ws://relay.example/",
            "https://User@relay.example/",
            "https://relay.example/?query",
            "https://relay.example/#fragment",
            "https://Relay.example/",
            "https://relay.example/%2f",
            "https://relay.example/%41",
            "https://relay.example/%",
            "ftp://relay.example/",
        ] {
            assert!(normalize_nostr_relay_url(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn scripted_resolution_rejects_mixed_and_forbidden_address_classes() {
        let public = "93.184.216.34".parse().expect("public address");
        let private = "10.0.0.1".parse().expect("private address");
        let loopback = "::ffff:127.0.0.1".parse().expect("mapped loopback");
        assert!(
            validate_resolved_addresses(vec![public], 443, HttpClientAddressPolicy::PublicOnly, 4,)
                .is_ok()
        );
        assert_eq!(
            validate_resolved_addresses(
                vec![public, private],
                443,
                HttpClientAddressPolicy::PublicOnly,
                4,
            )
            .expect_err("mixed resolution must reject")
            .code,
            "http_client_destination"
        );
        assert_eq!(
            validate_resolved_addresses(
                vec![loopback],
                80,
                HttpClientAddressPolicy::LoopbackOnly,
                4,
            )
            .expect("mapped loopback"),
            vec!["127.0.0.1:80".parse().expect("socket")]
        );
    }

    #[test]
    fn prepared_client_uses_only_its_scripted_validated_resolution() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime");
        let client = HttpClient::prepare(
            "http://localhost:7447/nostr",
            HttpClientAddressPolicy::LoopbackOnly,
            &HttpClientTrust::WebpkiRoots,
            None,
            HttpClientLimits::default(),
            runtime.handle().clone(),
        )
        .expect("prepared loopback client")
        .with_scripted_resolver(vec![
            "127.0.0.1".parse().expect("loopback"),
            "203.0.113.1".parse().expect("documentation address"),
        ]);
        let deadline = Instant::now()
            .checked_add(Duration::from_secs(1))
            .expect("bounded deadline");
        let error = runtime
            .block_on(client.resolve(deadline))
            .expect_err("mixed scripted resolution must reject as a whole");
        assert_eq!(error.code, "http_client_destination");
    }

    #[test]
    fn public_address_policy_conservatively_rejects_every_supported_special_class() {
        for address in [
            "0.0.0.1",
            "10.0.0.1",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.1.1",
            "172.16.0.1",
            "192.0.0.1",
            "192.0.2.1",
            "192.88.99.1",
            "192.168.0.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "240.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "::ffff:127.0.0.1",
            "100::1",
            "2001::1",
            "2001:db8::1",
            "2002:0a00:1::1",
            "3fff::1",
            "fc00::1",
            "fe80::1",
            "ff00::1",
        ] {
            let address = address.parse().expect("special address fixture");
            assert!(!is_public_address(normalized_ip(address)), "{address}");
        }
        for address in ["8.8.8.8", "93.184.216.34", "2606:4700:4700::1111"] {
            assert!(
                is_public_address(address.parse().expect("public address fixture")),
                "{address}"
            );
        }
    }

    #[test]
    fn shutdown_is_idempotent_and_stops_new_admission() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime");
        let client = HttpClient::prepare(
            "http://127.0.0.1:7447/nip11",
            HttpClientAddressPolicy::LoopbackOnly,
            &HttpClientTrust::WebpkiRoots,
            None,
            HttpClientLimits::default(),
            runtime.handle().clone(),
        )
        .expect("prepared loopback client");
        client.shutdown().expect("first shutdown");
        client.shutdown().expect("repeated shutdown");
        assert_eq!(
            ActiveRequest::acquire(&client)
                .err()
                .expect("shutdown client must reject new request resources")
                .code,
            "http_client_shutdown"
        );
    }

    #[test]
    fn media_type_base_is_case_insensitive_and_parameters_are_strict() {
        assert!(media_type_matches(
            b" Application/Nostr+Json ; charset=\"utf-8\" ",
            "application/nostr+json"
        ));
        assert!(media_type_matches(
            b"application/nostr+json;charset=utf-8",
            "application/nostr+json"
        ));
        for invalid in [
            b"application/nostr+jsonx".as_slice(),
            b"application/nostr+json;".as_slice(),
            b"application/nostr+json; charset".as_slice(),
            b"text/plain".as_slice(),
        ] {
            assert!(!media_type_matches(invalid, "application/nostr+json"));
        }
    }

    #[test]
    fn forbidden_headers_and_independent_limits_fail_closed() {
        let endpoint =
            HttpClientEndpoint::parse("https://relay.example/path").expect("canonical endpoint");
        let limits = HttpClientLimits::default();
        for name in [
            "host",
            "authorization",
            "proxy-authorization",
            "cookie",
            "content-length",
            "transfer-encoding",
            "connection",
            "upgrade",
            "accept-encoding",
        ] {
            assert_eq!(
                build_request(
                    &endpoint,
                    vec![HttpHeader {
                        name: name.to_owned(),
                        value: b"forbidden".to_vec(),
                    }],
                    &limits,
                )
                .expect_err("owned header must reject")
                .code,
                "http_client_header_forbidden"
            );
        }
        assert!(
            build_request(
                &endpoint,
                vec![HttpHeader {
                    name: "accept".to_owned(),
                    value: b"application/nostr+json".to_vec(),
                }],
                &limits,
            )
            .is_ok()
        );
    }
}
