#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use lkjscript::diff::{ChangeKind, ScalarValue};
use lkjscript::interpret::{RunResult, RuntimeValue};
use lkjscript::machine::{
    JSON_ENVELOPE_VERSION, MAX_JSON_INPUT_BYTES, RequestEnvelope, decode_request, encode_response,
};
use lkjscript::query::*;
use lkjscript::schema::*;
use lkjscript::transaction::*;
use lkjscript::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Command, Stdio};

fn round_trip<T>(value: &T)
where
    T: Serialize + DeserializeOwned + Eq + std::fmt::Debug,
{
    let compact = serde_json::to_vec(value).expect("encode");
    assert!(!compact.contains(&b'\n'));
    assert_eq!(
        serde_json::from_slice::<T>(&compact).expect("decode"),
        *value
    );
}

fn ids() -> (WorkspaceId, NodeId, NodeId) {
    let workspace = WorkspaceId::from_bytes([0x12; 16]);
    (
        workspace,
        NodeId::new(workspace, 1).expect("node 1"),
        NodeId::new(workspace, 2).expect("node 2"),
    )
}

#[test]
fn every_closed_machine_variant_round_trips() {
    let (workspace, first, second) = ids();
    let existing = NodeTarget::Existing(first);
    let local = NodeTarget::Local(LocalHandle::new(7));
    let value = ValueDraft::OperationResult {
        operation: existing,
        output: 0,
    };
    for target in [existing, local] {
        round_trip(&target);
    }
    for value in [ValueDraft::FunctionParameter(existing), value] {
        round_trip(&value);
    }
    for value in [
        ValueRef::FunctionParameter(first),
        ValueRef::OperationResult {
            operation: first,
            output: 0,
        },
    ] {
        round_trip(&value);
    }
    let drafts = vec![
        OperationDraft::ConstI64(1),
        OperationDraft::ConstBool(true),
        OperationDraft::AddI64 {
            lhs: value,
            rhs: value,
        },
        OperationDraft::Hole {
            expected: SemanticType::I64,
        },
        OperationDraft::Return { value },
    ];
    for draft in &drafts {
        round_trip(draft);
    }
    for kind in [
        OperationKind::ConstI64(1),
        OperationKind::ConstBool(true),
        OperationKind::AddI64 {
            lhs: ValueRef::OperationResult {
                operation: first,
                output: 0,
            },
            rhs: ValueRef::OperationResult {
                operation: second,
                output: 0,
            },
        },
        OperationKind::Hole {
            expected: SemanticType::I64,
        },
        OperationKind::Return {
            value: ValueRef::OperationResult {
                operation: first,
                output: 0,
            },
        },
    ] {
        round_trip(&kind);
    }
    for reference in [
        DirectReference::Definition { target: first },
        DirectReference::ValueOperand {
            index: 0,
            value: ValueRef::OperationResult {
                operation: first,
                output: 0,
            },
        },
    ] {
        round_trip(&reference);
    }
    let operations = vec![
        TransactionOp::CreatePackage {
            handle: LocalHandle::new(1),
            name: "p".to_owned(),
        },
        TransactionOp::CreateModule {
            handle: LocalHandle::new(2),
            package: existing,
            name: "m".to_owned(),
        },
        TransactionOp::CreateFunction {
            handle: LocalHandle::new(3),
            module: existing,
            name: "f".to_owned(),
            result: SemanticType::I64,
        },
        TransactionOp::CreateParameter {
            handle: LocalHandle::new(4),
            function: existing,
            name: "x".to_owned(),
            ty: SemanticType::Bool,
        },
        TransactionOp::CreateRegion {
            handle: LocalHandle::new(5),
            function: existing,
        },
        TransactionOp::CreateBlock {
            handle: LocalHandle::new(6),
            region: existing,
        },
        TransactionOp::CreateOperation {
            handle: LocalHandle::new(7),
            block: existing,
            before: Some(local),
            operation: drafts[2].clone(),
        },
        TransactionOp::SetFunctionBody {
            function: existing,
            region: local,
        },
        TransactionOp::SetEntryFunction {
            package: existing,
            function: local,
        },
        TransactionOp::RenameNode {
            node: existing,
            name: "renamed".to_owned(),
        },
        TransactionOp::ReplaceOperation {
            operation: existing,
            replacement: drafts[0].clone(),
        },
        TransactionOp::ReplaceOperand {
            operation: existing,
            index: 1,
            value,
        },
        TransactionOp::DeleteOwnedSubtree { root: existing },
        TransactionOp::RefineHole {
            hole: existing,
            replacement: drafts[2].clone(),
        },
    ];
    assert_eq!(operations.len(), TransactionOpCode::ALL.len());
    for (operation, code) in operations.iter().zip(TransactionOpCode::ALL) {
        round_trip(operation);
        let json = serde_json::to_string(operation).expect("transaction operation JSON name");
        assert!(json.contains(&format!("\"kind\":\"{}\"", code.machine_name())));
    }

    let repair = RepairTarget::Operand {
        operation: first,
        index: 1,
    };
    let cursors = vec![
        PageCursor::Blockers {
            workspace,
            revision: Revision::new(1),
            next: 1,
        },
        PageCursor::OwnerChain {
            workspace,
            revision: Revision::new(1),
            node: first,
            next: 1,
        },
        PageCursor::Body {
            workspace,
            revision: Revision::new(1),
            block: first,
            next: 1,
        },
        PageCursor::IncomingUses {
            workspace,
            revision: Revision::new(1),
            value: ValueRef::OperationResult {
                operation: first,
                output: 0,
            },
            next: 1,
        },
        PageCursor::DefinitionReferences {
            workspace,
            revision: Revision::new(1),
            target: first,
            next: 1,
        },
        PageCursor::Dependencies {
            workspace,
            revision: Revision::new(1),
            node: first,
            next: 1,
        },
        PageCursor::VisibleValues {
            workspace,
            revision: Revision::new(1),
            purpose: VisibleCursorPurpose::VisibleValues,
            target: repair,
            expected: SemanticType::I64,
            include_incompatible: true,
            next: 1,
        },
        PageCursor::Diff {
            workspace,
            from: Revision::new(1),
            to: Revision::new(2),
            next: 1,
        },
    ];
    for cursor in &cursors {
        round_trip(cursor);
    }
    for target in [RepairTarget::Hole(first), repair] {
        round_trip(&target);
    }
    for literal in [
        LiteralValue::I64(1),
        LiteralValue::Bool(true),
        LiteralValue::ExpectedType(SemanticType::I64),
    ] {
        round_trip(&literal);
    }
    for dependency in [
        DependencyFact::ValueOperand {
            index: 0,
            value: ValueRef::OperationResult {
                operation: first,
                output: 0,
            },
        },
        DependencyFact::Definition {
            slot: DefinitionSlot::PackageEntry,
            target: first,
        },
    ] {
        round_trip(&dependency);
    }
    let page = |after| PageRequest { after, limit: 2 };
    let queries = vec![
        Query::WorkspaceSummary,
        Query::Node {
            node: first,
            expand: true,
        },
        Query::Blockers { page: page(None) },
        Query::OwnerChain {
            node: first,
            page: page(None),
        },
        Query::Body {
            block: first,
            page: page(None),
        },
        Query::IncomingUses {
            value: ValueRef::OperationResult {
                operation: first,
                output: 0,
            },
            page: page(None),
        },
        Query::DefinitionReferences {
            target: first,
            page: page(None),
        },
        Query::Dependencies {
            node: first,
            page: page(None),
        },
        Query::VisibleValues {
            purpose: VisibleCursorPurpose::VisibleValues,
            target: repair,
            include_incompatible: true,
            page: page(None),
        },
        Query::LegalConstructors {
            target: repair,
            include_incompatible: true,
            values: page(None),
        },
        Query::SemanticDiff {
            from: Revision::INITIAL,
            page: page(None),
        },
        Query::RepairContext {
            target: repair,
            budget: ContextBudget {
                body_before: 1,
                body_after: 1,
                visible_values: 1,
                incoming_uses: 1,
                include_incompatible: true,
            },
        },
    ];
    assert_eq!(queries.len(), QueryCode::ALL.len());
    for (query, code) in queries.iter().zip(QueryCode::ALL) {
        round_trip(query);
        let json = serde_json::to_string(query).expect("query JSON name");
        assert!(json.contains(&format!("\"kind\":\"{}\"", code.machine_name())));
    }

    let nodes = vec![
        Node::WorkspaceRoot {
            packages: vec![first],
        },
        Node::Package {
            owner: first,
            name: "p".to_owned(),
            modules: vec![second],
            entry: Some(second),
        },
        Node::Module {
            owner: first,
            name: "m".to_owned(),
            functions: vec![second],
        },
        Node::Function {
            owner: first,
            name: "f".to_owned(),
            parameters: vec![second],
            result: SemanticType::I64,
            body: Some(second),
        },
        Node::Parameter {
            owner: first,
            ordinal: 0,
            name: "x".to_owned(),
            ty: SemanticType::Bool,
        },
        Node::Region {
            owner: first,
            blocks: vec![second],
        },
        Node::Block {
            owner: first,
            operations: vec![second],
            terminator: Some(second),
        },
        Node::Operation {
            owner: first,
            operation: OperationKind::Hole {
                expected: SemanticType::I64,
            },
        },
    ];
    for node in &nodes {
        round_trip(node);
    }

    let changes = vec![
        ChangeKind::Created {
            kind: NodeKind::Operation,
        },
        ChangeKind::Deleted {
            kind: NodeKind::Operation,
        },
        ChangeKind::Renamed {
            before: "a".to_owned(),
            after: "b".to_owned(),
        },
        ChangeKind::ScalarAttributeChanged {
            before: ScalarValue::I64(1),
            after: ScalarValue::I64(2),
        },
        ChangeKind::ContainmentChanged {
            before_count: 1,
            after_count: 2,
        },
        ChangeKind::OperandChanged {
            index: 0,
            before: None,
            after: Some(ValueRef::OperationResult {
                operation: first,
                output: 0,
            }),
        },
        ChangeKind::EntryFunctionChanged {
            before: None,
            after: Some(first),
        },
        ChangeKind::CompletenessChanged { complete: true },
        ChangeKind::OperationRefined {
            before: OperationCode::Hole,
            after: OperationCode::ConstI64,
            result_type: SemanticType::I64,
            replacement: OperationKind::ConstI64(1),
        },
        ChangeKind::AllocatedAndTombstoned,
    ];
    for change in &changes {
        round_trip(change);
    }
    for scalar in [
        ScalarValue::I64(1),
        ScalarValue::Bool(true),
        ScalarValue::Type(SemanticType::I64),
    ] {
        round_trip(&scalar);
    }
    for value in [
        RuntimeValue::Unit,
        RuntimeValue::Bool(true),
        RuntimeValue::I64(1),
    ] {
        round_trip(&value);
    }

    let empty = |next| Page::<CompletenessBlocker> {
        items: Vec::new(),
        next,
        total: Some(0),
    };
    let summary = WorkspaceSummary {
        workspace,
        revision: Revision::new(1),
        hash: SnapshotHash::from_bytes([1; 32]),
        root: first,
        node_count: 2,
        complete: true,
        blocker_count: 0,
        entry_count: 1,
    };
    let node_summary = NodeSummary {
        workspace,
        revision: Revision::new(1),
        node: first,
        kind: NodeKind::Operation,
        owner: Some(second),
        display_name: None,
        signature: None,
        value_type: Some(SemanticType::I64),
        complete: true,
        blocker_count: 0,
        child_count: 0,
        outgoing_reference_count: 0,
    };
    let repair_context = RepairContext {
        workspace,
        revision: Revision::new(1),
        target: RepairTarget::Hole(first),
        operation: first,
        operation_code: OperationCode::Hole,
        operand_index: None,
        expected_type: SemanticType::I64,
        use_mode: None,
        current_value: None,
        current_actual_type: None,
        owner_block: second,
        owner_function: second,
        ordinal: 0,
        function_signature: FunctionSignatureSummary {
            parameter_count: 0,
            result: SemanticType::I64,
        },
        owner_chain: Vec::new(),
        body_window: Vec::new(),
        visible_values: Page {
            items: Vec::new(),
            next: None,
            total: Some(0),
        },
        incoming_uses: Page {
            items: Vec::new(),
            next: None,
            total: Some(0),
        },
        legal_constructors: Vec::new(),
        blocker: None,
        refinement_operation: Some(TransactionOpCode::RefineHole),
    };
    let query_results = vec![
        QueryResult::WorkspaceSummary(summary.clone()),
        QueryResult::Node(NodeView {
            summary: node_summary,
            record: Some(Node::Operation {
                owner: second,
                operation: OperationKind::ConstI64(1),
            }),
        }),
        QueryResult::Blockers(empty(None)),
        QueryResult::OwnerChain(Page::<OwnerFact> {
            items: Vec::new(),
            next: None,
            total: Some(0),
        }),
        QueryResult::Body(Page::<BodyItem> {
            items: Vec::new(),
            next: None,
            total: Some(0),
        }),
        QueryResult::IncomingUses(Page::<UseSite> {
            items: Vec::new(),
            next: None,
            total: Some(0),
        }),
        QueryResult::DefinitionReferences(Page::<DefinitionReferenceSite> {
            items: Vec::new(),
            next: None,
            total: Some(0),
        }),
        QueryResult::Dependencies(Page::<DependencyFact> {
            items: Vec::new(),
            next: None,
            total: Some(0),
        }),
        QueryResult::VisibleValues(Page::<VisibleValue> {
            items: Vec::new(),
            next: None,
            total: Some(0),
        }),
        QueryResult::LegalConstructors(LegalConstructorsResult {
            target: RepairTarget::Hole(first),
            expected_type: SemanticType::I64,
            constructors: Vec::new(),
            visible_values: Page {
                items: Vec::new(),
                next: None,
                total: Some(0),
            },
        }),
        QueryResult::SemanticDiff(SemanticDiffPage {
            from: Revision::INITIAL,
            to: Revision::new(1),
            change_count: 0,
            change_digest: ChangeDigest::from_bytes([0; 32]),
            page: Page {
                items: Vec::new(),
                next: None,
                total: Some(0),
            },
        }),
        QueryResult::RepairContext(Box::new(repair_context)),
    ];
    for result in &query_results {
        round_trip(result);
    }

    let requests = [
        Request::CreateWorkspace,
        Request::ApplyTransaction(ApplyTransactionRequest {
            transaction: Transaction {
                workspace,
                base_revision: Revision::INITIAL,
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations,
            },
            response: TransactionResponseSpec {
                return_handles: vec![LocalHandle::new(1)],
            },
        }),
        Request::QueryBatch(QueryBatchRequest {
            workspace,
            revision: Revision::new(1),
            queries: queries
                .into_iter()
                .enumerate()
                .map(|(index, query)| QueryItem {
                    id: QueryId::new(index as u64 + 1),
                    query,
                })
                .collect(),
        }),
        Request::Run {
            workspace,
            revision: Revision::new(1),
            entry: first,
        },
        Request::Shutdown,
        Request::DescribeSchema,
    ];
    assert_eq!(requests.len(), RequestCode::ALL.len());
    for (request, code) in requests.iter().zip(RequestCode::ALL) {
        round_trip(request);
        let json = serde_json::to_string(request).expect("request JSON name");
        assert!(json.contains(&format!("\"kind\":\"{}\"", code.machine_name())));
        let envelope = RequestEnvelope {
            version: JSON_ENVELOPE_VERSION,
            request_id: RequestId::new(1),
            request: request.clone(),
        };
        round_trip(&envelope);
    }

    let responses = [
        Response::WorkspaceCreated(summary.clone()),
        Response::TransactionReceipt(TransactionReceipt {
            workspace,
            base_revision: Revision::INITIAL,
            revision: Revision::new(1),
            hash: SnapshotHash::from_bytes([2; 32]),
            published: true,
            created_count: 1,
            returned_bindings: vec![(LocalHandle::new(1), first)],
            change_count: 1,
            change_digest: ChangeDigest::from_bytes([3; 32]),
            complete_before: false,
            complete_after: true,
            blocker_count_before: 1,
            blocker_count_after: 0,
        }),
        Response::QueryBatchResult(QueryBatchResult {
            workspace,
            revision: Revision::new(1),
            results: vec![
                QueryItemResult {
                    id: QueryId::new(1),
                    outcome: QueryOutcome::Success(Box::new(QueryResult::WorkspaceSummary(
                        summary,
                    ))),
                },
                QueryItemResult {
                    id: QueryId::new(2),
                    outcome: QueryOutcome::Error(LkError::new(ErrorCode::NodeNotFound, "missing")),
                },
            ],
        }),
        Response::Run(RunResult {
            value: RuntimeValue::I64(42),
            compile_nanoseconds: 1,
            execute_nanoseconds: 2,
        }),
        Response::Acknowledged,
        Response::Error(LkError::new(ErrorCode::InvalidOperand, "invalid")),
        Response::SchemaDescription(Box::new(lkjscript::machine::schema_description())),
    ];
    assert_eq!(responses.len(), ResponseCode::ALL.len());
    for (response, code) in responses.iter().zip(ResponseCode::ALL) {
        round_trip(response);
        let json = serde_json::to_string(response).expect("response JSON name");
        assert!(json.contains(&format!("\"kind\":\"{}\"", code.machine_name())));
        let bytes = encode_response(RequestId::new(9), response, false).expect("response envelope");
        let decoded: lkjscript::machine::ResponseEnvelope =
            serde_json::from_slice(&bytes).expect("response decode");
        assert_eq!(decoded.response, *response);
    }
}

#[test]
fn strict_json_rejects_malformed_shapes_values_and_limits() {
    let workspace = WorkspaceId::from_bytes([0xab; 16]);
    let valid = format!(
        "{{\"version\":2,\"request_id\":1,\"request\":{{\"kind\":\"query_batch\",\"data\":{{\"workspace\":\"{workspace}\",\"revision\":0,\"queries\":[{{\"id\":1,\"query\":{{\"kind\":\"blockers\",\"data\":{{\"page\":{{\"limit\":1}}}}}}}}]}}}}}}"
    );
    assert!(decode_request(valid.as_bytes()).is_ok());
    let invalid = [
        valid.replacen("\"version\":2", "\"version\":3", 1),
        valid.replacen("\"request_id\":1", "\"request_id\":0", 1),
        valid.replacen("\"request_id\":1", "\"request_id\":-1", 1),
        valid.replacen("\"request_id\":1", "\"request_id\":18446744073709551616", 1),
        valid.replacen("\"workspace\":", "\"unknown\":0,\"workspace\":", 1),
        valid.replacen("\"query\":", "\"extra\":0,\"query\":", 1),
        valid.replacen("\"page\":", "\"extra\":0,\"page\":", 1),
        valid.replacen("\"kind\":\"blockers\"", "\"kind\":\"unknown\"", 1),
        valid.replacen(
            "\"kind\":\"query_batch\"",
            "\"kind\":\"query_batch\",\"kind\":\"query_batch\"",
            1,
        ),
        valid.replacen("\"request\":", "\"request\":{},\"request\":", 1),
        valid.replacen(
            &workspace.to_string(),
            &workspace.to_string().to_uppercase(),
            1,
        ),
        format!("{valid}{{}}"),
        "[]".to_owned(),
        "{\"version\":2}".to_owned(),
    ];
    for input in invalid {
        assert!(
            decode_request(input.as_bytes()).is_err(),
            "accepted: {input}"
        );
    }
    let node = NodeId::new(workspace, 1).expect("node");
    let strict_nested = [
        format!("{{\"kind\":\"existing\",\"data\":\"{node}\",\"extra\":0}}"),
        format!(
            "{{\"kind\":\"operation_result\",\"data\":{{\"operation\":{{\"kind\":\"existing\",\"data\":\"{node}\"}},\"output\":0,\"extra\":0}}}}"
        ),
        format!(
            "{{\"kind\":\"rename_node\",\"data\":{{\"node\":{{\"kind\":\"existing\",\"data\":\"{node}\"}},\"name\":\"x\",\"extra\":0}}}}"
        ),
        "{\"kind\":\"blockers\",\"data\":{\"page\":{\"limit\":1},\"extra\":0}}".to_owned(),
        format!(
            "{{\"kind\":\"body\",\"data\":{{\"workspace\":\"{workspace}\",\"revision\":1,\"block\":\"{node}\",\"next\":1,\"extra\":0}}}}"
        ),
    ];
    assert!(serde_json::from_str::<NodeTarget>(&strict_nested[0]).is_err());
    assert!(serde_json::from_str::<ValueDraft>(&strict_nested[1]).is_err());
    assert!(serde_json::from_str::<TransactionOp>(&strict_nested[2]).is_err());
    assert!(serde_json::from_str::<Query>(&strict_nested[3]).is_err());
    assert!(serde_json::from_str::<PageCursor>(&strict_nested[4]).is_err());
    assert!(serde_json::from_str::<Transaction>(&format!("{{\"workspace\":\"{workspace}\",\"base_revision\":0,\"mode\":\"commit\",\"operations\":[],\"extra\":0}}")).is_err());
    assert!(serde_json::from_str::<LocalHandle>("4294967296").is_err());
    assert!(serde_json::from_str::<Revision>("-1").is_err());
    assert!(serde_json::from_str::<IdempotencyKey>(&format!("\"{}\"", "A".repeat(32))).is_err());
    assert!(serde_json::from_str::<ChangeDigest>("\"00\"").is_err());
    let leading_node = format!("\"{workspace}:01\"");
    assert!(serde_json::from_str::<NodeId>(&leading_node).is_err());
    let zero_node = format!("\"{workspace}:0\"");
    assert!(serde_json::from_str::<NodeId>(&zero_node).is_err());
    assert!(serde_json::from_str::<SnapshotHash>(&format!("\"{}\"", "A".repeat(64))).is_err());
    assert!(decode_request(&vec![b' '; MAX_JSON_INPUT_BYTES + 1]).is_err());

    let nested_request = |depth: usize| {
        format!(
            "{{\"version\":2,\"request_id\":1,\"request\":{{\"data\":{}{},\"kind\":\"create_workspace\"}}}}",
            "[".repeat(depth),
            "]".repeat(depth)
        )
    };
    let below = decode_request(nested_request(80).as_bytes()).expect_err("unexpected data");
    assert!(!below.message.contains("recursion limit exceeded"));
    let beyond = decode_request(nested_request(160).as_bytes()).expect_err("recursion limit");
    assert!(
        beyond.message.contains("recursion limit exceeded"),
        "{}",
        beyond.message
    );
}

#[test]
fn cli_enforces_bounded_one_value_exit_and_pretty_contracts() {
    let usage = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .output()
        .expect("usage output");
    assert_eq!(usage.status.code(), Some(2));
    assert!(!usage.stderr.is_empty());
    assert_one_json(&usage.stdout);

    let compact = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .arg("schema")
        .output()
        .expect("compact schema");
    let pretty = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .args(["schema", "--pretty"])
        .output()
        .expect("pretty schema");
    assert!(compact.status.success() && pretty.status.success());
    assert!(compact.stderr.is_empty() && pretty.stderr.is_empty());
    assert!(!compact.stdout[..compact.stdout.len() - 1].contains(&b'\n'));
    assert!(pretty.stdout[..pretty.stdout.len() - 1].contains(&b'\n'));
    assert_eq!(
        serde_json::from_slice::<lkjscript::machine::SchemaDescription>(&compact.stdout)
            .expect("compact schema JSON"),
        serde_json::from_slice::<lkjscript::machine::SchemaDescription>(&pretty.stdout)
            .expect("pretty schema JSON")
    );

    for (size, expected_kind) in [
        (MAX_JSON_INPUT_BYTES, "invalid_json"),
        (MAX_JSON_INPUT_BYTES + 1, "input_too_large"),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
            .args(["--state", "/tmp/lkjscript-machine-json-missing", "rpc"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("bounded CLI");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(&vec![b' '; size])
            .expect("bounded input");
        let output = child.wait_with_output().expect("bounded output");
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stdout).contains(expected_kind));
        assert_one_json(&output.stdout);
        assert!(!output.stderr.is_empty());
    }
}

fn assert_one_json(bytes: &[u8]) {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let _ = serde::de::IgnoredAny::deserialize(&mut deserializer).expect("JSON value");
    deserializer.end().expect("single JSON value");
}
