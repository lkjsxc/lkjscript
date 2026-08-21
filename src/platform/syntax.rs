//! Bounded parser for the maintained textual source authority.
//!
//! Authored bytes retain comments and formatting. `semantic_bytes` is a canonical, comment-free
//! projection used only for semantic equality and compilation; it is never a second editable
//! authority.

use super::diagnostic::{Diagnostic, SourceLocation};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceLimits {
    pub maximum_bytes: usize,
    pub maximum_forms: usize,
    pub maximum_depth: usize,
    pub maximum_atom_bytes: usize,
    pub maximum_string_bytes: usize,
}

impl Default for SourceLimits {
    fn default() -> Self {
        Self {
            maximum_bytes: 1_048_576,
            maximum_forms: 100_000,
            maximum_depth: 256,
            maximum_atom_bytes: 256,
            maximum_string_bytes: 1_048_576,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSpan {
    pub byte_start: usize,
    pub byte_end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Atom(String);

impl Atom {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum FormKind {
    Atom(Atom),
    Integer(i64),
    String(String),
    List(Vec<Form>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Form {
    pub span: SourceSpan,
    pub value: FormKind,
}

impl Form {
    pub fn atom(&self) -> Option<&str> {
        match &self.value {
            FormKind::Atom(atom) => Some(atom.as_str()),
            _ => None,
        }
    }

    pub fn list(&self) -> Option<&[Form]> {
        match &self.value {
            FormKind::List(items) => Some(items),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceDocument {
    path: String,
    authored: Vec<u8>,
    forms: Vec<Form>,
    semantic_bytes: Vec<u8>,
}

impl SourceDocument {
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn authored_bytes(&self) -> &[u8] {
        &self.authored
    }

    pub fn forms(&self) -> &[Form] {
        &self.forms
    }

    pub fn semantic_bytes(&self) -> &[u8] {
        &self.semantic_bytes
    }
}

pub fn parse_source(
    path: impl Into<String>,
    bytes: &[u8],
    limits: SourceLimits,
) -> Result<SourceDocument, Diagnostic> {
    let path = path.into();
    if bytes.len() > limits.maximum_bytes {
        return Err(Diagnostic::at_end(
            "source_too_large",
            format!(
                "source has {} bytes; the limit is {}",
                bytes.len(),
                limits.maximum_bytes
            ),
            &path,
        ));
    }
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(Diagnostic::at_end(
            "source_bom",
            "UTF-8 byte order marks are not accepted",
            &path,
        ));
    }
    let text = std::str::from_utf8(bytes).map_err(|error| {
        let offset = error.valid_up_to();
        Diagnostic::source(
            "source_utf8",
            "source is not valid UTF-8",
            location_at(&path, bytes, offset),
        )
    })?;
    if let Some(offset) = bytes.iter().position(|byte| *byte == b'\r') {
        return Err(Diagnostic::source(
            "source_line_ending",
            "source uses a carriage return; accepted source uses LF line endings",
            location_at(&path, bytes, offset),
        ));
    }
    if let Some(offset) = bytes.iter().position(|byte| *byte == 0) {
        return Err(Diagnostic::source(
            "source_nul",
            "source contains a NUL byte",
            location_at(&path, bytes, offset),
        ));
    }

    let mut parser = Parser {
        path: &path,
        text,
        bytes,
        offset: 0,
        forms: 0,
        limits,
    };
    let mut roots = Vec::new();
    parser.skip_space_and_comments();
    while parser.offset < bytes.len() {
        if parser.peek() == Some(b')') {
            return Err(parser.error(
                "source_unexpected_close",
                "unexpected ')' without a matching open list",
            ));
        }
        roots.push(parser.parse_form(0)?);
        parser.skip_space_and_comments();
    }
    if roots.is_empty() {
        return Err(Diagnostic::at_end(
            "source_empty",
            "a module must contain one module form",
            &path,
        ));
    }
    let mut semantic_bytes = Vec::new();
    for (index, form) in roots.iter().enumerate() {
        if index != 0 {
            semantic_bytes.push(b'\n');
        }
        render_form(form, &mut semantic_bytes);
    }
    semantic_bytes.push(b'\n');
    Ok(SourceDocument {
        path,
        authored: bytes.to_vec(),
        forms: roots,
        semantic_bytes,
    })
}

struct Parser<'a> {
    path: &'a str,
    text: &'a str,
    bytes: &'a [u8],
    offset: usize,
    forms: usize,
    limits: SourceLimits,
}

impl Parser<'_> {
    fn parse_form(&mut self, depth: usize) -> Result<Form, Diagnostic> {
        if depth > self.limits.maximum_depth {
            return Err(self.error(
                "source_depth",
                format!(
                    "source nesting exceeds the limit of {}",
                    self.limits.maximum_depth
                ),
            ));
        }
        self.forms = self.forms.saturating_add(1);
        if self.forms > self.limits.maximum_forms {
            return Err(self.error(
                "source_form_count",
                format!(
                    "source form count exceeds the limit of {}",
                    self.limits.maximum_forms
                ),
            ));
        }
        let start = self.offset;
        let location = location_at(self.path, self.bytes, start);
        let value = match self.peek() {
            Some(b'(') => {
                self.offset += 1;
                self.skip_space_and_comments();
                let mut forms = Vec::new();
                loop {
                    match self.peek() {
                        Some(b')') => {
                            self.offset += 1;
                            break;
                        }
                        None => {
                            return Err(Diagnostic::source(
                                "source_unclosed_list",
                                "list reaches the end of the source without ')'",
                                location,
                            ));
                        }
                        _ => {
                            forms.push(self.parse_form(depth + 1)?);
                            self.skip_space_and_comments();
                        }
                    }
                }
                FormKind::List(forms)
            }
            Some(b'"') => FormKind::String(self.parse_string()?),
            Some(b')') => {
                return Err(self.error(
                    "source_unexpected_close",
                    "unexpected ')' without a matching open list",
                ));
            }
            Some(_) => self.parse_atom_or_integer()?,
            None => {
                return Err(self.error("source_unexpected_end", "expected a source form"));
            }
        };
        Ok(Form {
            span: SourceSpan {
                byte_start: start,
                byte_end: self.offset,
                line: location.line,
                column: location.column,
            },
            value,
        })
    }

    fn parse_string(&mut self) -> Result<String, Diagnostic> {
        let start = self.offset;
        self.offset += 1;
        let mut escaped = false;
        let mut closed = false;
        while self.offset < self.bytes.len() {
            let byte = self.bytes[self.offset];
            if !escaped && byte == b'"' {
                self.offset += 1;
                closed = true;
                break;
            }
            if !escaped && byte < 0x20 {
                return Err(self.error(
                    "source_string_control",
                    "a string contains an unescaped control character",
                ));
            }
            if !escaped && byte == b'\\' {
                escaped = true;
                self.offset += 1;
                continue;
            }
            escaped = false;
            self.offset += char_width(byte);
            if self.offset.saturating_sub(start + 1) > self.limits.maximum_string_bytes {
                return Err(Diagnostic::source(
                    "source_string_too_large",
                    format!(
                        "string exceeds the limit of {} encoded bytes",
                        self.limits.maximum_string_bytes
                    ),
                    location_at(self.path, self.bytes, start),
                ));
            }
        }
        if !closed {
            return Err(Diagnostic::source(
                "source_unclosed_string",
                "string reaches the end of the source without a closing quote",
                location_at(self.path, self.bytes, start),
            ));
        }
        serde_json::from_str(&self.text[start..self.offset]).map_err(|error| {
            Diagnostic::source(
                "source_string_escape",
                format!("string escape is invalid: {error}"),
                location_at(self.path, self.bytes, start),
            )
        })
    }

    fn parse_atom_or_integer(&mut self) -> Result<FormKind, Diagnostic> {
        let start = self.offset;
        while let Some(byte) = self.peek() {
            if byte.is_ascii_whitespace() || matches!(byte, b'(' | b')' | b';' | b'"') {
                break;
            }
            if !valid_atom_byte(byte) {
                return Err(self.error(
                    "source_atom_character",
                    format!("byte 0x{byte:02x} is not allowed in an atom"),
                ));
            }
            self.offset += 1;
            if self.offset - start > self.limits.maximum_atom_bytes {
                return Err(Diagnostic::source(
                    "source_atom_too_large",
                    format!(
                        "atom exceeds the limit of {} bytes",
                        self.limits.maximum_atom_bytes
                    ),
                    location_at(self.path, self.bytes, start),
                ));
            }
        }
        if self.offset == start {
            return Err(self.error("source_expected_form", "expected an atom"));
        }
        let token = &self.text[start..self.offset];
        if looks_like_integer(token) {
            validate_integer_spelling(token).map_err(|message| {
                Diagnostic::source(
                    "source_integer_canonical",
                    message,
                    location_at(self.path, self.bytes, start),
                )
            })?;
            let value = token.parse::<i64>().map_err(|_| {
                Diagnostic::source(
                    "source_integer_range",
                    "integer is outside the signed 64-bit range",
                    location_at(self.path, self.bytes, start),
                )
            })?;
            Ok(FormKind::Integer(value))
        } else {
            Ok(FormKind::Atom(Atom(token.to_owned())))
        }
    }

    fn skip_space_and_comments(&mut self) {
        loop {
            while matches!(self.peek(), Some(byte) if byte.is_ascii_whitespace()) {
                self.offset += 1;
            }
            if self.peek() != Some(b';') {
                break;
            }
            while let Some(byte) = self.peek() {
                self.offset += 1;
                if byte == b'\n' {
                    break;
                }
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.offset).copied()
    }

    fn error(&self, code: &str, message: impl Into<String>) -> Diagnostic {
        Diagnostic::source(
            code,
            message,
            location_at(self.path, self.bytes, self.offset),
        )
    }
}

fn render_form(form: &Form, output: &mut Vec<u8>) {
    match &form.value {
        FormKind::Atom(atom) => output.extend_from_slice(atom.as_str().as_bytes()),
        FormKind::Integer(value) => output.extend_from_slice(value.to_string().as_bytes()),
        FormKind::String(value) => match serde_json::to_string(value) {
            Ok(encoded) => output.extend_from_slice(encoded.as_bytes()),
            Err(_) => output.extend_from_slice(b"\"\""),
        },
        FormKind::List(forms) => {
            output.push(b'(');
            for (index, item) in forms.iter().enumerate() {
                if index != 0 {
                    output.push(b' ');
                }
                render_form(item, output);
            }
            output.push(b')');
        }
    }
}

fn location_at(path: &str, bytes: &[u8], offset: usize) -> SourceLocation {
    let bounded = offset.min(bytes.len());
    let mut line = 1usize;
    let mut line_start = 0usize;
    for (index, byte) in bytes[..bounded].iter().enumerate() {
        if *byte == b'\n' {
            line = line.saturating_add(1);
            line_start = index.saturating_add(1);
        }
    }
    let column = std::str::from_utf8(&bytes[line_start..bounded])
        .map(|prefix| prefix.chars().count().saturating_add(1))
        .unwrap_or(1);
    SourceLocation {
        path: path.to_owned(),
        byte_offset: bounded,
        line,
        column,
    }
}

fn char_width(first: u8) -> usize {
    match first {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

fn valid_atom_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'_' | b'.'
                | b'/'
                | b':'
                | b'+'
                | b'-'
                | b'*'
                | b'<'
                | b'>'
                | b'='
                | b'!'
                | b'?'
                | b'@'
                | b'%'
        )
}

fn looks_like_integer(token: &str) -> bool {
    let digits = token.strip_prefix('-').unwrap_or(token);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn validate_integer_spelling(token: &str) -> Result<(), &'static str> {
    if token == "-0" {
        return Err("negative zero is not a canonical integer");
    }
    let digits = token.strip_prefix('-').unwrap_or(token);
    if digits.len() > 1 && digits.starts_with('0') {
        return Err("an integer with more than one digit may not start with zero");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &[u8] = br#"; retained module comment
(module resources
  (export create-resource)
  (record CreateInput (title Text) (base (Option I64)))
  (task create-resource ((request HttpRequest)) HttpResponse
    (requires database clock)
    (let ((now (perform clock utc-now)))
      (call json.response 201 (record (created-at now))))))
"#;

    #[test]
    fn comments_and_formatting_do_not_enter_semantic_bytes() {
        let parsed = parse_source("src/resources.lkj", SOURCE, SourceLimits::default())
            .expect("representative source parses");
        assert_eq!(parsed.forms().len(), 1);
        assert_eq!(parsed.authored_bytes(), SOURCE);
        assert!(!parsed.semantic_bytes().contains(&b';'));
        let reparsed = parse_source("semantic", parsed.semantic_bytes(), SourceLimits::default())
            .expect("canonical semantic source reparses");
        assert_eq!(parsed.semantic_bytes(), reparsed.semantic_bytes());
    }

    #[test]
    fn diagnostics_name_exact_authored_location() {
        let error = parse_source(
            "src/bad.lkj",
            b"(module bad\n  (fn f () Text \"unterminated))",
            SourceLimits::default(),
        )
        .expect_err("unclosed string rejects");
        assert_eq!(error.code, "source_unclosed_string");
        assert_eq!(error.location.expect("location").line, 2);
    }

    #[test]
    fn hostile_bounds_apply_before_growth() {
        let limits = SourceLimits {
            maximum_bytes: 8,
            ..SourceLimits::default()
        };
        let error = parse_source("large", b"(module too-large)", limits)
            .expect_err("oversized source rejects");
        assert_eq!(error.code, "source_too_large");

        let limits = SourceLimits {
            maximum_depth: 2,
            ..SourceLimits::default()
        };
        let error = parse_source("deep", b"((((x))))", limits).expect_err("deep source rejects");
        assert_eq!(error.code, "source_depth");
    }

    #[test]
    fn noncanonical_and_out_of_range_integers_reject() {
        for source in [b"01".as_slice(), b"-0", b"9223372036854775808"] {
            let error = parse_source("integer", source, SourceLimits::default())
                .expect_err("invalid integer rejects");
            assert!(error.code.starts_with("source_integer_"));
        }
    }

    #[test]
    fn unicode_columns_are_scalar_based() {
        let error = parse_source(
            "unicode",
            "(module \"界\" ) )".as_bytes(),
            SourceLimits::default(),
        )
        .expect_err("foreign close rejects");
        let location = error.location.expect("location");
        assert_eq!(location.line, 1);
        assert_eq!(location.column, 15);
    }
}
