//! Typed bounded HTTP transport over the same prepared component handler used by tests.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{ExecutionError, ExecutionFailureClass};
use super::package::RunnerKind;
use super::runtime::{ResidentDeployment, ShutdownReceipt};
use super::semantic::{ResolvedField, ResolvedType};
use super::stream::{StreamLease, StreamRegistry};
use super::value::{MapKey, Value};
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::header::{CONTENT_TYPE, HeaderName, HeaderValue};
use axum::http::{Request, Response, StatusCode};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;

pub const HTTP_ADAPTER_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_HTTP_BODY_BYTES: usize = 64 * 1024 * 1024;
pub const MAXIMUM_HTTP_HEADER_BYTES: usize = 256 * 1024;
pub const MAXIMUM_HTTP_HEADERS: usize = 1_024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HttpLimits {
    pub contract_version: u16,
    pub maximum_request_body_bytes: usize,
    pub maximum_response_body_bytes: usize,
    pub maximum_header_bytes: usize,
    pub maximum_headers: usize,
}

impl Default for HttpLimits {
    fn default() -> Self {
        Self {
            contract_version: HTTP_ADAPTER_CONTRACT_VERSION,
            maximum_request_body_bytes: 1024 * 1024,
            maximum_response_body_bytes: 4 * 1024 * 1024,
            maximum_header_bytes: 32 * 1024,
            maximum_headers: 128,
        }
    }
}

impl HttpLimits {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != HTTP_ADAPTER_CONTRACT_VERSION {
            return Err(http_diagnostic(
                "http_contract",
                "HTTP adapter limits use a predecessor or foreign contract",
            ));
        }
        for (name, value, maximum) in [
            (
                "maximum_request_body_bytes",
                self.maximum_request_body_bytes,
                MAXIMUM_HTTP_BODY_BYTES,
            ),
            (
                "maximum_response_body_bytes",
                self.maximum_response_body_bytes,
                MAXIMUM_HTTP_BODY_BYTES,
            ),
            (
                "maximum_header_bytes",
                self.maximum_header_bytes,
                MAXIMUM_HTTP_HEADER_BYTES,
            ),
            (
                "maximum_headers",
                self.maximum_headers,
                MAXIMUM_HTTP_HEADERS,
            ),
        ] {
            if value == 0 || value > maximum {
                return Err(http_diagnostic(
                    "http_limit",
                    format!("{name} must be 1 through {maximum}"),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpHeader {
    pub name: String,
    pub value: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub query: String,
    pub headers: Vec<HttpHeader>,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<HttpHeader>,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HttpDispatchObservation {
    pub task_id: u64,
    pub queue_nanoseconds: u64,
    pub execution_nanoseconds: u64,
    pub instructions: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HttpServerReceipt {
    pub contract_version: u16,
    pub local_address: String,
    pub accepted_at_transport: bool,
    pub shutdown: ShutdownReceipt,
}

#[derive(Clone)]
pub struct HttpApplication {
    deployment: ResidentDeployment,
    limits: HttpLimits,
    streams: StreamRegistry,
}

impl HttpApplication {
    pub fn new(
        deployment: ResidentDeployment,
        limits: HttpLimits,
        streams: StreamRegistry,
    ) -> Result<Self, Diagnostic> {
        limits.validate()?;
        if deployment.target().runner != RunnerKind::Http {
            return Err(http_diagnostic(
                "http_runner_kind",
                "HTTP adapter requires an http runner target",
            ));
        }
        let signature = &deployment.target().port.signature;
        if signature.parameters != [request_type()] || signature.result != response_type() {
            return Err(http_diagnostic(
                "http_handler_signature",
                "HTTP handler must have the exact current structural request and response types",
            ));
        }
        if streams.limits().maximum_total_bytes
            < u64::try_from(limits.maximum_request_body_bytes).unwrap_or(u64::MAX)
        {
            return Err(http_diagnostic(
                "http_stream_limit",
                "stream total-byte limit is smaller than the accepted HTTP request body limit",
            ));
        }
        Ok(Self {
            deployment,
            limits,
            streams,
        })
    }

    pub fn router(self) -> Router {
        Router::new()
            .fallback(live_handler)
            .with_state(Arc::new(self))
    }

    pub async fn dispatch(
        &self,
        mut request: HttpRequest,
    ) -> Result<(HttpResponse, HttpDispatchObservation), ExecutionError> {
        validate_request(&request, &self.limits)?;
        let lease = self
            .streams
            .register_memory(std::mem::take(&mut request.body))?;
        self.dispatch_stream(request, &lease).await
    }

    async fn dispatch_stream(
        &self,
        request: HttpRequest,
        body: &StreamLease,
    ) -> Result<(HttpResponse, HttpDispatchObservation), ExecutionError> {
        let request = request_value(request, body.value())?;
        let receipt = self.deployment.invoke(vec![request]).await?;
        let response = response_value(receipt.value, &self.limits)?;
        Ok((
            response,
            HttpDispatchObservation {
                task_id: receipt.task_id,
                queue_nanoseconds: receipt.queue_nanoseconds,
                execution_nanoseconds: receipt.execution_nanoseconds,
                instructions: receipt.execution.instructions,
            },
        ))
    }

    pub async fn serve(
        self,
        listener: TcpListener,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Result<HttpServerReceipt, Diagnostic> {
        let local_address = listener
            .local_addr()
            .map_err(|error| http_io("http_listener_address", error))?;
        let deployment = self.deployment.clone();
        axum::serve(listener, self.router())
            .with_graceful_shutdown(shutdown)
            .await
            .map_err(|error| http_io("http_serve", error))?;
        let runtime_shutdown = deployment.shutdown().await;
        if runtime_shutdown.remaining_tasks != 0 || !runtime_shutdown.cleanup_failures.is_empty() {
            return Err(Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "http_shutdown_incomplete",
                format!(
                    "{} resident tasks and {} cleanup failures remained after HTTP shutdown",
                    runtime_shutdown.remaining_tasks,
                    runtime_shutdown.cleanup_failures.len()
                ),
            ));
        }
        Ok(HttpServerReceipt {
            contract_version: HTTP_ADAPTER_CONTRACT_VERSION,
            local_address: local_address.to_string(),
            accepted_at_transport: true,
            shutdown: runtime_shutdown,
        })
    }

    pub fn deployment(&self) -> &ResidentDeployment {
        &self.deployment
    }
}

async fn live_handler(
    State(application): State<Arc<HttpApplication>>,
    request: Request<Body>,
) -> Response<Body> {
    let method_is_head = request.method() == axum::http::Method::HEAD;
    let (request, body) = match decode_live_request(request, &application.limits) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let maximum_body_bytes = match u64::try_from(application.limits.maximum_request_body_bytes) {
        Ok(value) => value,
        Err(_) => {
            return static_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "HTTP request body limit is not representable",
            );
        }
    };
    let (lease, producer) = match application
        .streams
        .register_pipe_with_limit(maximum_body_bytes)
    {
        Ok(stream) => stream,
        Err(error) => return safe_error_response(&error, StatusCode::SERVICE_UNAVAILABLE),
    };
    let chunk_bytes = application.streams.limits().maximum_chunk_bytes;
    let deadline = Instant::now()
        .checked_add(Duration::from_millis(
            application
                .deployment
                .limits()
                .request_deadline_milliseconds,
        ))
        .unwrap_or_else(Instant::now);
    let mut pump = tokio::spawn(async move {
        let mut body = body.into_data_stream();
        let mut total = 0u64;
        let mut consumer_closed = false;
        while let Some(chunk) = body.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(_) => {
                    let error = ExecutionError::new(
                        ExecutionFailureClass::Capability,
                        "http_request_body_transport",
                        "HTTP request body transport failed",
                    );
                    producer.fail(error.clone());
                    return Err(error);
                }
            };
            let length = u64::try_from(chunk.len()).map_err(|_| {
                ExecutionError::resource(
                    "http_request_body_limit",
                    "HTTP request body chunk length is not representable",
                )
            })?;
            total = total.checked_add(length).ok_or_else(|| {
                ExecutionError::resource(
                    "http_request_body_limit",
                    "HTTP request body byte accounting overflowed",
                )
            })?;
            if total > maximum_body_bytes {
                let error = ExecutionError::resource(
                    "http_request_body_limit",
                    "HTTP request body exceeds the configured bytes",
                );
                producer.fail(error.clone());
                return Err(error);
            }
            if consumer_closed {
                continue;
            }
            for part in chunk.chunks(chunk_bytes) {
                if let Err(error) = producer.push(part.to_vec()).await {
                    if error.code == "stream_consumer_closed" {
                        consumer_closed = true;
                        break;
                    }
                    producer.fail(error.clone());
                    return Err(error);
                }
            }
        }
        if !consumer_closed {
            producer.finish();
        }
        Ok(())
    });
    let outcome = application.dispatch_stream(request, &lease).await;
    drop(lease);
    let remaining = deadline.saturating_duration_since(Instant::now());
    let pump_outcome = tokio::time::timeout(remaining, &mut pump).await;
    match pump_outcome {
        Ok(Ok(Ok(()))) => {}
        Ok(Ok(Err(error))) if error.code == "http_request_body_limit" => {
            return safe_error_response(&error, StatusCode::PAYLOAD_TOO_LARGE);
        }
        Ok(Ok(Err(error))) => {
            return safe_error_response(&error, StatusCode::BAD_REQUEST);
        }
        Ok(Err(_)) => {
            return static_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "request body task terminated unexpectedly",
            );
        }
        Err(_) => {
            pump.abort();
            let _ = pump.await;
            return static_response(
                StatusCode::REQUEST_TIMEOUT,
                "request body deadline exceeded",
            );
        }
    }
    match outcome {
        Ok((mut response, _)) => {
            if method_is_head {
                response.body.clear();
            }
            encode_live_response(response).unwrap_or_else(|error| {
                safe_error_response(&error, StatusCode::INTERNAL_SERVER_ERROR)
            })
        }
        Err(error) => {
            let status = match error.code.as_str() {
                "http_query_decode" => StatusCode::BAD_REQUEST,
                "resident_overloaded" => StatusCode::SERVICE_UNAVAILABLE,
                "execution_deadline" | "execution_cancelled" | "resident_shutting_down" => {
                    StatusCode::SERVICE_UNAVAILABLE
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            safe_error_response(&error, status)
        }
    }
}

fn decode_live_request(
    request: Request<Body>,
    limits: &HttpLimits,
) -> Result<(HttpRequest, Body), Response<Body>> {
    let (parts, body) = request.into_parts();
    let mut headers = Vec::with_capacity(parts.headers.len());
    let mut header_bytes = 0usize;
    if parts.headers.len() > limits.maximum_headers {
        return Err(static_response(
            StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
            "request headers exceed the configured count",
        ));
    }
    for (name, value) in &parts.headers {
        header_bytes = match header_bytes
            .checked_add(name.as_str().len())
            .and_then(|length| length.checked_add(value.as_bytes().len()))
        {
            Some(length) if length <= limits.maximum_header_bytes => length,
            _ => {
                return Err(static_response(
                    StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
                    "request headers exceed the configured bytes",
                ));
            }
        };
        headers.push(HttpHeader {
            name: name.as_str().to_owned(),
            value: value.as_bytes().to_vec(),
        });
    }
    if let Some(length) = parts.headers.get(axum::http::header::CONTENT_LENGTH) {
        let length = length
            .to_str()
            .ok()
            .and_then(|value| value.parse::<u64>().ok());
        if length.is_none() {
            return Err(static_response(
                StatusCode::BAD_REQUEST,
                "content-length is not a canonical unsigned integer",
            ));
        }
        if length.is_some_and(|length| {
            length > u64::try_from(limits.maximum_request_body_bytes).unwrap_or(u64::MAX)
        }) {
            return Err(static_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "request body exceeds the configured bytes",
            ));
        }
    }
    Ok((
        HttpRequest {
            method: parts.method.as_str().to_owned(),
            path: parts.uri.path().to_owned(),
            query: parts.uri.query().unwrap_or_default().to_owned(),
            headers,
            body: Vec::new(),
        },
        body,
    ))
}

fn encode_live_response(response: HttpResponse) -> Result<Response<Body>, ExecutionError> {
    let status = StatusCode::from_u16(response.status).map_err(|_| {
        protocol_error(
            "http_response_status",
            "application returned an invalid HTTP status",
        )
    })?;
    let mut builder = Response::builder().status(status);
    let headers = builder.headers_mut().ok_or_else(|| {
        protocol_error(
            "http_response_builder",
            "HTTP response builder lost its header map",
        )
    })?;
    for header in response.headers {
        let name = HeaderName::from_bytes(header.name.as_bytes()).map_err(|_| {
            protocol_error(
                "http_response_header_name",
                "application returned an invalid HTTP header name",
            )
        })?;
        if is_transport_owned_header(&name) {
            return Err(protocol_error(
                "http_response_transport_header",
                format!(
                    "application may not set transport-owned header '{}'",
                    name.as_str()
                ),
            ));
        }
        let value = HeaderValue::from_bytes(&header.value).map_err(|_| {
            protocol_error(
                "http_response_header_value",
                "application returned an invalid HTTP header value",
            )
        })?;
        headers.append(name, value);
    }
    builder.body(Body::from(response.body)).map_err(|_| {
        protocol_error(
            "http_response_builder",
            "application response could not be framed",
        )
    })
}

fn request_value(request: HttpRequest, body: Value) -> Result<Value, ExecutionError> {
    let query_parameters = decode_query(&request.query)?;
    Ok(Value::record(
        None,
        [
            ("method".to_owned(), Value::text(request.method)),
            ("path".to_owned(), Value::text(request.path)),
            ("query".to_owned(), Value::text(request.query)),
            ("query_parameters".to_owned(), query_parameters),
            ("headers".to_owned(), headers_value(request.headers)),
            ("body".to_owned(), body),
        ],
    ))
}

fn decode_query(query: &str) -> Result<Value, ExecutionError> {
    let decoded = decode_query_parameters(query)?;
    let mut parameters: std::collections::BTreeMap<MapKey, Value> =
        std::collections::BTreeMap::new();
    for (name, values) in decoded {
        parameters.insert(
            MapKey::Text(name),
            Value::List(Arc::new(values.into_iter().map(Value::text).collect())),
        );
    }
    Ok(Value::Map(Arc::new(parameters)))
}

pub(crate) fn decode_query_parameters(
    query: &str,
) -> Result<std::collections::BTreeMap<String, Vec<String>>, ExecutionError> {
    let mut parameters = std::collections::BTreeMap::new();
    if query.is_empty() {
        return Ok(parameters);
    }
    for pair in query.split('&') {
        if pair.is_empty() {
            return Err(protocol_error(
                "http_query_decode",
                "query contains an empty field",
            ));
        }
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        let name = decode_form_component(name)?;
        let value = decode_form_component(value)?;
        if name.is_empty() {
            return Err(protocol_error(
                "http_query_decode",
                "query field name is empty",
            ));
        }
        parameters.entry(name).or_insert_with(Vec::new).push(value);
    }
    Ok(parameters)
}

fn decode_form_component(value: &str) -> Result<String, ExecutionError> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            b'%' => {
                let Some(high) = bytes.get(index + 1).and_then(|value| hex(*value)) else {
                    return Err(protocol_error(
                        "http_query_decode",
                        "query contains an invalid percent escape",
                    ));
                };
                let Some(low) = bytes.get(index + 2).and_then(|value| hex(*value)) else {
                    return Err(protocol_error(
                        "http_query_decode",
                        "query contains an invalid percent escape",
                    ));
                };
                output.push((high << 4) | low);
                index += 3;
            }
            byte if byte.is_ascii() => {
                output.push(byte);
                index += 1;
            }
            _ => {
                return Err(protocol_error(
                    "http_query_decode",
                    "query must percent-encode non-ASCII bytes",
                ));
            }
        }
    }
    String::from_utf8(output).map_err(|_| {
        protocol_error(
            "http_query_decode",
            "query percent decoding did not produce UTF-8",
        )
    })
}

fn hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn headers_value(headers: Vec<HttpHeader>) -> Value {
    Value::List(Arc::new(
        headers
            .into_iter()
            .map(|header| {
                Value::record(
                    None,
                    [
                        ("name".to_owned(), Value::text(header.name)),
                        ("value".to_owned(), Value::bytes(header.value)),
                    ],
                )
            })
            .collect(),
    ))
}

fn response_value(value: Value, limits: &HttpLimits) -> Result<HttpResponse, ExecutionError> {
    let Value::Record {
        owner: None,
        fields,
    } = value
    else {
        return Err(protocol_error(
            "http_response_value",
            "HTTP handler returned a foreign value",
        ));
    };
    let status = match fields.get("status") {
        Some(Value::I64(value)) => u16::try_from(*value).map_err(|_| {
            protocol_error(
                "http_response_status",
                "HTTP response status is outside unsigned 16-bit range",
            )
        })?,
        _ => {
            return Err(protocol_error(
                "http_response_status",
                "HTTP response status is absent or not I64",
            ));
        }
    };
    if !(200..=599).contains(&status) {
        return Err(protocol_error(
            "http_response_status",
            "HTTP application response status must be 200 through 599",
        ));
    }
    let headers = decode_headers(fields.get("headers"), limits)?;
    let body = match fields.get("body") {
        Some(Value::Bytes(body)) if body.len() <= limits.maximum_response_body_bytes => {
            body.to_vec()
        }
        Some(Value::Bytes(_)) => {
            return Err(ExecutionError::resource(
                "http_response_body_limit",
                "HTTP response body exceeds the configured bytes",
            ));
        }
        _ => {
            return Err(protocol_error(
                "http_response_body",
                "HTTP response body is absent or not Bytes",
            ));
        }
    };
    Ok(HttpResponse {
        status,
        headers,
        body,
    })
}

fn decode_headers(
    value: Option<&Value>,
    limits: &HttpLimits,
) -> Result<Vec<HttpHeader>, ExecutionError> {
    let Some(Value::List(values)) = value else {
        return Err(protocol_error(
            "http_response_headers",
            "HTTP response headers are absent or not a list",
        ));
    };
    if values.len() > limits.maximum_headers {
        return Err(ExecutionError::resource(
            "http_response_header_count",
            "HTTP response header count exceeds the configured limit",
        ));
    }
    let mut bytes = 0usize;
    values
        .iter()
        .map(|value| {
            let Value::Record {
                owner: None,
                fields,
            } = value
            else {
                return Err(protocol_error(
                    "http_response_header",
                    "HTTP response header is not a structural record",
                ));
            };
            let name = match fields.get("name") {
                Some(Value::Text(name)) => name.to_string(),
                _ => {
                    return Err(protocol_error(
                        "http_response_header_name",
                        "HTTP response header name is absent or not Text",
                    ));
                }
            };
            let value = match fields.get("value") {
                Some(Value::Bytes(value)) => value.to_vec(),
                _ => {
                    return Err(protocol_error(
                        "http_response_header_value",
                        "HTTP response header value is absent or not Bytes",
                    ));
                }
            };
            bytes = bytes
                .checked_add(name.len())
                .and_then(|length| length.checked_add(value.len()))
                .ok_or_else(|| {
                    ExecutionError::resource(
                        "http_response_header_bytes",
                        "HTTP response header byte accounting overflowed",
                    )
                })?;
            if bytes > limits.maximum_header_bytes {
                return Err(ExecutionError::resource(
                    "http_response_header_bytes",
                    "HTTP response headers exceed the configured bytes",
                ));
            }
            Ok(HttpHeader { name, value })
        })
        .collect()
}

pub(crate) fn validate_request(
    request: &HttpRequest,
    limits: &HttpLimits,
) -> Result<(), ExecutionError> {
    if request.method.is_empty() || request.method.len() > 32 {
        return Err(protocol_error(
            "http_request_method",
            "HTTP request method is empty or excessive",
        ));
    }
    if request.path.is_empty() || request.path.len() > 16 * 1024 || request.query.len() > 16 * 1024
    {
        return Err(protocol_error(
            "http_request_target",
            "HTTP request target is empty or excessive",
        ));
    }
    if request.body.len() > limits.maximum_request_body_bytes {
        return Err(ExecutionError::resource(
            "http_request_body_limit",
            "HTTP request body exceeds the configured bytes",
        ));
    }
    if request.headers.len() > limits.maximum_headers {
        return Err(ExecutionError::resource(
            "http_request_header_count",
            "HTTP request header count exceeds the configured limit",
        ));
    }
    let bytes = request.headers.iter().try_fold(0usize, |total, header| {
        total
            .checked_add(header.name.len())
            .and_then(|value| value.checked_add(header.value.len()))
            .ok_or_else(|| {
                ExecutionError::resource(
                    "http_request_header_bytes",
                    "HTTP request header byte accounting overflowed",
                )
            })
    })?;
    if bytes > limits.maximum_header_bytes {
        return Err(ExecutionError::resource(
            "http_request_header_bytes",
            "HTTP request headers exceed the configured bytes",
        ));
    }
    Ok(())
}

pub fn request_type() -> ResolvedType {
    ResolvedType::Record(vec![
        field("method", ResolvedType::Text),
        field("path", ResolvedType::Text),
        field("query", ResolvedType::Text),
        field(
            "query_parameters",
            ResolvedType::Map(
                Box::new(ResolvedType::Text),
                Box::new(ResolvedType::List(Box::new(ResolvedType::Text))),
            ),
        ),
        field("headers", ResolvedType::List(Box::new(header_type()))),
        field("body", ResolvedType::Stream(Box::new(ResolvedType::Bytes))),
    ])
}

pub fn response_type() -> ResolvedType {
    ResolvedType::Record(vec![
        field("status", ResolvedType::I64),
        field("headers", ResolvedType::List(Box::new(header_type()))),
        field("body", ResolvedType::Bytes),
    ])
}

fn header_type() -> ResolvedType {
    ResolvedType::Record(vec![
        field("name", ResolvedType::Text),
        field("value", ResolvedType::Bytes),
    ])
}

fn field(name: &str, ty: ResolvedType) -> ResolvedField {
    ResolvedField {
        name: name.to_owned(),
        ty,
    }
}

fn is_transport_owned_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "connection"
            | "content-length"
            | "keep-alive"
            | "proxy-connection"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

fn safe_error_response(error: &ExecutionError, status: StatusCode) -> Response<Body> {
    let message = match error.class {
        ExecutionFailureClass::Resource => "request resource limit reached",
        ExecutionFailureClass::Cancelled => "request cancelled",
        ExecutionFailureClass::Trap
        | ExecutionFailureClass::Capability
        | ExecutionFailureClass::PossibleVisibility
        | ExecutionFailureClass::Infrastructure => "request could not be completed",
    };
    let mut response = static_response(status, message);
    response.headers_mut().insert(
        HeaderName::from_static("x-lkjscript-failure-class"),
        HeaderValue::from_static(match error.class {
            ExecutionFailureClass::Trap => "trap",
            ExecutionFailureClass::Capability => "capability",
            ExecutionFailureClass::PossibleVisibility => "possible_visibility",
            ExecutionFailureClass::Resource => "resource",
            ExecutionFailureClass::Cancelled => "cancelled",
            ExecutionFailureClass::Infrastructure => "infrastructure",
        }),
    );
    if error.code.len() <= 128
        && let Ok(value) = HeaderValue::from_str(&error.code)
    {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-lkjscript-failure-code"), value);
    }
    response
}

fn static_response(status: StatusCode, message: &'static str) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(message))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

fn protocol_error(code: &str, message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}

fn http_diagnostic(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}

fn http_io(code: &str, error: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Infrastructure, code, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{
        PreparedProgram, ResidentLimits, RunPolicy, SourceLimits, build_artifact, decode_package,
        load_artifact, parse_source, validate_package_documents,
    };
    use axum::body::to_bytes;
    use axum::http::Method;
    use bytes::Bytes;
    use futures_util::stream;
    use tower::ServiceExt;

    fn application() -> HttpApplication {
        let descriptor = decode_package(
            br#"{"contract_version":1,"package_id":"1234567890abcdef1234567890abcdef","name":"http-test","modules":[{"name":"main","path":"src/main.lkj"}],"dependencies":[],"targets":[{"name":"serve","component":"main.App","port":"request","runner":"http"}]}"#,
        )
        .expect("descriptor");
        let source = br#"(module main
  (export App)
  (extern from-text ((value Text)) Bytes core.bytes.from-text)
  (extern text-equal ((left Text) (right Text)) Bool core.text.equal)
  (fn request ((request (Record
      (method Text)
      (path Text)
      (query Text)
      (query_parameters (Map Text (List Text)))
      (headers (List (Record (name Text) (value Bytes))))
      (body (Stream Bytes)))))
    (Record
      (status I64)
      (headers (List (Record (name Text) (value Bytes))))
      (body Bytes))
    (if (call text-equal (field request path) "/health")
        (record _
          (status 200)
          (headers (list (Record (name Text) (value Bytes))
            (record _ (name "content-type") (value (call from-text "text/plain")))))
          (body (call from-text "ready")))
        (record _
          (status 404)
          (headers (list (Record (name Text) (value Bytes))))
          (body (call from-text "not found")))))
  (component App
    (port request
      (Function ((Record
        (method Text)
        (path Text)
        (query Text)
        (query_parameters (Map Text (List Text)))
        (headers (List (Record (name Text) (value Bytes))))
        (body (Stream Bytes))))
        (Record
          (status I64)
          (headers (List (Record (name Text) (value Bytes))))
          (body Bytes)))
      (function request))))
"#;
        let document =
            parse_source("src/main.lkj", source, SourceLimits::default()).expect("source");
        let package = validate_package_documents(descriptor, vec![document], &[]).expect("package");
        let (artifact, _) = build_artifact(&package, &[&package]).expect("artifact");
        let program = Arc::new(
            PreparedProgram::prepare(load_artifact(&artifact).expect("load")).expect("prepare"),
        );
        let deployment = ResidentDeployment::prepare(
            program,
            "serve",
            Vec::new(),
            ResidentLimits::default(),
            RunPolicy::default(),
        )
        .expect("deployment");
        HttpApplication::new(
            deployment,
            HttpLimits::default(),
            crate::platform::StreamRegistry::new(crate::platform::StreamLimits::default())
                .expect("stream registry"),
        )
        .expect("http application")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn in_memory_transport_uses_the_prepared_component_handler() {
        let application = application();
        let router = application.clone().router();
        let response = router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/health")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            to_bytes(response.into_body(), 64)
                .await
                .expect("body")
                .as_ref(),
            b"ready"
        );
        assert_eq!(application.deployment().observe().completed, 1);
        assert_eq!(application.deployment().shutdown().await.remaining_tasks, 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn malformed_application_response_and_body_limits_are_closed() {
        let application = application();
        let oversized = HttpRequest {
            method: "POST".to_owned(),
            path: "/".to_owned(),
            query: String::new(),
            headers: Vec::new(),
            body: vec![0; application.limits.maximum_request_body_bytes + 1],
        };
        let error = application
            .dispatch(oversized)
            .await
            .expect_err("oversized body");
        assert_eq!(error.class, ExecutionFailureClass::Resource);
        assert_eq!(application.deployment().observe().admitted, 0);
        assert_eq!(application.deployment().shutdown().await.remaining_tasks, 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn chunked_body_limit_is_enforced_when_the_handler_ignores_the_body() {
        let application = application();
        let limit = application.limits.maximum_request_body_bytes;
        let body = Body::from_stream(stream::iter([
            Ok::<_, std::io::Error>(Bytes::from(vec![0; limit / 2 + 1])),
            Ok::<_, std::io::Error>(Bytes::from(vec![0; limit / 2 + 1])),
        ]));
        let response = application
            .clone()
            .router()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/health")
                    .body(body)
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(application.deployment().observe().completed, 1);
        assert_eq!(application.deployment().shutdown().await.remaining_tasks, 0);
    }

    #[test]
    fn transport_failures_publish_bounded_classification_without_messages() {
        let error = ExecutionError::new(
            ExecutionFailureClass::Infrastructure,
            "database_connection",
            "sensitive provider detail",
        );
        let response = safe_error_response(&error, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            response.headers()["x-lkjscript-failure-class"],
            "infrastructure"
        );
        assert_eq!(
            response.headers()["x-lkjscript-failure-code"],
            "database_connection"
        );
    }

    #[test]
    fn exact_signature_and_predecessor_limits_reject() {
        assert!(
            HttpLimits {
                contract_version: 0,
                ..HttpLimits::default()
            }
            .validate()
            .is_err()
        );
        assert_eq!(request_type(), request_type());
        assert_eq!(response_type(), response_type());
    }
}
