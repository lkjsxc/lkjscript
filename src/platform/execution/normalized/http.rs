//! Exact structural HTTP boundary for normalized resident deployments.

use super::capability::NormalizedAdapterKind;
use super::resident::NormalizedResidentDeployment;
use super::resource::NormalizedResourceScope;
use super::value::{NormalizedMapKey, NormalizedRecord, NormalizedValue};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionError, ExecutionFailureClass};
use crate::platform::http::{
    HTTP_ADAPTER_CONTRACT_VERSION, HttpDispatchObservation, HttpHeader, HttpLimits, HttpRequest,
    HttpResponse, HttpServerReceipt, decode_live_request, decode_query_parameters,
    encode_live_response, execution_error_status, finish_request_body_pump, pump_request_body,
    safe_error_response, static_response, validate_request,
};
use crate::platform::kernel::{
    Name, RequirementReference, StructuralTypeField, TypeForm, TypeObjectInterner,
};
use crate::platform::package::RunnerKind;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, Response, StatusCode};
use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;

#[derive(Clone)]
pub(crate) struct NormalizedHttpApplication {
    resident: NormalizedResidentDeployment,
    limits: HttpLimits,
    stream_requirement: RequirementReference,
}

impl NormalizedHttpApplication {
    pub(crate) fn new(
        resident: NormalizedResidentDeployment,
        limits: HttpLimits,
    ) -> Result<Self, Diagnostic> {
        limits.validate()?;
        if resident.target().runner != RunnerKind::Http {
            return Err(http_diagnostic(
                "normalized_http_runner_kind",
                "normalized HTTP adapter requires an http runner target",
            ));
        }
        let program = resident.program();
        let port = program
            .ports
            .get(resident.target().port.0 as usize)
            .ok_or_else(|| {
                http_corrupt(
                    "normalized_http_port",
                    "selected HTTP target port escaped the exact runtime table",
                )
            })?;
        if port.function_type != http_function_type()? {
            return Err(http_diagnostic(
                "normalized_http_handler_signature",
                "HTTP handler must have the exact current structural request and response types",
            ));
        }
        let mut stream_requirements = resident
            .deployment()
            .observation()
            .grants
            .iter()
            .filter_map(|(requirement, grant)| {
                (grant.adapter_kind == NormalizedAdapterKind::ByteStream).then_some(*requirement)
            });
        let stream_requirement = stream_requirements.next().ok_or_else(|| {
            http_diagnostic(
                "normalized_http_stream_grant",
                "HTTP deployment requires one exact byte-stream capability grant",
            )
        })?;
        if stream_requirements.next().is_some() {
            return Err(http_diagnostic(
                "normalized_http_stream_grant_ambiguous",
                "HTTP deployment has more than one possible request-body stream grant",
            ));
        }
        let maximum_stream_bytes = resident
            .deployment()
            .observation()
            .resources
            .streams
            .maximum_total_bytes;
        if maximum_stream_bytes
            < u64::try_from(limits.maximum_request_body_bytes).unwrap_or(u64::MAX)
        {
            return Err(http_diagnostic(
                "normalized_http_stream_limit",
                "stream total-byte limit is smaller than the accepted HTTP request body limit",
            ));
        }
        Ok(Self {
            resident,
            limits,
            stream_requirement,
        })
    }

    pub(crate) async fn dispatch(
        &self,
        mut request: HttpRequest,
    ) -> Result<(HttpResponse, HttpDispatchObservation), ExecutionError> {
        validate_request(&request, &self.limits)?;
        let query_parameters = decode_query_parameters(&request.query)?;
        let resources = NormalizedResourceScope::new()?;
        let body = self.resident.deployment().register_memory_stream(
            self.stream_requirement,
            &resources,
            std::mem::take(&mut request.body),
        )?;
        let request = request_value(request, query_parameters, body)?;
        let receipt = self
            .resident
            .invoke_scoped(resources, vec![request])
            .await?;
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

    pub(crate) fn router(self) -> Router {
        Router::new()
            .fallback(normalized_live_handler)
            .with_state(Arc::new(self))
    }

    pub(crate) async fn serve(
        self,
        listener: TcpListener,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Result<HttpServerReceipt, Diagnostic> {
        let local_address = listener.local_addr().map_err(|error| {
            http_io(
                "normalized_http_listener_address",
                format!("normalized HTTP listener address failed: {error}"),
            )
        })?;
        let resident = self.resident.clone();
        let serving = axum::serve(listener, self.router())
            .with_graceful_shutdown(shutdown)
            .await
            .map_err(|error| {
                http_io(
                    "normalized_http_serve",
                    format!("normalized HTTP server failed: {error}"),
                )
            });
        let shutdown = resident.shutdown().await;
        if let Err(mut error) = serving {
            if shutdown.remaining_tasks != 0 {
                error.notes.push(format!(
                    "{} resident tasks remained after server failure cleanup",
                    shutdown.remaining_tasks
                ));
            }
            error
                .notes
                .extend(shutdown.cleanup_failures.iter().map(|failure| {
                    format!("adapter cleanup failed with safe code '{}'", failure.code)
                }));
            return Err(error);
        }
        if shutdown.remaining_tasks != 0 || !shutdown.cleanup_failures.is_empty() {
            return Err(http_io(
                "normalized_http_shutdown_incomplete",
                format!(
                    "{} resident tasks and {} cleanup failures remained after normalized HTTP shutdown",
                    shutdown.remaining_tasks,
                    shutdown.cleanup_failures.len()
                ),
            ));
        }
        Ok(HttpServerReceipt {
            contract_version: HTTP_ADAPTER_CONTRACT_VERSION,
            local_address: local_address.to_string(),
            accepted_at_transport: true,
            shutdown,
        })
    }

    pub(crate) fn resident(&self) -> &NormalizedResidentDeployment {
        &self.resident
    }
}

async fn normalized_live_handler(
    State(application): State<Arc<NormalizedHttpApplication>>,
    request: Request<Body>,
) -> Response<Body> {
    let method_is_head = request.method() == axum::http::Method::HEAD;
    let (request, body) = match decode_live_request(request, &application.limits) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let query_parameters = match decode_query_parameters(&request.query) {
        Ok(parameters) => parameters,
        Err(error) => return safe_error_response(&error, StatusCode::BAD_REQUEST),
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
    let resources = match NormalizedResourceScope::new() {
        Ok(resources) => resources,
        Err(error) => return safe_error_response(&error, StatusCode::SERVICE_UNAVAILABLE),
    };
    let (body_value, producer) = match application.resident.deployment().register_pipe_stream(
        application.stream_requirement,
        &resources,
        maximum_body_bytes,
    ) {
        Ok(stream) => stream,
        Err(error) => return safe_error_response(&error, StatusCode::SERVICE_UNAVAILABLE),
    };
    let request = match request_value(request, query_parameters, body_value) {
        Ok(request) => request,
        Err(error) => return safe_error_response(&error, StatusCode::INTERNAL_SERVER_ERROR),
    };
    let chunk_bytes = application
        .resident
        .deployment()
        .observation()
        .resources
        .streams
        .maximum_chunk_bytes;
    let deadline = Instant::now()
        .checked_add(Duration::from_millis(
            application.resident.limits().request_deadline_milliseconds,
        ))
        .unwrap_or_else(Instant::now);
    let pump = tokio::spawn(pump_request_body(
        body,
        producer,
        maximum_body_bytes,
        chunk_bytes,
    ));
    let outcome = application
        .resident
        .invoke_scoped(resources, vec![request])
        .await
        .and_then(|receipt| response_value(receipt.value, &application.limits));
    if let Err(response) = finish_request_body_pump(pump, deadline).await {
        return response;
    }
    match outcome {
        Ok(mut response) => {
            if method_is_head {
                response.body.clear();
            }
            encode_live_response(response).unwrap_or_else(|error| {
                safe_error_response(&error, StatusCode::INTERNAL_SERVER_ERROR)
            })
        }
        Err(error) => safe_error_response(&error, execution_error_status(&error)),
    }
}

fn http_function_type() -> Result<crate::platform::kernel::TypeObjectDigest, Diagnostic> {
    let mut types = TypeObjectInterner::default();
    let i64_type = types.intern(TypeForm::I64)?;
    let bytes_type = types.intern(TypeForm::Bytes)?;
    let text_type = types.intern(TypeForm::Text)?;
    let stream_type = types.intern(TypeForm::Stream { item: bytes_type })?;
    let header_type = types.intern(TypeForm::StructuralRecord {
        fields: vec![
            type_field("name", text_type)?,
            type_field("value", bytes_type)?,
        ],
    })?;
    let header_list = types.intern(TypeForm::List { item: header_type })?;
    let text_list = types.intern(TypeForm::List { item: text_type })?;
    let query_map = types.intern(TypeForm::Map {
        key: text_type,
        value: text_list,
    })?;
    let request = types.intern(TypeForm::StructuralRecord {
        fields: vec![
            type_field("body", stream_type)?,
            type_field("headers", header_list)?,
            type_field("method", text_type)?,
            type_field("path", text_type)?,
            type_field("query", text_type)?,
            type_field("query_parameters", query_map)?,
        ],
    })?;
    let response = types.intern(TypeForm::StructuralRecord {
        fields: vec![
            type_field("body", bytes_type)?,
            type_field("headers", header_list)?,
            type_field("status", i64_type)?,
        ],
    })?;
    types.intern(TypeForm::Function {
        parameters: vec![request],
        result: response,
    })
}

fn type_field(
    name: &'static str,
    ty: crate::platform::kernel::TypeObjectDigest,
) -> Result<StructuralTypeField, Diagnostic> {
    Ok(StructuralTypeField {
        name: Name::new(name)?,
        ty,
    })
}

fn request_value(
    request: HttpRequest,
    query_parameters: BTreeMap<String, Vec<String>>,
    body: NormalizedValue,
) -> Result<NormalizedValue, ExecutionError> {
    let query_parameters = query_parameters
        .into_iter()
        .map(|(name, values)| {
            (
                NormalizedMapKey::Text(name),
                NormalizedValue::List(Arc::new(
                    values.into_iter().map(NormalizedValue::text).collect(),
                )),
            )
        })
        .collect();
    let headers = headers_value(request.headers)?;
    structural([
        ("body", body),
        ("headers", headers),
        ("method", NormalizedValue::text(request.method)),
        ("path", NormalizedValue::text(request.path)),
        ("query", NormalizedValue::text(request.query)),
        (
            "query_parameters",
            NormalizedValue::Map(Arc::new(query_parameters)),
        ),
    ])
}

fn headers_value(headers: Vec<HttpHeader>) -> Result<NormalizedValue, ExecutionError> {
    let headers = headers
        .into_iter()
        .map(|header| {
            structural([
                ("name", NormalizedValue::text(header.name)),
                ("value", NormalizedValue::bytes(header.value)),
            ])
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(NormalizedValue::List(Arc::new(headers)))
}

fn structural<const N: usize>(
    fields: [(&'static str, NormalizedValue); N],
) -> Result<NormalizedValue, ExecutionError> {
    let fields = fields
        .into_iter()
        .map(|(name, value)| {
            Name::new(name).map(|name| (name, value)).map_err(|_| {
                protocol_error(
                    "normalized_http_static_field",
                    "built-in HTTP field name is invalid",
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(NormalizedValue::Record(NormalizedRecord::Structural {
        fields: Arc::new(fields),
    }))
}

fn response_value(
    value: NormalizedValue,
    limits: &HttpLimits,
) -> Result<HttpResponse, ExecutionError> {
    let fields = exact_structural(&value, &["body", "headers", "status"], "response")?;
    let body = match &fields[0].1 {
        NormalizedValue::Bytes(bytes) if bytes.len() <= limits.maximum_response_body_bytes => {
            bytes.to_vec()
        }
        NormalizedValue::Bytes(_) => {
            return Err(ExecutionError::resource(
                "http_response_body_limit",
                "HTTP response body exceeds the configured bytes",
            ));
        }
        _ => {
            return Err(protocol_error(
                "http_response_body",
                "HTTP response body is not Bytes",
            ));
        }
    };
    let headers = decode_headers(&fields[1].1, limits)?;
    let status = match fields[2].1 {
        NormalizedValue::I64(value) => u16::try_from(value).map_err(|_| {
            protocol_error(
                "http_response_status",
                "HTTP response status is outside unsigned 16-bit range",
            )
        })?,
        _ => {
            return Err(protocol_error(
                "http_response_status",
                "HTTP response status is not I64",
            ));
        }
    };
    if !(200..=599).contains(&status) {
        return Err(protocol_error(
            "http_response_status",
            "HTTP application response status must be 200 through 599",
        ));
    }
    Ok(HttpResponse {
        status,
        headers,
        body,
    })
}

fn decode_headers(
    value: &NormalizedValue,
    limits: &HttpLimits,
) -> Result<Vec<HttpHeader>, ExecutionError> {
    let NormalizedValue::List(headers) = value else {
        return Err(protocol_error(
            "http_response_headers",
            "HTTP response headers are not a list",
        ));
    };
    if headers.len() > limits.maximum_headers {
        return Err(ExecutionError::resource(
            "http_response_header_count",
            "HTTP response header count exceeds the configured limit",
        ));
    }
    let mut total = 0usize;
    headers
        .iter()
        .map(|header| {
            let fields = exact_structural(header, &["name", "value"], "header")?;
            let name = match &fields[0].1 {
                NormalizedValue::Text(name) => name.to_string(),
                _ => {
                    return Err(protocol_error(
                        "http_response_header_name",
                        "HTTP response header name is not Text",
                    ));
                }
            };
            let value = match &fields[1].1 {
                NormalizedValue::Bytes(value) => value.to_vec(),
                _ => {
                    return Err(protocol_error(
                        "http_response_header_value",
                        "HTTP response header value is not Bytes",
                    ));
                }
            };
            total = total
                .checked_add(name.len())
                .and_then(|bytes| bytes.checked_add(value.len()))
                .ok_or_else(|| {
                    ExecutionError::resource(
                        "http_response_header_bytes",
                        "HTTP response header byte accounting overflowed",
                    )
                })?;
            if total > limits.maximum_header_bytes {
                return Err(ExecutionError::resource(
                    "http_response_header_bytes",
                    "HTTP response headers exceed the configured bytes",
                ));
            }
            Ok(HttpHeader { name, value })
        })
        .collect()
}

fn exact_structural<'a>(
    value: &'a NormalizedValue,
    expected: &[&str],
    subject: &'static str,
) -> Result<&'a [(Name, NormalizedValue)], ExecutionError> {
    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = value else {
        return Err(protocol_error(
            "http_response_value",
            format!("HTTP {subject} is not a structural record"),
        ));
    };
    if fields.len() != expected.len()
        || fields
            .iter()
            .zip(expected)
            .any(|((name, _), expected)| name.as_str() != *expected)
    {
        return Err(protocol_error(
            "http_response_value",
            format!("HTTP {subject} fields do not equal the exact current shape"),
        ));
    }
    Ok(fields)
}

fn protocol_error(code: &'static str, message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}

fn http_diagnostic(code: &'static str, message: &'static str) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}

fn http_corrupt(code: &'static str, message: &'static str) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}

fn http_io(code: &'static str, message: String) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Infrastructure, code, message)
}
