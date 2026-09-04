//! Typed bounded HTTP transport over the same prepared component handler used by tests.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{ExecutionError, ExecutionFailureClass};
use super::kernel::{Name, StructuralTypeField, TypeForm, TypeObjectDigest, TypeObjectInterner};
use super::runtime::ShutdownReceipt;
use super::semantic_id::HttpRouteId;
use super::stream::ByteStreamProducer;
use axum::body::Body;
use axum::http::header::{CONTENT_TYPE, HeaderName, HeaderValue};
use axum::http::{Request, Response, StatusCode};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub const HTTP_ADAPTER_CONTRACT_VERSION: u16 = 3;
pub const MAXIMUM_HTTP_BODY_BYTES: usize = 64 * 1024 * 1024;
pub const MAXIMUM_HTTP_HEADER_BYTES: usize = 256 * 1024;
pub const MAXIMUM_HTTP_HEADERS: usize = 1_024;

const fn http_adapter_contract_version() -> u16 {
    HTTP_ADAPTER_CONTRACT_VERSION
}

/// One generation-neutral construction of the exact semantic HTTP boundary.
///
/// Runtime admission, project creation, and focused contract tests consume these same canonical
/// type objects. Transport values and resident runtime indexes remain separate representations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct HttpSemanticTypes {
    pub(crate) i64_type: TypeObjectDigest,
    pub(crate) bytes_type: TypeObjectDigest,
    pub(crate) text_type: TypeObjectDigest,
    pub(crate) stream_type: TypeObjectDigest,
    pub(crate) header_type: TypeObjectDigest,
    pub(crate) header_list_type: TypeObjectDigest,
    pub(crate) request_type: TypeObjectDigest,
    pub(crate) response_type: TypeObjectDigest,
    pub(crate) function_type: TypeObjectDigest,
}

pub(crate) fn semantic_http_types(
    types: &mut TypeObjectInterner,
) -> Result<HttpSemanticTypes, Diagnostic> {
    let i64_type = types.intern(TypeForm::I64)?;
    let bytes_type = types.intern(TypeForm::Bytes)?;
    let text_type = types.intern(TypeForm::Text)?;
    let stream_type = types.intern(TypeForm::Stream { item: bytes_type })?;
    let header_type = types.intern(TypeForm::StructuralRecord {
        fields: vec![
            semantic_type_field("name", text_type)?,
            semantic_type_field("value", bytes_type)?,
        ],
    })?;
    let header_list_type = types.intern(TypeForm::List { item: header_type })?;
    let text_list_type = types.intern(TypeForm::List { item: text_type })?;
    let query_map_type = types.intern(TypeForm::Map {
        key: text_type,
        value: text_list_type,
    })?;
    let request_type = types.intern(TypeForm::StructuralRecord {
        fields: vec![
            semantic_type_field("body", stream_type)?,
            semantic_type_field("headers", header_list_type)?,
            semantic_type_field("method", text_type)?,
            semantic_type_field("path", text_type)?,
            semantic_type_field("query", text_type)?,
            semantic_type_field("query_parameters", query_map_type)?,
        ],
    })?;
    let response_type = types.intern(TypeForm::StructuralRecord {
        fields: vec![
            semantic_type_field("body", bytes_type)?,
            semantic_type_field("headers", header_list_type)?,
            semantic_type_field("status", i64_type)?,
        ],
    })?;
    let function_type = types.intern(TypeForm::Function {
        parameters: vec![request_type],
        result: response_type,
    })?;
    Ok(HttpSemanticTypes {
        i64_type,
        bytes_type,
        text_type,
        stream_type,
        header_type,
        header_list_type,
        request_type,
        response_type,
        function_type,
    })
}

pub(crate) fn semantic_http_route_function_type(
    types: &mut TypeObjectInterner,
    capture_count: usize,
) -> Result<TypeObjectDigest, Diagnostic> {
    let http = semantic_http_types(types)?;
    if capture_count == 0 {
        return Ok(http.function_type);
    }
    let mut parameters = Vec::with_capacity(capture_count.saturating_add(1));
    parameters.push(http.request_type);
    parameters.extend(std::iter::repeat_n(http.text_type, capture_count));
    types.intern(TypeForm::Function {
        parameters,
        result: http.response_type,
    })
}

fn semantic_type_field(
    name: &'static str,
    ty: TypeObjectDigest,
) -> Result<StructuralTypeField, Diagnostic> {
    Ok(StructuralTypeField {
        name: Name::new(name)?,
        ty,
    })
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HttpLimits {
    #[serde(skip, default = "http_adapter_contract_version")]
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
    pub route: Option<HttpRouteId>,
    pub matcher_steps: u64,
    pub captures: u64,
    pub capture_bytes: u64,
    pub task_id: Option<u64>,
    pub queue_nanoseconds: u64,
    pub execution_nanoseconds: u64,
    pub instructions: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HttpServerReceipt {
    #[serde(skip_serializing)]
    pub contract_version: u16,
    pub local_address: String,
    pub accepted_at_transport: bool,
    pub matcher_nodes: u64,
    pub shutdown: ShutdownReceipt,
}

pub(crate) async fn pump_request_body(
    body: Body,
    producer: ByteStreamProducer,
    maximum_body_bytes: u64,
    chunk_bytes: usize,
) -> Result<(), ExecutionError> {
    if chunk_bytes == 0 {
        return Err(ExecutionError::new(
            ExecutionFailureClass::Infrastructure,
            "http_stream_chunk_limit",
            "HTTP body pump received a zero stream chunk bound",
        ));
    }
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
}

pub(crate) async fn finish_request_body_pump(
    mut pump: tokio::task::JoinHandle<Result<(), ExecutionError>>,
    deadline: Instant,
) -> Result<(), Response<Body>> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    match tokio::time::timeout(remaining, &mut pump).await {
        Ok(Ok(Ok(()))) => Ok(()),
        Ok(Ok(Err(error))) if error.code == "http_request_body_limit" => {
            Err(safe_error_response(&error, StatusCode::PAYLOAD_TOO_LARGE))
        }
        Ok(Ok(Err(error))) => Err(safe_error_response(&error, StatusCode::BAD_REQUEST)),
        Ok(Err(_)) => Err(static_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "request body task terminated unexpectedly",
        )),
        Err(_) => {
            pump.abort();
            let _ = pump.await;
            Err(static_response(
                StatusCode::REQUEST_TIMEOUT,
                "request body deadline exceeded",
            ))
        }
    }
}

pub(crate) fn decode_live_request(
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

pub(crate) fn encode_live_response(
    response: HttpResponse,
) -> Result<Response<Body>, ExecutionError> {
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

pub(crate) fn validate_request(
    request: &HttpRequest,
    limits: &HttpLimits,
) -> Result<(), ExecutionError> {
    super::kernel::validate_http_route_key(&request.method, &request.path).map_err(|_| {
        protocol_error(
            "http_request_route_key",
            "HTTP request method or path is not a valid exact route key",
        )
    })?;
    if request.query.len() > 16 * 1024 {
        return Err(protocol_error(
            "http_request_target",
            "HTTP request query is excessive",
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

pub(crate) fn execution_error_status(error: &ExecutionError) -> StatusCode {
    match error.code.as_str() {
        "http_query_decode" => StatusCode::BAD_REQUEST,
        code if code.starts_with("http_client_") => StatusCode::BAD_GATEWAY,
        "resident_overloaded" => StatusCode::SERVICE_UNAVAILABLE,
        "execution_deadline" | "execution_cancelled" | "resident_shutting_down" => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub(crate) fn safe_error_response(error: &ExecutionError, status: StatusCode) -> Response<Body> {
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

pub(crate) fn static_response(status: StatusCode, message: &'static str) -> Response<Body> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn oracle_field(name: &'static str, ty: TypeObjectDigest) -> StructuralTypeField {
        StructuralTypeField {
            name: Name::new(name).expect("normative HTTP field name"),
            ty,
        }
    }

    #[test]
    fn semantic_http_type_owner_matches_the_independent_normative_field_oracle() {
        let mut actual_interner = TypeObjectInterner::default();
        let actual = semantic_http_types(&mut actual_interner).expect("shared HTTP semantic types");

        // Deliberately reconstruct the normative shape without calling the shared helper. This
        // preserves an implementation-disjoint oracle for field names, order, and nested forms.
        let mut oracle = TypeObjectInterner::default();
        let i64_type = oracle.intern(TypeForm::I64).expect("I64");
        let bytes_type = oracle.intern(TypeForm::Bytes).expect("Bytes");
        let text_type = oracle.intern(TypeForm::Text).expect("Text");
        let stream_type = oracle
            .intern(TypeForm::Stream { item: bytes_type })
            .expect("Stream Bytes");
        let header_type = oracle
            .intern(TypeForm::StructuralRecord {
                fields: vec![
                    oracle_field("name", text_type),
                    oracle_field("value", bytes_type),
                ],
            })
            .expect("HTTP header");
        let header_list_type = oracle
            .intern(TypeForm::List { item: header_type })
            .expect("HTTP header list");
        let text_list_type = oracle
            .intern(TypeForm::List { item: text_type })
            .expect("query value list");
        let query_map_type = oracle
            .intern(TypeForm::Map {
                key: text_type,
                value: text_list_type,
            })
            .expect("query map");
        let request_type = oracle
            .intern(TypeForm::StructuralRecord {
                fields: vec![
                    oracle_field("body", stream_type),
                    oracle_field("headers", header_list_type),
                    oracle_field("method", text_type),
                    oracle_field("path", text_type),
                    oracle_field("query", text_type),
                    oracle_field("query_parameters", query_map_type),
                ],
            })
            .expect("HTTP request");
        let response_type = oracle
            .intern(TypeForm::StructuralRecord {
                fields: vec![
                    oracle_field("body", bytes_type),
                    oracle_field("headers", header_list_type),
                    oracle_field("status", i64_type),
                ],
            })
            .expect("HTTP response");
        let function_type = oracle
            .intern(TypeForm::Function {
                parameters: vec![request_type],
                result: response_type,
            })
            .expect("HTTP handler");
        let expected = HttpSemanticTypes {
            i64_type,
            bytes_type,
            text_type,
            stream_type,
            header_type,
            header_list_type,
            request_type,
            response_type,
            function_type,
        };

        assert_eq!(actual, expected);
        assert_eq!(actual_interner.into_objects(), oracle.into_objects());
    }

    #[test]
    fn transport_limits_and_query_decoding_are_strict() {
        let error = HttpLimits {
            contract_version: 0,
            ..HttpLimits::default()
        }
        .validate()
        .expect_err("predecessor HTTP contract must reject");
        assert_eq!(error.code, "http_contract");

        let query =
            decode_query_parameters("name=one+two&name=three%2Ffour").expect("canonical query");
        assert_eq!(
            query.get("name"),
            Some(&vec!["one two".to_owned(), "three/four".to_owned()])
        );
        assert_eq!(
            decode_query_parameters("name=%zz")
                .expect_err("malformed escape must reject")
                .code,
            "http_query_decode"
        );
    }

    #[test]
    fn transport_failures_publish_bounded_classification_without_messages() {
        let error = ExecutionError::new(
            ExecutionFailureClass::Infrastructure,
            "adapter_connection",
            "sensitive provider detail",
        );
        let response = safe_error_response(&error, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            response.headers()["x-lkjscript-failure-class"],
            "infrastructure"
        );
        assert_eq!(
            response.headers()["x-lkjscript-failure-code"],
            "adapter_connection"
        );
    }

    #[test]
    fn request_validation_owns_transport_bounds() {
        let limits = HttpLimits {
            maximum_request_body_bytes: 4,
            ..HttpLimits::default()
        };
        let request = HttpRequest {
            method: "POST".to_owned(),
            path: "/".to_owned(),
            query: String::new(),
            headers: Vec::new(),
            body: vec![0; 5],
        };
        assert_eq!(
            validate_request(&request, &limits)
                .expect_err("oversized request body must reject")
                .code,
            "http_request_body_limit"
        );
    }
}
