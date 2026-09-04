//! Signature-indexed structural HTTP boundary for normalized resident deployments.

use super::capability::NormalizedAdapterKind;
use super::prepare::{NormalizedEntryPoint, NormalizedHttpRoute};
use super::resident::NormalizedResidentDeployment;
use super::resource::NormalizedResourceScope;
use super::value::PortIndex;
use super::value::{NormalizedMapKey, NormalizedRecord, NormalizedValue};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionError, ExecutionFailureClass};
use crate::platform::http::{
    HTTP_ADAPTER_CONTRACT_VERSION, HttpDispatchObservation, HttpHeader, HttpLimits, HttpRequest,
    HttpResponse, HttpRuntimeObservation, HttpServerReceipt, decode_live_request,
    decode_query_parameters, encode_live_response, execution_error_status,
    finish_request_body_pump, pump_request_body, safe_error_response, semantic_http_types,
    static_response, validate_request,
};
use crate::platform::kernel::{
    HttpRoutePatternSegment, HttpRouteSelector, Name, ParameterUse, RequirementReference,
    TypeObjectInterner,
};
use crate::platform::package::RunnerKind;
use crate::platform::semantic_id::HttpRouteId;
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
    matcher: PreparedHttpMatcher,
}

#[derive(Clone)]
struct PreparedHttpMatcher {
    exact: BTreeMap<Arc<str>, BTreeMap<Arc<str>, usize>>,
    patterns: BTreeMap<Arc<str>, PatternTrie>,
    nodes: u64,
}

#[derive(Clone, Default)]
struct PatternTrie {
    nodes: Vec<PatternTrieNode>,
}

#[derive(Clone, Default)]
struct PatternTrieNode {
    literals: BTreeMap<Arc<str>, usize>,
    capture: Option<usize>,
    route: Option<usize>,
}

struct RouteSelection {
    route: Option<usize>,
    matcher_steps: u64,
}

struct SelectedRoute {
    route: HttpRouteId,
    port: PortIndex,
    captures: Vec<NormalizedValue>,
    capture_bytes: u64,
}

impl PreparedHttpMatcher {
    fn new(routes: &[NormalizedHttpRoute]) -> Result<Self, Diagnostic> {
        validate_matcher_route_set(routes)?;
        let mut exact = BTreeMap::<Arc<str>, BTreeMap<Arc<str>, usize>>::new();
        let mut patterns = BTreeMap::<Arc<str>, PatternTrie>::new();
        for (index, route) in routes.iter().enumerate() {
            match &route.selector {
                HttpRouteSelector::Exact { path } => {
                    if exact
                        .entry(route.method.clone())
                        .or_default()
                        .insert(Arc::from(path.as_str()), index)
                        .is_some()
                    {
                        return Err(http_corrupt(
                            "normalized_http_matcher_exact_duplicate",
                            "prepared exact HTTP index repeats one method/path",
                        ));
                    }
                }
                HttpRouteSelector::Pattern { segments } => {
                    patterns
                        .entry(route.method.clone())
                        .or_default()
                        .insert(segments, index)?;
                }
            }
        }
        let trie_nodes = patterns.values().try_fold(0usize, |total, trie| {
            total.checked_add(trie.nodes.len()).ok_or_else(|| {
                http_corrupt(
                    "normalized_http_matcher_nodes",
                    "prepared HTTP matcher node accounting overflowed",
                )
            })
        })?;
        let nodes = trie_nodes
            .checked_add(
                routes
                    .iter()
                    .filter(|route| matches!(route.selector, HttpRouteSelector::Exact { .. }))
                    .count(),
            )
            .ok_or_else(|| {
                http_corrupt(
                    "normalized_http_matcher_nodes",
                    "prepared HTTP matcher node accounting overflowed",
                )
            })?;
        let maximum_nodes =
            crate::platform::kernel::contract::MAXIMUM_HTTP_PATTERN_SEGMENTS_PER_TARGET
                .checked_add(crate::platform::kernel::contract::MAXIMUM_HTTP_ROUTES_PER_TARGET)
                .and_then(|value| {
                    value.checked_add(
                        crate::platform::kernel::contract::MAXIMUM_HTTP_ROUTES_PER_TARGET,
                    )
                })
                .ok_or_else(|| {
                    http_corrupt(
                        "normalized_http_matcher_nodes",
                        "HTTP matcher node bound overflowed",
                    )
                })?;
        if nodes > maximum_nodes {
            return Err(http_corrupt(
                "normalized_http_matcher_nodes",
                "prepared HTTP matcher exceeds its finite node bound",
            ));
        }
        Ok(Self {
            exact,
            patterns,
            nodes: u64::try_from(nodes).map_err(|_| {
                http_corrupt(
                    "normalized_http_matcher_nodes",
                    "prepared HTTP matcher node count is not representable",
                )
            })?,
        })
    }

    fn select(&self, method: &str, path: &str) -> RouteSelection {
        if let Some(route) = self
            .exact
            .get(method)
            .and_then(|paths| paths.get(path))
            .copied()
        {
            return RouteSelection {
                route: Some(route),
                matcher_steps: 1,
            };
        }
        let Some(trie) = self.patterns.get(method) else {
            return RouteSelection {
                route: None,
                matcher_steps: 1,
            };
        };
        let Some(path) = path.strip_prefix('/') else {
            return RouteSelection {
                route: None,
                matcher_steps: 1,
            };
        };
        let mut steps = 0u64;
        let route = trie.select(path.split('/'), &mut steps);
        RouteSelection {
            route,
            matcher_steps: steps.saturating_add(1),
        }
    }
}

impl PatternTrie {
    fn insert(
        &mut self,
        segments: &[HttpRoutePatternSegment],
        route: usize,
    ) -> Result<(), Diagnostic> {
        if self.nodes.is_empty() {
            self.nodes.push(PatternTrieNode::default());
        }
        let mut node = 0usize;
        for segment in segments {
            let next = match segment {
                HttpRoutePatternSegment::Literal(literal) => {
                    if let Some(next) = self.nodes[node].literals.get(literal.as_str()).copied() {
                        next
                    } else {
                        let next = self.nodes.len();
                        self.nodes.push(PatternTrieNode::default());
                        self.nodes[node]
                            .literals
                            .insert(Arc::from(literal.as_str()), next);
                        next
                    }
                }
                HttpRoutePatternSegment::Capture(_) => {
                    if let Some(next) = self.nodes[node].capture {
                        next
                    } else {
                        let next = self.nodes.len();
                        self.nodes.push(PatternTrieNode::default());
                        self.nodes[node].capture = Some(next);
                        next
                    }
                }
            };
            node = next;
        }
        if self.nodes[node].route.replace(route).is_some() {
            return Err(http_corrupt(
                "normalized_http_matcher_pattern_duplicate",
                "prepared HTTP matcher repeats one pattern language",
            ));
        }
        Ok(())
    }

    fn select<'a>(&self, segments: std::str::Split<'a, char>, steps: &mut u64) -> Option<usize> {
        self.select_node(0, segments, steps)
    }

    fn select_node<'a>(
        &self,
        node_index: usize,
        mut segments: std::str::Split<'a, char>,
        steps: &mut u64,
    ) -> Option<usize> {
        *steps = steps.saturating_add(1);
        let node = self.nodes.get(node_index)?;
        let Some(segment) = segments.next() else {
            return node.route;
        };
        if segment.is_empty() {
            return None;
        }
        if let Some(next) = node.literals.get(segment).copied()
            && let Some(route) = self.select_node(next, segments.clone(), steps)
        {
            return Some(route);
        }
        node.capture
            .and_then(|next| self.select_node(next, segments, steps))
    }
}

fn validate_matcher_route_set(routes: &[NormalizedHttpRoute]) -> Result<(), Diagnostic> {
    let mut route_bytes = 0usize;
    let mut pattern_segments = 0usize;
    let mut previous: Option<&NormalizedHttpRoute> = None;
    for route in routes {
        crate::platform::kernel::validate_http_route_method(&route.method).map_err(|_| {
            http_corrupt(
                "normalized_http_route_key",
                "normalized HTTP route contains an invalid method",
            )
        })?;
        route.selector.validate_local().map_err(|_| {
            http_corrupt(
                "normalized_http_route_selector",
                "normalized HTTP route contains an invalid selector",
            )
        })?;
        if let Some(previous) = previous
            && previous
                .method
                .as_bytes()
                .cmp(route.method.as_bytes())
                .then_with(|| {
                    crate::platform::kernel::http_route_selector_cmp(
                        &previous.selector,
                        &route.selector,
                    )
                })
                != std::cmp::Ordering::Less
        {
            return Err(http_corrupt(
                "normalized_http_route_order",
                "normalized HTTP routes are duplicate or noncanonical",
            ));
        }
        previous = Some(route);
        route_bytes = route_bytes
            .checked_add(route.method.len())
            .and_then(|value| value.checked_add(route.selector.key_bytes()))
            .ok_or_else(|| {
                http_corrupt(
                    "normalized_http_route_bytes",
                    "normalized HTTP route-key byte accounting overflowed",
                )
            })?;
        if let HttpRouteSelector::Pattern { segments } = &route.selector {
            pattern_segments = pattern_segments
                .checked_add(segments.len())
                .ok_or_else(|| {
                    http_corrupt(
                        "normalized_http_route_pattern_segments",
                        "normalized HTTP pattern-segment accounting overflowed",
                    )
                })?;
        }
    }
    if route_bytes > crate::platform::kernel::contract::MAXIMUM_HTTP_ROUTE_KEY_BYTES_PER_TARGET
        || pattern_segments
            > crate::platform::kernel::contract::MAXIMUM_HTTP_PATTERN_SEGMENTS_PER_TARGET
    {
        return Err(http_corrupt(
            "normalized_http_route_bounds",
            "normalized HTTP routes exceed aggregate selector bounds",
        ));
    }
    for (index, left) in routes.iter().enumerate() {
        for right in routes.iter().skip(index + 1) {
            if left.port == right.port
                && left.selector.capture_names() != right.selector.capture_names()
            {
                return Err(http_corrupt(
                    "normalized_http_route_shared_port_signature",
                    "normalized HTTP routes sharing a port disagree on capture names",
                ));
            }
            if left.method != right.method {
                continue;
            }
            match (&left.selector, &right.selector) {
                (
                    HttpRouteSelector::Exact { path: left },
                    HttpRouteSelector::Exact { path: right },
                ) if left == right => {
                    return Err(http_corrupt(
                        "normalized_http_route_duplicate_language",
                        "normalized exact HTTP routes repeat one match language",
                    ));
                }
                (
                    HttpRouteSelector::Pattern {
                        segments: left_segments,
                    },
                    HttpRouteSelector::Pattern {
                        segments: right_segments,
                    },
                ) if crate::platform::kernel::http_route_patterns_overlap(
                    left_segments,
                    right_segments,
                ) && !crate::platform::kernel::http_route_pattern_strictly_more_specific(
                    left_segments,
                    right_segments,
                ) && !crate::platform::kernel::http_route_pattern_strictly_more_specific(
                    right_segments,
                    left_segments,
                ) =>
                {
                    return Err(http_corrupt(
                        "normalized_http_route_overlap",
                        "normalized HTTP patterns overlap without strict specificity",
                    ));
                }
                _ => {}
            }
        }
    }
    Ok(())
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
        if resident.target().port.is_some()
            || resident.target().http_routes.is_empty()
            || resident.target().http_routes.len()
                > crate::platform::kernel::contract::MAXIMUM_HTTP_ROUTES_PER_TARGET
        {
            return Err(http_corrupt(
                "normalized_http_route_topology",
                "normalized HTTP target must have one bounded route set and no universal port",
            ));
        }
        let program = resident.program();
        let matcher = PreparedHttpMatcher::new(&resident.target().http_routes)?;
        for route in resident.target().http_routes.iter() {
            let port = program.ports.get(route.port.0 as usize).ok_or_else(|| {
                http_corrupt(
                    "normalized_http_route_port",
                    "selected HTTP route port escaped the exact runtime table",
                )
            })?;
            let expected_type = crate::platform::http::semantic_http_route_function_type(
                &mut TypeObjectInterner::default(),
                route.selector.capture_count(),
            )?;
            let function = match &port.entry {
                NormalizedEntryPoint::Function(function) => *function,
                _ => {
                    return Err(http_corrupt(
                        "normalized_http_handler_function",
                        "HTTP route port is not backed by one prepared function",
                    ));
                }
            };
            if port.component != resident.target().component
                || port.function_type != expected_type
                || function != route.function
            {
                return Err(http_diagnostic(
                    "normalized_http_handler_signature",
                    "each HTTP route must select its exact function-backed component port with the selector-indexed request/capture/response type",
                ));
            }
            let handler = program.functions.get(function.0 as usize).ok_or_else(|| {
                http_corrupt(
                    "normalized_http_handler_function",
                    "HTTP route function escaped the prepared function table",
                )
            })?;
            let http = semantic_http_types(&mut TypeObjectInterner::default())?;
            let captures = route.selector.capture_names();
            if handler.parameters.len() != captures.len().saturating_add(1)
                || handler
                    .parameters
                    .first()
                    .is_none_or(|parameter| parameter.ty != http.request_type)
                || handler.result != http.response_type
                || route.capture_parameters.len() != captures.len()
            {
                return Err(http_corrupt(
                    "normalized_http_handler_parameters",
                    "HTTP route handler parameters disagree with its selector",
                ));
            }
            for ((parameter, capture), expected_parameter) in handler
                .parameters
                .iter()
                .skip(1)
                .zip(&captures)
                .zip(route.capture_parameters.iter())
            {
                if parameter.parameter != *expected_parameter
                    || parameter.name.as_str() != capture.as_str()
                    || parameter.ty != http.text_type
                    || parameter.use_mode != ParameterUse::Unrestricted
                    || parameter.resource_requirement.is_some()
                {
                    return Err(http_corrupt(
                        "normalized_http_handler_capture",
                        "HTTP route capture disagrees with its prepared handler parameter",
                    ));
                }
            }
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
            matcher,
        })
    }

    pub(crate) async fn dispatch(
        &self,
        mut request: HttpRequest,
    ) -> Result<(HttpResponse, HttpDispatchObservation), ExecutionError> {
        validate_request(&request, &self.limits)?;
        let method_is_head = request.method == "HEAD";
        let (selected, matcher_steps) = self.select_route(&request.method, &request.path)?;
        let Some(selected) = selected else {
            return Ok((
                unmatched_response(),
                HttpDispatchObservation {
                    route: None,
                    matcher_steps,
                    captures: 0,
                    capture_bytes: 0,
                    task_id: None,
                    queue_nanoseconds: 0,
                    execution_nanoseconds: 0,
                    instructions: 0,
                },
            ));
        };
        let query_parameters = decode_query_parameters(&request.query)?;
        let resources = NormalizedResourceScope::new()?;
        let body = self.resident.deployment().register_memory_stream(
            self.stream_requirement,
            &resources,
            std::mem::take(&mut request.body),
        )?;
        let request = request_value(request, query_parameters, body)?;
        let capture_count = u64::try_from(selected.captures.len()).map_err(|_| {
            ExecutionError::resource(
                "http_route_capture_count",
                "HTTP route capture count is not representable",
            )
        })?;
        let mut arguments = Vec::with_capacity(selected.captures.len().saturating_add(1));
        arguments.push(request);
        arguments.extend(selected.captures);
        let receipt = self
            .resident
            .invoke_port_scoped(resources, selected.port, arguments)
            .await?;
        let mut response = response_value(receipt.value, &self.limits)?;
        if method_is_head {
            response.body.clear();
        }
        Ok((
            response,
            HttpDispatchObservation {
                route: Some(selected.route),
                matcher_steps,
                captures: capture_count,
                capture_bytes: selected.capture_bytes,
                task_id: Some(receipt.task_id),
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
        let matcher_nodes = self.matcher.nodes;
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
        let resident_observation = resident.observe();
        let permit_observation = resident.observe_permits();
        let runtime = HttpRuntimeObservation {
            resident: resident_observation,
            admission_permits: permit_observation.admission_permits,
            maximum_admission_permits: permit_observation.maximum_admission_permits,
            worker_permits: permit_observation.worker_permits,
            maximum_worker_permits: permit_observation.maximum_worker_permits,
        };
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
            if runtime.admission_permits != 0 || runtime.worker_permits != 0 {
                error.notes.push(format!(
                    "{} admission permits and {} worker permits remained after server failure cleanup",
                    runtime.admission_permits, runtime.worker_permits
                ));
            }
            return Err(error);
        }
        if shutdown.remaining_tasks != 0
            || !shutdown.cleanup_failures.is_empty()
            || runtime.resident.queued != 0
            || runtime.resident.active != 0
            || runtime.admission_permits != 0
            || runtime.worker_permits != 0
        {
            return Err(http_io(
                "normalized_http_shutdown_incomplete",
                format!(
                    "{} resident tasks, {} admission permits, {} worker permits, and {} cleanup failures remained after normalized HTTP shutdown",
                    shutdown.remaining_tasks,
                    runtime.admission_permits,
                    runtime.worker_permits,
                    shutdown.cleanup_failures.len()
                ),
            ));
        }
        Ok(HttpServerReceipt {
            contract_version: HTTP_ADAPTER_CONTRACT_VERSION,
            local_address: local_address.to_string(),
            accepted_at_transport: true,
            matcher_nodes,
            runtime,
            shutdown,
        })
    }

    pub(crate) fn resident(&self) -> &NormalizedResidentDeployment {
        &self.resident
    }

    fn select_route(
        &self,
        method: &str,
        path: &str,
    ) -> Result<(Option<SelectedRoute>, u64), ExecutionError> {
        let selection = self.matcher.select(method, path);
        if selection.matcher_steps > self.matcher.nodes.saturating_add(1) {
            return Err(protocol_error(
                "http_route_match_work",
                "HTTP route matcher exceeded its prepared node bound",
            ));
        }
        let Some(index) = selection.route else {
            return Ok((None, selection.matcher_steps));
        };
        let route = self
            .resident
            .target()
            .http_routes
            .get(index)
            .ok_or_else(|| {
                protocol_error(
                    "http_route_match_leaf",
                    "HTTP route matcher selected a foreign prepared leaf",
                )
            })?;
        let (captures, capture_bytes) = capture_arguments(&route.selector, path)?;
        Ok((
            Some(SelectedRoute {
                route: route.route,
                port: route.port,
                captures,
                capture_bytes,
            }),
            selection.matcher_steps,
        ))
    }
}

fn capture_arguments(
    selector: &HttpRouteSelector,
    path: &str,
) -> Result<(Vec<NormalizedValue>, u64), ExecutionError> {
    let HttpRouteSelector::Pattern { segments } = selector else {
        return Ok((Vec::new(), 0));
    };
    let path = path.strip_prefix('/').ok_or_else(|| {
        protocol_error(
            "http_route_capture_path",
            "matched HTTP pattern received a path without its leading slash",
        )
    })?;
    let mut values = path.split('/');
    let mut captures = Vec::with_capacity(selector.capture_count());
    let mut bytes = 0u64;
    for segment in segments {
        let value = values.next().ok_or_else(|| {
            protocol_error(
                "http_route_capture_path",
                "matched HTTP pattern received too few path segments",
            )
        })?;
        match segment {
            HttpRoutePatternSegment::Literal(literal) if literal != value => {
                return Err(protocol_error(
                    "http_route_capture_literal",
                    "matched HTTP pattern literal drifted during capture construction",
                ));
            }
            HttpRoutePatternSegment::Capture(_) => {
                let length = u64::try_from(value.len()).map_err(|_| {
                    ExecutionError::resource(
                        "http_route_capture_bytes",
                        "HTTP capture byte length is not representable",
                    )
                })?;
                bytes = bytes.checked_add(length).ok_or_else(|| {
                    ExecutionError::resource(
                        "http_route_capture_bytes",
                        "HTTP capture byte accounting overflowed",
                    )
                })?;
                captures.push(NormalizedValue::text(value.to_owned()));
            }
            HttpRoutePatternSegment::Literal(_) => {}
        }
    }
    if values.next().is_some() || captures.len() != selector.capture_count() {
        return Err(protocol_error(
            "http_route_capture_path",
            "matched HTTP pattern path or capture count drifted during construction",
        ));
    }
    Ok((captures, bytes))
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
    if let Err(error) = validate_request(&request, &application.limits) {
        return safe_error_response(&error, execution_error_status(&error));
    }
    let (selected, _matcher_steps) = match application.select_route(&request.method, &request.path)
    {
        Ok(selection) => selection,
        Err(error) => {
            return safe_error_response(&error, StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let Some(selected) = selected else {
        drop(body);
        return encode_live_response(unmatched_response()).unwrap_or_else(|error| {
            safe_error_response(&error, StatusCode::INTERNAL_SERVER_ERROR)
        });
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
    let mut arguments = Vec::with_capacity(selected.captures.len().saturating_add(1));
    arguments.push(request);
    arguments.extend(selected.captures);
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
        .invoke_port_scoped(resources, selected.port, arguments)
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

fn unmatched_response() -> HttpResponse {
    HttpResponse {
        status: 404,
        headers: Vec::new(),
        body: Vec::new(),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::execution::normalized::value::FunctionIndex;

    #[test]
    fn prepared_matcher_uses_exact_then_comparable_pattern_specificity() {
        let route = |ordinal: u64, selector: HttpRouteSelector, port: u32| -> NormalizedHttpRoute {
            NormalizedHttpRoute {
                route: HttpRouteId::migrate(b"prepared-http-specificity", ordinal),
                method: Arc::from("GET"),
                selector,
                port: PortIndex(port),
                function: FunctionIndex(port),
                capture_parameters: Arc::from([]),
            }
        };
        let routes = vec![
            route(
                0,
                HttpRouteSelector::exact("/api/posts/featured").unwrap(),
                0,
            ),
            route(
                1,
                HttpRouteSelector::parse_pattern("/api/posts/{id}").unwrap(),
                1,
            ),
            route(
                2,
                HttpRouteSelector::parse_pattern("/api/{category}/{id}").unwrap(),
                2,
            ),
            route(
                3,
                HttpRouteSelector::parse_pattern("/{scope}/{category}/{id}").unwrap(),
                3,
            ),
        ];
        let matcher = PreparedHttpMatcher::new(&routes).unwrap();

        for (path, expected) in [
            ("/api/posts/featured", Some(0)),
            ("/api/posts/42", Some(1)),
            ("/api/users/42", Some(2)),
            ("/other/users/42", Some(3)),
            ("/api/users", None),
        ] {
            let selection = matcher.select("GET", path);
            assert_eq!(selection.route, expected, "{path}");
            assert!((1..=matcher.nodes.saturating_add(1)).contains(&selection.matcher_steps));
        }
    }
}
