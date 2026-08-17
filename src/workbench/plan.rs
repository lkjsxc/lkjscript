use super::{ContextPacket, ContextPacketDigest, MAX_WORKBENCH_INPUT_BYTES};
use crate::ids::{DraftSymbol, IdempotencyKey, NodeId, Revision, WorkspaceId};
use crate::interpret::{RunPolicy, RuntimeValue};
use crate::transaction::{
    ApplyTransactionRequest, Transaction, TransactionMode, TransactionOp, TransactionResponseSpec,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const MAX_PLAN_DEPTH: usize = 32;
const MAX_PLAN_ITEMS: usize = 65_536;
const MAX_PLAN_ERROR_BYTES: usize = 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanErrorCode {
    InvalidUtf8,
    InputTooLarge,
    Syntax,
    DepthExceeded,
    ItemLimitExceeded,
    DuplicateField,
    UnknownAlias,
    PacketRequired,
    PacketMismatch,
    Shape,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanError {
    pub code: PlanErrorCode,
    pub line: u32,
    pub column: u32,
    pub byte_offset: u64,
    pub message: String,
}

impl PlanError {
    fn at(code: PlanErrorCode, location: Location, message: impl Into<String>) -> Self {
        let mut message = message.into();
        if message.len() > MAX_PLAN_ERROR_BYTES {
            let mut end = MAX_PLAN_ERROR_BYTES;
            while !message.is_char_boundary(end) {
                end -= 1;
            }
            message.truncate(end);
        }
        Self {
            code,
            line: location.line,
            column: location.column,
            byte_offset: u64::try_from(location.offset).unwrap_or(u64::MAX),
            message,
        }
    }

    fn shape(message: impl Into<String>) -> Self {
        Self::at(PlanErrorCode::Shape, Location::start(), message)
    }
}

impl fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at line {}, column {}",
            self.message, self.line, self.column
        )
    }
}

impl std::error::Error for PlanError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedEditPlan {
    pub packet: Option<ContextPacketDigest>,
    pub request: ApplyTransactionRequest,
    pub alias_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedRunPlan {
    pub packet: Option<ContextPacketDigest>,
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub entry: NodeId,
    pub arguments: Vec<RuntimeValue>,
    pub policy: RunPolicy,
    pub alias_count: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EditDocument {
    #[serde(default)]
    packet: Option<ContextPacketDigest>,
    workspace: WorkspaceId,
    base_revision: Revision,
    #[serde(default)]
    idempotency_key: Option<IdempotencyKey>,
    operations: Vec<TransactionOp>,
    #[serde(default)]
    return_symbols: Vec<DraftSymbol>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RunDocument {
    #[serde(default)]
    packet: Option<ContextPacketDigest>,
    workspace: WorkspaceId,
    revision: Revision,
    entry: NodeId,
    arguments: Vec<RuntimeValue>,
    policy: RunPolicy,
}

pub fn parse_edit_plan(
    bytes: &[u8],
    mode: TransactionMode,
    packet: Option<&ContextPacket>,
) -> Result<ParsedEditPlan, PlanError> {
    let parsed = parse_document(bytes, "plan", packet)?;
    let document: EditDocument = serde_json::from_value(parsed.value)
        .map_err(|error| PlanError::shape(format!("edit plan shape is invalid: {error}")))?;
    bind_packet(
        document.packet,
        document.workspace,
        document.base_revision,
        parsed.alias_count,
        packet,
    )?;
    if mode == TransactionMode::ValidateOnly && document.idempotency_key.is_some() {
        return Err(PlanError::shape(
            "validate-only plans cannot carry an idempotency key",
        ));
    }
    Ok(ParsedEditPlan {
        packet: document.packet,
        request: ApplyTransactionRequest {
            transaction: Transaction {
                workspace: document.workspace,
                base_revision: document.base_revision,
                idempotency_key: document.idempotency_key,
                mode,
                operations: document.operations,
            },
            response: TransactionResponseSpec {
                return_symbols: document.return_symbols,
            },
        },
        alias_count: parsed.alias_count,
    })
}

pub fn parse_run_plan(
    bytes: &[u8],
    packet: Option<&ContextPacket>,
) -> Result<ParsedRunPlan, PlanError> {
    let parsed = parse_document(bytes, "run", packet)?;
    let document: RunDocument = serde_json::from_value(parsed.value)
        .map_err(|error| PlanError::shape(format!("run plan shape is invalid: {error}")))?;
    bind_packet(
        document.packet,
        document.workspace,
        document.revision,
        parsed.alias_count,
        packet,
    )?;
    Ok(ParsedRunPlan {
        packet: document.packet,
        workspace: document.workspace,
        revision: document.revision,
        entry: document.entry,
        arguments: document.arguments,
        policy: document.policy,
        alias_count: parsed.alias_count,
    })
}

fn bind_packet(
    declared: Option<ContextPacketDigest>,
    workspace: WorkspaceId,
    revision: Revision,
    alias_count: u64,
    packet: Option<&ContextPacket>,
) -> Result<(), PlanError> {
    match (declared, packet) {
        (None, None) if alias_count == 0 => Ok(()),
        (None, Some(_)) => Err(PlanError::at(
            PlanErrorCode::PacketMismatch,
            Location::start(),
            "a supplied packet must be named by its digest in the plan",
        )),
        (None, None) => Err(PlanError::at(
            PlanErrorCode::PacketRequired,
            Location::start(),
            "packet aliases require an exact packet",
        )),
        (Some(_), None) => Err(PlanError::at(
            PlanErrorCode::PacketRequired,
            Location::start(),
            "the declared packet was not supplied",
        )),
        (Some(digest), Some(packet)) => {
            if packet.digest != digest {
                return Err(PlanError::at(
                    PlanErrorCode::PacketMismatch,
                    Location::start(),
                    "plan and supplied packet digests differ",
                ));
            }
            if packet.payload.workspace != workspace {
                return Err(PlanError::at(
                    PlanErrorCode::PacketMismatch,
                    Location::start(),
                    "plan and packet workspaces differ",
                ));
            }
            if packet.payload.revision != revision {
                return Err(PlanError::at(
                    PlanErrorCode::PacketMismatch,
                    Location::start(),
                    "plan and packet revisions differ",
                ));
            }
            Ok(())
        }
    }
}

struct ParsedDocument {
    value: Value,
    alias_count: u64,
}

fn parse_document(
    bytes: &[u8],
    expected_root: &str,
    packet: Option<&ContextPacket>,
) -> Result<ParsedDocument, PlanError> {
    if bytes.len() > MAX_WORKBENCH_INPUT_BYTES {
        return Err(PlanError::at(
            PlanErrorCode::InputTooLarge,
            Location::start(),
            "workbench input exceeds the byte policy",
        ));
    }
    let input = std::str::from_utf8(bytes).map_err(|error| {
        PlanError::at(
            PlanErrorCode::InvalidUtf8,
            Location::from_offset(bytes, error.valid_up_to()),
            "workbench plan is not valid UTF-8",
        )
    })?;
    let aliases = packet
        .map(|packet| {
            packet
                .payload
                .aliases
                .iter()
                .map(|alias| (alias.alias.clone(), alias.node.to_string()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    Parser::new(input, aliases).parse(expected_root)
}

#[derive(Clone, Copy)]
struct Location {
    offset: usize,
    line: u32,
    column: u32,
}

impl Location {
    const fn start() -> Self {
        Self {
            offset: 0,
            line: 1,
            column: 1,
        }
    }

    fn from_offset(bytes: &[u8], offset: usize) -> Self {
        let mut line = 1_u32;
        let mut column = 1_u32;
        for byte in bytes.iter().take(offset) {
            if *byte == b'\n' {
                line = line.saturating_add(1);
                column = 1;
            } else {
                column = column.saturating_add(1);
            }
        }
        Self {
            offset,
            line,
            column,
        }
    }
}

enum TokenKind {
    OpenObject,
    CloseObject,
    OpenList,
    CloseList,
    OpenVariant,
    CloseVariant,
    Atom(String),
    String(String),
    Number(Number),
    Alias(String),
}

struct Token {
    kind: TokenKind,
    location: Location,
}

struct Lexer<'a> {
    input: &'a str,
    offset: usize,
    line: u32,
    column: u32,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            offset: 0,
            line: 1,
            column: 1,
        }
    }

    fn location(&self) -> Location {
        Location {
            offset: self.offset,
            line: self.line,
            column: self.column,
        }
    }

    fn next(&mut self) -> Result<Option<Token>, PlanError> {
        self.skip_whitespace();
        if self.offset == self.input.len() {
            return Ok(None);
        }
        let location = self.location();
        let byte = self.input.as_bytes()[self.offset];
        let kind = match byte {
            b'{' => {
                self.advance_ascii(1);
                TokenKind::OpenObject
            }
            b'}' => {
                self.advance_ascii(1);
                TokenKind::CloseObject
            }
            b'[' => {
                self.advance_ascii(1);
                TokenKind::OpenList
            }
            b']' => {
                self.advance_ascii(1);
                TokenKind::CloseList
            }
            b'(' => {
                self.advance_ascii(1);
                TokenKind::OpenVariant
            }
            b')' => {
                self.advance_ascii(1);
                TokenKind::CloseVariant
            }
            b'"' => TokenKind::String(self.string(location)?),
            _ => self.atom(location)?,
        };
        Ok(Some(Token { kind, location }))
    }

    fn skip_whitespace(&mut self) {
        while self.offset < self.input.len() {
            match self.input.as_bytes()[self.offset] {
                b' ' | b'\t' | b'\r' => self.advance_ascii(1),
                b'\n' => {
                    self.offset += 1;
                    self.line = self.line.saturating_add(1);
                    self.column = 1;
                }
                _ => break,
            }
        }
    }

    fn string(&mut self, location: Location) -> Result<String, PlanError> {
        let start = self.offset;
        self.advance_ascii(1);
        let mut escaped = false;
        while self.offset < self.input.len() {
            let byte = self.input.as_bytes()[self.offset];
            if byte == b'\n' || byte == b'\r' {
                return Err(PlanError::at(
                    PlanErrorCode::Syntax,
                    self.location(),
                    "a quoted string cannot contain a raw line break",
                ));
            }
            self.advance_ascii(1);
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                let spelling = &self.input[start..self.offset];
                return serde_json::from_str::<String>(spelling).map_err(|error| {
                    PlanError::at(
                        PlanErrorCode::Syntax,
                        location,
                        format!("invalid JSON string literal: {error}"),
                    )
                });
            }
        }
        Err(PlanError::at(
            PlanErrorCode::Syntax,
            location,
            "unterminated quoted string",
        ))
    }

    fn atom(&mut self, location: Location) -> Result<TokenKind, PlanError> {
        let start = self.offset;
        while self.offset < self.input.len() {
            let byte = self.input.as_bytes()[self.offset];
            if byte.is_ascii_whitespace() || b"{}[]()".contains(&byte) {
                break;
            }
            if matches!(byte, b',' | b';' | b'=') {
                return Err(PlanError::at(
                    PlanErrorCode::Syntax,
                    self.location(),
                    "commas, semicolons, and equals signs are not part of the plan grammar",
                ));
            }
            let character = self.input[self.offset..]
                .chars()
                .next()
                .ok_or_else(|| PlanError::at(PlanErrorCode::Syntax, location, "invalid token"))?;
            self.offset += character.len_utf8();
            self.column = self.column.saturating_add(1);
        }
        let atom = &self.input[start..self.offset];
        if atom.is_empty() {
            return Err(PlanError::at(
                PlanErrorCode::Syntax,
                location,
                "expected a plan token",
            ));
        }
        if let Some(alias) = atom.strip_prefix('@') {
            if !valid_alias(alias) {
                return Err(PlanError::at(
                    PlanErrorCode::Syntax,
                    location,
                    "packet aliases must match @[a-z][a-z0-9_]*",
                ));
            }
            return Ok(TokenKind::Alias(alias.to_owned()));
        }
        if numeric_spelling(atom) {
            let number = atom.parse::<Number>().map_err(|_| {
                PlanError::at(
                    PlanErrorCode::Syntax,
                    location,
                    "integer literal is outside the JSON integer domain",
                )
            })?;
            return Ok(TokenKind::Number(number));
        }
        if !valid_atom(atom) {
            return Err(PlanError::at(
                PlanErrorCode::Syntax,
                location,
                "bare atoms may contain ASCII letters, digits, underscore, hyphen, dot, or colon",
            ));
        }
        Ok(TokenKind::Atom(atom.to_owned()))
    }

    fn advance_ascii(&mut self, count: usize) {
        self.offset += count;
        self.column = self
            .column
            .saturating_add(u32::try_from(count).unwrap_or(u32::MAX));
    }
}

fn valid_alias(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes[0].is_ascii_lowercase()
        && bytes[1..]
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
}

fn valid_atom(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'))
}

fn numeric_spelling(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty() && digits.len() <= 20 && digits.bytes().all(|byte| byte.is_ascii_digit())
}

enum Frame {
    Object {
        fields: Map<String, Value>,
        keys: BTreeSet<String>,
        pending: Option<(String, Location)>,
        location: Location,
    },
    List {
        values: Vec<Value>,
        location: Location,
    },
    Variant {
        kind: Option<(String, Location)>,
        data: Option<Value>,
        location: Location,
    },
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    aliases: BTreeMap<String, String>,
    frames: Vec<Frame>,
    root: Option<Value>,
    items: usize,
    alias_count: u64,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str, aliases: BTreeMap<String, String>) -> Self {
        Self {
            lexer: Lexer::new(input),
            aliases,
            frames: Vec::new(),
            root: None,
            items: 0,
            alias_count: 0,
        }
    }

    fn parse(mut self, expected_root: &str) -> Result<ParsedDocument, PlanError> {
        let root = self.lexer.next()?.ok_or_else(|| {
            PlanError::at(PlanErrorCode::Syntax, Location::start(), "empty plan input")
        })?;
        match root.kind {
            TokenKind::Atom(value) if value == expected_root => {}
            _ => {
                return Err(PlanError::at(
                    PlanErrorCode::Syntax,
                    root.location,
                    format!("expected {expected_root} document"),
                ));
            }
        }
        let opening = self.lexer.next()?.ok_or_else(|| {
            PlanError::at(
                PlanErrorCode::Syntax,
                self.lexer.location(),
                "expected document object",
            )
        })?;
        if !matches!(opening.kind, TokenKind::OpenObject) {
            return Err(PlanError::at(
                PlanErrorCode::Syntax,
                opening.location,
                "expected an object after the document name",
            ));
        }
        self.frames.push(Frame::Object {
            fields: Map::new(),
            keys: BTreeSet::new(),
            pending: None,
            location: opening.location,
        });
        while let Some(token) = self.lexer.next()? {
            self.consume(token)?;
            if self.root.is_some() {
                if let Some(trailing) = self.lexer.next()? {
                    return Err(PlanError::at(
                        PlanErrorCode::Syntax,
                        trailing.location,
                        "trailing input after the document",
                    ));
                }
                break;
            }
        }
        if !self.frames.is_empty() {
            let location = self
                .frames
                .last()
                .map(frame_location)
                .unwrap_or_else(|| self.lexer.location());
            return Err(PlanError::at(
                PlanErrorCode::Syntax,
                location,
                "unterminated plan container",
            ));
        }
        let value = self.root.ok_or_else(|| {
            PlanError::at(
                PlanErrorCode::Syntax,
                self.lexer.location(),
                "unterminated plan document",
            )
        })?;
        Ok(ParsedDocument {
            value,
            alias_count: self.alias_count,
        })
    }

    fn consume(&mut self, token: Token) -> Result<(), PlanError> {
        match token.kind {
            TokenKind::CloseObject => self.close_object(token.location),
            TokenKind::CloseList => self.close_list(token.location),
            TokenKind::CloseVariant => self.close_variant(token.location),
            TokenKind::OpenObject => self.open(Frame::Object {
                fields: Map::new(),
                keys: BTreeSet::new(),
                pending: None,
                location: token.location,
            }),
            TokenKind::OpenList => self.open(Frame::List {
                values: Vec::new(),
                location: token.location,
            }),
            TokenKind::OpenVariant => self.open(Frame::Variant {
                kind: None,
                data: None,
                location: token.location,
            }),
            TokenKind::Atom(value) => self.atom_value(value, token.location),
            TokenKind::String(value) => self.add_value(Value::String(value), token.location),
            TokenKind::Number(value) => self.add_value(Value::Number(value), token.location),
            TokenKind::Alias(alias) => {
                let value = self.aliases.get(alias.as_str()).ok_or_else(|| {
                    PlanError::at(
                        if self.aliases.is_empty() {
                            PlanErrorCode::PacketRequired
                        } else {
                            PlanErrorCode::UnknownAlias
                        },
                        token.location,
                        format!("unknown packet alias @{alias}"),
                    )
                })?;
                self.alias_count = self.alias_count.checked_add(1).ok_or_else(|| {
                    PlanError::at(
                        PlanErrorCode::ItemLimitExceeded,
                        token.location,
                        "alias accounting overflow",
                    )
                })?;
                self.add_value(Value::String(value.clone()), token.location)
            }
        }
    }

    fn open(&mut self, frame: Frame) -> Result<(), PlanError> {
        if self.frames.len() >= MAX_PLAN_DEPTH {
            return Err(PlanError::at(
                PlanErrorCode::DepthExceeded,
                frame_location(&frame),
                "plan nesting exceeds the depth policy",
            ));
        }
        if let Some(Frame::Object { pending: None, .. }) = self.frames.last() {
            return Err(PlanError::at(
                PlanErrorCode::Syntax,
                frame_location(&frame),
                "an object field name must precede its value",
            ));
        }
        if let Some(Frame::Variant { kind: None, .. }) = self.frames.last() {
            return Err(PlanError::at(
                PlanErrorCode::Syntax,
                frame_location(&frame),
                "a tagged variant must begin with a kind atom",
            ));
        }
        self.bump_item(frame_location(&frame))?;
        self.frames.push(frame);
        Ok(())
    }

    fn atom_value(&mut self, value: String, location: Location) -> Result<(), PlanError> {
        if let Some(Frame::Object {
            pending,
            keys,
            fields: _,
            location: _,
        }) = self.frames.last_mut()
            && pending.is_none()
        {
            if !keys.insert(value.clone()) {
                return Err(PlanError::at(
                    PlanErrorCode::DuplicateField,
                    location,
                    format!("duplicate object field {value}"),
                ));
            }
            *pending = Some((value, location));
            return Ok(());
        }
        if let Some(Frame::Variant { kind, data, .. }) = self.frames.last_mut()
            && kind.is_none()
        {
            if data.is_some() {
                return Err(PlanError::at(
                    PlanErrorCode::Syntax,
                    location,
                    "variant kind must precede its payload",
                ));
            }
            *kind = Some((value, location));
            return Ok(());
        }
        let scalar = match value.as_str() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            "null" => Value::Null,
            _ => Value::String(value),
        };
        self.add_value(scalar, location)
    }

    fn close_object(&mut self, location: Location) -> Result<(), PlanError> {
        let frame = self.frames.pop().ok_or_else(|| {
            PlanError::at(
                PlanErrorCode::Syntax,
                location,
                "unexpected closing object delimiter",
            )
        })?;
        match frame {
            Frame::Object {
                fields,
                pending: None,
                ..
            } => self.finish_container(Value::Object(fields), location),
            Frame::Object {
                pending: Some((field, field_location)),
                ..
            } => Err(PlanError::at(
                PlanErrorCode::Syntax,
                field_location,
                format!("object field {field} has no value"),
            )),
            other => {
                self.frames.push(other);
                Err(PlanError::at(
                    PlanErrorCode::Syntax,
                    location,
                    "object delimiter closes a different container",
                ))
            }
        }
    }

    fn close_list(&mut self, location: Location) -> Result<(), PlanError> {
        let frame = self.frames.pop().ok_or_else(|| {
            PlanError::at(
                PlanErrorCode::Syntax,
                location,
                "unexpected closing list delimiter",
            )
        })?;
        match frame {
            Frame::List { values, .. } => self.finish_container(Value::Array(values), location),
            other => {
                self.frames.push(other);
                Err(PlanError::at(
                    PlanErrorCode::Syntax,
                    location,
                    "list delimiter closes a different container",
                ))
            }
        }
    }

    fn close_variant(&mut self, location: Location) -> Result<(), PlanError> {
        let frame = self.frames.pop().ok_or_else(|| {
            PlanError::at(
                PlanErrorCode::Syntax,
                location,
                "unexpected closing variant delimiter",
            )
        })?;
        match frame {
            Frame::Variant {
                kind: Some((kind, _)),
                data,
                ..
            } => {
                let mut object = Map::new();
                object.insert("kind".to_owned(), Value::String(kind));
                if let Some(data) = data {
                    object.insert("data".to_owned(), data);
                }
                self.finish_container(Value::Object(object), location)
            }
            Frame::Variant { kind: None, .. } => Err(PlanError::at(
                PlanErrorCode::Syntax,
                location,
                "tagged variant has no kind",
            )),
            other => {
                self.frames.push(other);
                Err(PlanError::at(
                    PlanErrorCode::Syntax,
                    location,
                    "variant delimiter closes a different container",
                ))
            }
        }
    }

    fn finish_container(&mut self, value: Value, location: Location) -> Result<(), PlanError> {
        if self.frames.is_empty() {
            if self.root.replace(value).is_some() {
                return Err(PlanError::at(
                    PlanErrorCode::Syntax,
                    location,
                    "multiple plan documents are not allowed",
                ));
            }
            Ok(())
        } else {
            self.add_value(value, location)
        }
    }

    fn add_value(&mut self, value: Value, location: Location) -> Result<(), PlanError> {
        self.bump_item(location)?;
        let frame = self.frames.last_mut().ok_or_else(|| {
            PlanError::at(
                PlanErrorCode::Syntax,
                location,
                "value appears outside the document object",
            )
        })?;
        match frame {
            Frame::Object {
                fields, pending, ..
            } => {
                let (field, _) = pending.take().ok_or_else(|| {
                    PlanError::at(
                        PlanErrorCode::Syntax,
                        location,
                        "object value has no field name",
                    )
                })?;
                fields.insert(field, value);
                Ok(())
            }
            Frame::List { values, .. } => {
                values.push(value);
                Ok(())
            }
            Frame::Variant { kind, data, .. } => {
                if kind.is_none() {
                    return Err(PlanError::at(
                        PlanErrorCode::Syntax,
                        location,
                        "tagged variant must begin with a kind atom",
                    ));
                }
                if data.replace(value).is_some() {
                    return Err(PlanError::at(
                        PlanErrorCode::Syntax,
                        location,
                        "tagged variant accepts at most one payload value",
                    ));
                }
                Ok(())
            }
        }
    }

    fn bump_item(&mut self, location: Location) -> Result<(), PlanError> {
        self.items = self.items.checked_add(1).ok_or_else(|| {
            PlanError::at(
                PlanErrorCode::ItemLimitExceeded,
                location,
                "plan item accounting overflow",
            )
        })?;
        if self.items > MAX_PLAN_ITEMS {
            return Err(PlanError::at(
                PlanErrorCode::ItemLimitExceeded,
                location,
                "plan exceeds the item policy",
            ));
        }
        Ok(())
    }
}

fn frame_location(frame: &Frame) -> Location {
    match frame {
        Frame::Object { location, .. }
        | Frame::List { location, .. }
        | Frame::Variant { location, .. } => *location,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::WorkspaceId;
    use crate::transaction::TransactionOp;

    fn workspace() -> WorkspaceId {
        WorkspaceId::from_bytes([0x11; 16])
    }

    #[test]
    fn compact_plan_normalizes_directly_to_the_typed_transaction() {
        let source = format!(
            "plan {{ workspace {} base_revision 0 packet null operations [\n\
             (create_package {{ symbol app name \"deployment\" }})\n\
             ] return_symbols [app] }}",
            workspace()
        );
        let parsed = parse_edit_plan(source.as_bytes(), TransactionMode::Commit, None)
            .expect("compact plan");
        assert_eq!(parsed.request.transaction.workspace, workspace());
        assert_eq!(parsed.request.transaction.base_revision, Revision::INITIAL);
        assert!(matches!(
            parsed.request.transaction.operations.as_slice(),
            [TransactionOp::CreatePackage { symbol, name }]
                if *symbol == DraftSymbol::new("app") && name == "deployment"
        ));
        assert_eq!(
            parsed.request.response.return_symbols,
            vec![DraftSymbol::new("app")]
        );
    }

    #[test]
    fn parser_rejects_duplicates_trailing_input_and_mismatched_delimiters() {
        for (source, code) in [
            (
                "plan { workspace x workspace y }",
                PlanErrorCode::DuplicateField,
            ),
            ("plan {} trailing", PlanErrorCode::Syntax),
            ("plan { operations [ }", PlanErrorCode::Syntax),
            ("plan { operations [(x a b)] }", PlanErrorCode::Syntax),
        ] {
            assert_eq!(
                parse_edit_plan(source.as_bytes(), TransactionMode::Commit, None)
                    .expect_err("malformed plan")
                    .code,
                code
            );
        }
    }

    #[test]
    fn quoted_alias_spelling_is_not_alias_resolution() {
        let source = format!(
            "plan {{ workspace {} base_revision 0 packet null operations [\n\
             (create_package {{ symbol app name \"@n1\" }})] }}",
            workspace()
        );
        let parsed = parse_edit_plan(source.as_bytes(), TransactionMode::Commit, None)
            .expect("quoted alias name");
        assert_eq!(parsed.alias_count, 0);
        assert!(matches!(
            parsed.request.transaction.operations.as_slice(),
            [TransactionOp::CreatePackage { name, .. }] if name == "@n1"
        ));
    }

    #[test]
    fn unquoted_alias_without_packet_rejects_at_its_location() {
        let source = format!(
            "plan {{\n workspace {}\n base_revision 0\n operations [\n\
             (rename_node {{ node @n1 name x }})] }}",
            workspace()
        );
        let error = parse_edit_plan(source.as_bytes(), TransactionMode::Commit, None)
            .expect_err("packet required");
        assert_eq!(error.code, PlanErrorCode::PacketRequired);
        assert_eq!(error.line, 5);
    }
}
