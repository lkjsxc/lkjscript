use std::path::PathBuf;

use lkjscript_core::Limits;

use super::{
    check_foundation_file_bytes, DiagnosticCategory, SourceDiagnostic, SourceFile, SourceNode,
    SourceOrigin, SourcePosition, SourceResult, SourceSpan, SyntaxKind, Token, TokenKind,
};

struct Line<'a> {
    text: &'a str,
    start: usize,
    content_end: usize,
    line: u32,
}

pub(super) fn parse_file(
    source: &str,
    origin: SourceOrigin,
    path: PathBuf,
    limits: &Limits,
) -> SourceResult<SourceFile> {
    let exact_source_len = u64::try_from(source.len()).map_err(|_| {
        resource_error(
            &origin,
            SourceSpan::zero(),
            "source exceeds the Edition 1 byte-addressable span limit",
        )
    })?;
    check_foundation_file_bytes(&origin, exact_source_len)?;
    if source.len() > u32::MAX as usize {
        return Err(resource_error(
            &origin,
            SourceSpan::zero(),
            "source exceeds the Edition 1 byte-addressable span limit",
        ));
    }
    let exact_source_sha256 = lkjscript_core::sha256(source.as_bytes());
    let lines = source_lines(source);
    let lexed = lex(&lines, &origin)?;
    check_file_limits(&lexed.tokens, limits, &origin)?;
    let syntax = parse_tokens(&lexed.tokens, &origin)?;
    validate_top_level(&syntax, limits, &origin)?;
    let forms = syntax.iter().map(SourceNode::project).collect();
    Ok(SourceFile {
        path,
        origin,
        exact_source_len,
        exact_source_sha256,
        forms,
        syntax,
        tokens: lexed.tokens,
        trailing_trivia: lexed.trailing_trivia,
    })
}

fn source_lines(source: &str) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut number = 1_u32;
    for segment in source.split_inclusive('\n') {
        let segment_start = start;
        start += segment.len();
        let without_lf = segment.strip_suffix('\n').unwrap_or(segment);
        let text = without_lf.strip_suffix('\r').unwrap_or(without_lf);
        lines.push(Line {
            text,
            start: segment_start,
            content_end: segment_start + text.len(),
            line: number,
        });
        number = number.saturating_add(1);
    }
    if !source.is_empty() && !source.ends_with('\n') && lines.is_empty() {
        lines.push(Line {
            text: source,
            start: 0,
            content_end: source.len(),
            line: 1,
        });
    }
    lines
}

struct Lexed {
    tokens: Vec<Token>,
    trailing_trivia: Vec<String>,
}

fn lex(lines: &[Line<'_>], origin: &SourceOrigin) -> SourceResult<Lexed> {
    let mut output = Vec::new();
    let mut pending_trivia = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = &lines[index];
        if line.text.is_empty() || line.text.starts_with(";;") {
            pending_trivia.push(line.text.to_string());
            index += 1;
            continue;
        }
        if let Some(name) = text_open_name(line.text) {
            let (mut text_tokens, next) = lex_text_block(
                lines,
                index,
                name,
                origin,
                std::mem::take(&mut pending_trivia),
            )?;
            output.append(&mut text_tokens);
            index = next;
            continue;
        }
        reject_whitespace(line, origin)?;
        if let Some(name) = line.text.strip_prefix('/') {
            validate_name(name, line, "close marker", origin)?;
            output.push(Token {
                kind: TokenKind::Close(name.to_string()),
                span: line_span(line),
                leading_trivia: std::mem::take(&mut pending_trivia),
            });
        } else if let Some(name) = line.text.strip_suffix('/') {
            validate_name(name, line, "open marker", origin)?;
            output.push(Token {
                kind: TokenKind::Open(name.to_string()),
                span: line_span(line),
                leading_trivia: std::mem::take(&mut pending_trivia),
            });
        } else {
            validate_name(line.text, line, "atom", origin)?;
            output.push(Token {
                kind: TokenKind::Atom(line.text.to_string()),
                span: line_span(line),
                leading_trivia: std::mem::take(&mut pending_trivia),
            });
        }
        index += 1;
    }
    Ok(Lexed {
        tokens: output,
        trailing_trivia: pending_trivia,
    })
}

fn text_open_name(line: &str) -> Option<&'static str> {
    match line {
        "str/" => Some("str"),
        "name/" => Some("name"),
        "import/" => Some("import"),
        _ => None,
    }
}

fn lex_text_block(
    lines: &[Line<'_>],
    open: usize,
    name: &str,
    origin: &SourceOrigin,
    leading_trivia: Vec<String>,
) -> SourceResult<(Vec<Token>, usize)> {
    let close = format!("/{name}");
    let escaped_close = format!("\\{close}");
    let mut content = Vec::new();
    let mut index = open + 1;
    while let Some(line) = lines.get(index) {
        if line.text == close {
            if name != "str" {
                validate_single_line_text(name, &content, &lines[open], origin)?;
            }
            let text_start = lines
                .get(open + 1)
                .map_or(line.start, |content_line| content_line.start);
            let text_end = if index == open + 1 {
                text_start
            } else {
                lines[index - 1].content_end
            };
            let text_span = span_at(lines, text_start, text_end);
            return Ok((
                vec![
                    Token {
                        kind: TokenKind::Open(name.to_string()),
                        span: line_span(&lines[open]),
                        leading_trivia,
                    },
                    Token {
                        kind: TokenKind::Str(content.join("\n")),
                        span: text_span,
                        leading_trivia: Vec::new(),
                    },
                    Token {
                        kind: TokenKind::Close(name.to_string()),
                        span: line_span(line),
                        leading_trivia: Vec::new(),
                    },
                ],
                index + 1,
            ));
        }
        if line.text == escaped_close {
            content.push(close.clone());
        } else {
            content.push(line.text.to_string());
        }
        index += 1;
    }
    Err(SourceDiagnostic::new(
        "LKJ-SRC-UNMATCHED-MARKER",
        DiagnosticCategory::SourceSyntax,
        format!("unclosed {name}/ text block; expected /{name}"),
        origin.clone(),
        line_span(&lines[open]),
    ))
}

fn validate_single_line_text(
    name: &str,
    content: &[String],
    open: &Line<'_>,
    origin: &SourceOrigin,
) -> SourceResult<()> {
    if content.len() != 1 || content[0].is_empty() {
        return Err(syntax_error(
            origin,
            line_span(open),
            format!("{name}/ needs exactly one non-empty text line"),
        ));
    }
    if content[0].trim() != content[0] {
        return Err(syntax_error(
            origin,
            line_span(open),
            format!("{name}/ text cannot have surrounding whitespace"),
        ));
    }
    Ok(())
}

fn reject_whitespace(line: &Line<'_>, origin: &SourceOrigin) -> SourceResult<()> {
    if line.text.chars().any(char::is_whitespace) {
        return Err(syntax_error(
            origin,
            line_span(line),
            "lkjscript uses one column-one marker or atom per line",
        ));
    }
    Ok(())
}

fn validate_name(
    name: &str,
    line: &Line<'_>,
    kind: &str,
    origin: &SourceOrigin,
) -> SourceResult<()> {
    if name.is_empty() {
        return Err(syntax_error(
            origin,
            line_span(line),
            format!("empty {kind}"),
        ));
    }
    if !is_source_identifier(name) {
        let hint = if name.contains('"') {
            "; use str/ ... /str instead of quotes"
        } else {
            ""
        };
        return Err(syntax_error(
            origin,
            line_span(line),
            format!("invalid {kind} {name:?}{hint}"),
        ));
    }
    Ok(())
}

pub(super) fn is_source_identifier(name: &str) -> bool {
    !name.is_empty() && name.as_bytes().iter().copied().all(is_ident_byte)
}

fn is_ident_byte(byte: u8) -> bool {
    matches!(
        byte,
        b'a'..=b'z'
            | b'A'..=b'Z'
            | b'0'..=b'9'
            | b'_'
            | b'-'
            | b'+'
            | b'*'
            | b'='
            | b'!'
            | b'?'
            | b'<'
            | b'>'
            | b'.'
            | b':'
    )
}

fn check_file_limits(tokens: &[Token], limits: &Limits, origin: &SourceOrigin) -> SourceResult<()> {
    let token_count = u32::try_from(tokens.len()).unwrap_or(u32::MAX);
    if token_count > limits.max_tokens_per_file {
        return Err(resource_error(
            origin,
            tokens
                .first()
                .map_or(SourceSpan::zero(), |token| token.span),
            format!(
                "token budget exceeded ({token_count} > {}); split via import",
                limits.max_tokens_per_file
            ),
        ));
    }
    let mut depth = 0_u32;
    let mut child_stack = Vec::new();
    for token in tokens {
        match &token.kind {
            TokenKind::Open(_) => {
                count_child(&mut child_stack, limits, origin, token.span)?;
                depth = depth.saturating_add(1);
                if depth > limits.max_nest_depth {
                    return Err(resource_error(
                        origin,
                        token.span,
                        format!(
                            "nest depth exceeded (>{}); extract a def",
                            limits.max_nest_depth
                        ),
                    ));
                }
                child_stack.push(0_u32);
            }
            TokenKind::Close(_) => {
                child_stack.pop();
                depth = depth.saturating_sub(1);
            }
            TokenKind::Atom(_) | TokenKind::Str(_) => {
                count_child(&mut child_stack, limits, origin, token.span)?;
            }
        }
    }
    Ok(())
}

fn count_child(
    stack: &mut [u32],
    limits: &Limits,
    origin: &SourceOrigin,
    span: SourceSpan,
) -> SourceResult<()> {
    if let Some(count) = stack.last_mut() {
        *count = count.saturating_add(1);
        if *count > limits.max_children {
            return Err(resource_error(
                origin,
                span,
                format!(
                    "too many children (>{}); split args / extract helper",
                    limits.max_children
                ),
            ));
        }
    }
    Ok(())
}

fn parse_tokens(tokens: &[Token], origin: &SourceOrigin) -> SourceResult<Vec<SourceNode>> {
    let mut index = 0;
    let mut forms = Vec::new();
    while index < tokens.len() {
        let (expression, next) = parse_expr(tokens, index, origin)?;
        forms.push(expression);
        index = next;
    }
    Ok(forms)
}

fn parse_expr(
    tokens: &[Token],
    index: usize,
    origin: &SourceOrigin,
) -> SourceResult<(SourceNode, usize)> {
    match tokens.get(index) {
        Some(Token {
            kind: TokenKind::Atom(name),
            span,
            leading_trivia,
        }) => Ok((
            atom_from_name(name, *span, leading_trivia.clone(), origin)?,
            index + 1,
        )),
        Some(Token {
            kind: TokenKind::Str(value),
            span,
            leading_trivia,
        }) => Ok((
            SourceNode {
                kind: SyntaxKind::Str {
                    value: value.clone(),
                },
                span: *span,
                leading_trivia: leading_trivia.clone(),
                before_close_trivia: Vec::new(),
                children: Vec::new(),
            },
            index + 1,
        )),
        Some(Token {
            kind: TokenKind::Open(name),
            ..
        }) => parse_element(tokens, index, name, origin),
        Some(Token {
            kind: TokenKind::Close(name),
            span,
            ..
        }) => Err(SourceDiagnostic::new(
            "LKJ-SRC-UNMATCHED-MARKER",
            DiagnosticCategory::SourceSyntax,
            format!("unexpected close marker /{name}"),
            origin.clone(),
            *span,
        )),
        None => Err(syntax_error(
            origin,
            SourceSpan::zero(),
            "unexpected end of input",
        )),
    }
}

fn parse_element(
    tokens: &[Token],
    index: usize,
    name: &str,
    origin: &SourceOrigin,
) -> SourceResult<(SourceNode, usize)> {
    let open_span = tokens[index].span;
    let leading_trivia = tokens[index].leading_trivia.clone();
    let mut cursor = index + 1;
    let mut children = Vec::new();
    loop {
        match tokens.get(cursor) {
            Some(Token {
                kind: TokenKind::Close(close),
                span: close_span,
                ..
            }) if close == name => {
                let span = SourceSpan {
                    start: open_span.start,
                    end: close_span.end,
                };
                let before_close_trivia = tokens[cursor].leading_trivia.clone();
                if name == "str" {
                    return match children.as_slice() {
                        [SourceNode {
                            kind: SyntaxKind::Str { value },
                            ..
                        }] => Ok((
                            SourceNode {
                                kind: SyntaxKind::Str {
                                    value: value.clone(),
                                },
                                span,
                                leading_trivia,
                                before_close_trivia,
                                children: Vec::new(),
                            },
                            cursor + 1,
                        )),
                        _ => Err(syntax_error(
                            origin,
                            span,
                            "str/ must contain one lkjscript text value",
                        )),
                    };
                }
                return Ok((
                    SourceNode {
                        kind: SyntaxKind::Call {
                            name: name.to_string(),
                        },
                        span,
                        leading_trivia,
                        before_close_trivia,
                        children,
                    },
                    cursor + 1,
                ));
            }
            Some(Token {
                kind: TokenKind::Close(other),
                span,
                ..
            }) => {
                return Err(SourceDiagnostic::new(
                    "LKJ-SRC-UNMATCHED-MARKER",
                    DiagnosticCategory::SourceSyntax,
                    format!("mismatched close marker /{other}; expected /{name}"),
                    origin.clone(),
                    *span,
                )
                .with_related(
                    format!("opening marker {name}/"),
                    origin.clone(),
                    open_span,
                ));
            }
            None => {
                return Err(SourceDiagnostic::new(
                    "LKJ-SRC-UNMATCHED-MARKER",
                    DiagnosticCategory::SourceSyntax,
                    format!("unclosed marker {name}/; expected /{name}"),
                    origin.clone(),
                    open_span,
                ));
            }
            _ => {
                let (child, next) = parse_expr(tokens, cursor, origin)?;
                children.push(child);
                cursor = next;
            }
        }
    }
}

fn atom_from_name(
    name: &str,
    span: SourceSpan,
    leading_trivia: Vec<String>,
    origin: &SourceOrigin,
) -> SourceResult<SourceNode> {
    let kind = if name == "unit" {
        SyntaxKind::Unit
    } else if name == "nil" {
        return Err(syntax_error(
            origin,
            span,
            "nil was removed; use unit, none/ T /none, or empty-list/ T /empty-list",
        ));
    } else if name == "true" {
        SyntaxKind::Bool { value: true }
    } else if name == "false" {
        SyntaxKind::Bool { value: false }
    } else {
        let unsigned = name.strip_prefix('-').unwrap_or(name);
        if is_ascii_digits(unsigned) {
            let value = name.parse::<i64>().map_err(|_| {
                syntax_error(origin, span, format!("I64 literal out of range: {name}"))
            })?;
            SyntaxKind::I64 { value }
        } else if let Some((whole, fraction)) = unsigned.split_once('.') {
            if is_ascii_digits(whole) && is_ascii_digits(fraction) && !fraction.contains('.') {
                let value = name.parse::<f64>().map_err(|_| {
                    syntax_error(origin, span, format!("invalid F64 literal: {name}"))
                })?;
                if !value.is_finite() {
                    return Err(syntax_error(
                        origin,
                        span,
                        format!("F64 literal must be finite: {name}"),
                    ));
                }
                SyntaxKind::F64 { value }
            } else if looks_numeric(name) {
                return Err(syntax_error(
                    origin,
                    span,
                    format!("invalid numeric literal: {name}"),
                ));
            } else {
                SyntaxKind::Symbol {
                    name: name.to_string(),
                }
            }
        } else if looks_numeric(name) || is_non_finite_spelling(name) {
            return Err(syntax_error(
                origin,
                span,
                format!("invalid numeric literal: {name}"),
            ));
        } else {
            SyntaxKind::Symbol {
                name: name.to_string(),
            }
        }
    };
    Ok(SourceNode {
        kind,
        span,
        leading_trivia,
        before_close_trivia: Vec::new(),
        children: Vec::new(),
    })
}

fn is_ascii_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

fn looks_numeric(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.first().is_some_and(u8::is_ascii_digit)
        || matches!(bytes, [b'-' | b'+', digit, ..] if digit.is_ascii_digit())
        || matches!(bytes, [b'-' | b'+', b'.'])
        || matches!(bytes, [b'-' | b'+', b'.', digit, ..] if digit.is_ascii_digit())
        || matches!(bytes, [b'-' | b'+', b'-' | b'+', digit, ..] if digit.is_ascii_digit() || *digit == b'.')
        || matches!(bytes, [b'.'])
        || matches!(bytes, [b'.', digit, ..] if digit.is_ascii_digit())
}

fn is_non_finite_spelling(text: &str) -> bool {
    matches!(
        text.to_ascii_lowercase().as_str(),
        "nan" | "inf" | "infinity" | "-nan" | "-inf" | "-infinity" | "+nan" | "+inf" | "+infinity"
    )
}

fn validate_top_level(
    forms: &[SourceNode],
    limits: &Limits,
    origin: &SourceOrigin,
) -> SourceResult<()> {
    let count = u32::try_from(forms.len()).unwrap_or(u32::MAX);
    if count > limits.max_toplevel_forms {
        return Err(resource_error(
            origin,
            forms.first().map_or(SourceSpan::zero(), |form| form.span),
            format!(
                "too many top-level forms ({count} > {}); split via import",
                limits.max_toplevel_forms
            ),
        ));
    }
    for form in forms {
        match &form.kind {
            SyntaxKind::Call { name }
                if matches!(
                    name.as_str(),
                    "def" | "main" | "import" | "product" | "trait" | "impl"
                ) =>
            {
                validate_declaration_shape(form, name, origin)?
            }
            _ => {
                return Err(syntax_error(
                    origin,
                    form.span,
                    "top-level must be def, main, import, product, trait, or impl; top-level do was removed",
                ));
            }
        }
    }
    Ok(())
}

fn validate_declaration_shape(
    form: &SourceNode,
    name: &str,
    origin: &SourceOrigin,
) -> SourceResult<()> {
    let valid = match name {
        "import" => matches!(
            form.children.as_slice(),
            [SourceNode {
                kind: SyntaxKind::Str { value },
                ..
            }] if !value.is_empty()
        ),
        "main" => matches!(
            form.children.as_slice(),
            [SourceNode {
                kind: SyntaxKind::Call { name },
                children,
                ..
            }, _] if name == "sig"
                && children.iter().any(|node| matches!(node.kind, SyntaxKind::Symbol { ref name } if name == "->"))
        ),
        "def" => matches!(
            form.children.as_slice(),
            [SourceNode {
                kind: SyntaxKind::Call { name: name_marker },
                children: name_children,
                ..
            }, SourceNode {
                kind: SyntaxKind::Call { name: function_marker },
                children: function_children,
                ..
            }]
                if name_marker == "name"
                    && function_marker == "fn"
                    && matches!(name_children.as_slice(), [SourceNode { kind: SyntaxKind::Str { value }, .. }] if !value.is_empty())
                    && valid_function_shape(function_children)
        ),
        "product" => matches!(
            form.children.as_slice(),
            [SourceNode {
                kind: SyntaxKind::Call { name: name_marker },
                children: name_children,
                ..
            }, SourceNode {
                kind: SyntaxKind::Call { name: fields_marker },
                children: fields,
                ..
            }]
                if name_marker == "name"
                    && fields_marker == "fields"
                    && matches!(name_children.as_slice(), [SourceNode { kind: SyntaxKind::Str { value }, .. }] if !value.is_empty())
                    && fields.iter().all(valid_product_field_shape)
        ),
        "trait" => matches!(
            form.children.as_slice(),
            [SourceNode {
                kind: SyntaxKind::Call { name: name_marker },
                children: name_children,
                ..
            }]
                if name_marker == "name"
                    && matches!(name_children.as_slice(), [SourceNode { kind: SyntaxKind::Str { value }, .. }] if !value.is_empty())
        ),
        "impl" => matches!(
            form.children.as_slice(),
            [SourceNode {
                kind: SyntaxKind::Call { name: trait_marker },
                children: trait_children,
                ..
            }, SourceNode {
                kind: SyntaxKind::Call { name: for_marker },
                children: for_children,
                ..
            }]
                if trait_marker == "trait"
                    && for_marker == "for"
                    && trait_children.len() == 1
                    && !for_children.is_empty()
        ),
        _ => false,
    };
    if valid {
        return Ok(());
    }
    Err(syntax_error(
        origin,
        form.span,
        format!("malformed top-level {name} declaration shape"),
    ))
}

fn valid_function_shape(children: &[SourceNode]) -> bool {
    let mut index = 0;
    if matches!(children.get(index), Some(SourceNode { kind: SyntaxKind::Call { name }, children, .. }) if name == "forall" && !children.is_empty())
    {
        index += 1;
    }
    if matches!(children.get(index), Some(SourceNode { kind: SyntaxKind::Call { name }, children, .. }) if name == "bounds" && !children.is_empty() && children.iter().all(|bound| matches!(bound, SourceNode { kind: SyntaxKind::Call { name }, children, .. } if name == "bound" && children.len() == 2)))
    {
        index += 1;
    }
    matches!(
        children.get(index..),
        Some([
            SourceNode { kind: SyntaxKind::Call { name: signature }, children: signature_children, .. },
            SourceNode { kind: SyntaxKind::Call { name: parameters }, .. },
            _
        ]) if signature == "sig"
            && parameters == "params"
            && signature_children.iter().any(|node| matches!(node.kind, SyntaxKind::Symbol { ref name } if name == "->"))
    )
}

fn valid_product_field_shape(field: &SourceNode) -> bool {
    matches!(
        field,
        SourceNode {
            kind: SyntaxKind::Call { name },
            children,
            ..
        } if name == "field"
            && matches!(children.as_slice(), [
                SourceNode { kind: SyntaxKind::Call { name: name_marker }, children: name_children, .. },
                SourceNode { kind: SyntaxKind::Call { name: type_marker }, children: type_children, .. }
            ] if name_marker == "name"
                && type_marker == "type"
                && !type_children.is_empty()
                && matches!(name_children.as_slice(), [SourceNode { kind: SyntaxKind::Str { value }, .. }] if !value.is_empty()))
    )
}

fn line_span(line: &Line<'_>) -> SourceSpan {
    SourceSpan {
        start: SourcePosition {
            byte: line.start as u32,
            line: line.line,
            column: 1,
        },
        end: SourcePosition {
            byte: line.content_end as u32,
            line: line.line,
            column: line.text.chars().count() as u32 + 1,
        },
    }
}

fn span_at(lines: &[Line<'_>], start: usize, end: usize) -> SourceSpan {
    SourceSpan {
        start: position_at(lines, start),
        end: position_at(lines, end),
    }
}

fn position_at(lines: &[Line<'_>], byte: usize) -> SourcePosition {
    for line in lines.iter().rev() {
        if byte >= line.start {
            let local_end = byte.min(line.content_end);
            let column = line.text[..local_end.saturating_sub(line.start)]
                .chars()
                .count() as u32
                + 1;
            return SourcePosition {
                byte: byte as u32,
                line: line.line,
                column,
            };
        }
    }
    SourcePosition {
        byte: byte as u32,
        line: 1,
        column: 1,
    }
}

fn syntax_error(
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

fn resource_error(
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
