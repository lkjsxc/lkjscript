use crate::source::{DiagnosticCategory, SourceDiagnostic, SourceOrigin, SourceResult, SourceSpan};

pub(super) fn syntax_error(
    origin: &SourceOrigin,
    span: SourceSpan,
    message: impl Into<String>,
) -> SourceDiagnostic {
    SourceDiagnostic::new(
        "LKJ-SRC-SYNTAX",
        DiagnosticCategory::SourceSyntax,
        message,
        origin.clone(),
        span,
    )
}

pub(super) fn resource_error(
    origin: &SourceOrigin,
    span: SourceSpan,
    message: impl Into<String>,
) -> SourceDiagnostic {
    SourceDiagnostic::new(
        "LKJ-SRC-LIMIT",
        DiagnosticCategory::ResourceLimit,
        message,
        origin.clone(),
        span,
    )
}

pub(super) fn clone_string(
    value: &str,
    origin: &SourceOrigin,
    span: SourceSpan,
    context: &str,
) -> SourceResult<String> {
    let mut output = String::new();
    output
        .try_reserve_exact(value.len())
        .map_err(|_| allocation_error(origin, span, context))?;
    output.push_str(value);
    Ok(output)
}

pub(super) fn clone_bytes(
    value: &[u8],
    origin: &SourceOrigin,
    span: SourceSpan,
    context: &str,
) -> SourceResult<Vec<u8>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(value.len())
        .map_err(|_| allocation_error(origin, span, context))?;
    output.extend_from_slice(value);
    Ok(output)
}

pub(super) fn clone_trivia(
    values: &[String],
    origin: &SourceOrigin,
    span: SourceSpan,
) -> SourceResult<Vec<String>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(values.len())
        .map_err(|_| allocation_error(origin, span, "source trivia"))?;
    for value in values {
        output.push(clone_string(value, origin, span, "source trivia text")?);
    }
    Ok(output)
}

pub(super) fn allocation_error(
    origin: &SourceOrigin,
    span: SourceSpan,
    context: &str,
) -> SourceDiagnostic {
    SourceDiagnostic::new(
        "LKJ-SRC-HOST",
        DiagnosticCategory::SourceLoading,
        format!("host could not reserve memory for {context}"),
        origin.clone(),
        span,
    )
}
