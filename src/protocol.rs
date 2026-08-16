use crate::error::LkError;
use crate::ids::{NodeId, Revision, WorkspaceId};
use crate::interpret::{RunPolicy, RunResult, RuntimeValue};
use crate::machine_contract::{DescribeSchemaRequest, DescribeSchemaResult};
use crate::query::{QueryBatchRequest, QueryBatchResult, WorkspaceSummary};
use crate::transaction::{ApplyTransactionRequest, TransactionReceipt};
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u16 = 6;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RequestCode {
    CreateWorkspace,
    ApplyTransaction,
    QueryBatch,
    Run,
    Shutdown,
    DescribeSchema,
}

impl RequestCode {
    pub const ALL: [Self; 6] = [
        Self::CreateWorkspace,
        Self::ApplyTransaction,
        Self::QueryBatch,
        Self::Run,
        Self::Shutdown,
        Self::DescribeSchema,
    ];

    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::CreateWorkspace => "create_workspace",
            Self::ApplyTransaction => "apply_transaction",
            Self::QueryBatch => "query_batch",
            Self::Run => "run",
            Self::Shutdown => "shutdown",
            Self::DescribeSchema => "describe_schema",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ResponseCode {
    WorkspaceCreated,
    TransactionReceipt,
    QueryBatchResult,
    Run,
    Acknowledged,
    Error,
    DescribeSchema,
}

impl ResponseCode {
    pub const ALL: [Self; 7] = [
        Self::WorkspaceCreated,
        Self::TransactionReceipt,
        Self::QueryBatchResult,
        Self::Run,
        Self::Acknowledged,
        Self::Error,
        Self::DescribeSchema,
    ];

    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::WorkspaceCreated => "workspace_created",
            Self::TransactionReceipt => "transaction_receipt",
            Self::QueryBatchResult => "query_batch_result",
            Self::Run => "run",
            Self::Acknowledged => "acknowledged",
            Self::Error => "error",
            Self::DescribeSchema => "describe_schema",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Request {
    CreateWorkspace,
    ApplyTransaction(ApplyTransactionRequest),
    QueryBatch(QueryBatchRequest),
    Run {
        workspace: WorkspaceId,
        revision: Revision,
        entry: NodeId,
        arguments: Vec<RuntimeValue>,
        policy: RunPolicy,
    },
    Shutdown,
    DescribeSchema(DescribeSchemaRequest),
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Response {
    WorkspaceCreated(WorkspaceSummary),
    TransactionReceipt(TransactionReceipt),
    QueryBatchResult(QueryBatchResult),
    Run(RunResult),
    Acknowledged,
    Error(LkError),
    DescribeSchema(Box<DescribeSchemaResult>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_codes_have_unique_names() {
        for (index, code) in RequestCode::ALL.into_iter().enumerate() {
            assert!(
                RequestCode::ALL[..index]
                    .iter()
                    .all(|prior| prior.machine_name() != code.machine_name())
            );
        }
        for (index, code) in ResponseCode::ALL.into_iter().enumerate() {
            assert!(
                ResponseCode::ALL[..index]
                    .iter()
                    .all(|prior| prior.machine_name() != code.machine_name())
            );
        }
    }

    #[test]
    fn logical_request_and_response_json_are_strict() {
        let request = serde_json::to_vec(&Request::CreateWorkspace).expect("request JSON");
        assert_eq!(
            serde_json::from_slice::<Request>(&request).expect("request decode"),
            Request::CreateWorkspace
        );
        assert!(
            serde_json::from_slice::<Request>(br#"{"kind":"create_workspace","unknown":true}"#)
                .is_err()
        );

        let response = serde_json::to_vec(&Response::Acknowledged).expect("response JSON");
        assert_eq!(
            serde_json::from_slice::<Response>(&response).expect("response decode"),
            Response::Acknowledged
        );
        assert!(
            serde_json::from_slice::<Response>(br#"{"kind":"acknowledged","unknown":null}"#)
                .is_err()
        );
    }
}
