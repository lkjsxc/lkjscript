//! Exact Graph 5 binding for the deployment-owned outbound HTTP client.

use super::capability::{NormalizedAdapterKind, NormalizedCallPolicy, NormalizedCapabilityAdapter};
use super::resource::NormalizedResourceScope;
use super::value::{NormalizedRecord, NormalizedValue};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::http::HttpHeader;
use crate::platform::http_client::HttpClient;
use crate::platform::kernel::{
    DeclarationReference, ExternalVisibility, Idempotency, Name, OperationReference,
};
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct NormalizedHttpClientAdapter {
    interface: DeclarationReference,
    get: OperationReference,
    operations: BTreeSet<OperationReference>,
    client: HttpClient,
}

impl NormalizedHttpClientAdapter {
    pub(crate) fn new(
        interface: DeclarationReference,
        get: OperationReference,
        client: HttpClient,
    ) -> Result<Self, Diagnostic> {
        if get.package != interface.package {
            return Err(Diagnostic::new(
                DiagnosticClass::Capability,
                "normalized_http_client_operation_package",
                "HTTP client operation must share the exact interface package",
            ));
        }
        Ok(Self {
            interface,
            get,
            operations: BTreeSet::from([get]),
            client,
        })
    }
}

impl NormalizedCapabilityAdapter for NormalizedHttpClientAdapter {
    fn kind(&self) -> NormalizedAdapterKind {
        NormalizedAdapterKind::HttpClient
    }

    fn interface(&self) -> DeclarationReference {
        self.interface
    }

    fn operations(&self) -> &BTreeSet<OperationReference> {
        &self.operations
    }

    fn call(
        &self,
        policy: &NormalizedCallPolicy,
        arguments: Vec<NormalizedValue>,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        if policy.grant.interface != self.interface
            || policy.operation != self.get
            || policy.idempotency != Idempotency::Idempotent
            || policy.external_visibility != ExternalVisibility::Possible
        {
            return Err(http_client_runtime(
                "normalized_http_client_policy",
                "HTTP client call policy disagrees with its exact interface operation",
            ));
        }
        let [NormalizedValue::List(headers)] = arguments.as_slice() else {
            return Err(http_client_argument(
                "HTTP client get expects one ordered header list",
            ));
        };
        let headers = headers
            .iter()
            .map(decode_header)
            .collect::<Result<Vec<_>, _>>()?;
        let response = self.client.get(headers, control)?;
        let headers = response
            .headers
            .into_iter()
            .map(|header| {
                structural_record([
                    ("name", NormalizedValue::text(header.name)),
                    ("value", NormalizedValue::bytes(header.value)),
                ])
            })
            .collect::<Result<Vec<_>, _>>()?;
        structural_record([
            ("body", NormalizedValue::bytes(response.body)),
            ("headers", NormalizedValue::List(Arc::new(headers))),
            ("status", NormalizedValue::I64(i64::from(response.status))),
        ])
    }

    fn shutdown(&self) -> Result<(), ExecutionError> {
        self.client.shutdown()
    }
}

fn decode_header(value: &NormalizedValue) -> Result<HttpHeader, ExecutionError> {
    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = value else {
        return Err(http_client_argument(
            "HTTP client request header is not a structural record",
        ));
    };
    let [
        (name_field, NormalizedValue::Text(name)),
        (value_field, NormalizedValue::Bytes(value)),
    ] = fields.as_slice()
    else {
        return Err(http_client_argument(
            "HTTP client request header has a foreign structural shape",
        ));
    };
    if name_field.as_str() != "name" || value_field.as_str() != "value" {
        return Err(http_client_argument(
            "HTTP client request header fields are not the exact current shape",
        ));
    }
    Ok(HttpHeader {
        name: name.to_string(),
        value: value.to_vec(),
    })
}

fn structural_record<const N: usize>(
    fields: [(&str, NormalizedValue); N],
) -> Result<NormalizedValue, ExecutionError> {
    let mut fields = fields
        .into_iter()
        .map(|(name, value)| {
            Name::new(name).map(|name| (name, value)).map_err(|_| {
                http_client_runtime(
                    "normalized_http_client_shape",
                    "HTTP client adapter could not construct its exact result shape",
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    fields.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(NormalizedValue::Record(NormalizedRecord::Structural {
        fields: Arc::new(fields),
    }))
}

fn http_client_argument(message: &'static str) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Capability,
        "http_client_argument",
        message,
    )
}

fn http_client_runtime(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}
