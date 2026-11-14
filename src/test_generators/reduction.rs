// Balanced deletion candidates preserve literals, comments and macro token trees.
#[derive(Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}
struct Token {
    kind: u8,
    text: String,
    start: usize,
    end: usize,
}
struct Group {
    start: usize,
    end: usize,
    opaque: bool,
}
fn starts(bytes: &[u8], at: usize, needle: &[u8]) -> bool {
    at <= bytes.len() && needle.len() <= bytes.len() - at && &bytes[at..at + needle.len()] == needle
}
fn word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}
fn tokenize(source: &str) -> Vec<Token> {
    let bytes = source.as_bytes();
    let mut at = 0usize;
    let mut tokens = Vec::new();
    while at < bytes.len() {
        if bytes[at].is_ascii_whitespace() {
            at += 1;
            continue;
        }
        if starts(bytes, at, b"//") {
            while at < bytes.len() && bytes[at] != b'\n' {
                at += 1;
            }
            continue;
        }
        if starts(bytes, at, b"/*") {
            at += 2;
            let mut depth = 1usize;
            while at < bytes.len() && depth > 0 {
                if starts(bytes, at, b"/*") {
                    depth += 1;
                    at += 2;
                } else if starts(bytes, at, b"*/") {
                    depth -= 1;
                    at += 2;
                } else {
                    at += 1;
                }
            }
            continue;
        }
        let begin = at;
        let mut raw = at;
        if starts(bytes, raw, b"br") || starts(bytes, raw, b"cr") {
            raw += 1;
        }
        if raw < bytes.len() && bytes[raw] == b'r' {
            raw += 1;
            let hashes = raw;
            while raw < bytes.len() && bytes[raw] == b'#' {
                raw += 1;
            }
            if raw < bytes.len() && bytes[raw] == b'"' {
                let count = raw - hashes;
                at = raw + 1;
                while at < bytes.len() {
                    if bytes[at] == b'"' {
                        let mut end = at + 1;
                        while end < bytes.len() && end - at - 1 < count && bytes[end] == b'#' {
                            end += 1;
                        }
                        if end - at - 1 == count {
                            at = end;
                            break;
                        }
                    }
                    at += 1;
                }
                tokens.push(Token {
                    kind: 1,
                    text: source[begin..at].to_string(),
                    start: begin,
                    end: at,
                });
                continue;
            }
        }
        if (bytes[at] == b'b' || bytes[at] == b'c')
            && at + 1 < bytes.len()
            && (bytes[at + 1] == b'"' || bytes[at + 1] == b'\'')
        {
            at += 1;
        }
        let quote = bytes[at];
        if quote == b'"' || quote == b'\'' {
            if quote == b'\''
                && at + 1 < bytes.len()
                && (bytes[at + 1].is_ascii_alphabetic() || bytes[at + 1] == b'_')
            {
                let mut end = at + 2;
                while end < bytes.len() && word(bytes[end]) {
                    end += 1;
                }
                if end == bytes.len() || bytes[end] != b'\'' {
                    at = end;
                    tokens.push(Token {
                        kind: 2,
                        text: source[begin..at].to_string(),
                        start: begin,
                        end: at,
                    });
                    continue;
                }
            }
            at += 1;
            while at < bytes.len() {
                if bytes[at] == b'\\' {
                    at = (at + 2).min(bytes.len());
                } else if bytes[at] == quote {
                    at += 1;
                    break;
                } else {
                    at += 1;
                }
            }
            tokens.push(Token {
                kind: 1,
                text: source[begin..at].to_string(),
                start: begin,
                end: at,
            });
            continue;
        }
        if word(bytes[at]) {
            while at < bytes.len() && word(bytes[at]) {
                at += 1;
            }
            tokens.push(Token {
                kind: 3,
                text: source[begin..at].to_string(),
                start: begin,
                end: at,
            });
            continue;
        }
        let mut width = 1usize;
        for pair in vec![
            "::".to_string(),
            "->".to_string(),
            "=>".to_string(),
            "&&".to_string(),
            "||".to_string(),
            "==".to_string(),
            "!=".to_string(),
            "<=".to_string(),
            ">=".to_string(),
            "..".to_string(),
        ] {
            if starts(bytes, at, pair.as_bytes()) {
                width = 2;
                break;
            }
        }
        at += width;
        while at < bytes.len() && (bytes[at] & 192) == 128 {
            at += 1;
        }
        tokens.push(Token {
            kind: 0,
            text: source[begin..at].to_string(),
            start: begin,
            end: at,
        });
    }
    tokens
}
fn closing(text: &str) -> &'static str {
    match text {
        "(" => ")",
        "[" => "]",
        "{" => "}",
        _ => "",
    }
}
fn continuations(text: &str) -> bool {
    matches!(
        text,
        "else"
            | "{"
            | "."
            | "?"
            | "("
            | "["
            | "+"
            | "-"
            | "*"
            | "/"
            | "%"
            | "&"
            | "|"
            | "^"
            | "&&"
            | "||"
            | "=="
            | "!="
            | "<"
            | ">"
            | "<="
            | ">="
            | "="
    )
}
fn collect(
    tokens: &[Token],
    pairs: &[usize],
    first: usize,
    last: usize,
    top: bool,
    spans: &mut Vec<Span>,
) {
    let mut start = first;
    let mut at = first;
    while at < last {
        let token = &tokens[at];
        if token.kind != 0 {
            at += 1;
            continue;
        }
        if token.text == ";" {
            if start <= at {
                spans.push(Span {
                    start: tokens[start].start,
                    end: token.end,
                });
            }
            start = at + 1;
            at += 1;
            continue;
        }
        let end = pairs[at];
        if end == (!0usize) {
            at += 1;
            continue;
        }
        let mut lead = start;
        while lead + 1 < tokens.len()
            && tokens[lead].text == "#"
            && tokens[lead + 1].text == "["
            && pairs[lead + 1] != (!0usize)
        {
            lead = pairs[lead + 1] + 1;
        }
        if lead + 1 < tokens.len() && tokens[lead].kind == 2 && tokens[lead + 1].text == ":" {
            lead += 2;
        }
        let leading = if lead < tokens.len() {
            tokens[lead].text.as_str()
        } else {
            ""
        };
        let standalone = matches!(
            leading,
            "{" | "if" | "while" | "loop" | "for" | "match" | "unsafe" | "async"
        );
        let item = matches!(
            leading,
            "fn" | "pub" | "struct" | "enum" | "impl" | "trait" | "mod"
        );
        let next = if end + 1 < tokens.len() {
            tokens[end + 1].text.as_str()
        } else {
            ""
        };
        if token.text == "{" && (top || standalone || item) && !continuations(next) {
            let until = if next == ";" { end + 2 } else { end + 1 };
            if start < until {
                spans.push(Span {
                    start: tokens[start].start,
                    end: tokens[until - 1].end,
                });
            }
            start = until;
            at = until;
        } else {
            at = end + 1;
        }
    }
}
pub fn statement_spans(source: &str) -> Vec<Span> {
    let tokens = tokenize(source);
    let mut pairs = vec![(!0usize); tokens.len()];
    let mut groups: Vec<Group> = Vec::new();
    let mut stack: Vec<Group> = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.kind != 0 {
            continue;
        }
        if !closing(&token.text).is_empty() {
            let inherited = stack.last().map(|group| group.opaque).unwrap_or(false);
            let macro_call =
                index >= 2 && tokens[index - 1].text == "!" && tokens[index - 2].kind == 3;
            let macro_definition = index >= 3
                && tokens[index - 1].kind == 3
                && tokens[index - 2].text == "!"
                && tokens[index - 3].text == "macro_rules";
            let attribute = index >= 1 && tokens[index - 1].text == "#";
            stack.push(Group {
                start: index,
                end: 0,
                opaque: inherited || macro_call || macro_definition || attribute,
            });
        } else if matches!(token.text.as_str(), ")" | "]" | "}") {
            let matched = {
                match stack.last() {
                    Some(group) => closing(&tokens[group.start].text) == token.text,
                    None => false,
                }
            };
            if matched {
                let mut group = stack.pop().unwrap();
                group.end = index;
                pairs[group.start] = index;
                groups.push(group);
            }
        }
    }
    let mut spans = Vec::new();
    collect(&tokens, &pairs, 0, tokens.len(), true, &mut spans);
    for group in &groups {
        if tokens[group.start].text == "{" && !group.opaque {
            collect(
                &tokens,
                &pairs,
                group.start + 1,
                group.end,
                false,
                &mut spans,
            );
        }
    }
    let mut index = 0usize;
    while index < spans.len() {
        let mut largest = index;
        let mut next = index + 1;
        while next < spans.len() {
            if spans[next].end - spans[next].start > spans[largest].end - spans[largest].start
                || (spans[next].end - spans[next].start
                    == spans[largest].end - spans[largest].start
                    && spans[next].start < spans[largest].start)
            {
                largest = next;
            }
            next += 1;
        }
        let saved = spans[index].clone();
        spans[index] = spans[largest].clone();
        spans[largest] = saved;
        index += 1;
    }
    let mut unique: Vec<Span> = Vec::new();
    for span in spans {
        let same = !unique.is_empty() && unique[unique.len() - 1] == span;
        if !same {
            unique.push(span);
        }
    }
    let spans = unique;
    spans
}
pub fn ir_spans(source: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut begin = 0usize;
    for (at, byte) in source.as_bytes().iter().enumerate() {
        if *byte == b'\n' {
            if super::trim(&source[begin..at]).starts_with('@') {
                spans.push(Span {
                    start: begin,
                    end: at + 1,
                });
            }
            begin = at + 1;
        }
    }
    if begin < source.len() && super::trim(&source[begin..]).starts_with('@') {
        spans.push(Span {
            start: begin,
            end: source.len(),
        });
    }
    spans
}
pub fn self_test() -> Result<(), String> {
    let source = "fn probe<'a>(text:&'a str)->i64{let raw=r##\"{ fake(); }\"##;let quoted=\"; }\\\"\";/* outer /* fake(); */ comment */let character='}';if true{let inner=1;sink(inner);}else{sink(2);}vec![{let macro_tokens=3;macro_tokens};2];42}";
    let spans = statement_spans(source);
    let mut pieces: Vec<String> = Vec::new();
    for span in &spans {
        pieces.push(source[span.start..span.end].to_string());
    }
    for needed in vec![
        "let raw=r##\"{ fake(); }\"##;".to_string(),
        "let inner=1;".to_string(),
        "sink(inner);".to_string(),
        "vec![{let macro_tokens=3;macro_tokens};2];".to_string(),
    ] {
        if !pieces.contains(&needed) {
            return Err(format!("E_REDUCER missing balanced span {needed}"));
        }
    }
    for excluded in vec!["fake();".to_string(), "let macro_tokens=3;".to_string()] {
        if pieces.contains(&excluded) {
            return Err(format!("E_REDUCER entered an opaque token tree {excluded}"));
        }
    }
    Ok(())
}
