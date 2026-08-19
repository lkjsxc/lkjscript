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
fn public_byte_value_encoding_is_one_canonical_strict_shape() {
    let canonical = br#"{"kind":"bytes","data":"_w"}"#;
    assert_eq!(
        serde_json::from_slice::<RuntimeValue>(canonical).unwrap(),
        RuntimeValue::Bytes(ByteString::from_slice(&[0xff]).unwrap())
    );
    assert_eq!(
        serde_json::to_vec(&RuntimeValue::Bytes(
            ByteString::from_slice(&[0xff]).unwrap()
        ))
        .unwrap(),
        canonical
    );
    for malformed in [
        br#"{"kind":"bytes","data":"_w="}"#.as_slice(),
        br#"{"kind":"bytes","data":"_x"}"#.as_slice(),
        br#"{"kind":"bytes","data":"_ w"}"#.as_slice(),
        br#"{"kind":"bytes","data":1}"#.as_slice(),
        br#"{"kind":"bytes","data":"_w","extra":0}"#.as_slice(),
        br#"{"kind":"bytes","data":"_w","data":"_w"}"#.as_slice(),
        br#"{"kind":"unknown","data":"_w"}"#.as_slice(),
    ] {
        assert!(serde_json::from_slice::<RuntimeValue>(malformed).is_err());
    }
}

#[test]
fn every_closed_machine_variant_round_trips() {
    let (workspace, first, second) = ids();
    let existing = NodeTarget::Existing(first);
    let local = NodeTarget::Draft(DraftSymbol::new("s7"));
    let value = ValueDraft::OperationResult {
        operation: existing,
        output: 0,
    };
    for target in [existing, local] {
        round_trip(&target);
    }
    for value in [
        ValueDraft::FunctionParameter(existing),
        ValueDraft::BlockArgument(local),
        value.clone(),
        ValueDraft::InlineExpression(Box::new(ExpressionKindDraft::ConstI64(1))),
    ] {
        round_trip(&value);
    }
    for value in [
        ValueRef::FunctionParameter(first),
        ValueRef::BlockArgument(first),
        ValueRef::OperationResult {
            operation: first,
            output: 0,
        },
    ] {
        round_trip(&value);
    }
    let drafts = vec![
        OperationDraft::ConstUnit,
        OperationDraft::ConstI64(1),
        OperationDraft::ConstBool(true),
        OperationDraft::AddI64 {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        OperationDraft::LtI64 {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        OperationDraft::Call {
            function: existing,
            arguments: vec![value.clone()],
        },
        OperationDraft::Hole {
            expected: SemanticType::I64.into(),
        },
        OperationDraft::If {
            condition: value.clone(),
            result: TypeDraft::I64,
            then_region: existing,
            else_region: local,
        },
        OperationDraft::ForI64 {
            start: value.clone(),
            end_exclusive: value.clone(),
            step: 1,
            initial: value.clone(),
            carried: TypeDraft::I64,
            body_region: local,
        },
        OperationDraft::Return {
            value: value.clone(),
        },
        OperationDraft::Yield {
            value: value.clone(),
        },
        OperationDraft::ConstructProduct {
            product: existing,
            fields: vec![ProductFieldValueDraft {
                field: local,
                value: value.clone(),
            }],
        },
        OperationDraft::ProjectField {
            value: value.clone(),
            field: existing,
        },
        OperationDraft::ConstructVariant {
            variant: existing,
            payload: Some(value.clone()),
        },
        OperationDraft::MatchSum {
            scrutinee: value.clone(),
            result: TypeDraft::I64,
            arms: vec![MatchArmOperationDraft {
                variant: existing,
                region: local,
            }],
        },
        OperationDraft::ConstBytes(ByteString::from_slice(b"LKJM").unwrap()),
        OperationDraft::BytesLen {
            value: value.clone(),
        },
        OperationDraft::BytesAt {
            value: value.clone(),
            index: value.clone(),
        },
        OperationDraft::BytesSlice {
            value: value.clone(),
            start: value.clone(),
            length: value.clone(),
        },
        OperationDraft::BytesEqual {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        OperationDraft::BytesConcat {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        OperationDraft::EqualI64 {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        OperationDraft::NotBool {
            value: value.clone(),
        },
        OperationDraft::AndBool {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        OperationDraft::OrBool {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        OperationDraft::ConstText(TextString::try_from_str("lkjwork").unwrap()),
        OperationDraft::TextLen {
            value: value.clone(),
        },
        OperationDraft::TextEqual {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        OperationDraft::TextConcat {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        OperationDraft::SequenceEmpty { sequence: existing },
        OperationDraft::SequenceLen {
            sequence: existing,
            value: value.clone(),
        },
        OperationDraft::SequenceGet {
            sequence: existing,
            value: value.clone(),
            index: value.clone(),
        },
        OperationDraft::SequenceAppend {
            sequence: existing,
            value: value.clone(),
            element: value.clone(),
        },
        OperationDraft::SequenceReplace {
            sequence: existing,
            value: value.clone(),
            index: value.clone(),
            element: value.clone(),
        },
    ];
    assert_eq!(drafts.len(), OperationCode::ALL.len());
    for (draft, code) in drafts.iter().zip(OperationCode::ALL) {
        assert_eq!(draft.code(), code);
        round_trip(draft);
    }
    let yielding = |symbol| YieldingBodyDraft {
        operations: vec![ExpressionDraft {
            symbol: Some(DraftSymbol::new(&format!("s{symbol}"))),
            operation: ExpressionKindDraft::ConstI64(1),
        }],
        yield_value: ValueDraft::OperationResult {
            operation: NodeTarget::Draft(DraftSymbol::new(&format!("s{symbol}"))),
            output: 0,
        },
    };
    let expression_variants = vec![
        ExpressionKindDraft::ConstUnit,
        ExpressionKindDraft::ConstBool(true),
        ExpressionKindDraft::ConstI64(1),
        ExpressionKindDraft::AddI64 {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        ExpressionKindDraft::LtI64 {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        ExpressionKindDraft::Call {
            function: local,
            arguments: vec![value.clone()],
        },
        ExpressionKindDraft::Hole {
            expected: SemanticType::I64.into(),
        },
        ExpressionKindDraft::If {
            condition: value.clone(),
            result: SemanticType::I64.into(),
            then_body: yielding(30),
            else_body: yielding(31),
        },
        ExpressionKindDraft::ForI64 {
            start: value.clone(),
            end_exclusive: value.clone(),
            step: 1,
            initial: value.clone(),
            carried: SemanticType::I64.into(),
            index_symbol: DraftSymbol::new("s32"),
            carried_symbol: DraftSymbol::new("s33"),
            body: yielding(34),
        },
        ExpressionKindDraft::ConstructProduct {
            product: existing,
            fields: vec![ProductFieldValueDraft {
                field: local,
                value: value.clone(),
            }],
        },
        ExpressionKindDraft::ProjectField {
            value: value.clone(),
            field: existing,
        },
        ExpressionKindDraft::ConstructVariant {
            variant: existing,
            payload: Some(value.clone()),
        },
        ExpressionKindDraft::MatchSum {
            scrutinee: value.clone(),
            result: TypeDraft::I64,
            arms: vec![MatchArmDraft {
                variant: existing,
                payload_symbol: Some(DraftSymbol::new("s35")),
                body: yielding(36),
            }],
        },
        ExpressionKindDraft::ConstBytes(ByteString::from_slice(b"LKJM").unwrap()),
        ExpressionKindDraft::BytesLen {
            value: value.clone(),
        },
        ExpressionKindDraft::BytesAt {
            value: value.clone(),
            index: value.clone(),
        },
        ExpressionKindDraft::BytesSlice {
            value: value.clone(),
            start: value.clone(),
            length: value.clone(),
        },
        ExpressionKindDraft::BytesEqual {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        ExpressionKindDraft::BytesConcat {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        ExpressionKindDraft::EqualI64 {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        ExpressionKindDraft::NotBool {
            value: value.clone(),
        },
        ExpressionKindDraft::AndBool {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        ExpressionKindDraft::OrBool {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        ExpressionKindDraft::ConstText(TextString::try_from_str("lkjwork").unwrap()),
        ExpressionKindDraft::TextLen {
            value: value.clone(),
        },
        ExpressionKindDraft::TextEqual {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        ExpressionKindDraft::TextConcat {
            lhs: value.clone(),
            rhs: value.clone(),
        },
        ExpressionKindDraft::SequenceEmpty { sequence: existing },
        ExpressionKindDraft::SequenceLen {
            sequence: existing,
            value: value.clone(),
        },
        ExpressionKindDraft::SequenceGet {
            sequence: existing,
            value: value.clone(),
            index: value.clone(),
        },
        ExpressionKindDraft::SequenceAppend {
            sequence: existing,
            value: value.clone(),
            element: value.clone(),
        },
        ExpressionKindDraft::SequenceReplace {
            sequence: existing,
            value: value.clone(),
            index: value.clone(),
            element: value.clone(),
        },
    ];
    for (index, operation) in expression_variants.into_iter().enumerate() {
        round_trip(&ExpressionDraft {
            symbol: Some(DraftSymbol::new(&format!(
                "s{}",
                100 + u32::try_from(index).expect("index")
            ))),
            operation,
        });
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
        OperationKind::ConstructProduct {
            product: first,
            fields: vec![ProductFieldValue {
                field: second,
                value: ValueRef::FunctionParameter(first),
            }],
        },
        OperationKind::ProjectField {
            value: ValueRef::FunctionParameter(first),
            field: second,
        },
        OperationKind::ConstructVariant {
            variant: first,
            payload: None,
        },
        OperationKind::MatchSum {
            scrutinee: ValueRef::FunctionParameter(first),
            result: SemanticType::I64,
            arms: vec![MatchArm {
                variant: first,
                region: second,
            }],
        },
        OperationKind::ConstBytes(ByteString::from_slice(b"LKJM").unwrap()),
        OperationKind::BytesLen {
            value: ValueRef::FunctionParameter(first),
        },
        OperationKind::BytesAt {
            value: ValueRef::FunctionParameter(first),
            index: ValueRef::FunctionParameter(second),
        },
        OperationKind::BytesSlice {
            value: ValueRef::FunctionParameter(first),
            start: ValueRef::FunctionParameter(second),
            length: ValueRef::FunctionParameter(second),
        },
        OperationKind::BytesEqual {
            lhs: ValueRef::FunctionParameter(first),
            rhs: ValueRef::FunctionParameter(second),
        },
        OperationKind::BytesConcat {
            lhs: ValueRef::FunctionParameter(first),
            rhs: ValueRef::FunctionParameter(second),
        },
        OperationKind::EqualI64 {
            lhs: ValueRef::FunctionParameter(first),
            rhs: ValueRef::FunctionParameter(second),
        },
        OperationKind::NotBool {
            value: ValueRef::FunctionParameter(first),
        },
        OperationKind::AndBool {
            lhs: ValueRef::FunctionParameter(first),
            rhs: ValueRef::FunctionParameter(second),
        },
        OperationKind::OrBool {
            lhs: ValueRef::FunctionParameter(first),
            rhs: ValueRef::FunctionParameter(second),
        },
        OperationKind::ConstText(TextString::try_from_str("lkjwork").unwrap()),
        OperationKind::TextLen {
            value: ValueRef::FunctionParameter(first),
        },
        OperationKind::TextEqual {
            lhs: ValueRef::FunctionParameter(first),
            rhs: ValueRef::FunctionParameter(second),
        },
        OperationKind::TextConcat {
            lhs: ValueRef::FunctionParameter(first),
            rhs: ValueRef::FunctionParameter(second),
        },
        OperationKind::SequenceEmpty { sequence: first },
        OperationKind::SequenceLen {
            sequence: first,
            value: ValueRef::FunctionParameter(first),
        },
        OperationKind::SequenceGet {
            sequence: first,
            value: ValueRef::FunctionParameter(first),
            index: ValueRef::FunctionParameter(second),
        },
        OperationKind::SequenceAppend {
            sequence: first,
            value: ValueRef::FunctionParameter(first),
            element: ValueRef::FunctionParameter(second),
        },
        OperationKind::SequenceReplace {
            sequence: first,
            value: ValueRef::FunctionParameter(first),
            index: ValueRef::FunctionParameter(second),
            element: ValueRef::FunctionParameter(second),
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
            symbol: DraftSymbol::new("s1"),
            name: "p".to_owned(),
        },
        TransactionOp::CreateModule {
            symbol: DraftSymbol::new("s2"),
            package: existing,
            name: "m".to_owned(),
        },
        TransactionOp::CreateFunction {
            symbol: DraftSymbol::new("s3"),
            module: existing,
            name: "f".to_owned(),
            parameters: vec![FunctionParameterDraft {
                symbol: DraftSymbol::new("s4"),
                name: "x".to_owned(),
                ty: SemanticType::Bool.into(),
            }],
            result: SemanticType::I64.into(),
            body: None,
        },
        TransactionOp::DefineFunctionBody {
            function: first,
            body: FunctionBodyDraft {
                operations: vec![ExpressionDraft {
                    symbol: Some(DraftSymbol::new("s5")),
                    operation: ExpressionKindDraft::ConstI64(1),
                }],
                return_value: ValueDraft::OperationResult {
                    operation: NodeTarget::Draft(DraftSymbol::new("s5")),
                    output: 0,
                },
            },
        },
        TransactionOp::ReplaceFunctionBody {
            function: first,
            body: FunctionBodyDraft {
                operations: vec![ExpressionDraft {
                    symbol: Some(DraftSymbol::new("s15")),
                    operation: ExpressionKindDraft::ConstI64(2),
                }],
                return_value: ValueDraft::OperationResult {
                    operation: NodeTarget::Draft(DraftSymbol::new("s15")),
                    output: 0,
                },
            },
        },
        TransactionOp::InsertExpression {
            block: first,
            before: Some(second),
            expression: ExpressionDraft {
                symbol: Some(DraftSymbol::new("s6")),
                operation: ExpressionKindDraft::If {
                    condition: ValueDraft::FunctionParameter(existing),
                    result: SemanticType::I64.into(),
                    then_body: YieldingBodyDraft {
                        operations: vec![ExpressionDraft {
                            symbol: Some(DraftSymbol::new("s8")),
                            operation: ExpressionKindDraft::ConstI64(1),
                        }],
                        yield_value: ValueDraft::OperationResult {
                            operation: NodeTarget::Draft(DraftSymbol::new("s8")),
                            output: 0,
                        },
                    },
                    else_body: YieldingBodyDraft {
                        operations: vec![ExpressionDraft {
                            symbol: Some(DraftSymbol::new("s9")),
                            operation: ExpressionKindDraft::ConstI64(2),
                        }],
                        yield_value: ValueDraft::OperationResult {
                            operation: NodeTarget::Draft(DraftSymbol::new("s9")),
                            output: 0,
                        },
                    },
                },
            },
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
            value: value.clone(),
        },
        TransactionOp::DeleteOwnedSubtree { root: existing },
        TransactionOp::RefineHole {
            hole: existing,
            replacement: drafts[2].clone(),
        },
        TransactionOp::CreateProductType {
            symbol: DraftSymbol::new("s10"),
            module: local,
            name: "Product".to_owned(),
            fields: vec![ProductFieldDraft {
                symbol: DraftSymbol::new("s11"),
                name: "value".to_owned(),
                ty: TypeDraft::I64,
            }],
        },
        TransactionOp::CreateSumType {
            symbol: DraftSymbol::new("s12"),
            module: local,
            name: "Sum".to_owned(),
            variants: vec![SumVariantDraft {
                symbol: DraftSymbol::new("s13"),
                name: "none".to_owned(),
                payload: None,
            }],
        },
        TransactionOp::CreateSequenceType {
            symbol: DraftSymbol::new("s16"),
            module: local,
            name: "Sequence".to_owned(),
            element: TypeDraft::Text,
        },
        TransactionOp::CreateBuildTarget {
            symbol: DraftSymbol::new("s17"),
            name: "target".to_owned(),
            definition: BuildTargetDefinition::Product(ProductTargetDefinition {
                application: first,
            }),
        },
        TransactionOp::ReplaceBuildTarget {
            target: first,
            definition: BuildTargetDefinition::Product(ProductTargetDefinition {
                application: second,
            }),
        },
        TransactionOp::AddReleaseTargetExport {
            target: first,
            name: "legacy".to_owned(),
            item: second,
        },
        TransactionOp::SetReleaseTargetExport {
            target: first,
            name: "entry".to_owned(),
            item: second,
        },
        TransactionOp::SetApplicationQueryBoundary {
            target: first,
            query_entry: TargetItem {
                release_target: first,
                item: second,
            },
            query: TargetItem {
                release_target: first,
                item: second,
            },
        },
        TransactionOp::AddApplicationTargetTest {
            target: first,
            case: TargetApplicationTestCase {
                name: "query".to_owned(),
                target: TargetItem {
                    release_target: first,
                    item: second,
                },
                arguments: vec![TargetValue::I64(1)],
                expected: TargetTestExpectation::Value(TargetValue::I64(1)),
                policy: RunPolicy {
                    fuel: 1,
                    maximum_frames: 1,
                },
            },
        },
    ];
    for type_draft in [
        TypeDraft::Unit,
        TypeDraft::Bool,
        TypeDraft::I64,
        TypeDraft::Bytes,
        TypeDraft::Text,
        TypeDraft::Nominal(NodeTarget::Draft(DraftSymbol::new("s14"))),
        TypeDraft::Nominal(NodeTarget::Existing(first)),
    ] {
        round_trip(&type_draft);
    }

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
        PageCursor::NominalType {
            workspace,
            revision: Revision::new(1),
            declaration: first,
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
        LiteralValue::Bytes(ByteString::from_slice(b"x").unwrap()),
        LiteralValue::Text(TextString::try_from_str("x").unwrap()),
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
            constructors: page(None),
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
        Query::NominalType {
            declaration: first,
            page: page(None),
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
            targets: Vec::new(),
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
            types: Vec::new(),
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
            arguments: Vec::new(),
            operations: vec![second],
            terminator: Some(second),
        },
        Node::BlockArgument {
            owner: first,
            ordinal: 0,
            ty: SemanticType::I64,
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
        ChangeKind::DefinitionChanged {
            before: first,
            after: second,
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
        ChangeKind::FunctionBodyChanged {
            before_items: 1,
            after_items: 2,
            added_items: 1,
            removed_items: 0,
            modified_items: 0,
        },
    ];
    for change in &changes {
        round_trip(change);
    }
    for scalar in [
        ScalarValue::I64(1),
        ScalarValue::Bool(true),
        ScalarValue::Type(SemanticType::I64),
        ScalarValue::Bytes(ByteString::from_slice(b"x").unwrap()),
        ScalarValue::Text(TextString::try_from_str("x").unwrap()),
    ] {
        round_trip(&scalar);
    }
    for value in [
        RuntimeValue::Unit,
        RuntimeValue::Bool(true),
        RuntimeValue::I64(1),
        RuntimeValue::Bytes(ByteString::from_slice(b"x").unwrap()),
        RuntimeValue::Text(TextString::try_from_str("x").unwrap()),
    ] {
        round_trip(&value);
    }
    round_trip(&RuntimeValue::Product {
        ty: first,
        fields: vec![lkjscript::RuntimeFieldValue {
            field: second,
            value: RuntimeValue::I64(7),
        }],
    });
    round_trip(&RuntimeValue::Sum {
        ty: first,
        variant: second,
        payload: Some(Box::new(RuntimeValue::Bool(true))),
    });
    round_trip(&RuntimeValue::Sequence {
        ty: first,
        elements: vec![RuntimeValue::Text(TextString::try_from_str("x").unwrap())],
    });
    let mut deepest = RuntimeValue::Unit;
    for _ in 1..lkjscript::interpret::MAX_RUNTIME_VALUE_DEPTH {
        deepest = RuntimeValue::Sum {
            ty: first,
            variant: second,
            payload: Some(Box::new(deepest)),
        };
    }
    round_trip(&deepest);

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
        durable_identity_count: 2,
        function_local_reference_count: 0,
        anchor_count: 0,
        tombstone_count: 0,
        complete: true,
        blocker_count: 0,
        entry_count: 1,
    };
    let node_summary = NodeSummary {
        workspace,
        revision: Revision::new(1),
        node: first,
        identity_class: lkjscript::NodeIdentityClass::Durable,
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
        enclosing_regions: vec![
            EnclosingRegionFact {
                region: first,
                owner_operation: second,
                role: RegionRole::IfThen,
            },
            EnclosingRegionFact {
                region: second,
                owner_operation: first,
                role: RegionRole::ForBody,
            },
        ],
        visible_block_arguments: vec![BlockArgumentFact {
            argument: first,
            block: second,
            region: second,
            ordinal: 0,
            role: BlockArgumentRole::LoopIndex,
            ty: SemanticType::I64,
        }],
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
        legal_constructor_count: 0,
        legal_constructors: Vec::new(),
        nominal_type: None,
        nominal_type_continuation: None,
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
            constructors: Page {
                items: Vec::new(),
                next: None,
                total: Some(0),
            },
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
                return_symbols: vec![DraftSymbol::new("s1")],
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
            arguments: vec![
                RuntimeValue::Unit,
                RuntimeValue::Bool(false),
                RuntimeValue::I64(-7),
            ],
            policy: lkjscript::RunPolicy {
                fuel: 777,
                maximum_frames: 33,
            },
        },
        Request::Shutdown,
        Request::DescribeSchema(DescribeSchemaRequest::manifest()),
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
            returned_bindings: vec![(DraftSymbol::new("s1"), first)],
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
            lowering_nanoseconds: 0,
            core_verification_nanoseconds: 0,
            execute_nanoseconds: 2,
        }),
        Response::Acknowledged,
        Response::Error(
            LkError::new(ErrorCode::InvalidOperand, "invalid")
                .at_operation(2)
                .for_symbol(DraftSymbol::new("s7")),
        ),
        Response::DescribeSchema(Box::new(
            lkjscript::machine::describe_schema(&DescribeSchemaRequest::manifest())
                .expect("manifest schema"),
        )),
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
    for malformed in [
        r#""unknown""#,
        r#"{"nominal":{"kind":"unknown","data":1}}"#,
        r#"{"nominal":{"kind":"local","data":1},"extra":true}"#,
        r#"{"nominal":{}}"#,
    ] {
        assert!(serde_json::from_str::<TypeDraft>(malformed).is_err());
    }
    assert!(
        serde_json::from_str::<ExpressionDraft>(
            r#"{"symbol":1,"operation":{"kind":"unknown","data":null}}"#
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<ExpressionDraft>(
            r#"{"symbol":1,"operation":{"kind":"const_unit","extra":1}}"#
        )
        .is_err()
    );
    for malformed in [
        r#"{"kind":"construct_product","data":{"product":{"kind":"local","data":1},"fields":[],"extra":0}}"#,
        r#"{"kind":"match_sum","data":{"scrutinee":{"kind":"operation_result","data":{"operation":{"kind":"local","data":1},"output":0}},"result":{"kind":"i64"},"arms":[{"variant":{"kind":"local","data":2}}]}}"#,
        r#"{"kind":"construct_variant","data":{"variant":{"kind":"local","data":2},"payload":null,"extra":0}}"#,
    ] {
        assert!(serde_json::from_str::<OperationDraft>(malformed).is_err());
    }
    for malformed in [
        r#"{"kind":"inline_expression"}"#,
        r#"{"kind":"inline_expression","data":null}"#,
        r#"{"kind":"inline_expression","data":{"kind":"const_i64","data":1,"extra":0}}"#,
    ] {
        assert!(serde_json::from_str::<ValueDraft>(malformed).is_err());
    }
    let workspace = WorkspaceId::from_bytes([0xab; 16]);
    let valid = format!(
        "{{\"version\":12,\"request_id\":1,\"request\":{{\"kind\":\"query_batch\",\"data\":{{\"workspace\":\"{workspace}\",\"revision\":0,\"queries\":[{{\"id\":1,\"query\":{{\"kind\":\"blockers\",\"data\":{{\"page\":{{\"limit\":1}}}}}}}}]}}}}}}"
    );
    assert!(decode_request(valid.as_bytes()).is_ok());
    let invalid = [
        valid.replacen("\"version\":12", "\"version\":11", 1),
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
        "{\"version\":3}".to_owned(),
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
    let nominal_result_missing_name = format!(
        "{{\"declaration\":\"{node}\",\"kind\":\"product_type\",\"owner\":\"{node}\",\"layout\":{{\"representable\":false}},\"members\":{{\"items\":[]}}}}"
    );
    assert!(serde_json::from_str::<NominalTypeResult>(&nominal_result_missing_name).is_err());
    let malformed_product_node = format!(
        "{{\"kind\":\"product_field\",\"data\":{{\"owner\":\"{node}\",\"ordinal\":0,\"name\":\"x\",\"ty\":\"i64\",\"extra\":0}}}}"
    );
    assert!(serde_json::from_str::<Node>(&malformed_product_node).is_err());
    let run = format!(
        "{{\"kind\":\"run\",\"data\":{{\"workspace\":\"{workspace}\",\"revision\":1,\"entry\":\"{node}\",\"arguments\":[],\"policy\":{{\"fuel\":1,\"maximum_frames\":1}}}}}}"
    );
    assert!(serde_json::from_str::<Request>(&run).is_ok());
    assert!(serde_json::from_str::<Request>(&run.replacen("\"arguments\":[],", "", 1)).is_err());
    assert!(
        serde_json::from_str::<Request>(&run.replacen(
            ",\"policy\":{\"fuel\":1,\"maximum_frames\":1}",
            "",
            1
        ))
        .is_err()
    );
    assert!(
        serde_json::from_str::<Request>(&run.replacen(
            "\"maximum_frames\":1",
            "\"maximum_frames\":1,\"extra\":0",
            1
        ))
        .is_err()
    );
    assert!(
        serde_json::from_str::<Request>(&run.replacen(
            "\"arguments\":[]",
            "\"arguments\":[{\"kind\":\"unknown\"}]",
            1
        ))
        .is_err()
    );
    let product = format!(
        "{{\"kind\":\"product\",\"data\":{{\"ty\":\"{node}\",\"fields\":[],\"extra\":0}}}}"
    );
    assert!(serde_json::from_str::<RuntimeValue>(&product).is_err());
    let sum = format!(
        "{{\"kind\":\"sum\",\"data\":{{\"ty\":\"{node}\",\"variant\":\"{node}\",\"payload\":{{\"kind\":\"unit\"}},\"extra\":0}}}}"
    );
    assert!(serde_json::from_str::<RuntimeValue>(&sum).is_err());
    assert!(serde_json::from_str::<Transaction>(&format!("{{\"workspace\":\"{workspace}\",\"base_revision\":0,\"mode\":\"commit\",\"operations\":[],\"extra\":0}}")).is_err());
    assert!(serde_json::from_str::<DraftSymbol>("4294967296").is_err());
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
    let compact_result = serde_json::from_slice::<DescribeSchemaResult>(&compact.stdout)
        .expect("compact schema JSON");
    assert_eq!(
        compact_result,
        serde_json::from_slice::<DescribeSchemaResult>(&pretty.stdout).expect("pretty schema JSON")
    );
    let DescribeSchemaResult::Manifest(manifest) = compact_result else {
        panic!("default schema manifest")
    };

    let roots = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .args(["schema", "--root", "limits", "--root", "error"])
        .output()
        .expect("root schema");
    assert!(roots.status.success());
    let DescribeSchemaResult::Roots(roots) =
        serde_json::from_slice(&roots.stdout).expect("roots JSON")
    else {
        panic!("roots result")
    };
    assert_eq!(
        roots.roots,
        vec![
            lkjscript::machine::SchemaRoot::Error,
            lkjscript::machine::SchemaRoot::Limits,
        ]
    );
    assert!(roots.definitions.iter().any(|item| item.name == "error"));
    assert!(roots.definitions.iter().any(|item| item.name == "limits"));

    let full = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .args(["schema", "--full"])
        .output()
        .expect("full schema");
    assert!(full.status.success());
    assert!(matches!(
        serde_json::from_slice::<DescribeSchemaResult>(&full.stdout).expect("full JSON"),
        DescribeSchemaResult::Full { .. }
    ));

    let unchanged = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .args([
            "schema",
            "--full",
            "--known-digest",
            &manifest.digest.to_string(),
        ])
        .output()
        .expect("unchanged schema");
    assert!(unchanged.status.success());
    assert_eq!(
        serde_json::from_slice::<DescribeSchemaResult>(&unchanged.stdout).expect("unchanged JSON"),
        DescribeSchemaResult::Unchanged {
            digest: manifest.digest
        }
    );

    for arguments in [
        vec!["schema", "--full", "--root", "runtime_value"],
        vec!["schema", "--root", "unknown"],
        vec![
            "schema",
            "--root",
            "runtime_value",
            "--root",
            "runtime_value",
        ],
        vec!["schema", "--known-digest", "ABCDEF"],
    ] {
        let invalid = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
            .args(arguments)
            .output()
            .expect("invalid schema flags");
        assert_eq!(invalid.status.code(), Some(2));
        assert!(invalid.stdout.len() < 2048);
        assert_one_json(&invalid.stdout);
    }

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
