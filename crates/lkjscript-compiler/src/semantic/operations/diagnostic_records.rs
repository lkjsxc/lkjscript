use crate::semantic::schema::{
    Certainty, DiagnosticCategory, DiagnosticCode, DiagnosticRecord, RelatedRecord, RepairRecord,
    Severity,
};
use crate::source::{SourceDiagnostic, ValidatedSourceTree};

pub(super) fn node(
    code: DiagnosticCode,
    category: DiagnosticCategory,
    tree: &ValidatedSourceTree,
    index: u64,
    message: String,
    expected: Option<String>,
    actual: Option<String>,
) -> DiagnosticRecord {
    let node = usize::try_from(index)
        .ok()
        .and_then(|host_index| tree.nodes().get(host_index));
    let source = node.map_or(tree.root_origin(), |node| node.origin());
    let span = node.map_or(crate::source::SourceSpan::zero(), |node| node.span());
    DiagnosticRecord {
        schema: lkjscript_contracts::DIAGNOSTICS.to_string(),
        contract: lkjscript_contracts::DIAGNOSTICS_DIGEST.to_hex(),
        code,
        severity: Severity::Error,
        category,
        certainty: Certainty::Guaranteed,
        primary_node: node.map(|node| node.id().index()),
        primary_source: source.logical_path().to_string(),
        primary_span: crate::semantic::tree::span_record(span),
        related: Vec::new(),
        declaration: node
            .and_then(|node| crate::semantic::tree::containing_declaration(tree, node))
            .map(|decl| decl.key().to_hex()),
        binding: None,
        expected,
        actual,
        effect_mismatch: None,
        ownership_path: None,
        agent_rendering: format!("code={code:?};node={index};message={message}"),
        human_rendering: message,
        repairs: Vec::new(),
    }
}

pub(crate) fn source_failure(error: &SourceDiagnostic) -> Option<DiagnosticRecord> {
    let code = match error.code() {
        "LKJ-SRC-UNMATCHED-MARKER" => DiagnosticCode::UnmatchedMarker,
        "LKJ-DECL-DUPLICATE" => DiagnosticCode::DuplicateDeclaration,
        _ => return None,
    };
    let mut record = standalone(
        code,
        error.origin().logical_path(),
        error.primary_span(),
        error.message(),
    );
    record.category = if matches!(code, DiagnosticCode::DuplicateDeclaration) {
        DiagnosticCategory::Declaration
    } else {
        DiagnosticCategory::SourceSyntax
    };
    record.related = error
        .related_spans()
        .iter()
        .map(|related| RelatedRecord {
            label: related.label().to_string(),
            node: None,
            source: related.origin().logical_path().to_string(),
            span: crate::semantic::tree::span_record(related.span()),
        })
        .collect();
    Some(record)
}

pub(crate) fn stale(path: &str, message: &str) -> DiagnosticRecord {
    let mut record = standalone(
        DiagnosticCode::StaleEdit,
        path,
        crate::source::SourceSpan::zero(),
        message,
    );
    record.repairs.push(RepairRecord::RefreshSnapshot);
    record
}

fn standalone(
    code: DiagnosticCode,
    path: &str,
    span: crate::source::SourceSpan,
    message: &str,
) -> DiagnosticRecord {
    DiagnosticRecord {
        schema: lkjscript_contracts::DIAGNOSTICS.to_string(),
        contract: lkjscript_contracts::DIAGNOSTICS_DIGEST.to_hex(),
        code,
        severity: Severity::Error,
        category: DiagnosticCategory::Edit,
        certainty: Certainty::Guaranteed,
        primary_node: None,
        primary_source: path.to_string(),
        primary_span: crate::semantic::tree::span_record(span),
        related: Vec::new(),
        declaration: None,
        binding: None,
        expected: None,
        actual: None,
        effect_mismatch: None,
        ownership_path: None,
        agent_rendering: format!("code={code:?};message={message}"),
        human_rendering: message.to_string(),
        repairs: Vec::new(),
    }
}
