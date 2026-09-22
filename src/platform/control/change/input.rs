//! Change-only multiline framing and a flat syntax arena. Compact responses keep their own
//! physical-record grammar. No expanded flat text or publicly addressable private labels exist.

use super::*;
use crate::platform::control::compact::{
    MAXIMUM_COMPACT_FIELDS, MAXIMUM_COMPACT_INPUT_BYTES, MAXIMUM_COMPACT_RECORDS,
};

// The existing compact format admits this many field tokens. Parentheses and atoms each cost
// one structural token, across the entire input, independently of physical record count.
pub(crate) const MAXIMUM_STRUCTURAL_TOKENS: usize =
    MAXIMUM_COMPACT_RECORDS * MAXIMUM_COMPACT_FIELDS;
pub(crate) const MAXIMUM_STRUCTURAL_SYNTAX_NODES: usize =
    crate::platform::change::MAXIMUM_CHANGE_ALLOCATED_IDENTITIES as usize;

#[derive(Debug)]
pub(super) struct ChangeInput {
    pub records: Vec<CompactRecord>,
    pub blocks: BTreeMap<String, Block>,
    pub units: Vec<Block>,
    pub public_labels: BTreeSet<String>,
}

#[derive(Clone, Debug)]
pub(super) struct Block {
    pub syntax: Vec<Syntax>,
    pub root: usize,
    pub locals: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub(super) struct Syntax {
    pub kind: SyntaxKind,
    pub location: SourceLocation,
}

#[derive(Clone, Debug)]
pub(super) enum SyntaxKind {
    Atom { value: String, quoted: bool },
    List(Vec<usize>),
}

impl Block {
    pub fn subtree(&self, root: usize) -> Result<Self, Diagnostic> {
        let mut order = Vec::new();
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            order.push(id);
            if let SyntaxKind::List(children) = &self.syntax[id].kind {
                pending.extend(children.iter().rev().copied());
            }
        }
        let indices: BTreeMap<_, _> = order
            .iter()
            .enumerate()
            .map(|(new, old)| (*old, new))
            .collect();
        let mut syntax = Vec::new();
        syntax
            .try_reserve(order.len())
            .map_err(|_| resource(&self.syntax[root].location, "body syntax allocation failed"))?;
        for id in order {
            let mut node = self.syntax[id].clone();
            if let SyntaxKind::List(children) = &mut node.kind {
                for child in children {
                    *child = indices[child];
                }
            }
            syntax.push(node);
        }
        Ok(Self {
            syntax,
            root: 0,
            locals: self.locals.clone(),
        })
    }

    pub fn list(&self, id: usize) -> Result<&[usize], Diagnostic> {
        match &self.syntax[id].kind {
            SyntaxKind::List(items) => Ok(items),
            _ => Err(self.error(
                id,
                "change_block_expression",
                "expected a parenthesized expression or clause",
            )),
        }
    }

    pub fn atom(&self, id: usize) -> Result<&str, Diagnostic> {
        match &self.syntax[id].kind {
            SyntaxKind::Atom {
                value,
                quoted: false,
            } => Ok(value),
            _ => Err(self.error(
                id,
                "change_block_atom",
                "expected an unquoted name or typed reference atom",
            )),
        }
    }

    pub fn text(&self, id: usize) -> Result<&str, Diagnostic> {
        match &self.syntax[id].kind {
            SyntaxKind::Atom { value, .. } => Ok(value),
            _ => Err(self.error(id, "change_block_atom", "expected a string or atom")),
        }
    }

    pub fn head(&self, id: usize) -> Option<&str> {
        let SyntaxKind::List(items) = &self.syntax[id].kind else {
            return None;
        };
        items.first().and_then(|id| self.atom(*id).ok())
    }

    pub fn error(&self, id: usize, code: &str, message: impl Into<String>) -> Diagnostic {
        Diagnostic::source(code, message, self.syntax[id].location.clone())
    }

    pub fn field(&self, id: usize, name: &str) -> Result<CompactField, Diagnostic> {
        Ok(CompactField {
            name: name.to_owned(),
            value: self.atom(id)?.to_owned(),
            location: self.syntax[id].location.clone(),
        })
    }
}

pub(super) fn parse(path: &str, input: &[u8]) -> Result<ChangeInput, Vec<Diagnostic>> {
    if input.len() > MAXIMUM_COMPACT_INPUT_BYTES {
        // Retain the established outer-bound diagnostic.
        return parse_records(path, input).map(|records| ChangeInput {
            records,
            blocks: BTreeMap::new(),
            units: Vec::new(),
            public_labels: BTreeSet::new(),
        });
    }
    let text = std::str::from_utf8(input)
        .map_err(|_| parse_records(path, input).err().unwrap_or_default())?;
    let mut result = ChangeInput {
        records: Vec::new(),
        blocks: BTreeMap::new(),
        units: Vec::new(),
        public_labels: BTreeSet::new(),
    };
    let mut diagnostics = Vec::new();
    let mut active: Option<(String, CompactRecord, usize, usize)> = None;
    let mut offset = 0;
    let mut records = 0_usize;
    let mut tokens = 0_usize;
    let mut nodes = 0_usize;
    for (line_index, line) in text.split_inclusive('\n').enumerate() {
        let trimmed = line.trim_matches(|c: char| c.is_ascii_whitespace());
        let operation = trimmed.split_ascii_whitespace().next().unwrap_or("");
        if let Some((label, header, start, first_line)) = &active {
            let ending = if header.operation == "declarations.begin" {
                "declarations.end"
            } else {
                "expression.end"
            };
            if operation == ending {
                if trimmed != ending {
                    return Err(vec![line_error(
                        path,
                        offset,
                        line_index + 1,
                        "change_block_end",
                        "block end marker must be an otherwise empty standalone line",
                    )]);
                }
                let block = tokenize(
                    path,
                    &text[*start..offset],
                    *start,
                    *first_line,
                    &mut tokens,
                    &mut nodes,
                    &mut result.public_labels,
                )
                .map_err(|error| vec![error])?;
                if header.operation == "declarations.begin" {
                    result.units.push(block);
                } else if result.blocks.insert(label.clone(), block).is_some() {
                    return Err(vec![field_error(
                        header,
                        "as",
                        "change_expression_duplicate",
                        "expression block root is defined more than once",
                    )]);
                }
                active = None;
            } else if matches!(operation, "expression.block" | "declarations.begin") {
                return Err(vec![line_error(
                    path,
                    offset,
                    line_index + 1,
                    "change_block_nested",
                    "declaration and expression blocks cannot contain another block header",
                )]);
            }
            offset += line.len();
            continue;
        }
        if !trimmed.is_empty() {
            records += 1;
            if records > MAXIMUM_COMPACT_RECORDS {
                return Err(vec![line_error(
                    path,
                    offset,
                    line_index + 1,
                    "control_record_count",
                    format!(
                        "compact input exceeds the {MAXIMUM_COMPACT_RECORDS}-record format bound"
                    ),
                )]);
            }
        }
        let parsed = parse_records(path, line.as_bytes()).map_err(|mut errors| {
            for error in &mut errors {
                if let Some(location) = &mut error.location {
                    location.byte_offset += offset;
                    location.line += line_index;
                }
            }
            errors
        });
        match parsed {
            Err(errors) => diagnostics.extend(errors),
            Ok(parsed) => {
                for mut record in parsed {
                    record.location.byte_offset += offset;
                    record.location.line += line_index;
                    for field in &mut record.fields {
                        field.location.byte_offset += offset;
                        field.location.line += line_index;
                        reserve_labels(&field.value, &mut result.public_labels);
                    }
                    match record.operation.as_str() {
                        "expression.end" | "declarations.end" => {
                            return Err(vec![record_error(
                                &record,
                                "change_block_stray_end",
                                "block end marker has no matching open block",
                            )]);
                        }
                        "expression.block" => {
                            check_fields(&record, &["as"]).map_err(|error| vec![error])?;
                            let label = symbol(&record, "as").map_err(|error| vec![error])?;
                            active =
                                Some((label, record.clone(), offset + line.len(), line_index + 2));
                        }
                        "declarations.begin" => {
                            check_fields(&record, &[]).map_err(|error| vec![error])?;
                            active =
                                Some((String::new(), record, offset + line.len(), line_index + 2));
                            continue;
                        }
                        _ => {}
                    }
                    result.records.push(record);
                }
            }
        }
        offset += line.len();
    }
    if let Some((_, header, _, _)) = active {
        diagnostics.push(record_error(
            &header,
            "change_block_unclosed",
            "block is missing its matching declarations.end or expression.end marker",
        ));
    }
    if diagnostics.is_empty() {
        Ok(result)
    } else {
        Err(diagnostics)
    }
}

fn reserve_labels(value: &str, labels: &mut BTreeSet<String>) {
    // Reserve every user-spelled symbol, including parameter:$R. These are *only* source
    // spellings; generated node/binder identities are never added to the public inventory.
    for suffix in value.split('$').skip(1) {
        let length = suffix
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            .count();
        if length != 0 {
            labels.insert(format!("${}", &suffix[..length]));
        }
    }
}

fn line_error(
    path: &str,
    byte_offset: usize,
    line: usize,
    code: &str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::source(
        code,
        message,
        SourceLocation {
            path: path.to_owned(),
            byte_offset,
            line,
            column: 1,
        },
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "one cumulative request admission and original source span"
)]
fn tokenize(
    path: &str,
    body: &str,
    start: usize,
    first_line: usize,
    tokens: &mut usize,
    nodes: &mut usize,
    public_labels: &mut BTreeSet<String>,
) -> Result<Block, Diagnostic> {
    let mut syntax: Vec<Syntax> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut roots = Vec::new();
    let mut cursor = 0;
    let mut line = first_line;
    let mut column = 1;
    let bytes = body.as_bytes();
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte.is_ascii_whitespace() {
            cursor += 1;
            if byte == b'\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
            continue;
        }
        if byte == b';' {
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
            continue;
        }
        let location = SourceLocation {
            path: path.to_owned(),
            byte_offset: start + cursor,
            line,
            column,
        };
        *tokens = tokens
            .checked_add(1)
            .ok_or_else(|| resource(&location, "structural token accounting overflowed"))?;
        if *tokens > MAXIMUM_STRUCTURAL_TOKENS {
            return Err(resource(
                &location,
                "complete input exceeds its structural token bound",
            ));
        }
        if byte == b')' {
            if stack.pop().is_none() {
                return Err(Diagnostic::source(
                    "change_block_parenthesis",
                    "unmatched closing parenthesis",
                    location,
                ));
            }
            cursor += 1;
            column += 1;
            continue;
        }
        *nodes = nodes
            .checked_add(1)
            .ok_or_else(|| resource(&location, "syntax node accounting overflowed"))?;
        if *nodes > MAXIMUM_STRUCTURAL_SYNTAX_NODES {
            return Err(resource(
                &location,
                "complete input exceeds its structural syntax-node bound",
            ));
        }
        let (kind, next) = match byte {
            b'(' => (SyntaxKind::List(Vec::new()), cursor + 1),
            b'"' => {
                // The shared decoder stops at the closing quote and rejects a physical newline.
                // Do not rescan the rest of a dense line for every small literal.
                let segment = &body[cursor..];
                let (value, consumed) = crate::platform::control::compact::parse_quoted(
                    path,
                    segment.as_bytes(),
                    segment,
                    0,
                    0,
                )
                .map_err(|mut error| {
                    if let Some(at) = &mut error.location {
                        at.byte_offset += location.byte_offset;
                        at.line = location.line;
                        at.column += location.column - 1;
                    }
                    error
                })?;
                (
                    SyntaxKind::Atom {
                        value,
                        quoted: true,
                    },
                    cursor + consumed,
                )
            }
            _ => {
                let end = bytes[cursor..]
                    .iter()
                    .position(|byte| {
                        byte.is_ascii_whitespace() || matches!(byte, b'(' | b')' | b';')
                    })
                    .map_or(bytes.len(), |n| cursor + n);
                let value = &body[cursor..end];
                if !value.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric()
                        || matches!(
                            byte,
                            b'_' | b'-' | b'.' | b'/' | b':' | b'+' | b'@' | b'%' | b'$'
                        )
                }) {
                    return Err(Diagnostic::source(
                        "change_block_atom",
                        "structural atoms use existing ASCII typed-reference spellings; text requires double quotes",
                        location,
                    ));
                }
                reserve_labels(value, public_labels);
                (
                    SyntaxKind::Atom {
                        value: value.to_owned(),
                        quoted: false,
                    },
                    end,
                )
            }
        };
        let id = syntax.len();
        syntax
            .try_reserve(1)
            .map_err(|_| resource(&location, "structural syntax allocation failed"))?;
        syntax.push(Syntax {
            kind,
            location: location.clone(),
        });
        if let Some(parent) = stack.last().copied() {
            if let SyntaxKind::List(children) = &mut syntax[parent].kind {
                children
                    .try_reserve(1)
                    .map_err(|_| resource(&location, "structural child allocation failed"))?;
                children.push(id);
            }
        } else {
            if !roots.is_empty() {
                return Err(Diagnostic::source(
                    "change_block_root_count",
                    "expression block requires exactly one expression with no trailing tokens",
                    location,
                ));
            }
            roots.push(id);
        }
        if byte == b'(' {
            stack
                .try_reserve(1)
                .map_err(|_| resource(&location, "structural syntax stack allocation failed"))?;
            stack.push(id);
        } else if bytes
            .get(next)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && !matches!(byte, b'(' | b')' | b';'))
        {
            return Err(Diagnostic::source(
                "change_block_separator",
                "a quoted value requires a token separator",
                location,
            ));
        }
        column += body[cursor..next].chars().count();
        cursor = next;
    }
    if let Some(id) = stack.last() {
        return Err(Diagnostic::source(
            "change_block_parenthesis",
            "expression is incomplete at expression.end",
            syntax[*id].location.clone(),
        ));
    }
    let root = roots.first().copied().ok_or_else(|| {
        line_error(
            path,
            start,
            first_line,
            "change_block_root_count",
            "expression block requires exactly one parenthesized expression",
        )
    })?;
    let block = Block {
        syntax,
        root,
        locals: BTreeMap::new(),
    };
    block.list(root)?;
    Ok(block)
}

fn resource(location: &SourceLocation, message: &str) -> Diagnostic {
    let mut error = Diagnostic::new(DiagnosticClass::Resource, "change_block_capacity", message);
    error.location = Some(location.clone());
    error
}
