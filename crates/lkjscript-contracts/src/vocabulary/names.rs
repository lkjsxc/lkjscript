pub const SIMPLE_TYPE_NAMES: &[&str] = &[
    "never",
    "unit",
    "bool",
    "i64",
    "f64",
    "string",
    "path",
    "symbol",
    "input-stream",
    "output-stream",
    "file-reader",
    "file-writer",
    "file-appender",
    "directory",
    "tcp-listener",
    "tcp-stream",
    "sqlite-connection",
    "sqlite-statement",
    "terminal-session",
];

pub const BYTE_TEXT_FOUNDATION_TYPE_NAMES: &[&str] = &[
    "bytes",
    "byte-vector",
    "byte-slice",
    "byte-slice-mut",
    "str",
];

pub const TYPE_CONSTRUCTOR_NAMES: &[&str] = &["capability", "product", "list", "option", "result"];

pub const BUILTIN_ERROR_NAMES: &[&str] = &["numeric-error", "utf8-error", "system-error"];
pub const COMPILER_TRAIT_NAMES: &[&str] = &["copy", "clone", "drop", "send", "sync"];
pub const PRELUDE_TYPE_NAMES: &[&str] = &[
    "option",
    "result",
    "numeric-error",
    "utf8-error",
    "system-error",
];
pub const PRELUDE_VARIANT_NAMES: &[&str] = &[
    "none",
    "some",
    "ok",
    "err",
    "non-finite",
    "out-of-range",
    "fractional",
    "inexact",
    "unexpected-continuation",
    "invalid-leading-byte",
    "missing-continuation",
    "overlong-encoding",
    "surrogate",
    "io",
    "network",
    "terminal",
    "time",
    "random",
    "sqlite",
    "utf8",
    "unsupported",
];

pub const CONTEXTUAL_FORM_NAMES: &[&str] = &[
    "def",
    "main",
    "imports",
    "import",
    "module",
    "declarations",
    "product",
    "enum",
    "trait",
    "impl",
    "name",
    "fn",
    "sig",
    "inputs",
    "output",
    "params",
    "forall",
    "bounds",
    "bound",
    "type",
    "fields",
    "for",
    "field",
    "variant",
    "variants",
    "variant-field",
    "bind",
    "let",
    "var",
    "set",
    "if",
    "while",
    "loop",
    "return",
    "break",
    "continue",
    "trap",
    "exit",
    "do",
    "quote",
    "string-literal",
    "bytes-literal",
    "move",
    "borrow",
    "borrow-mut",
    "product-value",
    "with-field",
    "empty-list",
    "none",
    "match",
    "cases",
    "case",
    "pattern",
    "wildcard",
    "variant-pattern",
    "bind-pattern",
    "hole",
    "goal",
    "public",
];

pub const RESERVED_WORDS: &[&str] = &[
    "true",
    "false",
    "unit",
    "never",
    "bool",
    "i64",
    "f64",
    "string",
    "path",
    "symbol",
    "input-stream",
    "output-stream",
    "file-reader",
    "file-writer",
    "file-appender",
    "directory",
    "tcp-listener",
    "tcp-stream",
    "sqlite-connection",
    "sqlite-statement",
    "terminal-session",
    "bytes",
    "byte-vector",
    "byte-slice",
    "byte-slice-mut",
    "str",
    "capability",
    "product",
    "list",
    "option",
    "result",
];

pub fn is_identifier(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_lowercase() {
        return false;
    }
    let mut previous_hyphen = false;
    for byte in bytes.iter().copied() {
        if byte == b'-' {
            if previous_hyphen {
                return false;
            }
            previous_hyphen = true;
        } else if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
            previous_hyphen = false;
        } else {
            return false;
        }
    }
    !previous_hyphen
}
