//! Exact command-local protocol for the foreground runtime session.

use crate::error::{LkError, Result};
use crate::instance::{
    HostExecutionReceipt, InstanceCreateReceipt, InstanceCreateRequest, InstanceDeleteRequest,
    InstanceEventRequest, InstanceFakeHostRequest, InstanceHistoryPage, InstanceHostRequest,
    InstanceId, InstanceInspection, InstanceResumeRequest, InstanceTransitionReceipt,
};
use crate::runtime::{RUNTIME_CONTRACT_VERSION, RuntimeInspection, RuntimeKernel};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRequestEnvelope {
    pub version: u16,
    pub request_id: u64,
    pub request: RuntimeRequest,
}

impl RuntimeRequestEnvelope {
    pub fn validate(&self) -> Result<()> {
        if self.version != RUNTIME_CONTRACT_VERSION {
            return Err(LkError::new(
                crate::ErrorCode::ProtocolMalformed,
                "runtime session contract version is unsupported",
            ));
        }
        if self.request_id == 0 {
            return Err(LkError::new(
                crate::ErrorCode::ProtocolMalformed,
                "runtime session request ID must be nonzero",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RuntimeRequest {
    Create {
        application: String,
        request: InstanceCreateRequest,
    },
    ValidateEvent(InstanceEventRequest),
    ApplyEvent(InstanceEventRequest),
    ExecuteHost(InstanceHostRequest),
    FakeOutcome(InstanceFakeHostRequest),
    ValidateResume(InstanceResumeRequest),
    Resume(InstanceResumeRequest),
    InspectInstance {
        instance: InstanceId,
    },
    History {
        instance: InstanceId,
        start_revision: u64,
        limit: usize,
    },
    Delete(InstanceDeleteRequest),
    InspectRuntime,
    Shutdown,
}

impl RuntimeRequest {
    pub fn execute(self, kernel: &mut RuntimeKernel) -> Result<RuntimeResponse> {
        match self {
            Self::Create {
                application,
                request,
            } => kernel
                .create_from_path(&request, Path::new(&application))
                .map(RuntimeResponse::Created),
            Self::ValidateEvent(request) => kernel
                .validate_event(&request)
                .map(RuntimeResponse::Transition),
            Self::ApplyEvent(request) => kernel
                .apply_event(&request)
                .map(RuntimeResponse::Transition),
            Self::ExecuteHost(request) => kernel
                .execute_host(&request)
                .map(RuntimeResponse::HostOutcome),
            Self::FakeOutcome(request) => kernel
                .record_fake_outcome(&request)
                .map(RuntimeResponse::HostOutcome),
            Self::ValidateResume(request) => kernel
                .validate_resume(&request)
                .map(RuntimeResponse::Transition),
            Self::Resume(request) => kernel.resume(&request).map(RuntimeResponse::Transition),
            Self::InspectInstance { instance } => kernel
                .inspect_instance(instance)
                .map(RuntimeResponse::Instance),
            Self::History {
                instance,
                start_revision,
                limit,
            } => kernel
                .history(instance, start_revision, limit)
                .map(RuntimeResponse::History),
            Self::Delete(request) => kernel.delete(request).map(RuntimeResponse::Instance),
            Self::InspectRuntime => Ok(RuntimeResponse::Runtime(kernel.inspection())),
            Self::Shutdown => Ok(RuntimeResponse::Shutdown),
        }
    }

    pub const fn requests_shutdown(&self) -> bool {
        matches!(self, Self::Shutdown)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RuntimeResponse {
    Created(InstanceCreateReceipt),
    Transition(InstanceTransitionReceipt),
    HostOutcome(HostExecutionReceipt),
    Instance(InstanceInspection),
    History(InstanceHistoryPage),
    Runtime(RuntimeInspection),
    Shutdown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeResponseEnvelope {
    pub version: u16,
    pub request_id: u64,
    pub response: RuntimeResponse,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeErrorEnvelope<'a> {
    pub version: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<u64>,
    pub error: &'a LkError,
}

pub fn success(request_id: u64, response: RuntimeResponse) -> RuntimeResponseEnvelope {
    RuntimeResponseEnvelope {
        version: RUNTIME_CONTRACT_VERSION,
        request_id,
        response,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::strict_json;

    #[test]
    fn request_envelope_is_closed_correlated_and_version_exact() {
        let request = RuntimeRequestEnvelope {
            version: RUNTIME_CONTRACT_VERSION,
            request_id: 7,
            request: RuntimeRequest::InspectRuntime,
        };
        let canonical = serde_json::to_vec(&request).expect("encode runtime request");
        assert_eq!(
            strict_json::<RuntimeRequestEnvelope>(&canonical, "runtime request")
                .expect("decode runtime request"),
            request
        );
        request.validate().expect("validate runtime request");

        let text = String::from_utf8(canonical).expect("runtime request UTF-8");
        for malformed in [
            text.replacen("\"version\":1", "\"version\":0", 1),
            text.replacen("\"request_id\":7", "\"request_id\":7,\"request_id\":7", 1),
            text.replacen("{\"version\":1", "{\"unknown\":0,\"version\":1", 1),
            text.replacen("inspect_runtime", "unknown_runtime_request", 1),
            format!("{text} {{}}"),
        ] {
            let decoded =
                strict_json::<RuntimeRequestEnvelope>(malformed.as_bytes(), "runtime request");
            if let Ok(decoded) = decoded {
                assert!(
                    decoded.validate().is_err(),
                    "accepted malformed {malformed}"
                );
            }
        }

        let zero = RuntimeRequestEnvelope {
            request_id: 0,
            ..request
        };
        assert!(zero.validate().is_err());
    }
}
