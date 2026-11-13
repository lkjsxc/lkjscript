//! Rust-compatible surface reader. The syntax tree is an arena, not a recursive tree.
mod default;
pub(crate) mod formatting;
use crate::ast::*;
use crate::diagnostics::{BlockedRegion, RecoveryDomain, RecoveryOwner};
use std::collections::BTreeSet;

#[derive(Clone, PartialEq, Eq)]
enum TokenKind {
    Ident,
    Number,
    String,
    Char,
    Bytes,
    Lifetime,
    Symbol,
    End,
}
#[derive(Clone)]
struct Token {
    kind: TokenKind,
    text: String,
    span: Span,
}

fn ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}
fn ident_continue(b: u8) -> bool {
    ident_start(b) || b.is_ascii_digit()
}
fn span_at(start: usize, end: usize, line: usize, column: usize) -> Span {
    Span {
        file: 0,
        start,
        end,
        line,
        column,
    }
}

struct Lexer<'a> {
    source: &'a str,
    at: usize,
    line: usize,
    column: usize,
}
impl Lexer<'_> {
    fn byte(&self, n: usize) -> u8 {
        if self.at + n < self.source.len() {
            self.source.as_bytes()[self.at + n]
        } else {
            0
        }
    }
    fn advance(&mut self) {
        if self.byte(0) == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        self.at += 1;
    }
    fn here(&self) -> Span {
        span_at(self.at, self.at, self.line, self.column)
    }
    fn error(&self, message: &str) -> Diagnostic {
        Diagnostic::new(self.here(), "E_LEX", message)
    }
    fn quoted(&mut self, quote: u8, bytes: bool) -> Result<String, Diagnostic> {
        self.advance();
        let mut result = String::new();
        while self.at < self.source.len() && self.byte(0) != quote {
            if self.byte(0) == b'\\' {
                self.advance();
                let b = self.byte(0);
                self.advance();
                match b {
                    b'n' => result.push('\n'),
                    b'r' => result.push('\r'),
                    b't' => result.push('\t'),
                    b'0' => result.push('\0'),
                    b'\\' => result.push('\\'),
                    b'\'' => result.push('\''),
                    b'"' => result.push('"'),
                    b'x' => {
                        let start = self.at;
                        let mut value: u32 = 0;
                        let mut i = 0;
                        while i < 2 {
                            let b = self.byte(0);
                            let digit = if b.is_ascii_digit() {
                                (b - b'0') as u32
                            } else if (b'a'..=b'f').contains(&b) {
                                (b - b'a' + 10) as u32
                            } else if (b'A'..=b'F').contains(&b) {
                                (b - b'A' + 10) as u32
                            } else {
                                return Err(self.error("expected two hexadecimal digits after \\x"));
                            };
                            value = value * 16 + digit;
                            self.advance();
                            i += 1;
                        }
                        if value > 127 && !bytes {
                            return Err(Diagnostic::new(
                                span_at(start, self.at, self.line, self.column),
                                "E_LEX",
                                "non-ASCII \\x escape in a UTF-8 string",
                            ));
                        }
                        match char::from_u32(value) {
                            Some(ch) => result.push(ch),
                            None => return Err(self.error("invalid character escape")),
                        }
                    }
                    b'u' => {
                        if bytes {
                            return Err(
                                self.error("Unicode escapes are not valid in byte literals")
                            );
                        }
                        if self.byte(0) != b'{' {
                            return Err(self.error("expected { after \\u"));
                        }
                        self.advance();
                        let mut digits = String::new();
                        while self.at < self.source.len() && self.byte(0) != b'}' {
                            if self.byte(0) != b'_' {
                                digits.push(self.byte(0) as char);
                            }
                            self.advance();
                        }
                        if self.byte(0) != b'}' {
                            return Err(self.error("unterminated Unicode escape"));
                        }
                        self.advance();
                        let value = match u32::from_str_radix(&digits, 16) {
                            Ok(value) => value,
                            Err(_) => return Err(self.error("invalid Unicode escape")),
                        };
                        match char::from_u32(value) {
                            Some(ch) => result.push(ch),
                            None => return Err(self.error("invalid Unicode scalar")),
                        }
                    }
                    b'\n' => {
                        while self.at < self.source.len() && self.byte(0).is_ascii_whitespace() {
                            self.advance();
                        }
                    }
                    _ => return Err(self.error("unknown escape sequence")),
                }
            } else {
                let rest = &self.source[self.at..];
                let ch = match rest.chars().next() {
                    Some(ch) => ch,
                    None => return Err(self.error("unterminated literal")),
                };
                if bytes && !ch.is_ascii() {
                    return Err(self.error("non-ASCII byte literals require hexadecimal escapes"));
                }
                let n = ch.len_utf8();
                let mut i = 0;
                while i < n {
                    self.advance();
                    i += 1;
                }
                result.push(ch);
            }
        }
        if self.at == self.source.len() {
            return Err(self.error("unterminated literal"));
        }
        self.advance();
        Ok(result)
    }
    fn lex(&mut self) -> Result<Vec<Token>, Diagnostic> {
        let mut tokens = Vec::new();
        while self.at < self.source.len() {
            let b = self.byte(0);
            if b.is_ascii_whitespace() {
                self.advance();
                continue;
            }
            if b == b'/' && self.byte(1) == b'/' {
                while self.at < self.source.len() && self.byte(0) != b'\n' {
                    self.advance();
                }
                continue;
            }
            if b == b'/' && self.byte(1) == b'*' {
                let start = self.here();
                self.advance();
                self.advance();
                let mut depth = 1usize;
                while self.at < self.source.len() && depth != 0 {
                    if self.byte(0) == b'/' && self.byte(1) == b'*' {
                        depth += 1;
                        self.advance();
                        self.advance();
                    } else if self.byte(0) == b'*' && self.byte(1) == b'/' {
                        depth -= 1;
                        self.advance();
                        self.advance();
                    } else {
                        self.advance();
                    }
                }
                if depth != 0 {
                    return Err(Diagnostic::new(
                        start,
                        "E_LEX",
                        "unterminated block comment",
                    ));
                }
                continue;
            }
            let mut span = self.here();
            let kind;
            let text;
            if b == b'b' && (self.byte(1) == b'"' || self.byte(1) == b'\'') {
                self.advance();
                let quote = self.byte(0);
                let s = self.quoted(quote, true)?;
                if quote == b'\'' {
                    if s.chars().count() != 1 {
                        return Err(Diagnostic::new(
                            span,
                            "E_LEX",
                            "a byte literal has one byte",
                        ));
                    }
                    kind = TokenKind::Number;
                    text = format!("{}u8", s.chars().next().unwrap() as u32);
                } else {
                    kind = TokenKind::Bytes;
                    text = s;
                }
            } else if b == b'r' && (self.byte(1) == b'"' || self.byte(1) == b'#') {
                self.advance();
                let mut hashes = 0usize;
                while self.byte(0) == b'#' {
                    hashes += 1;
                    self.advance();
                }
                if self.byte(0) != b'"' {
                    return Err(self.error("expected a raw string opening quote"));
                }
                self.advance();
                let content = self.at;
                loop {
                    if self.at == self.source.len() {
                        return Err(Diagnostic::new(span, "E_LEX", "unterminated raw string"));
                    }
                    if self.byte(0) == b'"' {
                        let mut i = 0usize;
                        while i < hashes && self.byte(i + 1) == b'#' {
                            i += 1;
                        }
                        if i == hashes {
                            text = self.source[content..self.at].to_string();
                            self.advance();
                            i = 0;
                            while i < hashes {
                                self.advance();
                                i += 1;
                            }
                            break;
                        }
                    }
                    self.advance();
                }
                kind = TokenKind::String;
            } else if b == b'"' {
                kind = TokenKind::String;
                text = self.quoted(b'"', false)?;
            } else if b == b'\'' {
                let mut end = self.at + 1;
                while end < self.source.len() && ident_continue(self.source.as_bytes()[end]) {
                    end += 1;
                }
                if end > self.at + 1
                    && (end == self.source.len() || self.source.as_bytes()[end] != b'\'')
                {
                    while self.at < end {
                        self.advance();
                    }
                    kind = TokenKind::Lifetime;
                    text = self.source[span.start..end].to_string();
                } else {
                    kind = TokenKind::Char;
                    text = self.quoted(b'\'', false)?;
                    if text.chars().count() != 1 {
                        return Err(Diagnostic::new(
                            span,
                            "E_LEX",
                            "a character literal has one Unicode scalar",
                        ));
                    }
                }
            } else if ident_start(b) {
                self.advance();
                while self.at < self.source.len() && ident_continue(self.byte(0)) {
                    self.advance();
                }
                kind = TokenKind::Ident;
                text = self.source[span.start..self.at].to_string();
            } else if b.is_ascii_digit() {
                self.advance();
                while self.at < self.source.len() {
                    let n = self.byte(0);
                    if ident_continue(n)
                        || (n == b'.' && self.byte(1).is_ascii_digit())
                        || (n == b'+' || n == b'-')
                            && self.at > span.start
                            && (self.source.as_bytes()[self.at - 1] == b'e'
                                || self.source.as_bytes()[self.at - 1] == b'E')
                            && self.source.as_bytes()[span.start..self.at - 1]
                                .iter()
                                .all(|b| b.is_ascii_digit() || *b == b'_' || *b == b'.')
                    {
                        self.advance();
                    } else {
                        break;
                    }
                }
                kind = TokenKind::Number;
                text = self.source[span.start..self.at].to_string();
            } else {
                let rest = &self.source[self.at..];
                let mut width = 1;
                let triples = ["<<=", ">>=", "..="];
                let mut i = 0;
                while i < triples.len() {
                    if rest.starts_with(triples[i]) {
                        width = 3;
                    }
                    i += 1;
                }
                if width == 1 {
                    let doubles = [
                        "::", "->", "=>", "==", "!=", "<=", ">=", "&&", "||", "+=", "-=", "*=",
                        "/=", "%=", "&=", "|=", "^=", "<<", ">>", "..",
                    ];
                    i = 0;
                    while i < doubles.len() {
                        if rest.starts_with(doubles[i]) {
                            width = 2;
                        }
                        i += 1;
                    }
                }
                if !b"(){}[];,:.#?+-*/%&|^!~=<>".contains(&b) {
                    return Err(self.error("unsupported character in source"));
                }
                i = 0;
                while i < width {
                    self.advance();
                    i += 1;
                }
                kind = TokenKind::Symbol;
                text = self.source[span.start..self.at].to_string();
            }
            span.end = self.at;
            tokens.push(Token { kind, text, span });
        }
        tokens.push(Token {
            kind: TokenKind::End,
            text: String::new(),
            span: self.here(),
        });
        Ok(tokens)
    }
}

pub fn parse(source: &str) -> Result<Program, Diagnostic> {
    let mut program = Program::default();
    program.scopes.push(ModuleScope {
        path: String::new(),
        parent: None,
        crate_root: 0,
    });
    program.files.push(SourceFile {
        path: "<source>".to_string(),
        crate_root: 0,
    });
    let items = parse_file(&mut program, source, 0, 0)?;
    program.items = items;
    Ok(program)
}

/// Syntax recovery is for reporting only; diagnostics prevent source compilation.
pub struct ParseReport {
    pub items: Vec<Item>,
    pub diagnostics: Vec<Diagnostic>,
    pub blocked: Vec<BlockedRegion>,
}

/// Append one source file to shared arenas, refusing every recovered error.
pub fn parse_file(
    program: &mut Program,
    source: &str,
    file: FileId,
    scope: ModuleId,
) -> Result<Vec<Item>, Diagnostic> {
    let report = parse_file_report(program, source, file, scope);
    match report.diagnostics.into_iter().next() {
        Some(error) => Err(error),
        None => Ok(report.items),
    }
}

/// Continue at balanced item and statement boundaries. Skipped input is named as
/// blocked, never replaced with fabricated types or expressions for compilation.
pub fn parse_file_report(
    program: &mut Program,
    source: &str,
    file: FileId,
    scope: ModuleId,
) -> ParseReport {
    let mut lexer = Lexer {
        source,
        at: 0,
        line: 1,
        column: 1,
    };
    let mut tokens = match lexer.lex() {
        Ok(tokens) => tokens,
        Err(mut error) => {
            error.file = file;
            error.path = program.files[file].path.clone();
            let mut span = lexer.here();
            span.file = file;
            span.end = source.len();
            return ParseReport {
                items: Vec::new(),
                diagnostics: vec![error],
                blocked: vec![BlockedRegion {
                    domain: None,
                    path: program.files[file].path.clone(),
                    span,
                    kind: "file".to_string(),
                    reason: "lexing failed; remaining token boundaries are unknown".to_string(),
                }],
            };
        }
    };
    for token in &mut tokens {
        token.span.file = file;
    }
    let mut parser = Parser {
        tokens,
        at: 0,
        module: program.scopes[scope].path.clone(),
        scope,
        program: std::mem::take(program),
        diagnostics: Vec::new(),
        blocked: Vec::new(),
        diagnostic_keys: BTreeSet::new(),
    };
    if parser.program.types.is_empty() {
        parser.add_type(Type::Infer, Span::default());
        parser.add_type(Type::Tuple(Vec::new()), Span::default());
    }
    let items = match parser.items(false) {
        Ok(items) => items,
        Err(error) => {
            parser.record(error);
            Vec::new()
        }
    };
    *program = parser.program;
    ParseReport {
        items,
        diagnostics: parser.diagnostics,
        blocked: parser.blocked,
    }
}

enum TypeFrame {
    Parse,
    Reference {
        mutable: bool,
        lifetime: String,
        span: Span,
    },
    Array {
        span: Span,
    },
    List {
        span: Span,
        items: Vec<TypeId>,
        comma: bool,
        pending: bool,
        kind: TypeListKind,
    },
    FunctionResult {
        span: Span,
        params: Vec<TypeId>,
        path: Option<Path>,
    },
    Path {
        span: Span,
        path: Path,
        position: TypePathPosition,
    },
    Arguments {
        span: Span,
        path: Path,
        turbofish: bool,
        pending: bool,
    },
}

enum TypeListKind {
    Tuple,
    Function,
    Callable(Path),
}

enum TypePathPosition {
    Part,
    Arguments,
    Turbofish,
}

struct Parser {
    tokens: Vec<Token>,
    at: usize,
    program: Program,
    module: String,
    scope: ModuleId,
    diagnostics: Vec<Diagnostic>,
    blocked: Vec<BlockedRegion>,
    diagnostic_keys: BTreeSet<(FileId, usize, usize, String, String)>,
}

enum BlockPart {
    Skipped,
    Statement(Stmt),
    Tail(ExprId),
}

impl Parser {
    fn record(&mut self, mut error: Diagnostic) {
        if error.path.is_empty() {
            error.path = self.program.files[error.file].path.clone();
        }
        // Unterminated nested regions can propagate the same root error.
        if self.diagnostic_keys.insert((
            error.file,
            error.line,
            error.column,
            error.code.clone(),
            error.message.clone(),
        )) {
            self.diagnostics.push(error);
        }
    }

    fn recovery_domain(&self, owner: Option<RecoveryOwner>) -> RecoveryDomain {
        let kind = self.token().text.clone();
        let name = if matches!(
            kind.as_str(),
            "fn" | "struct" | "enum" | "trait" | "type" | "const" | "mod"
        ) {
            self.tokens
                .get(self.at + 1)
                .filter(|token| token.kind == TokenKind::Ident)
                .map(|token| token.text.clone())
        } else {
            None
        };
        RecoveryDomain {
            scope: self.scope,
            kind,
            name,
            owner,
        }
    }

    fn recover_region(
        &mut self,
        start: usize,
        kind: &str,
        error: Diagnostic,
        domain: Option<RecoveryDomain>,
    ) {
        let span = self.tokens[start].span;
        let code = error.code.clone();
        self.record(error);
        // Propagating an EOF error through nested regions must not rescan the
        // same suffix once per nesting level.
        let mut at = if self.token().kind == TokenKind::End {
            self.at
        } else {
            start
        };
        let mut delimiters: Vec<&str> = Vec::new();
        let block_statement = kind == "statement"
            && matches!(
                self.tokens[start].kind,
                TokenKind::Ident | TokenKind::Symbol
            )
            && matches!(
                self.tokens[start].text.as_str(),
                "{" | "if" | "while" | "loop" | "for" | "match" | "unsafe" | "async"
            );
        while self.tokens[at].kind != TokenKind::End {
            let token = &self.tokens[at];
            if at > start
                && at >= self.at
                && delimiters.is_empty()
                && token.kind == TokenKind::Ident
                && matches!(
                    token.text.as_str(),
                    "let"
                        | "fn"
                        | "struct"
                        | "enum"
                        | "trait"
                        | "impl"
                        | "mod"
                        | "use"
                        | "type"
                        | "pub"
                        | "static"
                )
            {
                // A missing semicolon must not swallow the next declaration.
                // Only keyword tokens at the enclosing delimiter depth qualify.
                break;
            }
            if token.kind == TokenKind::Symbol {
                match token.text.as_str() {
                    "(" | "[" | "{" => delimiters.push(token.text.as_str()),
                    ")" | "]" | "}" => {
                        let expected = match token.text.as_str() {
                            ")" => "(",
                            "]" => "[",
                            _ => "{",
                        };
                        if delimiters.last().copied() == Some(expected) {
                            delimiters.pop();
                            if expected == "{"
                                && delimiters.is_empty()
                                && (kind == "item" || block_statement)
                            {
                                at += 1;
                                if self.tokens[at].kind != TokenKind::Ident
                                    || self.tokens[at].text != "else"
                                {
                                    break;
                                }
                                continue;
                            }
                        } else if token.text == "}" && !delimiters.contains(&"{") {
                            // Leave the enclosing block's close for its caller.
                            break;
                        }
                    }
                    ";" if delimiters.is_empty() => {
                        at += 1;
                        break;
                    }
                    _ => {}
                }
            }
            at += 1;
        }
        if at == start && self.tokens[at].kind != TokenKind::End {
            at += 1;
        }
        self.at = at;
        let mut extent = span;
        if at > start {
            extent.end = self.tokens[at - 1].span.end;
        }
        self.blocked.push(BlockedRegion {
            domain,
            path: self.program.files[span.file].path.clone(),
            span: extent,
            kind: kind.to_string(),
            reason: format!("not analyzed after {code}"),
        });
    }

    fn token(&self) -> &Token {
        &self.tokens[self.at]
    }
    fn is(&self, text: &str) -> bool {
        (self.token().kind == TokenKind::Ident
            || self.token().kind == TokenKind::Symbol
            || self.token().kind == TokenKind::Lifetime)
            && self.token().text == text
    }
    fn take(&mut self, text: &str) -> bool {
        if self.is(text) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn err(&self, message: &str) -> Diagnostic {
        Diagnostic::new(self.token().span, "E_PARSE", message)
    }
    fn unsupported(&self, name: &str) -> Diagnostic {
        Diagnostic::new(
            self.token().span,
            "E_UNSUPPORTED",
            &format!("unsupported construct: {name}"),
        )
    }
    fn expect(&mut self, text: &str) -> Result<(), Diagnostic> {
        if self.take(text) {
            Ok(())
        } else {
            Err(self.err(&format!("expected `{text}`, found `{}`", self.token().text)))
        }
    }
    fn ident(&mut self) -> Result<String, Diagnostic> {
        if self.token().kind != TokenKind::Ident {
            return Err(self.err("expected an identifier"));
        }
        let text = self.token().text.clone();
        self.at += 1;
        Ok(text)
    }
    fn finish_span(&self, mut span: Span) -> Span {
        if self.at > 0 {
            span.end = self.tokens[self.at - 1].span.end;
        }
        span
    }
    fn add_type(&mut self, kind: Type, span: Span) -> TypeId {
        let id = self.program.types.len();
        self.program.types.push(TypeNode {
            kind,
            span,
            scope: self.scope,
        });
        id
    }
    fn add_expr(&mut self, kind: Expr, span: Span) -> ExprId {
        let id = self.program.exprs.len();
        self.program.exprs.push(ExprNode {
            kind,
            span,
            scope: self.scope,
        });
        id
    }
    fn add_pattern(&mut self, kind: Pattern, span: Span) -> PatternId {
        let id = self.program.patterns.len();
        self.program.patterns.push(PatternNode {
            kind,
            span,
            scope: self.scope,
        });
        id
    }
    fn attrs(&mut self) -> Result<Vec<Attribute>, Diagnostic> {
        let mut attrs = Vec::new();
        while self.take("#") {
            let span = self.tokens[self.at - 1].span;
            let inner = self.take("!");
            self.expect("[")?;
            let name = self.ident()?;
            let mut args = Vec::new();
            if self.take("(") {
                let mut depth = 1usize;
                while depth != 0 {
                    if self.token().kind == TokenKind::End {
                        return Err(self.err("unterminated attribute"));
                    }
                    if self.is("(") {
                        depth += 1;
                    }
                    if self.is(")") {
                        depth -= 1;
                    }
                    if depth != 0 && !self.is(",") {
                        args.push(self.token().text.clone());
                    }
                    self.at += 1;
                }
            } else if self.take("=") {
                if self.token().kind == TokenKind::End {
                    return Err(self.err("expected attribute value"));
                }
                args.push(self.token().text.clone());
                self.at += 1;
            }
            self.expect("]")?;
            let attr = Attribute {
                name,
                args,
                span: self.finish_span(span),
            };
            if inner {
                if attr.name == "cfg" {
                    return Err(Diagnostic::new(
                        attr.span,
                        "E_UNSUPPORTED",
                        "inner cfg attributes are not supported; help: put #[cfg(...)] on an enclosing module or declaration",
                    ));
                }
                self.program.attrs.push(attr);
            } else {
                attrs.push(attr);
            }
        }
        Ok(attrs)
    }
    fn enabled(&self, attrs: &[Attribute]) -> Result<bool, Diagnostic> {
        fn predicate(args: &[String], at: &mut usize, span: Span) -> Result<bool, Diagnostic> {
            let Some(name) = args.get(*at) else {
                return Err(Diagnostic::new(span, "E_PARSE", "missing cfg predicate"));
            };
            *at += 1;
            if matches!(name.as_str(), "all" | "any" | "not") {
                if args.get(*at).map(String::as_str) != Some("(") {
                    return Err(Diagnostic::new(
                        span,
                        "E_PARSE",
                        "cfg combinator needs parentheses",
                    ));
                }
                *at += 1;
                let mut values = Vec::new();
                while args.get(*at).is_some_and(|part| part != ")") {
                    values.push(predicate(args, at, span)?);
                }
                if args.get(*at).map(String::as_str) != Some(")") {
                    return Err(Diagnostic::new(
                        span,
                        "E_PARSE",
                        "unterminated cfg predicate",
                    ));
                }
                *at += 1;
                return match name.as_str() {
                    "all" => Ok(values.iter().all(|value| *value)),
                    "any" => Ok(values.iter().any(|value| *value)),
                    "not" if values.len() == 1 => Ok(!values[0]),
                    _ => Err(Diagnostic::new(
                        span,
                        "E_PARSE",
                        "cfg not needs exactly one predicate",
                    )),
                };
            }
            if args.get(*at).map(String::as_str) == Some("=") {
                *at += 1;
                let Some(value) = args.get(*at) else {
                    return Err(Diagnostic::new(span, "E_PARSE", "missing cfg value"));
                };
                *at += 1;
                return match name.as_str() {
                    "target_os" => Ok(value == "linux"),
                    "target_arch" => Ok(value == "x86_64"),
                    "target_family" => Ok(value == "unix"),
                    "target_pointer_width" => Ok(value == "64"),
                    "target_endian" => Ok(value == "little"),
                    "feature" => Ok(false),
                    _ => Err(Diagnostic::new(
                        span,
                        "E_UNSUPPORTED",
                        &format!(
                            "cfg predicate {name} is not supported; help: use test, target_os, target_arch, target_family, target_pointer_width or target_endian"
                        ),
                    )),
                };
            }
            match name.as_str() {
                "test" | "windows" | "false" => Ok(false),
                "unix" | "true" => Ok(true),
                _ => Err(Diagnostic::new(
                    span,
                    "E_UNSUPPORTED",
                    &format!(
                        "cfg predicate {name} is not supported; help: use #[cfg(test)] for test-only declarations"
                    ),
                )),
            }
        }
        for attr in attrs {
            if attr.name == "cfg" {
                let mut at = 0;
                let value = predicate(&attr.args, &mut at, attr.span)?;
                if at != attr.args.len() {
                    return Err(Diagnostic::new(
                        attr.span,
                        "E_PARSE",
                        "cfg needs exactly one predicate",
                    ));
                }
                if !value {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
    // Disabled declarations do not contribute names, bodies or recovery sites.
    // Only balanced tokens are needed, just as for Rust conditional compilation.
    fn skip_disabled(&mut self) -> Result<(), Diagnostic> {
        let mut delimiters = Vec::new();
        let mut braced_item = false;
        while self.token().kind != TokenKind::End {
            let token = self.token();
            if delimiters.is_empty()
                && matches!(
                    token.text.as_str(),
                    "fn" | "struct" | "enum" | "trait" | "impl" | "mod"
                )
            {
                braced_item = true;
            }
            if token.kind == TokenKind::Symbol {
                match token.text.as_str() {
                    "(" | "[" | "{" => delimiters.push(token.text.clone()),
                    ")" | "]" | "}" => {
                        let expected = match token.text.as_str() {
                            ")" => "(",
                            "]" => "[",
                            _ => "{",
                        };
                        if delimiters.last().map(String::as_str) != Some(expected) {
                            return Err(self.err("unbalanced disabled declaration"));
                        }
                        delimiters.pop();
                        if expected == "{" && delimiters.is_empty() && braced_item {
                            self.at += 1;
                            return Ok(());
                        }
                    }
                    ";" if delimiters.is_empty() => {
                        self.at += 1;
                        return Ok(());
                    }
                    _ => {}
                }
            }
            self.at += 1;
        }
        if delimiters.is_empty() {
            Ok(())
        } else {
            Err(self.err("unterminated disabled declaration"))
        }
    }
    fn visibility(&mut self) -> Result<Visibility, Diagnostic> {
        if !self.take("pub") {
            return Ok(Visibility::Private);
        }
        let visibility = if self.take("(") {
            let v = if self.take("in") {
                self.restricted_visibility()?
            } else if self.take("crate") {
                Visibility::Crate
            } else if self.take("self") {
                Visibility::Private
            } else if self.take("super") {
                if self.program.scopes[self.scope].parent.is_none() {
                    return Err(self.err("pub(super) has no parent module"));
                }
                Visibility::Parent
            } else {
                return Err(self.unsupported("restricted visibility path"));
            };
            self.expect(")")?;
            v
        } else {
            Visibility::Public
        };
        Ok(visibility)
    }

    fn restricted_visibility(&mut self) -> Result<Visibility, Diagnostic> {
        let span = self.token().span;
        let mut root = self.scope;
        if self.take("crate") {
            root = self.program.scopes[self.scope].crate_root;
        } else if self.take("self") {
        } else if self.take("super") {
            loop {
                root = self.program.scopes[root].parent.ok_or_else(|| {
                    Diagnostic::new(span, "E_PRIVACY", "visibility path has no parent module")
                })?;
                if !self.is("::")
                    || !self
                        .tokens
                        .get(self.at + 1)
                        .is_some_and(|token| token.text == "super")
                {
                    break;
                }
                self.at += 2;
            }
        } else {
            return Err(Diagnostic::new(
                span,
                "E_PARSE",
                "visibility path must start with crate, self, or super",
            ));
        }
        let mut path = self.program.scopes[root].path.clone();
        while self.take("::") {
            let segment = self.ident()?;
            if matches!(segment.as_str(), "crate" | "self" | "super" | "Self") {
                return Err(Diagnostic::new(
                    span,
                    "E_PARSE",
                    "visibility path qualifiers must precede module names",
                ));
            }
            if !path.is_empty() {
                path.push_str("::");
            }
            path.push_str(&segment);
        }
        // Module ancestors exist before either inline or external-file bodies
        // are parsed. Matching their identities never follows a use alias.
        let mut ancestor = Some(self.scope);
        while let Some(scope) = ancestor {
            let module = &self.program.scopes[scope];
            if module.path == path
                && module.crate_root == self.program.scopes[self.scope].crate_root
            {
                return Ok(Visibility::Restricted(scope));
            }
            ancestor = module.parent;
        }
        Err(Diagnostic::new(
            span,
            "E_PRIVACY",
            "visibility path must name an ancestor module in this crate",
        ))
    }

    fn items(&mut self, nested: bool) -> Result<Vec<Item>, Diagnostic> {
        let mut items = Vec::new();
        while self.token().kind != TokenKind::End && !(nested && self.is("}")) {
            if self.take(";") {
                continue;
            }
            let start = self.at;
            let scope = self.scope;
            let module = self.module.clone();
            let mut domain = None;
            let mut disabled = false;
            let result = (|| {
                let attrs = self.attrs()?;
                if !self.enabled(&attrs)? {
                    self.skip_disabled()?;
                    disabled = true;
                    return Ok(None);
                }
                if self.token().kind == TokenKind::End {
                    return Ok(None);
                }
                let public = self.visibility()?;
                domain = Some(self.recovery_domain(None));
                self.item(attrs, public).map(Some)
            })();
            match result {
                Ok(Some(item)) => items.push(item),
                Ok(None) if disabled => continue,
                Ok(None) => break,
                Err(error) => {
                    self.scope = scope;
                    self.module = module;
                    self.recover_region(start, "item", error, domain);
                }
            }
        }
        Ok(items)
    }
    fn item(&mut self, attrs: Vec<Attribute>, public: Visibility) -> Result<Item, Diagnostic> {
        if self.is("fn") {
            return Ok(Item::Function(self.function(attrs, public)?));
        }
        if self.is("struct") {
            return Ok(Item::Struct(self.structure(attrs, public)?));
        }
        if self.is("enum") {
            return Ok(Item::Enum(self.enumeration(attrs, public)?));
        }
        if self.is("trait") {
            return Ok(Item::Trait(self.trait_item(public)?));
        }
        if self.is("impl") {
            return Ok(Item::Impl(self.impl_item()?));
        }
        if self.take("use") {
            return Ok(Item::Use(self.use_item(public)?));
        }
        if self.take("mod") {
            let span = self.tokens[self.at - 1].span;
            let name = self.ident()?;
            let parent = self.module.clone();
            let parent_scope = self.scope;
            let scope = self.program.scopes.len();
            let module_path = if parent.is_empty() {
                name.clone()
            } else {
                format!("{parent}::{name}")
            };
            self.program.scopes.push(ModuleScope {
                path: module_path.clone(),
                parent: Some(parent_scope),
                crate_root: self.program.scopes[parent_scope].crate_root,
            });
            let external = self.take(";");
            let mut items = Vec::new();
            if !external {
                self.expect("{")?;
                self.module = module_path;
                self.scope = scope;
                items = self.items(true)?;
                self.expect("}")?;
                self.module = parent.clone();
                self.scope = parent_scope;
            }
            let id = self.program.modules.len();
            self.program.modules.push(Module {
                scope,
                name,
                parent,
                items,
                external,
                public,
                span: self.finish_span(span),
            });
            return Ok(Item::Module(id));
        }
        if self.take("type") {
            let span = self.tokens[self.at - 1].span;
            let name = self.ident()?;
            let generics = self.generics()?;
            self.expect("=")?;
            let ty = self.ty()?;
            self.expect(";")?;
            let id = self.program.aliases.len();
            self.program.aliases.push(Alias {
                name,
                module: self.module.clone(),
                generics,
                ty,
                public,
                span: self.finish_span(span),
            });
            return Ok(Item::Alias(id));
        }
        if self.take("const") {
            let span = self.tokens[self.at - 1].span;
            if self.is("fn") {
                return Err(self.unsupported("const fn"));
            }
            let name = self.ident()?;
            self.expect(":")?;
            let ty = self.ty()?;
            self.expect("=")?;
            let value = self.expr(true)?;
            self.expect(";")?;
            let id = self.program.constants.len();
            self.program.constants.push(Constant {
                name,
                module: self.module.clone(),
                ty,
                value,
                public,
                span: self.finish_span(span),
            });
            return Ok(Item::Constant(id));
        }
        Err(self.unsupported(&self.token().text))
    }
    fn gt(&mut self) -> Result<(), Diagnostic> {
        if self.take(">") {
            return Ok(());
        }
        if self.is(">>") || self.is(">>=") || self.is(">=") {
            self.tokens[self.at].text.remove(0);
            self.tokens[self.at].span.start += 1;
            self.tokens[self.at].span.column += 1;
            return Ok(());
        }
        Err(self.err("expected `>`"))
    }
    fn at_gt(&self) -> bool {
        self.is(">") || self.is(">>") || self.is(">>=") || self.is(">=")
    }
    fn generic_args(&mut self) -> Result<Vec<TypeId>, Diagnostic> {
        self.expect("<")?;
        let mut args = Vec::new();
        while !self.at_gt() {
            if self.token().kind == TokenKind::Lifetime {
                let span = self.token().span;
                let name = self.token().text.clone();
                self.at += 1;
                args.push(self.add_type(
                    Type::Path(Path {
                        declaration: None,
                        name,
                        args: Vec::new(),
                    }),
                    span,
                ));
            } else {
                args.push(self.ty()?);
            }
            if !self.take(",") {
                break;
            }
        }
        self.gt()?;
        Ok(args)
    }
    fn bounds(&mut self) -> Result<Vec<TypeId>, Diagnostic> {
        let mut bounds = Vec::new();
        loop {
            if self.token().kind == TokenKind::Lifetime {
                let span = self.token().span;
                let name = self.token().text.clone();
                self.at += 1;
                bounds.push(self.add_type(
                    Type::Path(Path {
                        declaration: None,
                        name,
                        args: Vec::new(),
                    }),
                    span,
                ));
            } else {
                bounds.push(self.ty()?);
            }
            if !self.take("+") {
                break;
            }
        }
        Ok(bounds)
    }
    fn generics(&mut self) -> Result<Vec<GenericParam>, Diagnostic> {
        let mut params = Vec::new();
        if !self.take("<") {
            return Ok(params);
        }
        while !self.at_gt() {
            let span = self.token().span;
            let lifetime = self.token().kind == TokenKind::Lifetime;
            let name;
            if lifetime {
                name = self.token().text.clone();
                self.at += 1;
            } else {
                if self.is("const") {
                    return Err(self.unsupported("const generic parameters"));
                }
                name = self.ident()?;
            }
            let bounds = if self.take(":") {
                self.bounds()?
            } else {
                Vec::new()
            };
            if self.is("=") {
                return Err(self.unsupported("default generic arguments"));
            }
            params.push(GenericParam {
                name,
                bounds,
                lifetime,
                span: self.finish_span(span),
            });
            if !self.take(",") {
                break;
            }
        }
        self.gt()?;
        Ok(params)
    }
    fn where_clause(&mut self, generics: &mut [GenericParam]) -> Result<(), Diagnostic> {
        if !self.take("where") {
            return Ok(());
        }
        while !self.is("{") && !self.is(";") {
            let name = self.ident()?;
            self.expect(":")?;
            let bounds = self.bounds()?;
            let mut found = false;
            let mut i = 0;
            while i < generics.len() {
                if generics[i].name == name {
                    generics[i].bounds.extend(bounds.clone());
                    found = true;
                    break;
                }
                i += 1;
            }
            if !found {
                return Err(self.unsupported("where predicate on a non-parameter type"));
            }
            if !self.take(",") {
                break;
            }
        }
        Ok(())
    }
    fn value_path(&mut self) -> Result<Path, Diagnostic> {
        let absolute = self.take("::");
        let mut name = self.ident()?;
        if absolute {
            name = format!("::{name}");
        }
        let mut args = Vec::new();
        while self.take("::") {
            if self.is("<") {
                if !args.is_empty() {
                    return Err(self.unsupported("multiple generic argument lists in one path"));
                }
                args = self.generic_args()?;
                if !self.take("::") {
                    break;
                }
            }
            let part = self.ident()?;
            name.push_str("::");
            name.push_str(&part);
        }
        self.capability_path(&name)?;
        Ok(Path {
            name,
            args,
            declaration: None,
        })
    }
    fn capability_path(&self, name: &str) -> Result<(), Diagnostic> {
        if name == "std::thread::scope" {
            return Ok(());
        }
        let denied = [
            "std::fs",
            "std::env",
            "std::time",
            "std::net",
            "std::process",
            "std::thread",
            "std::io",
            "std::sync",
            "std::cell",
            "core::cell",
        ];
        let mut i = 0;
        while i < denied.len() {
            if name == denied[i] || name.starts_with(&format!("{}::", denied[i])) {
                return Err(Diagnostic::new(
                    self.token().span,
                    "E_CAPABILITY",
                    &format!("`{name}` is unavailable; use a capability derived from World"),
                ));
            }
            i += 1;
        }
        Ok(())
    }

    fn callable_type(
        &mut self,
        span: Span,
        params: Vec<TypeId>,
        path: Option<Path>,
        result: TypeId,
    ) -> TypeId {
        match path {
            Some(mut path) => {
                let tuple = self.add_type(Type::Tuple(params), self.finish_span(span));
                path.args = vec![tuple, result];
                self.add_type(Type::Path(path), self.finish_span(span))
            }
            None => self.add_type(Type::Function { params, result }, self.finish_span(span)),
        }
    }

    // Every input-shaped type edge is a continuation on this heap stack.
    // Arena indices are returned between frames; neither parsing nor dropping
    // nested types relies on the embedding thread's native stack size.
    fn ty(&mut self) -> Result<TypeId, Diagnostic> {
        let mut frames = vec![TypeFrame::Parse];
        let mut value = 0;
        while let Some(frame) = frames.pop() {
            match frame {
                TypeFrame::Parse => {
                    let span = self.token().span;
                    while self.take("&") || self.take("&&") {
                        if self.tokens[self.at - 1].text == "&&" {
                            frames.push(TypeFrame::Reference {
                                mutable: false,
                                lifetime: String::new(),
                                span,
                            });
                        }
                        let lifetime = if self.token().kind == TokenKind::Lifetime {
                            let name = self.token().text.clone();
                            self.at += 1;
                            name
                        } else {
                            String::new()
                        };
                        let mutable = self.take("mut");
                        frames.push(TypeFrame::Reference {
                            mutable,
                            lifetime,
                            span,
                        });
                    }
                    if self.take("_") {
                        value = 0;
                    } else if self.take("!") {
                        value = self.add_type(Type::Never, span);
                    } else if self.take("[") {
                        frames.push(TypeFrame::Array { span });
                        frames.push(TypeFrame::Parse);
                    } else if self.take("(") {
                        frames.push(TypeFrame::List {
                            span,
                            items: Vec::new(),
                            comma: false,
                            pending: false,
                            kind: TypeListKind::Tuple,
                        });
                    } else if self.take("fn") {
                        self.expect("(")?;
                        frames.push(TypeFrame::List {
                            span,
                            items: Vec::new(),
                            comma: false,
                            pending: false,
                            kind: TypeListKind::Function,
                        });
                    } else {
                        if self.is("dyn") || self.is("impl") || self.is("*") || self.is("for") {
                            return Err(self.unsupported(&self.token().text));
                        }
                        let absolute = self.take("::");
                        let mut name = self.ident()?;
                        if absolute {
                            name = format!("::{name}");
                        }
                        frames.push(TypeFrame::Path {
                            span,
                            path: Path {
                                declaration: None,
                                name,
                                args: Vec::new(),
                            },
                            position: TypePathPosition::Part,
                        });
                    }
                }
                TypeFrame::Reference {
                    mutable,
                    lifetime,
                    span,
                } => {
                    value = self.add_type(
                        Type::Ref {
                            mutable,
                            lifetime,
                            inner: value,
                        },
                        self.finish_span(span),
                    );
                }
                TypeFrame::Array { span } => {
                    if self.take(";") {
                        if self.token().kind != TokenKind::Number {
                            return Err(self.unsupported("non-literal array length"));
                        }
                        let len = self.token().text.clone();
                        self.at += 1;
                        self.expect("]")?;
                        value = self
                            .add_type(Type::Array { inner: value, len }, self.finish_span(span));
                    } else {
                        self.expect("]")?;
                        value = self.add_type(Type::Slice(value), self.finish_span(span));
                    }
                }
                TypeFrame::List {
                    span,
                    mut items,
                    mut comma,
                    pending,
                    kind,
                } => {
                    let mut closed = false;
                    if pending {
                        items.push(value);
                        if self.take(",") {
                            comma = true;
                        } else {
                            self.expect(")")?;
                            closed = true;
                        }
                    }
                    if !closed {
                        closed = self.take(")");
                    }
                    if !closed {
                        frames.push(TypeFrame::List {
                            span,
                            items,
                            comma,
                            pending: true,
                            kind,
                        });
                        frames.push(TypeFrame::Parse);
                        continue;
                    }
                    let path = match kind {
                        TypeListKind::Tuple => {
                            value = if items.len() == 1 && !comma {
                                items[0]
                            } else {
                                self.add_type(Type::Tuple(items), self.finish_span(span))
                            };
                            continue;
                        }
                        TypeListKind::Function => None,
                        TypeListKind::Callable(path) => Some(path),
                    };
                    if self.take("->") {
                        frames.push(TypeFrame::FunctionResult {
                            span,
                            params: items,
                            path,
                        });
                        frames.push(TypeFrame::Parse);
                    } else {
                        value = self.callable_type(span, items, path, 1);
                    }
                }
                TypeFrame::FunctionResult { span, params, path } => {
                    value = self.callable_type(span, params, path, value);
                }
                TypeFrame::Path {
                    span,
                    mut path,
                    position,
                } => {
                    let mut complete = false;
                    let allow_arguments = match position {
                        TypePathPosition::Part => true,
                        TypePathPosition::Arguments => false,
                        TypePathPosition::Turbofish => {
                            if self.take("::") {
                                path.name.push_str("::");
                                path.name.push_str(&self.ident()?);
                                true
                            } else {
                                complete = true;
                                false
                            }
                        }
                    };
                    if allow_arguments && self.is("<") {
                        if !path.args.is_empty() {
                            return Err(
                                self.unsupported("multiple generic argument lists in one path")
                            );
                        }
                        self.at += 1;
                        frames.push(TypeFrame::Arguments {
                            span,
                            path,
                            turbofish: false,
                            pending: false,
                        });
                        continue;
                    }
                    if !complete && self.take("::") {
                        if self.is("<") {
                            if !path.args.is_empty() {
                                return Err(
                                    self.unsupported("multiple generic argument lists in one path")
                                );
                            }
                            self.at += 1;
                            frames.push(TypeFrame::Arguments {
                                span,
                                path,
                                turbofish: true,
                                pending: false,
                            });
                        } else {
                            path.name.push_str("::");
                            path.name.push_str(&self.ident()?);
                            frames.push(TypeFrame::Path {
                                span,
                                path,
                                position: TypePathPosition::Part,
                            });
                        }
                        continue;
                    }
                    self.capability_path(&path.name)?;
                    if (path.name == "Fn" || path.name == "FnMut" || path.name == "FnOnce")
                        && self.take("(")
                    {
                        frames.push(TypeFrame::List {
                            span,
                            items: Vec::new(),
                            comma: false,
                            pending: false,
                            kind: TypeListKind::Callable(path),
                        });
                    } else {
                        value = self.add_type(Type::Path(path), self.finish_span(span));
                    }
                }
                TypeFrame::Arguments {
                    span,
                    mut path,
                    turbofish,
                    pending,
                } => {
                    let mut parsed = pending;
                    if pending {
                        path.args.push(value);
                    } else if self.token().kind == TokenKind::Lifetime {
                        let lifetime_span = self.token().span;
                        let name = self.token().text.clone();
                        self.at += 1;
                        path.args.push(self.add_type(
                            Type::Path(Path {
                                declaration: None,
                                name,
                                args: Vec::new(),
                            }),
                            lifetime_span,
                        ));
                        parsed = true;
                    } else if !self.at_gt() {
                        frames.push(TypeFrame::Arguments {
                            span,
                            path,
                            turbofish,
                            pending: true,
                        });
                        frames.push(TypeFrame::Parse);
                        continue;
                    }
                    if parsed && self.take(",") && !self.at_gt() {
                        frames.push(TypeFrame::Arguments {
                            span,
                            path,
                            turbofish,
                            pending: false,
                        });
                        continue;
                    }
                    self.gt()?;
                    frames.push(TypeFrame::Path {
                        span,
                        path,
                        position: if turbofish {
                            TypePathPosition::Turbofish
                        } else {
                            TypePathPosition::Arguments
                        },
                    });
                }
            }
        }
        Ok(value)
    }
    fn function(&mut self, attrs: Vec<Attribute>, public: Visibility) -> Result<usize, Diagnostic> {
        let span = self.token().span;
        self.expect("fn")?;
        let name = self.ident()?;
        let mut generics = self.generics()?;
        self.expect("(")?;
        let mut params = Vec::new();
        while !self.is(")") {
            let start = self.token().span;
            let reference = self.take("&");
            let lifetime = if reference && self.token().kind == TokenKind::Lifetime {
                let s = self.token().text.clone();
                self.at += 1;
                s
            } else {
                String::new()
            };
            let mutable = self.take("mut");
            let param_name = self.ident()?;
            let mut ty;
            if param_name == "self" && !self.is(":") {
                ty = self.add_type(
                    Type::Path(Path {
                        declaration: None,
                        name: "Self".to_string(),
                        args: Vec::new(),
                    }),
                    start,
                );
                if reference {
                    ty = self.add_type(
                        Type::Ref {
                            mutable,
                            lifetime,
                            inner: ty,
                        },
                        start,
                    );
                }
            } else {
                if reference {
                    return Err(self.unsupported("reference pattern in parameter position"));
                }
                self.expect(":")?;
                ty = self.ty()?;
            }
            params.push(Param {
                name: param_name,
                ty,
                mutable: mutable && !reference,
                span: self.finish_span(start),
            });
            if !self.take(",") {
                break;
            }
        }
        self.expect(")")?;
        let result = if self.take("->") { self.ty()? } else { 1 };
        self.where_clause(&mut generics)?;
        let body = if self.take(";") {
            None
        } else {
            Some(self.block()?)
        };
        let id = self.program.functions.len();
        self.program.functions.push(Function {
            name,
            module: self.module.clone(),
            generics,
            params,
            result,
            body,
            attrs,
            public,
            span: self.finish_span(span),
        });
        Ok(id)
    }
    fn fields(&mut self, named: bool, end: &str) -> Result<Vec<Field>, Diagnostic> {
        let mut fields = Vec::new();
        while !self.is(end) {
            let span = self.token().span;
            for attr in self.attrs()? {
                if attr.name == "cfg" {
                    return Err(Diagnostic::new(
                        attr.span,
                        "E_UNSUPPORTED",
                        "conditional fields are not supported; help: put #[cfg(...)] on the whole struct or enum declaration",
                    ));
                }
            }
            let public = self.visibility()?;
            let name = if named {
                let n = self.ident()?;
                self.expect(":")?;
                n
            } else {
                fields.len().to_string()
            };
            let ty = self.ty()?;
            fields.push(Field {
                name,
                ty,
                public,
                span: self.finish_span(span),
            });
            if !self.take(",") {
                break;
            }
        }
        self.expect(end)?;
        Ok(fields)
    }
    fn structure(
        &mut self,
        attrs: Vec<Attribute>,
        public: Visibility,
    ) -> Result<usize, Diagnostic> {
        let span = self.token().span;
        self.expect("struct")?;
        let name = self.ident()?;
        let mut generics = self.generics()?;
        self.where_clause(&mut generics)?;
        let tuple = self.take("(");
        let unit = !tuple && !self.is("{");
        let fields;
        if tuple {
            fields = self.fields(false, ")")?;
            self.expect(";")?;
        } else if self.take("{") {
            fields = self.fields(true, "}")?;
        } else {
            self.expect(";")?;
            fields = Vec::new();
        }
        let id = self.program.structs.len();
        self.program.structs.push(Struct {
            name,
            module: self.module.clone(),
            generics,
            fields,
            tuple,
            unit,
            attrs,
            public,
            span: self.finish_span(span),
        });
        self.derive_default(id)?;
        Ok(id)
    }
    fn enumeration(
        &mut self,
        attrs: Vec<Attribute>,
        public: Visibility,
    ) -> Result<usize, Diagnostic> {
        let span = self.token().span;
        self.expect("enum")?;
        let name = self.ident()?;
        let mut generics = self.generics()?;
        self.where_clause(&mut generics)?;
        self.expect("{")?;
        let mut variants = Vec::new();
        while !self.is("}") {
            let start = self.token().span;
            let variant_name = self.ident()?;
            let mut style = VariantStyle::Unit;
            let fields = if self.take("(") {
                style = VariantStyle::Tuple;
                self.fields(false, ")")?
            } else if self.take("{") {
                style = VariantStyle::Named;
                self.fields(true, "}")?
            } else {
                Vec::new()
            };
            for field in &fields {
                if field.public != Visibility::Private {
                    return Err(Diagnostic::new(
                        field.span,
                        "E_PARSE",
                        "enum variant fields inherit the enum visibility",
                    ));
                }
            }
            let discriminant = if self.take("=") {
                Some(self.expr(true)?)
            } else {
                None
            };
            variants.push(Variant {
                name: variant_name,
                fields,
                style,
                discriminant,
                span: self.finish_span(start),
            });
            if !self.take(",") {
                break;
            }
        }
        self.expect("}")?;
        let id = self.program.enums.len();
        self.program.enums.push(Enum {
            name,
            module: self.module.clone(),
            generics,
            variants,
            attrs,
            public,
            span: self.finish_span(span),
        });
        Ok(id)
    }
    fn associated_type(&mut self) -> Result<AssociatedType, Diagnostic> {
        let span = self.token().span;
        self.expect("type")?;
        let name = self.ident()?;
        let bounds = if self.take(":") {
            self.bounds()?
        } else {
            Vec::new()
        };
        let value = if self.take("=") {
            Some(self.ty()?)
        } else {
            None
        };
        self.expect(";")?;
        Ok(AssociatedType {
            name,
            bounds,
            value,
            span: self.finish_span(span),
        })
    }
    fn trait_item(&mut self, public: Visibility) -> Result<usize, Diagnostic> {
        let span = self.token().span;
        self.expect("trait")?;
        let name = self.ident()?;
        let mut generics = self.generics()?;
        let bounds = if self.take(":") {
            self.bounds()?
        } else {
            Vec::new()
        };
        self.where_clause(&mut generics)?;
        self.expect("{")?;
        let mut methods = Vec::new();
        let mut associated = Vec::new();
        while !self.is("}") {
            if self.token().kind == TokenKind::End {
                return Err(self.err("unterminated trait"));
            }
            let start = self.at;
            let mut domain = None;
            let result = (|| {
                let attrs = self.attrs()?;
                if !self.enabled(&attrs)? {
                    self.skip_disabled()?;
                    return Ok(());
                }
                domain =
                    Some(self.recovery_domain(Some(RecoveryOwner::Trait { name: name.clone() })));
                if self.is("fn") {
                    methods.push(self.function(attrs, Visibility::Private)?);
                } else if self.is("type") {
                    associated.push(self.associated_type()?);
                } else {
                    return Err(self.unsupported("trait item"));
                }
                Ok(())
            })();
            if let Err(error) = result {
                self.recover_region(start, "item", error, domain);
            }
        }
        self.expect("}")?;
        let id = self.program.traits.len();
        self.program.traits.push(Trait {
            name,
            module: self.module.clone(),
            generics,
            bounds,
            methods,
            associated,
            public,
            span: self.finish_span(span),
        });
        Ok(id)
    }
    fn impl_item(&mut self) -> Result<usize, Diagnostic> {
        let span = self.token().span;
        self.expect("impl")?;
        let mut generics = self.generics()?;
        let first = self.ty()?;
        let trait_ty;
        let target;
        if self.take("for") {
            trait_ty = Some(first);
            target = self.ty()?;
        } else {
            trait_ty = None;
            target = first;
        }
        self.where_clause(&mut generics)?;
        self.expect("{")?;
        let mut methods = Vec::new();
        let mut associated = Vec::new();
        while !self.is("}") {
            if self.token().kind == TokenKind::End {
                return Err(self.err("unterminated impl"));
            }
            let start = self.at;
            let mut domain = None;
            let result = (|| {
                let attrs = self.attrs()?;
                if !self.enabled(&attrs)? {
                    self.skip_disabled()?;
                    return Ok(());
                }
                let public = self.visibility()?;
                domain = Some(
                    self.recovery_domain(Some(RecoveryOwner::Implementation { target, trait_ty })),
                );
                if self.is("fn") {
                    methods.push(self.function(attrs, public)?);
                } else if self.is("type") {
                    associated.push(self.associated_type()?);
                } else {
                    return Err(self.unsupported("impl item"));
                }
                Ok(())
            })();
            if let Err(error) = result {
                self.recover_region(start, "item", error, domain);
            }
        }
        self.expect("}")?;
        let id = self.program.impls.len();
        self.program.impls.push(Impl {
            module: self.module.clone(),
            generics,
            target,
            trait_ty,
            methods,
            associated,
            span: self.finish_span(span),
        });
        Ok(id)
    }
    fn use_item(&mut self, public: Visibility) -> Result<usize, Diagnostic> {
        let first = self.program.uses.len();
        let span = self.tokens[self.at - 1].span;
        self.use_tree(String::new(), public, span)?;
        self.expect(";")?;
        Ok(first)
    }
    fn use_tree(
        &mut self,
        prefix: String,
        public: Visibility,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if self.take("{") {
            while !self.is("}") {
                self.use_tree(prefix.clone(), public, span)?;
                if !self.take(",") {
                    break;
                }
            }
            self.expect("}")?;
            return Ok(());
        }
        if self.take("*") {
            let path = prefix.trim_end_matches("::").to_string();
            self.capability_path(&path)?;
            self.program.uses.push(Use {
                module: self.module.clone(),
                path,
                alias: String::new(),
                glob: true,
                public,
                span: self.finish_span(span),
            });
            return Ok(());
        }
        let name = self.ident()?;
        let path = if name == "self" && !prefix.is_empty() {
            prefix.trim_end_matches("::").to_string()
        } else {
            format!("{prefix}{name}")
        };
        if self.take("::") {
            return self.use_tree(format!("{path}::"), public, span);
        }
        let alias = if self.take("as") {
            self.ident()?
        } else if name == "self" && !prefix.is_empty() {
            prefix
                .trim_end_matches("::")
                .rsplit("::")
                .next()
                .unwrap()
                .to_string()
        } else {
            name
        };
        self.capability_path(&path)?;
        self.program.uses.push(Use {
            module: self.module.clone(),
            path,
            alias,
            glob: false,
            public,
            span: self.finish_span(span),
        });
        Ok(())
    }
    fn block(&mut self) -> Result<BlockId, Diagnostic> {
        let span = self.token().span;
        self.expect("{")?;
        let mut statements = Vec::new();
        let mut tail = None;
        while !self.is("}") {
            if self.token().kind == TokenKind::End {
                return Err(self.err("unterminated block"));
            }
            if self.take(";") {
                continue;
            }
            let start = self.at;
            let scope = self.scope;
            let module = self.module.clone();
            match self.block_part() {
                Ok(BlockPart::Skipped) => {}
                Ok(BlockPart::Statement(statement)) => statements.push(statement),
                Ok(BlockPart::Tail(expression)) => {
                    tail = Some(expression);
                    break;
                }
                Err(error) => {
                    self.scope = scope;
                    self.module = module;
                    self.recover_region(start, "statement", error, None);
                }
            }
        }
        self.expect("}")?;
        let id = self.program.blocks.len();
        self.program.blocks.push(Block {
            statements,
            tail,
            span: self.finish_span(span),
        });
        Ok(id)
    }

    fn block_part(&mut self) -> Result<BlockPart, Diagnostic> {
        if self.take("let") {
            let start = self.tokens[self.at - 1].span;
            let pattern = self.pattern()?;
            let ty = if self.take(":") {
                Some(self.ty()?)
            } else {
                None
            };
            let value = if self.take("=") {
                Some(self.expr(true)?)
            } else {
                None
            };
            let (pattern, value, ty) = if self.take("else") {
                let value = value.ok_or_else(|| self.err("let-else requires an initializer"))?;
                let block = self.block()?;
                let otherwise = self.add_expr(Expr::Block(block), self.program.blocks[block].span);
                let otherwise =
                    self.add_expr(Expr::Diverge(otherwise), self.program.blocks[block].span);
                // A match produces exactly the bindings exported by the pattern.
                // Generated tuple bindings do not repeat ref binding modes.
                let mut pending = vec![pattern];
                let mut bindings = Vec::new();
                let mut seen = BTreeSet::new();
                while let Some(id) = pending.pop() {
                    match self.program.patterns[id].kind.clone() {
                        Pattern::Bind { name, mutable, .. } => {
                            if seen.insert(name.clone()) {
                                bindings.push((name, mutable));
                            }
                        }
                        Pattern::Tuple(fields) | Pattern::Variant { fields, .. } => {
                            pending.extend(fields.into_iter().rev())
                        }
                        Pattern::Struct { fields, .. } => {
                            pending.extend(fields.into_iter().rev().map(|(_, pattern)| pattern))
                        }
                        Pattern::Ref { inner, .. } => pending.push(inner),
                        Pattern::Or(alternatives) => {
                            if let Some(first) = alternatives.first() {
                                pending.push(*first);
                            }
                        }
                        _ => {}
                    }
                }
                let mut result = Vec::new();
                let mut outer = Vec::new();
                for (name, mutable) in bindings {
                    result.push(self.add_expr(
                        Expr::Path(Path {
                            declaration: None,
                            name: name.clone(),
                            args: Vec::new(),
                        }),
                        start,
                    ));
                    outer.push(self.add_pattern(
                        Pattern::Bind {
                            name,
                            mutable,
                            by_ref: false,
                            ref_mut: false,
                        },
                        start,
                    ));
                }
                let output = self.add_expr(Expr::Tuple(result), start);
                let wildcard = self.add_pattern(Pattern::Wild, start);
                let initializer = if let Some(ty) = ty {
                    // The generated tail reads the typed binding after its
                    // initializer. Keep that ordering in lexical use positions
                    // so reference cleanup cannot run before the tail read.
                    let initializer_span = self.program.exprs[value].span;
                    let block_span = self.finish_span(start);
                    let tail_span = Span {
                        start: initializer_span.end,
                        end: initializer_span.end,
                        ..initializer_span
                    };
                    let binding_span = Span {
                        end: initializer_span.end,
                        ..start
                    };
                    let name = format!("$let_else{}", self.program.exprs.len());
                    let binding = self.add_pattern(
                        Pattern::Bind {
                            name: name.clone(),
                            mutable: false,
                            by_ref: false,
                            ref_mut: false,
                        },
                        start,
                    );
                    let path = self.add_expr(
                        Expr::Path(Path {
                            declaration: None,
                            name,
                            args: Vec::new(),
                        }),
                        tail_span,
                    );
                    let block = self.program.blocks.len();
                    self.program.blocks.push(Block {
                        statements: vec![Stmt::Let {
                            pattern: binding,
                            ty: Some(ty),
                            value: Some(value),
                            span: binding_span,
                        }],
                        tail: Some(path),
                        span: block_span,
                    });
                    self.add_expr(Expr::Block(block), block_span)
                } else {
                    value
                };
                let value = self.add_expr(
                    Expr::Match {
                        value: initializer,
                        arms: vec![
                            Arm {
                                pattern,
                                guard: None,
                                value: output,
                                span: start,
                            },
                            Arm {
                                pattern: wildcard,
                                guard: None,
                                value: otherwise,
                                span: start,
                            },
                        ],
                    },
                    self.finish_span(start),
                );
                let pattern = self.add_pattern(Pattern::Tuple(outer), start);
                (pattern, Some(value), None)
            } else {
                (pattern, value, ty)
            };
            self.expect(";")?;
            return Ok(BlockPart::Statement(Stmt::Let {
                pattern,
                ty,
                value,
                span: self.finish_span(start),
            }));
        }
        if self.is("fn")
            || self.is("const")
            || self.is("struct")
            || self.is("enum")
            || self.is("use")
            || self.is("type")
            || self.is("#")
        {
            let attrs = self.attrs()?;
            if !self.enabled(&attrs)? {
                self.skip_disabled()?;
                return Ok(BlockPart::Skipped);
            }
            let public = self.visibility()?;
            let item = self.item(attrs, public)?;
            return Ok(BlockPart::Statement(Stmt::Item(item)));
        }
        let (expression, standalone) = self.statement_expr()?;
        if self.take(";") {
            Ok(BlockPart::Statement(Stmt::Expr {
                value: expression,
                semicolon: true,
            }))
        } else if self.is("}") {
            Ok(BlockPart::Tail(expression))
        } else {
            if !standalone {
                return Err(self.err("expected `;` after expression"));
            }
            Ok(BlockPart::Statement(Stmt::Expr {
                value: expression,
                semicolon: false,
            }))
        }
    }
    /// Block expressions in statement or match-arm position end before an
    /// ambiguous following expression. A dot or question mark continues the
    /// expression; grouping starts an ordinary value expression instead. Keep
    /// the written-form distinction here because parentheses have no AST node.
    fn statement_expr(&mut self) -> Result<(ExprId, bool), Diagnostic> {
        let block = self.is("{")
            || self.is("if")
            || self.is("while")
            || self.is("loop")
            || self.is("for")
            || self.is("match");
        if block {
            let expression = self.atom(true)?;
            if self.is("as") {
                return Err(self.err("parenthesize a block value before a cast"));
            }
            if self.is(".") || self.is("?") {
                Ok((self.expr_tail(expression, 1, true)?, false))
            } else {
                Ok((expression, true))
            }
        } else {
            Ok((self.expr(true)?, false))
        }
    }
    fn expr(&mut self, allow_struct: bool) -> Result<ExprId, Diagnostic> {
        self.expr_bp(1, allow_struct)
    }
    fn expr_bp(&mut self, min: u8, allow_struct: bool) -> Result<ExprId, Diagnostic> {
        let first = self.prefix(allow_struct)?;
        self.expr_tail(first, min, allow_struct)
    }
    fn expression_end(&self) -> bool {
        self.token().kind == TokenKind::End
            || self.is(";")
            || self.is(",")
            || self.is("}")
            || self.is(")")
            || self.is("]")
            || self.is("=>")
    }
    fn prefix(&mut self, allow_struct: bool) -> Result<ExprId, Diagnostic> {
        let mut unary: Vec<(String, bool, Span)> = Vec::new();
        loop {
            let span = self.token().span;
            if self.take("&") {
                let mutable = self.take("mut");
                unary.push(("&".to_string(), mutable, span));
            } else if self.take("&&") {
                unary.push(("&".to_string(), false, span));
                let mutable = self.take("mut");
                unary.push(("&".to_string(), mutable, span));
            } else if self.is("-") || self.is("!") || self.is("*") {
                let op = self.token().text.clone();
                self.at += 1;
                unary.push((op, false, span));
            } else {
                break;
            }
        }
        let first = self.atom(allow_struct)?;
        let mut value = self.expr_tail(first, 13, allow_struct)?;
        while let Some((op, mutable, span)) = unary.pop() {
            let kind = if op == "&" {
                Expr::Borrow { mutable, value }
            } else {
                Expr::Unary { op, value }
            };
            value = self.add_expr(kind, self.finish_span(span));
        }
        Ok(value)
    }
    fn expr_tail(
        &mut self,
        mut left: ExprId,
        min: u8,
        allow_struct: bool,
    ) -> Result<ExprId, Diagnostic> {
        loop {
            let span = self.program.exprs[left].span;
            if self.take("(") {
                let args = self.expr_list(")")?;
                left = self.add_expr(Expr::Call { callee: left, args }, self.finish_span(span));
                continue;
            }
            if self.take("[") {
                let index = self.expr(true)?;
                self.expect("]")?;
                left = self.add_expr(Expr::Index { value: left, index }, self.finish_span(span));
                continue;
            }
            if self.take(".") {
                if self.is("await") {
                    return Err(self.unsupported("await"));
                }
                if self.token().kind == TokenKind::Number {
                    // In postfix position 0.1 denotes two tuple projections,
                    // even though the lexer can read it as a decimal token.
                    let text = self.token().text.clone();
                    let mut indices = Vec::new();
                    for part in text.split('.') {
                        let index = part
                            .replace('_', "")
                            .parse::<usize>()
                            .map_err(|_| self.err("tuple field requires a decimal index"))?;
                        indices.push(index.to_string());
                    }
                    self.at += 1;
                    for name in indices {
                        left = self
                            .add_expr(Expr::Field { value: left, name }, self.finish_span(span));
                    }
                    continue;
                }
                let name = self.ident()?;
                let types = if self.take("::") {
                    self.generic_args()?
                } else {
                    Vec::new()
                };
                if self.take("(") {
                    let args = self.expr_list(")")?;
                    left = self.add_expr(
                        Expr::Method {
                            receiver: left,
                            name,
                            types,
                            args,
                        },
                        self.finish_span(span),
                    );
                } else {
                    if !types.is_empty() {
                        return Err(self.err("method type arguments require a call"));
                    }
                    left = self.add_expr(Expr::Field { value: left, name }, self.finish_span(span));
                }
                continue;
            }
            if self.take("?") {
                left = self.add_expr(Expr::Try(left), self.finish_span(span));
                continue;
            }
            if self.is("as") && min <= 12 {
                self.at += 1;
                let ty = self.ty()?;
                left = self.add_expr(Expr::Cast { value: left, ty }, self.finish_span(span));
                continue;
            }
            if self.token().kind != TokenKind::Symbol {
                break;
            }
            let op = self.token().text.clone();
            let prec = match op.as_str() {
                "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>=" => 1,
                ".." | "..=" => 2,
                "||" => 3,
                "&&" => 4,
                "==" | "!=" | "<" | ">" | "<=" | ">=" => 5,
                "|" => 6,
                "^" => 7,
                "&" => 8,
                "<<" | ">>" => 9,
                "+" | "-" => 10,
                "*" | "/" | "%" => 11,
                _ => 0,
            };
            if prec == 0 || prec < min {
                break;
            }
            self.at += 1;
            if prec == 2 {
                let end = if self.expression_end() || (!allow_struct && self.is("{")) {
                    None
                } else {
                    Some(self.expr_bp(3, allow_struct)?)
                };
                if op == "..=" && end.is_none() {
                    return Err(self.err("inclusive range needs an end"));
                }
                left = self.add_expr(
                    Expr::Range {
                        start: Some(left),
                        end,
                        inclusive: op == "..=",
                    },
                    self.finish_span(span),
                );
                continue;
            }
            let right = self.expr_bp(if prec == 1 { 1 } else { prec + 1 }, allow_struct)?;
            let kind = if prec == 1 {
                Expr::Assign {
                    op,
                    place: left,
                    value: right,
                }
            } else {
                Expr::Binary { op, left, right }
            };
            left = self.add_expr(kind, self.finish_span(span));
        }
        Ok(left)
    }
    fn expr_list(&mut self, end: &str) -> Result<Vec<ExprId>, Diagnostic> {
        let mut values = Vec::new();
        while !self.is(end) {
            values.push(self.expr(true)?);
            if !self.take(",") {
                break;
            }
        }
        self.expect(end)?;
        Ok(values)
    }
    fn literal(&mut self) -> Result<ExprId, Diagnostic> {
        let token = self.token().clone();
        self.at += 1;
        let kind = match token.kind {
            TokenKind::Number => {
                let radix = token.text.starts_with("0x")
                    || token.text.starts_with("0b")
                    || token.text.starts_with("0o");
                let integer_suffix = [
                    "usize", "isize", "u64", "i64", "u32", "i32", "u16", "i16", "u8", "i8",
                ]
                .iter()
                .any(|suffix| token.text.ends_with(suffix));
                if !radix
                    && !integer_suffix
                    && (token.text.contains('.')
                        || token.text.contains('e')
                        || token.text.contains('E')
                        || token.text.ends_with("f64")
                        || token.text.ends_with("f32"))
                {
                    Expr::Float(token.text)
                } else {
                    Expr::Int(token.text)
                }
            }
            TokenKind::String => Expr::String(token.text),
            TokenKind::Bytes => Expr::Bytes(token.text.chars().map(|ch| ch as u8).collect()),
            TokenKind::Char => match token.text.chars().next() {
                Some(ch) => Expr::Char(ch),
                None => return Err(self.err("empty character literal")),
            },
            TokenKind::Ident if token.text == "true" || token.text == "false" => {
                Expr::Bool(token.text == "true")
            }
            _ => return Err(self.err("expected a literal")),
        };
        Ok(self.add_expr(kind, token.span))
    }
    fn atom(&mut self, allow_struct: bool) -> Result<ExprId, Diagnostic> {
        let span = self.token().span;
        if self.token().kind == TokenKind::Number
            || self.token().kind == TokenKind::String
            || self.token().kind == TokenKind::Char
            || self.token().kind == TokenKind::Bytes
            || self.is("true")
            || self.is("false")
        {
            return self.literal();
        }
        if self.is("(") {
            // Consecutive groups use a heap stack; a million parentheses need no native recursion.
            let mut openings = Vec::new();
            while self.take("(") {
                openings.push(self.tokens[self.at - 1].span);
            }
            let mut value;
            if self.is(")") {
                value = self.add_expr(Expr::Tuple(Vec::new()), self.token().span);
            } else {
                value = self.expr(true)?;
            }
            while let Some(start) = openings.pop() {
                if self.take(",") {
                    let mut fields = vec![value];
                    while !self.is(")") {
                        fields.push(self.expr(true)?);
                        if !self.take(",") {
                            break;
                        }
                    }
                    value = self.add_expr(Expr::Tuple(fields), self.finish_span(start));
                }
                self.expect(")")?;
                if !openings.is_empty() {
                    value = self.expr_tail(value, 1, true)?;
                }
            }
            return Ok(value);
        }
        if self.take("[") {
            if self.take("]") {
                return Ok(self.add_expr(Expr::Array(Vec::new()), self.finish_span(span)));
            }
            let first = self.expr(true)?;
            if self.take(";") {
                let count = self.expr(true)?;
                self.expect("]")?;
                return Ok(self.add_expr(
                    Expr::Repeat {
                        value: first,
                        count,
                    },
                    self.finish_span(span),
                ));
            }
            let mut values = vec![first];
            while self.take(",") {
                if self.is("]") {
                    break;
                }
                values.push(self.expr(true)?);
            }
            self.expect("]")?;
            return Ok(self.add_expr(Expr::Array(values), self.finish_span(span)));
        }
        if self.is("{") {
            let block = self.block()?;
            return Ok(self.add_expr(Expr::Block(block), self.finish_span(span)));
        }
        if self.take("if") {
            let pattern = if self.take("let") {
                let p = self.pattern()?;
                self.expect("=")?;
                Some(p)
            } else {
                None
            };
            let condition = self.expr(false)?;
            let then_block = self.block()?;
            let otherwise = if self.take("else") {
                if self.is("if") || self.is("{") {
                    Some(self.atom(true)?)
                } else {
                    return Err(self.err("expected `if` or a block after `else`"));
                }
            } else {
                None
            };
            let kind = match pattern {
                Some(pattern) => {
                    let body = self.add_expr(
                        Expr::Block(then_block),
                        self.program.blocks[then_block].span,
                    );
                    let fallback =
                        otherwise.unwrap_or_else(|| self.add_expr(Expr::Tuple(Vec::new()), span));
                    let wild = self.add_pattern(Pattern::Wild, span);
                    Expr::Match {
                        value: condition,
                        arms: vec![
                            Arm {
                                pattern,
                                guard: None,
                                value: body,
                                span,
                            },
                            Arm {
                                pattern: wild,
                                guard: None,
                                value: fallback,
                                span,
                            },
                        ],
                    }
                }
                None => Expr::If {
                    condition,
                    then_block,
                    otherwise,
                },
            };
            return Ok(self.add_expr(kind, self.finish_span(span)));
        }
        if self.take("while") {
            let pattern = if self.take("let") {
                let p = self.pattern()?;
                self.expect("=")?;
                Some(p)
            } else {
                None
            };
            let condition = self.expr(false)?;
            let body = self.block()?;
            let kind = match pattern {
                Some(pattern) => {
                    let matched = self.add_expr(Expr::Block(body), self.program.blocks[body].span);
                    let fallback = self.add_expr(Expr::Break(None), span);
                    let wild = self.add_pattern(Pattern::Wild, span);
                    let selection = self.add_expr(
                        Expr::Match {
                            value: condition,
                            arms: vec![
                                Arm {
                                    pattern,
                                    guard: None,
                                    value: matched,
                                    span,
                                },
                                Arm {
                                    pattern: wild,
                                    guard: None,
                                    value: fallback,
                                    span,
                                },
                            ],
                        },
                        span,
                    );
                    let body = self.program.blocks.len();
                    self.program.blocks.push(Block {
                        statements: vec![Stmt::Expr {
                            value: selection,
                            semicolon: true,
                        }],
                        tail: None,
                        span: self.finish_span(span),
                    });
                    Expr::Loop { body }
                }
                None => Expr::While { condition, body },
            };
            return Ok(self.add_expr(kind, self.finish_span(span)));
        }
        if self.take("loop") {
            let body = self.block()?;
            return Ok(self.add_expr(Expr::Loop { body }, self.finish_span(span)));
        }
        if self.take("for") {
            let pattern = self.pattern()?;
            self.expect("in")?;
            let iterator = self.expr(false)?;
            let body = self.block()?;
            return Ok(self.add_expr(
                Expr::For {
                    pattern,
                    iterator,
                    body,
                },
                self.finish_span(span),
            ));
        }
        if self.take("match") {
            let value = self.expr(false)?;
            self.expect("{")?;
            let mut arms = Vec::new();
            while !self.is("}") {
                let start = self.token().span;
                let pattern = self.pattern()?;
                let guard = if self.take("if") {
                    Some(self.expr(false)?)
                } else {
                    None
                };
                self.expect("=>")?;
                let (arm_value, block_like) = self.statement_expr()?;
                arms.push(Arm {
                    pattern,
                    guard,
                    value: arm_value,
                    span: self.finish_span(start),
                });
                if !self.take(",") && !self.is("}") {
                    if !block_like {
                        return Err(self.err("expected `,` between match arms"));
                    }
                }
            }
            self.expect("}")?;
            return Ok(self.add_expr(Expr::Match { value, arms }, self.finish_span(span)));
        }
        if self.take("return") {
            let value = if self.expression_end() {
                None
            } else {
                Some(self.expr(true)?)
            };
            return Ok(self.add_expr(Expr::Return(value), self.finish_span(span)));
        }
        if self.take("break") {
            if self.token().kind == TokenKind::Lifetime {
                return Err(self.unsupported("labeled break"));
            }
            let value = if self.expression_end() {
                None
            } else {
                Some(self.expr(true)?)
            };
            return Ok(self.add_expr(Expr::Break(value), self.finish_span(span)));
        }
        if self.take("continue") {
            if self.token().kind == TokenKind::Lifetime {
                return Err(self.unsupported("labeled continue"));
            }
            return Ok(self.add_expr(Expr::Continue, self.finish_span(span)));
        }
        if self.is("move") || self.is("|") || self.is("||") {
            let moved = self.take("move");
            let mut params = Vec::new();
            let mut patterns = Vec::new();
            if !self.take("||") {
                self.expect("|")?;
                while !self.is("|") {
                    let start = self.token().span;
                    let pattern = self.pattern_atom()?;
                    let (name, mutable) = match &self.program.patterns[pattern].kind {
                        Pattern::Bind {
                            name,
                            mutable,
                            by_ref: false,
                            ..
                        } => (name.clone(), *mutable),
                        _ => (format!("$closure_arg{}", params.len()), false),
                    };
                    patterns.push(pattern);
                    let ty = if self.take(":") { self.ty()? } else { 0 };
                    params.push(Param {
                        name,
                        ty,
                        mutable,
                        span: self.finish_span(start),
                    });
                    if !self.take(",") {
                        break;
                    }
                }
                self.expect("|")?;
            }
            let result = if self.take("->") {
                Some(self.ty()?)
            } else {
                None
            };
            let body = self.expr(true)?;
            return Ok(self.add_expr(
                Expr::Closure {
                    moved,
                    params,
                    patterns,
                    result,
                    body,
                },
                self.finish_span(span),
            ));
        }
        if self.is("..") || self.is("..=") {
            let inclusive = self.is("..=");
            self.at += 1;
            let end = if self.expression_end() {
                None
            } else {
                Some(self.expr_bp(3, allow_struct)?)
            };
            if inclusive && end.is_none() {
                return Err(self.err("inclusive range needs an end"));
            }
            return Ok(self.add_expr(
                Expr::Range {
                    start: None,
                    end,
                    inclusive,
                },
                self.finish_span(span),
            ));
        }
        if self.is("unsafe")
            || self.is("async")
            || self.is("yield")
            || self.is("const")
            || self.token().kind == TokenKind::Lifetime
        {
            return Err(self.unsupported(&self.token().text));
        }
        if self.token().kind != TokenKind::Ident && !self.is("::") {
            return Err(self.err("expected an expression"));
        }
        let path = self.value_path()?;
        if self.take("!") {
            if path.name == "println"
                || path.name == "print"
                || path.name == "eprintln"
                || path.name == "eprint"
            {
                return Err(Diagnostic::new(
                    span,
                    "E_CAPABILITY",
                    "ambient output is unavailable; use a World output capability",
                ));
            }
            let known = path.name == "assert"
                || path.name == "assert_eq"
                || path.name == "assert_ne"
                || path.name == "panic"
                || path.name == "vec"
                || path.name == "format"
                || path.name == "write"
                || path.name == "writeln"
                || path.name == "matches"
                || path.name == "unreachable"
                || path.name == "debug_assert"
                || path.name == "debug_assert_eq"
                || path.name == "debug_assert_ne";
            if !known {
                return Err(Diagnostic::new(
                    span,
                    "E_UNSUPPORTED_MACRO",
                    &format!("unsupported macro: {}!", path.name),
                ));
            }
            let end = if self.take("(") {
                ")"
            } else if self.take("[") {
                "]"
            } else if self.take("{") {
                "}"
            } else {
                return Err(self.err("expected macro delimiter"));
            };
            if path.name == "matches" {
                let value = self.expr(true)?;
                self.expect(",")?;
                let pattern = self.pattern()?;
                let guard = if self.take("if") {
                    Some(self.expr(true)?)
                } else {
                    None
                };
                self.take(",");
                self.expect(end)?;
                let yes = self.add_expr(Expr::Bool(true), span);
                let no = self.add_expr(Expr::Bool(false), span);
                let wildcard = self.add_pattern(Pattern::Wild, span);
                return Ok(self.add_expr(
                    Expr::Match {
                        value,
                        arms: vec![
                            Arm {
                                pattern,
                                guard,
                                value: yes,
                                span,
                            },
                            Arm {
                                pattern: wildcard,
                                guard: None,
                                value: no,
                                span,
                            },
                        ],
                    },
                    self.finish_span(span),
                ));
            }
            let mut args = Vec::new();
            let mut separator = ",".to_string();
            while !self.is(end) {
                args.push(self.expr(true)?);
                if self.take(";") {
                    separator = ";".to_string();
                } else if !self.take(",") {
                    break;
                }
            }
            self.expect(end)?;
            if matches!(path.name.as_str(), "format" | "write" | "writeln") {
                self.normalize_format(&path.name, &mut args, span)?;
            }
            return Ok(self.add_expr(
                Expr::Macro {
                    name: path.name,
                    args,
                    separator,
                },
                self.finish_span(span),
            ));
        }
        if allow_struct && self.take("{") {
            let mut fields = Vec::new();
            let mut base = None;
            while !self.is("}") {
                if self.take("..") {
                    base = Some(self.expr(true)?);
                    self.take(",");
                    break;
                }
                let field_span = self.token().span;
                let name = self.ident()?;
                let value = if self.take(":") {
                    self.expr(true)?
                } else {
                    self.add_expr(
                        Expr::Path(Path {
                            declaration: None,
                            name: name.clone(),
                            args: Vec::new(),
                        }),
                        field_span,
                    )
                };
                fields.push((name, value));
                if !self.take(",") {
                    break;
                }
            }
            self.expect("}")?;
            return Ok(self.add_expr(Expr::Struct { path, fields, base }, self.finish_span(span)));
        }
        Ok(self.add_expr(Expr::Path(path), self.finish_span(span)))
    }
    fn normalize_format(
        &mut self,
        name: &str,
        args: &mut Vec<ExprId>,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let format_index = usize::from(name == "write" || name == "writeln");
        if name == "writeln" && args.len() == 1 {
            let literal = self.add_expr(Expr::String(String::new()), span);
            args.push(literal);
        }
        let Some(&literal) = args.get(format_index) else {
            return Err(Diagnostic::new(
                span,
                "E_FORMAT",
                "format macro requires a literal format string",
            ));
        };
        let Expr::String(text) = self.program.exprs[literal].kind.clone() else {
            return Err(Diagnostic::new(
                span,
                "E_FORMAT",
                "format argument must be a string literal",
            ));
        };
        let mut parts = formatting::parse(&text, span)?;
        let offset = format_index + 1;
        let count = args.len() - offset;
        let mut named = std::collections::BTreeMap::new();
        let mut seen_named = false;
        for index in 0..count {
            if let Expr::Assign { op, place, value } =
                self.program.exprs[args[offset + index]].kind.clone()
            {
                if op == "=" {
                    if let Expr::Path(path) = &self.program.exprs[place].kind {
                        if path.declaration.is_none() && !path.name.contains("::") {
                            if named.insert(path.name.clone(), index).is_some() {
                                return Err(Diagnostic::new(
                                    span,
                                    "E_FORMAT",
                                    "format argument name is repeated",
                                ));
                            }
                            args[offset + index] = value;
                            self.program.exprs[place].kind = Expr::Tuple(Vec::new());
                            seen_named = true;
                            continue;
                        }
                    }
                }
            }
            if seen_named {
                return Err(Diagnostic::new(
                    span,
                    "E_FORMAT",
                    "positional format argument follows a named argument",
                ));
            }
        }
        let mut used = vec![false; count];
        let mut automatic = 0usize;
        let mut normalized = String::new();
        for part in &mut parts {
            match part {
                formatting::Part::Text(text) => {
                    normalized.push_str(&text.replace('{', "{{").replace('}', "}}"));
                }
                formatting::Part::Value {
                    argument, spelling, ..
                } => {
                    let index = if argument.is_empty() {
                        let index = automatic;
                        automatic += 1;
                        if index >= count {
                            return Err(Diagnostic::new(
                                span,
                                "E_FORMAT",
                                "format argument index is out of bounds",
                            ));
                        }
                        index
                    } else if argument.bytes().all(|value| value.is_ascii_digit()) {
                        let index = argument.parse::<usize>().map_err(|_| {
                            Diagnostic::new(
                                span,
                                "E_FORMAT",
                                "format argument index does not fit usize",
                            )
                        })?;
                        if index >= count {
                            return Err(Diagnostic::new(
                                span,
                                "E_FORMAT",
                                "format argument index is out of bounds",
                            ));
                        }
                        index
                    } else if let Some(index) = named.get(argument) {
                        *index
                    } else {
                        let index = args.len() - offset;
                        let value = self.add_expr(
                            Expr::Path(Path {
                                declaration: None,
                                name: argument.clone(),
                                args: Vec::new(),
                            }),
                            span,
                        );
                        args.push(value);
                        named.insert(argument.clone(), index);
                        index
                    };
                    if index >= args.len() - offset {
                        return Err(Diagnostic::new(
                            span,
                            "E_FORMAT",
                            "format argument index is out of bounds",
                        ));
                    }
                    if index < count {
                        used[index] = true;
                    }
                    normalized.push('{');
                    normalized.push_str(&index.to_string());
                    if !spelling.is_empty() {
                        normalized.push(':');
                        normalized.push_str(spelling);
                    }
                    normalized.push('}');
                }
            }
        }
        if used.iter().any(|used| !*used) {
            return Err(Diagnostic::new(
                span,
                "E_FORMAT",
                "format argument is never used",
            ));
        }
        if name == "writeln" {
            normalized.push('\n');
        }
        args[format_index] = self.add_expr(Expr::String(normalized), span);
        Ok(())
    }
    fn pattern(&mut self) -> Result<PatternId, Diagnostic> {
        self.take("|");
        let first = self.pattern_atom()?;
        if !self.take("|") {
            return Ok(first);
        }
        let span = self.program.patterns[first].span;
        let mut arms = vec![first];
        loop {
            arms.push(self.pattern_atom()?);
            if !self.take("|") {
                break;
            }
        }
        Ok(self.add_pattern(Pattern::Or(arms), self.finish_span(span)))
    }
    fn pattern_bound(&mut self) -> Result<ExprId, Diagnostic> {
        let span = self.token().span;
        let negative = self.take("-");
        let value =
            if self.token().kind == TokenKind::Ident && !self.is("true") && !self.is("false")
                || self.is("::")
            {
                let path = self.value_path()?;
                self.add_expr(Expr::Path(path), self.finish_span(span))
            } else {
                self.literal()?
            };
        if negative {
            Ok(self.add_expr(
                Expr::Unary {
                    op: "-".to_string(),
                    value,
                },
                self.finish_span(span),
            ))
        } else {
            Ok(value)
        }
    }
    fn pattern_range(
        &mut self,
        start: Option<ExprId>,
        span: Span,
    ) -> Result<PatternId, Diagnostic> {
        let inclusive = self.take("..=");
        if !inclusive {
            self.expect("..")?;
        }
        let end = if self.expression_end() || self.is("|") || self.is("if") || self.is("=") {
            None
        } else {
            Some(self.pattern_bound()?)
        };
        if inclusive && end.is_none() {
            return Err(self.err("inclusive range pattern needs an end"));
        }
        if start.is_none() && end.is_none() {
            return Err(self.unsupported("full range pattern; use _"));
        }
        if !self.expression_end() && !self.is("|") && !self.is("if") && !self.is("=") {
            return Err(self.unsupported("range endpoint expressions"));
        }
        Ok(self.add_pattern(
            Pattern::Range {
                start,
                end,
                inclusive,
            },
            self.finish_span(span),
        ))
    }
    fn tuple_pattern_field(&mut self) -> Result<PatternId, Diagnostic> {
        if self.is("..")
            && self
                .tokens
                .get(self.at + 1)
                .is_some_and(|token| token.text == "," || token.text == ")")
        {
            let span = self.token().span;
            self.at += 1;
            return Ok(self.add_pattern(Pattern::Rest, span));
        }
        self.pattern()
    }
    fn pattern_atom(&mut self) -> Result<PatternId, Diagnostic> {
        let span = self.token().span;
        if self.take("_") {
            return Ok(self.add_pattern(Pattern::Wild, span));
        }
        if self.is("..") || self.is("..=") {
            return self.pattern_range(None, span);
        }
        if self.take("&") {
            let mutable = self.take("mut");
            let inner = self.pattern_atom()?;
            return Ok(self.add_pattern(Pattern::Ref { mutable, inner }, self.finish_span(span)));
        }
        if self.take("(") {
            let mut fields = Vec::new();
            let mut comma = false;
            while !self.is(")") {
                fields.push(self.tuple_pattern_field()?);
                if !self.take(",") {
                    break;
                }
                comma = true;
            }
            self.expect(")")?;
            if fields.len() == 1
                && !comma
                && !matches!(self.program.patterns[fields[0]].kind, Pattern::Rest)
            {
                return Ok(fields[0]);
            }
            return Ok(self.add_pattern(Pattern::Tuple(fields), self.finish_span(span)));
        }
        if self.is("-")
            || self.token().kind == TokenKind::Number
            || self.token().kind == TokenKind::String
            || self.token().kind == TokenKind::Char
            || self.is("true")
            || self.is("false")
        {
            let value = self.pattern_bound()?;
            if self.is("..") || self.is("..=") {
                return self.pattern_range(Some(value), span);
            }
            return Ok(self.add_pattern(Pattern::Literal(value), self.finish_span(span)));
        }
        let by_ref = self.take("ref");
        let mutable = self.take("mut");
        let path = self.value_path()?;
        if self.is("..") || self.is("..=") {
            if by_ref || mutable {
                return Err(self.unsupported("binding range endpoint"));
            }
            let value = self.add_expr(Expr::Path(path), self.finish_span(span));
            return self.pattern_range(Some(value), span);
        }
        if self.take("(") {
            let mut fields = Vec::new();
            while !self.is(")") {
                fields.push(self.tuple_pattern_field()?);
                if !self.take(",") {
                    break;
                }
            }
            self.expect(")")?;
            return Ok(self.add_pattern(Pattern::Variant { path, fields }, self.finish_span(span)));
        }
        if self.take("{") {
            let mut fields = Vec::new();
            let mut rest = false;
            while !self.is("}") {
                if self.take("..") {
                    rest = true;
                    self.take(",");
                    break;
                }
                let start = self.token().span;
                let ref_field = self.take("ref");
                let mut_field = self.take("mut");
                let name = self.ident()?;
                let value = if self.take(":") {
                    self.pattern()?
                } else {
                    self.add_pattern(
                        Pattern::Bind {
                            name: name.clone(),
                            mutable: mut_field && !ref_field,
                            by_ref: ref_field,
                            ref_mut: mut_field && ref_field,
                        },
                        start,
                    )
                };
                fields.push((name, value));
                if !self.take(",") {
                    break;
                }
            }
            self.expect("}")?;
            return Ok(self.add_pattern(
                Pattern::Struct { path, fields, rest },
                self.finish_span(span),
            ));
        }
        if self.is("@") {
            return Err(self.unsupported("binding pattern with @"));
        }
        let is_path = path.name.contains("::") || path.name.as_bytes()[0].is_ascii_uppercase();
        let kind = if is_path {
            Pattern::Path(path)
        } else {
            Pattern::Bind {
                name: path.name,
                mutable: mutable && !by_ref,
                by_ref,
                ref_mut: mutable && by_ref,
            }
        };
        Ok(self.add_pattern(kind, self.finish_span(span)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn precedence_and_arena() {
        let p = parse("fn answer() -> i64 { 1 + 2 * 3 } fn main() { assert_eq!(answer(), 7); }")
            .unwrap();
        assert_eq!(p.functions.len(), 2);
        let body = p.functions[0].body.unwrap();
        let tail = p.blocks[body].tail.unwrap();
        match &p.exprs[tail].kind {
            Expr::Binary { op, right, .. } => {
                assert_eq!(op, "+");
                match &p.exprs[*right].kind {
                    Expr::Binary { op, .. } => assert_eq!(op, "*"),
                    _ => panic!("multiplication missing"),
                }
            }
            _ => panic!("addition missing"),
        }
    }
    #[test]
    fn numeric_suffixes_preserve_operators_and_decimal_exponents() {
        for literal in ["6usize", "6isize", "0x1e", "0x1eu8", "1_000u64"] {
            let source = format!("fn f(){{{literal}}}");
            let program = parse(&source).unwrap();
            assert!(matches!(program.exprs[0].kind, Expr::Int(_)), "{literal}");
        }
        for expression in ["6usize+2usize", "6isize-2isize", "0x1e+2", "0x1E-2"] {
            let source = format!("fn f(){{{expression}}}");
            let program = parse(&source).unwrap();
            let tail = program.blocks[program.functions[0].body.unwrap()]
                .tail
                .unwrap();
            assert!(
                matches!(program.exprs[tail].kind, Expr::Binary { .. }),
                "{expression}"
            );
        }
        for literal in ["1e+2", "1E-2", "1.25e+2f64", "1_0e-2"] {
            let source = format!("fn f(){{{literal}}}");
            let program = parse(&source).unwrap();
            assert!(matches!(program.exprs[0].kind, Expr::Float(_)), "{literal}");
        }
    }
    #[test]
    fn declarations_and_borrows() {
        let source = "use std::collections::BTreeMap; #[derive(Clone, Copy)] struct Pair<T> { a: T, b: T } enum Maybe<T> { No, Yes(T) } trait Get { type Item; fn get(&self) -> Self::Item; } impl<T: Copy> Get for Pair<T> { type Item = T; fn get(&self) -> T { self.a } } fn view<'a>(x: &'a [u8]) -> &'a [u8] { x }";
        let p = parse(source).unwrap();
        assert_eq!(p.structs.len(), 1);
        assert_eq!(p.enums.len(), 1);
        assert_eq!(p.traits.len(), 1);
        assert_eq!(p.impls.len(), 1);
        assert_eq!(p.functions.len(), 3);
    }
    #[test]
    fn control_and_patterns() {
        let p = parse("fn f(mut x: i64) -> i64 { let mut v: Vec<i64> = vec![1, 2]; for i in 0..3 { v.push(i); } while x > 0 { x -= 1; } match Some(x) { Some(v) if v == 0 => v, _ => 1, } }").unwrap();
        assert_eq!(p.functions.len(), 1);
        assert!(!p.patterns.is_empty());
    }
    #[test]
    fn nested_generics_and_parentheses() {
        let p = parse("fn f(x: Vec<Vec<i64>>) -> i64 { (((1) + 2) * 3) }").unwrap();
        assert_eq!(p.functions.len(), 1);
    }
    #[test]
    fn deep_grouping_is_iterative() {
        let mut source = String::from("fn f() -> i64 { ");
        let mut i = 0;
        while i < 20_000 {
            source.push('(');
            i += 1;
        }
        source.push('1');
        i = 0;
        while i < 20_000 {
            source.push(')');
            i += 1;
        }
        source.push_str(" }");
        let p = parse(&source).unwrap();
        assert_eq!(p.exprs.len(), 1);
    }
    #[test]
    fn diagnostics_name_rejected_constructs() {
        let bytes = parse("fn f() { let b = b\"\\xFF\\0\"; let c = b'\\x80'; }").unwrap();
        assert!(
            bytes
                .exprs
                .iter()
                .any(|e| matches!(&e.kind, Expr::Bytes(v) if *v == vec![255, 0]))
        );
        assert_eq!(
            parse("fn f() { let b = b\"é\"; }").unwrap_err().code,
            "E_LEX"
        );
        parse("fn f() { let strings = [\"&&\", \"||\", \"}\", \"!\", \"if\"]; }").unwrap();
        assert_eq!(
            parse("fn f() { println!(\"ambient\"); }").unwrap_err().code,
            "E_CAPABILITY"
        );
        assert_eq!(
            parse("fn f() { unsafe {} }").unwrap_err().code,
            "E_UNSUPPORTED"
        );
        assert_eq!(
            parse("fn f() { mystery!(); }").unwrap_err().code,
            "E_UNSUPPORTED_MACRO"
        );
        assert_eq!(parse("fn f() { /* missing }").unwrap_err().code, "E_LEX");
    }
    #[test]
    fn views_and_closure_syntax() {
        parse("fn f() { let mut x = 1; let r = &&mut x; }").unwrap();
        let p = parse("struct View<'a> { bytes: &'a [u8] } fn f(v: &mut Vec<i64>) { v.push(v.len() as i64); let cb = move |x: i64| -> i64 { x + 1 }; let y = cb(2); }").unwrap();
        assert_eq!(p.structs.len(), 1);
        assert_eq!(p.functions.len(), 1);
    }
}
