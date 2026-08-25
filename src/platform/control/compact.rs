//! Flat, line-oriented records shared by compact requests and responses.

use crate::platform::diagnostic::{Diagnostic, DiagnosticClass, SourceLocation};
use std::collections::BTreeSet;

pub const MAXIMUM_COMPACT_INPUT_BYTES: usize = 4 * 1_048_576;
pub const MAXIMUM_COMPACT_RECORDS: usize = 10_000;
const MAXIMUM_COMPACT_RECORD_BYTES: usize = 64 * 1_024;
const MAXIMUM_COMPACT_FIELDS: usize = 256;
const MAXIMUM_COMPACT_NAME_BYTES: usize = 64;
const MAXIMUM_COMPACT_VALUE_BYTES: usize = 1_048_576;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompactField {
    pub name: String,
    pub value: String,
    pub location: SourceLocation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompactRecord {
    pub operation: String,
    pub fields: Vec<CompactField>,
    pub location: SourceLocation,
}

/// Independent deterministic bounds for one finite compact response.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompactResponseLimits {
    pub maximum_bytes: usize,
    pub maximum_records: usize,
}

impl CompactResponseLimits {
    fn validate(self) -> Result<Self, Diagnostic> {
        if self.maximum_bytes == 0
            || self.maximum_bytes > isize::MAX as usize
            || self.maximum_records == 0
            || self.maximum_records > isize::MAX as usize
        {
            return Err(Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "control_response_limits",
                "compact response limits require nonzero addressable byte and record maxima",
            ));
        }
        Ok(self)
    }
}

/// Bounded owner of one finite compact response.
///
/// Each append is atomic with respect to this buffer: rendering, framing validation, resource
/// checks, and allocation all complete before the response bytes or counters change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompactResponseWriter {
    limits: CompactResponseLimits,
    bytes: Vec<u8>,
    records: usize,
}

impl CompactResponseWriter {
    pub fn new(limits: CompactResponseLimits) -> Result<Self, Diagnostic> {
        Ok(Self {
            limits: limits.validate()?,
            bytes: Vec::new(),
            records: 0,
        })
    }

    pub fn append_record(
        &mut self,
        operation: &str,
        fields: &[(&str, &str)],
    ) -> Result<(), Diagnostic> {
        let record = render_record(operation, fields)?;
        self.preflight(record.len(), 1)?;
        self.reserve(record.len())?;
        self.bytes.extend_from_slice(record.as_bytes());
        self.records += 1;
        Ok(())
    }

    /// Appends one or more complete serialized records, such as an executable-registry section.
    /// Empty input is a no-op. Nonempty input must contain only valid, nonblank, newline-terminated
    /// physical records.
    pub fn append_serialized_records(&mut self, bytes: &[u8]) -> Result<(), Diagnostic> {
        if bytes.is_empty() {
            return Ok(());
        }
        if bytes.last() != Some(&b'\n') {
            return Err(Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "control_response_records_newline",
                "serialized compact response records are not newline-complete",
            ));
        }
        let records = bytes.iter().filter(|byte| **byte == b'\n').count();
        self.preflight(bytes.len(), records)?;
        validate_serialized_records(bytes)?;
        self.reserve(bytes.len())?;
        self.bytes.extend_from_slice(bytes);
        self.records += records;
        Ok(())
    }

    pub const fn byte_count(&self) -> usize {
        self.bytes.len()
    }

    pub const fn record_count(&self) -> usize {
        self.records
    }

    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn preflight(
        &self,
        additional_bytes: usize,
        additional_records: usize,
    ) -> Result<(), Diagnostic> {
        let records = self
            .records
            .checked_add(additional_records)
            .ok_or_else(|| {
                response_budget_error(
                    "control_response_record_budget",
                    "compact response record accounting overflowed",
                )
            })?;
        if records > self.limits.maximum_records {
            return Err(response_budget_error(
                "control_response_record_budget",
                format!(
                    "compact response requires {records} records, exceeding its {}-record budget",
                    self.limits.maximum_records
                ),
            ));
        }
        let bytes = self
            .bytes
            .len()
            .checked_add(additional_bytes)
            .ok_or_else(|| {
                response_budget_error(
                    "control_response_byte_budget",
                    "compact response byte accounting overflowed",
                )
            })?;
        if bytes > self.limits.maximum_bytes {
            return Err(response_budget_error(
                "control_response_byte_budget",
                format!(
                    "compact response requires {bytes} bytes, exceeding its {}-byte budget",
                    self.limits.maximum_bytes
                ),
            ));
        }
        Ok(())
    }

    fn reserve(&mut self, additional: usize) -> Result<(), Diagnostic> {
        self.bytes.try_reserve_exact(additional).map_err(|_| {
            response_budget_error(
                "control_response_allocation",
                "compact response buffer allocation failed within its declared byte budget",
            )
        })
    }
}

fn validate_serialized_records(bytes: &[u8]) -> Result<(), Diagnostic> {
    for line in bytes.split_inclusive(|byte| *byte == b'\n') {
        match parse_records("<compact-response-records>", line) {
            Ok(records) if records.len() == 1 => {}
            Ok(_) => {
                return Err(Diagnostic::new(
                    DiagnosticClass::Infrastructure,
                    "control_response_records_blank",
                    "serialized compact response records contain a blank physical record",
                ));
            }
            Err(diagnostics) => {
                let code = diagnostics
                    .first()
                    .map_or("unknown", |diagnostic| diagnostic.code.as_str());
                return Err(Diagnostic::new(
                    DiagnosticClass::Infrastructure,
                    "control_response_records_invalid",
                    format!("serialized compact response record failed validation with {code}"),
                ));
            }
        }
    }
    Ok(())
}

fn response_budget_error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Resource, code, message)
}

/// Parses independent physical records and reports at most one syntax diagnostic per malformed
/// record. Records never span lines, so a malformed record cannot hide the location of later
/// independent records.
pub fn parse_records(path: &str, input: &[u8]) -> Result<Vec<CompactRecord>, Vec<Diagnostic>> {
    if input.len() > MAXIMUM_COMPACT_INPUT_BYTES {
        return Err(vec![Diagnostic::new(
            DiagnosticClass::Resource,
            "control_input_bytes",
            format!(
                "compact input contains {} bytes, exceeding the {}-byte format bound",
                input.len(),
                MAXIMUM_COMPACT_INPUT_BYTES
            ),
        )]);
    }
    let text = std::str::from_utf8(input).map_err(|error| {
        vec![source_error(
            path,
            input,
            error.valid_up_to(),
            "control_utf8",
            "compact input is not valid UTF-8",
        )]
    })?;
    let mut records = Vec::new();
    let mut diagnostics = Vec::new();
    let mut record_count = 0_usize;
    let mut offset = 0_usize;
    for line in text.split_inclusive('\n') {
        let body = line.strip_suffix('\n').unwrap_or(line);
        let body = body.strip_suffix('\r').unwrap_or(body);
        if body.as_bytes().iter().all(u8::is_ascii_whitespace) {
            offset = offset.saturating_add(line.len());
            continue;
        }
        record_count = record_count.saturating_add(1);
        if record_count > MAXIMUM_COMPACT_RECORDS {
            return Err(vec![source_error(
                path,
                input,
                offset,
                "control_record_count",
                format!("compact input exceeds the {MAXIMUM_COMPACT_RECORDS}-record format bound"),
            )]);
        }
        if body.len() > MAXIMUM_COMPACT_RECORD_BYTES {
            diagnostics.push(source_error(
                path,
                input,
                offset,
                "control_record_bytes",
                format!(
                    "compact record exceeds the {MAXIMUM_COMPACT_RECORD_BYTES}-byte format bound"
                ),
            ));
        } else {
            match parse_record(path, input, body, offset) {
                Ok(record) => records.push(record),
                Err(diagnostic) => diagnostics.push(diagnostic),
            }
        }
        offset = offset.saturating_add(line.len());
    }
    if diagnostics.is_empty() {
        Ok(records)
    } else {
        Err(diagnostics)
    }
}

/// Renders one record with the same deterministic escaping accepted by [`parse_records`].
pub fn render_record(operation: &str, fields: &[(&str, &str)]) -> Result<String, Diagnostic> {
    validate_operation(operation).map_err(|message| {
        Diagnostic::new(
            DiagnosticClass::Infrastructure,
            "control_render_operation",
            message,
        )
    })?;
    if fields.len() > MAXIMUM_COMPACT_FIELDS {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "control_render_fields",
            format!("compact record exceeds the {MAXIMUM_COMPACT_FIELDS}-field format bound"),
        ));
    }
    let mut names = BTreeSet::new();
    let mut rendered_bytes = operation
        .len()
        .checked_add(1)
        .ok_or_else(record_size_error)?;
    for (name, value) in fields {
        validate_field_name(name).map_err(|message| {
            Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "control_render_field",
                message,
            )
        })?;
        if !names.insert(*name) {
            return Err(Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "control_render_duplicate_field",
                format!("compact renderer received duplicate field '{name}'"),
            ));
        }
        if value.len() > MAXIMUM_COMPACT_VALUE_BYTES {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "control_render_value_bytes",
                format!(
                    "compact field '{name}' exceeds the {MAXIMUM_COMPACT_VALUE_BYTES}-byte format bound"
                ),
            ));
        }
        rendered_bytes = rendered_bytes
            .checked_add(2)
            .and_then(|bytes| bytes.checked_add(name.len()))
            .and_then(|bytes| bytes.checked_add(rendered_value_bytes(value)?))
            .ok_or_else(record_size_error)?;
        if rendered_bytes > MAXIMUM_COMPACT_RECORD_BYTES {
            return Err(record_size_error());
        }
    }
    let mut output = String::new();
    output.try_reserve_exact(rendered_bytes).map_err(|_| {
        response_budget_error(
            "control_render_allocation",
            "compact record buffer allocation failed within its format bound",
        )
    })?;
    output.push_str(operation);
    for (name, value) in fields {
        output.push(' ');
        output.push_str(name);
        output.push('=');
        render_value(value, &mut output);
    }
    output.push('\n');
    debug_assert_eq!(output.len(), rendered_bytes);
    Ok(output)
}

fn rendered_value_bytes(value: &str) -> Option<usize> {
    if !value.is_empty() && value.bytes().all(is_bare_byte) {
        return Some(value.len());
    }
    value.chars().try_fold(2_usize, |bytes, character| {
        let additional = match character {
            '"' | '\\' | '\n' | '\r' | '\t' => 2,
            character if character.is_control() => 4 + hexadecimal_digits(u32::from(character)),
            character => character.len_utf8(),
        };
        bytes.checked_add(additional)
    })
}

fn hexadecimal_digits(mut value: u32) -> usize {
    let mut digits = 1;
    while value >= 16 {
        value /= 16;
        digits += 1;
    }
    digits
}

fn record_size_error() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Resource,
        "control_render_record_bytes",
        format!(
            "compact output record exceeds the {MAXIMUM_COMPACT_RECORD_BYTES}-byte format bound"
        ),
    )
}

fn parse_record(
    path: &str,
    complete: &[u8],
    line: &str,
    line_offset: usize,
) -> Result<CompactRecord, Diagnostic> {
    let bytes = line.as_bytes();
    let mut cursor = skip_ascii_whitespace(bytes, 0);
    let operation_start = cursor;
    cursor = take_name(bytes, cursor, true);
    if operation_start == cursor {
        return Err(source_error(
            path,
            complete,
            line_offset.saturating_add(cursor),
            "control_operation",
            "compact record requires a closed operation name",
        ));
    }
    let operation = &line[operation_start..cursor];
    validate_operation(operation).map_err(|message| {
        source_error(
            path,
            complete,
            line_offset.saturating_add(operation_start),
            "control_operation",
            message,
        )
    })?;
    let mut fields = Vec::new();
    let mut names = BTreeSet::new();
    loop {
        let before_space = cursor;
        cursor = skip_ascii_whitespace(bytes, cursor);
        if cursor == bytes.len() {
            break;
        }
        if cursor == before_space {
            return Err(source_error(
                path,
                complete,
                line_offset.saturating_add(cursor),
                "control_field_separator",
                "compact fields must be separated by ASCII whitespace",
            ));
        }
        if fields.len() == MAXIMUM_COMPACT_FIELDS {
            return Err(source_error(
                path,
                complete,
                line_offset.saturating_add(cursor),
                "control_field_count",
                format!("compact record exceeds the {MAXIMUM_COMPACT_FIELDS}-field format bound"),
            ));
        }
        let name_start = cursor;
        cursor = take_name(bytes, cursor, false);
        if name_start == cursor {
            return Err(source_error(
                path,
                complete,
                line_offset.saturating_add(cursor),
                "control_field_name",
                "compact field requires a closed lowercase name",
            ));
        }
        let name = &line[name_start..cursor];
        validate_field_name(name).map_err(|message| {
            source_error(
                path,
                complete,
                line_offset.saturating_add(name_start),
                "control_field_name",
                message,
            )
        })?;
        if !names.insert(name.to_owned()) {
            return Err(source_error(
                path,
                complete,
                line_offset.saturating_add(name_start),
                "control_duplicate_field",
                format!("compact record repeats field '{name}'"),
            ));
        }
        if bytes.get(cursor) != Some(&b'=') {
            return Err(source_error(
                path,
                complete,
                line_offset.saturating_add(cursor),
                "control_field_equals",
                format!("compact field '{name}' requires '=' immediately after its name"),
            ));
        }
        cursor += 1;
        let value_start = cursor;
        let (value, next) = if bytes.get(cursor) == Some(&b'"') {
            parse_quoted(path, complete, line, line_offset, cursor)?
        } else {
            parse_bare(path, complete, line, line_offset, cursor)?
        };
        if value.len() > MAXIMUM_COMPACT_VALUE_BYTES {
            return Err(source_error(
                path,
                complete,
                line_offset.saturating_add(value_start),
                "control_value_bytes",
                format!(
                    "compact field '{name}' exceeds the {MAXIMUM_COMPACT_VALUE_BYTES}-byte format bound"
                ),
            ));
        }
        fields.push(CompactField {
            name: name.to_owned(),
            value,
            location: location(path, complete, line_offset.saturating_add(name_start)),
        });
        cursor = next;
    }
    Ok(CompactRecord {
        operation: operation.to_owned(),
        fields,
        location: location(path, complete, line_offset.saturating_add(operation_start)),
    })
}

fn parse_bare(
    path: &str,
    complete: &[u8],
    line: &str,
    line_offset: usize,
    start: usize,
) -> Result<(String, usize), Diagnostic> {
    let bytes = line.as_bytes();
    let mut cursor = start;
    while let Some(byte) = bytes.get(cursor).copied() {
        if byte.is_ascii_whitespace() {
            break;
        }
        if !is_bare_byte(byte) {
            return Err(source_error(
                path,
                complete,
                line_offset.saturating_add(cursor),
                "control_bare_value",
                "compact bare values use only ASCII token characters; quote this value",
            ));
        }
        cursor += 1;
    }
    if cursor == start {
        return Err(source_error(
            path,
            complete,
            line_offset.saturating_add(start),
            "control_value_missing",
            "compact field value is missing; use \"\" for an empty value",
        ));
    }
    Ok((line[start..cursor].to_owned(), cursor))
}

fn parse_quoted(
    path: &str,
    complete: &[u8],
    line: &str,
    line_offset: usize,
    start: usize,
) -> Result<(String, usize), Diagnostic> {
    let bytes = line.as_bytes();
    let mut cursor = start + 1;
    let mut segment = cursor;
    let mut value = String::new();
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'"' => {
                value.push_str(&line[segment..cursor]);
                return Ok((value, cursor + 1));
            }
            b'\\' => {
                value.push_str(&line[segment..cursor]);
                cursor += 1;
                let escape_offset = cursor;
                let escaped = bytes.get(cursor).copied().ok_or_else(|| {
                    source_error(
                        path,
                        complete,
                        line_offset.saturating_add(cursor),
                        "control_escape_truncated",
                        "compact quoted escape is truncated",
                    )
                })?;
                match escaped {
                    b'"' => value.push('"'),
                    b'\\' => value.push('\\'),
                    b'n' => value.push('\n'),
                    b'r' => value.push('\r'),
                    b't' => value.push('\t'),
                    b'u' => {
                        let (character, next) = parse_unicode_escape(
                            path,
                            complete,
                            bytes,
                            line_offset,
                            escape_offset,
                        )?;
                        value.push(character);
                        cursor = next;
                        segment = cursor;
                        continue;
                    }
                    _ => {
                        return Err(source_error(
                            path,
                            complete,
                            line_offset.saturating_add(cursor),
                            "control_escape_unknown",
                            "compact quoted value contains an unknown escape",
                        ));
                    }
                }
                cursor += 1;
                segment = cursor;
            }
            byte if byte.is_ascii_control() => {
                return Err(source_error(
                    path,
                    complete,
                    line_offset.saturating_add(cursor),
                    "control_quoted_control",
                    "compact quoted value contains an unescaped control character",
                ));
            }
            _ => cursor += 1,
        }
    }
    Err(source_error(
        path,
        complete,
        line_offset.saturating_add(start),
        "control_quote_unclosed",
        "compact quoted value is not closed on this record",
    ))
}

fn parse_unicode_escape(
    path: &str,
    complete: &[u8],
    bytes: &[u8],
    line_offset: usize,
    u_offset: usize,
) -> Result<(char, usize), Diagnostic> {
    if bytes.get(u_offset + 1) != Some(&b'{') {
        return Err(source_error(
            path,
            complete,
            line_offset.saturating_add(u_offset),
            "control_unicode_escape",
            "compact Unicode escape must use \\u{hex}",
        ));
    }
    let mut cursor = u_offset + 2;
    let digits_start = cursor;
    let mut value = 0_u32;
    while cursor < bytes.len() && bytes[cursor] != b'}' {
        if cursor.saturating_sub(digits_start) == 6 {
            return Err(source_error(
                path,
                complete,
                line_offset.saturating_add(cursor),
                "control_unicode_escape",
                "compact Unicode escape contains more than six hexadecimal digits",
            ));
        }
        let digit = hex_digit(bytes[cursor]).ok_or_else(|| {
            source_error(
                path,
                complete,
                line_offset.saturating_add(cursor),
                "control_unicode_escape",
                "compact Unicode escape contains a non-hexadecimal digit",
            )
        })?;
        value = value.saturating_mul(16).saturating_add(digit);
        cursor += 1;
    }
    if cursor == digits_start || bytes.get(cursor) != Some(&b'}') {
        return Err(source_error(
            path,
            complete,
            line_offset.saturating_add(u_offset),
            "control_unicode_escape",
            "compact Unicode escape is empty or unclosed",
        ));
    }
    let character = char::from_u32(value).ok_or_else(|| {
        source_error(
            path,
            complete,
            line_offset.saturating_add(u_offset),
            "control_unicode_scalar",
            "compact Unicode escape does not encode a Unicode scalar value",
        )
    })?;
    Ok((character, cursor + 1))
}

fn render_value(value: &str, output: &mut String) {
    if !value.is_empty() && value.bytes().all(is_bare_byte) {
        output.push_str(value);
        return;
    }
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str("\\u{");
                output.push_str(&format!("{:x}", u32::from(character)));
                output.push('}');
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

fn validate_operation(value: &str) -> Result<(), String> {
    validate_name(value, true, "operation")
}

fn validate_field_name(value: &str) -> Result<(), String> {
    validate_name(value, false, "field")
}

fn validate_name(value: &str, operation: bool, label: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > MAXIMUM_COMPACT_NAME_BYTES {
        return Err(format!(
            "compact {label} name requires 1 through {MAXIMUM_COMPACT_NAME_BYTES} bytes"
        ));
    }
    let mut bytes = value.bytes();
    if !bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        || !bytes.all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || byte == b'-'
                || (operation && byte == b'.')
        })
    {
        return Err(format!(
            "compact {label} name must start with a lowercase ASCII letter and contain only lowercase letters, digits, '-'{}",
            if operation { ", or '.'" } else { "" }
        ));
    }
    Ok(())
}

fn take_name(bytes: &[u8], start: usize, operation: bool) -> usize {
    let mut cursor = start;
    while let Some(byte) = bytes.get(cursor).copied() {
        if byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || byte == b'-'
            || (operation && byte == b'.')
        {
            cursor += 1;
        } else {
            break;
        }
    }
    cursor
}

fn skip_ascii_whitespace(bytes: &[u8], mut cursor: usize) -> usize {
    while bytes
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        cursor += 1;
    }
    cursor
}

fn is_bare_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'_' | b'-' | b'.' | b'/' | b':' | b'+' | b'@' | b'%' | b'$'
        )
}

fn hex_digit(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte - b'0')),
        b'a'..=b'f' => Some(u32::from(byte - b'a' + 10)),
        b'A'..=b'F' => Some(u32::from(byte - b'A' + 10)),
        _ => None,
    }
}

fn source_error(
    path: &str,
    complete: &[u8],
    offset: usize,
    code: &str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::source(code, message, location(path, complete, offset))
}

fn location(path: &str, complete: &[u8], offset: usize) -> SourceLocation {
    let bounded = offset.min(complete.len());
    let prefix = &complete[..bounded];
    let line_start = prefix
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |index| index + 1);
    let line = prefix
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
        .saturating_add(1);
    let column = std::str::from_utf8(&complete[line_start..bounded])
        .map(|value| value.chars().count().saturating_add(1))
        .unwrap_or(1);
    SourceLocation {
        path: path.to_owned(),
        byte_offset: bounded,
        line,
        column,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_round_trip_with_one_escape_rule() {
        let rendered = render_record(
            "create.module",
            &[
                ("symbol", "$notes"),
                ("name", "notes and \"ideas\""),
                ("empty", ""),
                ("unicode", "日本語"),
                ("line", "a\nb"),
            ],
        )
        .expect("render compact record");
        assert_eq!(
            rendered,
            "create.module symbol=$notes name=\"notes and \\\"ideas\\\"\" empty=\"\" unicode=\"日本語\" line=\"a\\nb\"\n"
        );
        let parsed = parse_records("change.lkjc", rendered.as_bytes())
            .expect("parse rendered compact record");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].operation, "create.module");
        assert_eq!(parsed[0].fields[1].value, "notes and \"ideas\"");
        assert_eq!(parsed[0].fields[2].value, "");
        assert_eq!(parsed[0].fields[3].value, "日本語");
        assert_eq!(parsed[0].fields[4].value, "a\nb");
    }

    #[test]
    fn malformed_lines_report_independent_exact_locations() {
        let diagnostics = parse_records(
            "broken.lkjc",
            b"create.module symbol=$ok name=one\ncreate.module symbol=$x symbol=$y\ncreate.module name=\"open\ncreate.module name=after\n",
        )
        .expect_err("two malformed records must reject the complete request");
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].code, "control_duplicate_field");
        assert_eq!(
            diagnostics[0].location.as_ref().map(|value| value.line),
            Some(2)
        );
        assert_eq!(diagnostics[1].code, "control_quote_unclosed");
        assert_eq!(
            diagnostics[1].location.as_ref().map(|value| value.line),
            Some(3)
        );
    }

    #[test]
    fn parser_rejects_unknown_escape_non_ascii_bare_and_trailing_token() {
        let cases: &[(&[u8], &str)] = &[
            (b"result value=\"\\q\"\n", "control_escape_unknown"),
            ("result value=日本語\n".as_bytes(), "control_bare_value"),
            (b"result value=\"ok\"tail\n", "control_field_separator"),
        ];
        for (input, code) in cases {
            let diagnostics = parse_records("case.lkjc", input)
                .expect_err("malformed compact record must reject");
            assert_eq!(diagnostics[0].code, *code);
        }
    }

    #[test]
    fn unicode_escape_accepts_scalars_and_rejects_surrogates() {
        let parsed = parse_records("unicode.lkjc", b"result value=\"\\u{1f642}\"\n")
            .expect("valid Unicode scalar escape");
        assert_eq!(parsed[0].fields[0].value, "🙂");
        let diagnostics = parse_records("unicode.lkjc", b"result value=\"\\u{d800}\"\n")
            .expect_err("surrogate escape must reject");
        assert_eq!(diagnostics[0].code, "control_unicode_scalar");
    }

    #[test]
    fn parser_rejects_invalid_utf8_and_duplicate_render_fields() {
        let diagnostics =
            parse_records("invalid.lkjc", &[0xff]).expect_err("invalid UTF-8 must reject");
        assert_eq!(diagnostics[0].code, "control_utf8");
        let error = render_record("result", &[("status", "ok"), ("status", "again")])
            .expect_err("renderer duplicate field must reject");
        assert_eq!(error.code, "control_render_duplicate_field");
    }

    #[test]
    fn record_bound_stops_before_unbounded_diagnostic_accumulation() {
        let mut input = String::new();
        for _ in 0..=MAXIMUM_COMPACT_RECORDS {
            input.push_str("!\n");
        }
        let diagnostics = parse_records("large.lkjc", input.as_bytes())
            .expect_err("record exhaustion must reject");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "control_record_count");
        assert_eq!(
            diagnostics[0].location.as_ref().map(|value| value.line),
            Some(MAXIMUM_COMPACT_RECORDS + 1)
        );
    }

    #[test]
    fn response_writer_enforces_bytes_and_records_independently_before_append() {
        let oversized_value = "x".repeat(MAXIMUM_COMPACT_RECORD_BYTES);
        let error = render_record("result", &[("value", &oversized_value)])
            .expect_err("record size must be checked before rendering its buffer");
        assert_eq!(error.code, "control_render_record_bytes");

        let one = render_record("result", &[("status", "ok")]).expect("one record");
        let mut byte_limited = CompactResponseWriter::new(CompactResponseLimits {
            maximum_bytes: one.len(),
            maximum_records: 2,
        })
        .expect("byte-limited writer");
        byte_limited
            .append_record("result", &[("status", "ok")])
            .expect("first record fits exactly");
        let error = byte_limited
            .append_record("next", &[("command", "again")])
            .expect_err("byte budget must reject before append");
        assert_eq!(error.code, "control_response_byte_budget");
        assert_eq!(byte_limited.byte_count(), one.len());
        assert_eq!(byte_limited.record_count(), 1);
        assert_eq!(byte_limited.finish(), one.as_bytes());

        let mut record_limited = CompactResponseWriter::new(CompactResponseLimits {
            maximum_bytes: MAXIMUM_COMPACT_INPUT_BYTES,
            maximum_records: 1,
        })
        .expect("record-limited writer");
        record_limited
            .append_record("result", &[("status", "ok")])
            .expect("first record fits");
        let bytes_before = record_limited.byte_count();
        let error = record_limited
            .append_record("next", &[("command", "again")])
            .expect_err("record budget must reject before append");
        assert_eq!(error.code, "control_response_record_budget");
        assert_eq!(record_limited.byte_count(), bytes_before);
        assert_eq!(record_limited.record_count(), 1);
    }

    #[test]
    fn response_writer_validates_and_counts_serialized_physical_records() {
        let section = b"owner.kind name=module\nowner.kind name=declaration\n";
        let mut writer = CompactResponseWriter::new(CompactResponseLimits {
            maximum_bytes: section.len(),
            maximum_records: 2,
        })
        .expect("section writer");
        writer
            .append_serialized_records(section)
            .expect("valid complete records");
        assert_eq!(writer.byte_count(), section.len());
        assert_eq!(writer.record_count(), 2);
        assert_eq!(writer.finish(), section);

        let mut record_limited = CompactResponseWriter::new(CompactResponseLimits {
            maximum_bytes: section.len(),
            maximum_records: 1,
        })
        .expect("record-limited section writer");
        let error = record_limited
            .append_serialized_records(section)
            .expect_err("physical section records must consume the record budget");
        assert_eq!(error.code, "control_response_record_budget");
        assert_eq!(record_limited.byte_count(), 0);
        assert_eq!(record_limited.record_count(), 0);

        for (bytes, code) in [
            (
                b"owner.kind name=module".as_slice(),
                "control_response_records_newline",
            ),
            (
                b"owner.kind name=module\n\n".as_slice(),
                "control_response_records_blank",
            ),
            (
                b"owner.kind name=\"open\n".as_slice(),
                "control_response_records_invalid",
            ),
        ] {
            let mut writer = CompactResponseWriter::new(CompactResponseLimits {
                maximum_bytes: MAXIMUM_COMPACT_INPUT_BYTES,
                maximum_records: MAXIMUM_COMPACT_RECORDS,
            })
            .expect("response writer");
            let error = writer
                .append_serialized_records(bytes)
                .expect_err("malformed serialized records must reject");
            assert_eq!(error.code, code);
            assert_eq!(writer.byte_count(), 0);
            assert_eq!(writer.record_count(), 0);
        }
    }

    #[test]
    fn response_writer_rejects_invalid_limit_domains() {
        for limits in [
            CompactResponseLimits {
                maximum_bytes: 0,
                maximum_records: 1,
            },
            CompactResponseLimits {
                maximum_bytes: 1,
                maximum_records: 0,
            },
            CompactResponseLimits {
                maximum_bytes: usize::MAX,
                maximum_records: 1,
            },
            CompactResponseLimits {
                maximum_bytes: 1,
                maximum_records: usize::MAX,
            },
        ] {
            let error = CompactResponseWriter::new(limits)
                .expect_err("zero and unaddressable limits must reject");
            assert_eq!(error.code, "control_response_limits");
        }

        CompactResponseWriter::new(CompactResponseLimits {
            maximum_bytes: MAXIMUM_COMPACT_INPUT_BYTES + 1,
            maximum_records: MAXIMUM_COMPACT_RECORDS + 1,
        })
        .expect("response budgets must be independent from compact input format bounds");
    }
}
