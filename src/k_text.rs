//! Text K is a flat instruction listing for inspection and adversarial checks.
use crate::ast::{Diagnostic, Span};
use crate::k::*;

type Result<T> = std::result::Result<T, Diagnostic>;

fn error(line: usize, message: &str) -> Diagnostic {
    Diagnostic::new(
        Span {
            start: 0,
            end: 0,
            line,
            column: 1,
        },
        "E_K_TEXT",
        message,
    )
}

fn hex(chars: &[char], at: &mut usize, line: usize) -> Result<u32> {
    let mut value = 0;
    for _ in 0..4 {
        let digit = chars
            .get(*at)
            .and_then(|c| c.to_digit(16))
            .ok_or_else(|| error(line, "expected four hexadecimal digits after \\u"))?;
        value = value * 16 + digit;
        *at += 1;
    }
    Ok(value)
}

fn words(text: &str, line: usize) -> Result<Vec<String>> {
    let chars: Vec<char> = text.chars().collect();
    let mut result = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        if chars[at].is_whitespace() {
            at += 1;
            continue;
        }
        if chars[at] == '#' {
            break;
        }
        let mut word = String::new();
        if chars[at] == '"' {
            at += 1;
            let mut closed = false;
            while at < chars.len() {
                let c = chars[at];
                at += 1;
                if c == '"' {
                    closed = true;
                    break;
                }
                if c != '\\' {
                    if c.is_control() {
                        return Err(error(line, "control character in quoted string"));
                    }
                    word.push(c);
                    continue;
                }
                let escaped = *chars
                    .get(at)
                    .ok_or_else(|| error(line, "unfinished string escape"))?;
                at += 1;
                match escaped {
                    '"' | '\\' | '/' => word.push(escaped),
                    'n' => word.push('\n'),
                    'r' => word.push('\r'),
                    't' => word.push('\t'),
                    'b' => word.push('\u{0008}'),
                    'f' => word.push('\u{000c}'),
                    'u' => {
                        let mut value = hex(&chars, &mut at, line)?;
                        if (0xd800..=0xdbff).contains(&value) {
                            if chars.get(at) != Some(&'\\') || chars.get(at + 1) != Some(&'u') {
                                return Err(error(line, "high surrogate needs a low surrogate"));
                            }
                            at += 2;
                            let low = hex(&chars, &mut at, line)?;
                            if !(0xdc00..=0xdfff).contains(&low) {
                                return Err(error(line, "invalid low surrogate"));
                            }
                            value = 0x10000 + (value - 0xd800) * 1024 + low - 0xdc00;
                        }
                        word.push(
                            char::from_u32(value)
                                .ok_or_else(|| error(line, "invalid Unicode scalar"))?,
                        );
                    }
                    _ => return Err(error(line, "unknown string escape")),
                }
            }
            if !closed {
                return Err(error(line, "unterminated quoted string"));
            }
            if at < chars.len() && !chars[at].is_whitespace() && chars[at] != '#' {
                return Err(error(line, "quoted word needs a separator"));
            }
        } else {
            while at < chars.len() && !chars[at].is_whitespace() && chars[at] != '#' {
                if chars[at] == '"' {
                    return Err(error(line, "quote inside an unquoted word"));
                }
                word.push(chars[at]);
                at += 1;
            }
        }
        result.push(word);
    }
    Ok(result)
}

fn word<'a>(words: &'a [String], at: usize, line: usize) -> Result<&'a str> {
    words
        .get(at)
        .map(String::as_str)
        .ok_or_else(|| error(line, "missing operand"))
}

fn number(words: &[String], at: usize, line: usize) -> Result<usize> {
    word(words, at, line)?
        .parse::<usize>()
        .map_err(|_| error(line, "invalid index or extent"))
}

fn exact(words: &[String], size: usize, line: usize) -> Result<()> {
    if words.len() != size {
        return Err(error(line, "wrong number of operands"));
    }
    Ok(())
}

fn flag(words: &[String], at: usize, yes: &str, no: &str, line: usize) -> Result<bool> {
    let value = word(words, at, line)?;
    if value == yes {
        Ok(true)
    } else if value == no {
        Ok(false)
    } else {
        Err(error(line, "invalid flag"))
    }
}

fn fields(words: &[String], start: usize, line: usize) -> Result<Vec<Field>> {
    if (words.len() - start) % 2 != 0 {
        return Err(error(line, "field needs a name and type index"));
    }
    let mut result = Vec::new();
    let mut at = start;
    while at < words.len() {
        result.push(Field {
            name: words[at].clone(),
            ty: number(words, at + 1, line)?,
        });
        at += 2;
    }
    Ok(result)
}

fn indices(words: &[String], start: usize, line: usize) -> Result<Vec<usize>> {
    let mut result = Vec::new();
    for at in start..words.len() {
        result.push(number(words, at, line)?);
    }
    Ok(result)
}

fn place(words: &[String], start: usize, line: usize) -> Result<Place> {
    let mut result = Place::local(number(words, start, line)?);
    let mut at = start + 1;
    while at < words.len() {
        let projection = match words[at].as_str() {
            "deref" => {
                at += 1;
                Projection::Deref
            }
            "field" => {
                let index = number(words, at + 1, line)?;
                at += 2;
                Projection::Field(index)
            }
            "index" => {
                let index = number(words, at + 1, line)?;
                at += 2;
                Projection::Index(index)
            }
            "variant" => {
                let tag = number(words, at + 1, line)?;
                let index = number(words, at + 2, line)?;
                at += 3;
                Projection::VariantField(tag, index)
            }
            _ => return Err(error(line, "unknown place projection")),
        };
        result.projections.push(projection);
    }
    Ok(result)
}

fn instruction(words: &[String], line: usize) -> Result<Instruction> {
    let source_line = word(words, 0, line)?
        .strip_prefix('@')
        .ok_or_else(|| error(line, "instruction needs @source-line"))?
        .parse::<usize>()
        .map_err(|_| error(line, "invalid source line"))?;
    let op = match word(words, 1, line)? {
        "int" => {
            exact(words, 4, line)?;
            Op::Int {
                dst: number(words, 2, line)?,
                value: word(words, 3, line)?
                    .parse::<i64>()
                    .map_err(|_| error(line, "invalid signed integer"))?,
            }
        }
        "float" => {
            exact(words, 4, line)?;
            Op::Float {
                dst: number(words, 2, line)?,
                bits: word(words, 3, line)?
                    .parse::<u64>()
                    .map_err(|_| error(line, "invalid float bit pattern"))?,
            }
        }
        "unit" => {
            exact(words, 3, line)?;
            Op::Unit {
                dst: number(words, 2, line)?,
            }
        }
        "read" => Op::Read {
            dst: number(words, 2, line)?,
            moving: flag(words, 3, "move", "copy", line)?,
            place: place(words, 4, line)?,
        },
        "array" => Op::Array {
            dst: number(words, 2, line)?,
            elements: indices(words, 3, line)?,
        },
        "slice-borrow" => Op::SliceBorrow {
            dst: number(words, 2, line)?,
            mutable: flag(words, 3, "mut", "shared", line)?,
            place: place(words, 4, line)?,
        },
        "borrow" => Op::Borrow {
            dst: number(words, 2, line)?,
            mutable: flag(words, 3, "mut", "shared", line)?,
            place: place(words, 4, line)?,
        },
        "assign" => Op::Assign {
            src: number(words, 2, line)?,
            place: place(words, 3, line)?,
        },
        "drop" => Op::Drop {
            place: place(words, 2, line)?,
        },
        "unary" => {
            exact(words, 5, line)?;
            Op::Unary {
                dst: number(words, 2, line)?,
                op: word(words, 3, line)?.to_string(),
                src: number(words, 4, line)?,
            }
        }
        "binary" => {
            exact(words, 6, line)?;
            Op::Binary {
                dst: number(words, 2, line)?,
                op: word(words, 3, line)?.to_string(),
                left: number(words, 4, line)?,
                right: number(words, 5, line)?,
            }
        }
        "call" => Op::Call {
            dst: number(words, 2, line)?,
            function: number(words, 3, line)?,
            args: indices(words, 4, line)?,
        },
        "struct" => Op::Struct {
            dst: number(words, 2, line)?,
            fields: indices(words, 3, line)?,
        },
        "variant" => Op::Variant {
            dst: number(words, 2, line)?,
            variant: number(words, 3, line)?,
            fields: indices(words, 4, line)?,
        },
        "tag" => {
            exact(words, 4, line)?;
            Op::Discriminant {
                dst: number(words, 2, line)?,
                src: number(words, 3, line)?,
            }
        }
        "tag-test" => {
            exact(words, 5, line)?;
            Op::TagTest {
                dst: number(words, 2, line)?,
                src: number(words, 3, line)?,
                variant: number(words, 4, line)?,
            }
        }
        "intrinsic" => Op::Intrinsic {
            dst: number(words, 2, line)?,
            name: word(words, 3, line)?.to_string(),
            args: indices(words, 4, line)?,
        },
        "if" => {
            exact(words, 3, line)?;
            Op::If {
                condition: number(words, 2, line)?,
            }
        }
        "return" => {
            exact(words, 3, line)?;
            Op::Return {
                src: number(words, 2, line)?,
            }
        }
        "trap" => {
            exact(words, 3, line)?;
            Op::Trap {
                message: word(words, 2, line)?.to_string(),
            }
        }
        name => {
            exact(words, 2, line)?;
            match name {
                "else" => Op::Else,
                "end" => Op::End,
                "loop" => Op::Loop,
                "break" => Op::Break,
                "continue" => Op::Continue,
                _ => return Err(error(line, "unknown instruction")),
            }
        }
    };
    Ok(Instruction {
        op,
        line: source_line,
    })
}

/// Parse text into unchecked K. Only check::check may grant backend access.
pub fn read(text: &str) -> Result<Program> {
    let mut program = Program {
        destructors: Vec::new(),
        types: Vec::new(),
        functions: Vec::new(),
    };
    let mut current: Option<Function> = None;
    let mut enumeration: Option<Type> = None;
    let mut last_line = 1;
    for (offset, line_text) in text.lines().enumerate() {
        let line = offset + 1;
        last_line = line;
        let words = words(line_text, line)?;
        if words.is_empty() {
            continue;
        }
        if let Some(ty) = &mut enumeration {
            match words[0].as_str() {
                "variant" => {
                    let variant = Variant {
                        name: word(&words, 1, line)?.to_string(),
                        fields: fields(&words, 2, line)?,
                    };
                    if let TypeKind::Enum { variants, .. } = &mut ty.kind {
                        variants.push(variant);
                    }
                }
                "endtype" => {
                    exact(&words, 1, line)?;
                    program.types.push(enumeration.take().unwrap());
                }
                _ => return Err(error(line, "expected variant or endtype")),
            }
            continue;
        }
        if let Some(function) = &mut current {
            match words[0].as_str() {
                "local" => {
                    exact(&words, 4, line)?;
                    function.locals.push(Local {
                        name: words[1].clone(),
                        ty: number(&words, 2, line)?,
                        mutable: flag(&words, 3, "mut", "let", line)?,
                    });
                }
                "endfn" => {
                    exact(&words, 1, line)?;
                    program.functions.push(current.take().unwrap());
                }
                _ => function.body.push(instruction(&words, line)?),
            }
            continue;
        }
        match words[0].as_str() {
            "type" => {
                let name = word(&words, 1, line)?.to_string();
                let kind = match word(&words, 2, line)? {
                    "unit" => {
                        exact(&words, 3, line)?;
                        TypeKind::Unit
                    }
                    "bool" => {
                        exact(&words, 3, line)?;
                        TypeKind::Bool
                    }
                    "f64" => {
                        exact(&words, 3, line)?;
                        TypeKind::F64
                    }
                    "string" => {
                        exact(&words, 3, line)?;
                        TypeKind::String
                    }
                    "world" => {
                        exact(&words, 3, line)?;
                        TypeKind::World
                    }
                    "int" => {
                        exact(&words, 5, line)?;
                        let bits = word(&words, 3, line)?
                            .parse::<u8>()
                            .map_err(|_| error(line, "invalid integer bit width"))?;
                        TypeKind::Int {
                            bits,
                            signed: flag(&words, 4, "signed", "unsigned", line)?,
                        }
                    }
                    "ref" | "slice" => {
                        exact(&words, 5, line)?;
                        let mutable = flag(&words, 3, "mut", "shared", line)?;
                        let inner = number(&words, 4, line)?;
                        if words[2] == "ref" {
                            TypeKind::Ref { inner, mutable }
                        } else {
                            TypeKind::Slice { inner, mutable }
                        }
                    }
                    "array" => {
                        exact(&words, 5, line)?;
                        TypeKind::Array {
                            inner: number(&words, 3, line)?,
                            len: number(&words, 4, line)?,
                        }
                    }
                    "vec" | "box" => {
                        exact(&words, 4, line)?;
                        let inner = number(&words, 3, line)?;
                        if words[2] == "vec" {
                            TypeKind::Vec { inner }
                        } else {
                            TypeKind::Box { inner }
                        }
                    }
                    "struct" => TypeKind::Struct {
                        copy: flag(&words, 3, "copy", "move", line)?,
                        fields: fields(&words, 4, line)?,
                    },
                    "enum" => {
                        exact(&words, 4, line)?;
                        TypeKind::Enum {
                            copy: flag(&words, 3, "copy", "move", line)?,
                            variants: Vec::new(),
                        }
                    }
                    _ => return Err(error(line, "unknown type shape")),
                };
                if matches!(kind, TypeKind::Enum { .. }) {
                    enumeration = Some(Type { name, kind });
                } else {
                    program.types.push(Type { name, kind });
                }
            }
            "destructor" => {
                exact(&words, 3, line)?;
                program
                    .destructors
                    .push((number(&words, 1, line)?, number(&words, 2, line)?));
            }
            "fn" => {
                if words.len() < 4 {
                    return Err(error(
                        line,
                        "function needs name, parameter count, and result type",
                    ));
                }
                let mut test = false;
                let mut return_source = None;
                let mut at = 4;
                while at < words.len() {
                    match words[at].as_str() {
                        "test" if !test => {
                            test = true;
                            at += 1;
                        }
                        "from" if return_source.is_none() => {
                            return_source = Some(number(&words, at + 1, line)?);
                            at += 2;
                        }
                        _ => return Err(error(line, "unknown or repeated function flag")),
                    }
                }
                current = Some(Function {
                    name: words[1].clone(),
                    params: number(&words, 2, line)?,
                    result: number(&words, 3, line)?,
                    return_source,
                    locals: Vec::new(),
                    body: Vec::new(),
                    test,
                });
            }
            _ => return Err(error(line, "expected type or function")),
        }
    }
    if enumeration.is_some() {
        return Err(error(last_line, "missing endtype"));
    }
    if current.is_some() {
        return Err(error(last_line, "missing endfn"));
    }
    Ok(program)
}

fn quoted(text: &str) -> String {
    let mut result = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => result.push_str(&format!("\\u{:04x}", c as u32)),
            _ => result.push(c),
        }
    }
    result.push('"');
    result
}

fn write_fields(text: &mut String, fields: &[Field]) {
    for field in fields {
        text.push_str(&format!(" {} {}", quoted(&field.name), field.ty));
    }
}

fn write_indices(text: &mut String, indices: &[usize]) {
    for index in indices {
        text.push_str(&format!(" {index}"));
    }
}

fn write_place(text: &mut String, place: &Place) {
    text.push_str(&format!(" {}", place.local));
    for projection in &place.projections {
        match projection {
            Projection::Deref => text.push_str(" deref"),
            Projection::Field(field) => text.push_str(&format!(" field {field}")),
            Projection::VariantField(variant, field) => {
                text.push_str(&format!(" variant {variant} {field}"))
            }
            Projection::Index(index) => text.push_str(&format!(" index {index}")),
        }
    }
}

fn write_instruction(text: &mut String, instruction: &Instruction) {
    text.push_str(&format!("@{} ", instruction.line));
    match &instruction.op {
        Op::Int { dst, value } => text.push_str(&format!("int {dst} {value}")),
        Op::Float { dst, bits } => text.push_str(&format!("float {dst} {bits}")),
        Op::Unit { dst } => text.push_str(&format!("unit {dst}")),
        Op::Read { dst, place, moving } => {
            text.push_str(&format!(
                "read {dst} {}",
                if *moving { "move" } else { "copy" }
            ));
            write_place(text, place);
        }
        Op::Borrow {
            dst,
            place,
            mutable,
        } => {
            text.push_str(&format!(
                "borrow {dst} {}",
                if *mutable { "mut" } else { "shared" }
            ));
            write_place(text, place);
        }
        Op::Array { dst, elements } => {
            text.push_str(&format!("array {dst}"));
            write_indices(text, elements);
        }
        Op::SliceBorrow {
            dst,
            place,
            mutable,
        } => {
            text.push_str(&format!(
                "slice-borrow {dst} {}",
                if *mutable { "mut" } else { "shared" }
            ));
            write_place(text, place);
        }
        Op::Assign { place, src } => {
            text.push_str(&format!("assign {src}"));
            write_place(text, place);
        }
        Op::Drop { place } => {
            text.push_str("drop");
            write_place(text, place);
        }
        Op::Unary { dst, op, src } => text.push_str(&format!("unary {dst} {} {src}", quoted(op))),
        Op::Binary {
            dst,
            op,
            left,
            right,
        } => text.push_str(&format!("binary {dst} {} {left} {right}", quoted(op))),
        Op::Call {
            dst,
            function,
            args,
        } => {
            text.push_str(&format!("call {dst} {function}"));
            write_indices(text, args);
        }
        Op::Struct { dst, fields } => {
            text.push_str(&format!("struct {dst}"));
            write_indices(text, fields);
        }
        Op::Variant {
            dst,
            variant,
            fields,
        } => {
            text.push_str(&format!("variant {dst} {variant}"));
            write_indices(text, fields);
        }
        Op::Discriminant { dst, src } => text.push_str(&format!("tag {dst} {src}")),
        Op::TagTest { dst, src, variant } => {
            text.push_str(&format!("tag-test {dst} {src} {variant}"))
        }
        Op::Intrinsic { dst, name, args } => {
            text.push_str(&format!("intrinsic {dst} {}", quoted(name)));
            write_indices(text, args);
        }
        Op::If { condition } => text.push_str(&format!("if {condition}")),
        Op::Else => text.push_str("else"),
        Op::End => text.push_str("end"),
        Op::Loop => text.push_str("loop"),
        Op::Break => text.push_str("break"),
        Op::Continue => text.push_str("continue"),
        Op::Return { src } => text.push_str(&format!("return {src}")),
        Op::Trap { message } => text.push_str(&format!("trap {}", quoted(message))),
    }
    text.push('\n');
}

pub fn write(program: &Program) -> String {
    let mut text = String::new();
    for ty in &program.types {
        text.push_str(&format!("type {} ", quoted(&ty.name)));
        match &ty.kind {
            TypeKind::Unit => text.push_str("unit"),
            TypeKind::Bool => text.push_str("bool"),
            TypeKind::F64 => text.push_str("f64"),
            TypeKind::String => text.push_str("string"),
            TypeKind::World => text.push_str("world"),
            TypeKind::Int { bits, signed } => text.push_str(&format!(
                "int {bits} {}",
                if *signed { "signed" } else { "unsigned" }
            )),
            TypeKind::Ref { inner, mutable } | TypeKind::Slice { inner, mutable } => {
                text.push_str(&format!(
                    "{} {} {inner}",
                    if matches!(ty.kind, TypeKind::Ref { .. }) {
                        "ref"
                    } else {
                        "slice"
                    },
                    if *mutable { "mut" } else { "shared" }
                ));
            }
            TypeKind::Array { inner, len } => text.push_str(&format!("array {inner} {len}")),
            TypeKind::Vec { inner } => text.push_str(&format!("vec {inner}")),
            TypeKind::Box { inner } => text.push_str(&format!("box {inner}")),
            TypeKind::Struct { fields, copy } => {
                text.push_str(if *copy { "struct copy" } else { "struct move" });
                write_fields(&mut text, fields);
            }
            TypeKind::Enum { variants, copy } => {
                text.push_str(if *copy { "enum copy\n" } else { "enum move\n" });
                for variant in variants {
                    text.push_str(&format!("variant {}", quoted(&variant.name)));
                    write_fields(&mut text, &variant.fields);
                    text.push('\n');
                }
                text.push_str("endtype");
            }
        }
        text.push('\n');
    }
    for (ty, function) in &program.destructors {
        text.push_str(&format!("destructor {ty} {function}\n"));
    }
    for function in &program.functions {
        text.push_str(&format!(
            "fn {} {} {}",
            quoted(&function.name),
            function.params,
            function.result
        ));
        if let Some(source) = function.return_source {
            text.push_str(&format!(" from {source}"));
        }
        if function.test {
            text.push_str(" test");
        }
        text.push('\n');
        for local in &function.locals {
            text.push_str(&format!(
                "local {} {} {}\n",
                quoted(&local.name),
                local.ty,
                if local.mutable { "mut" } else { "let" }
            ));
        }
        for instruction in &function.body {
            write_instruction(&mut text, instruction);
        }
        text.push_str("endfn\n");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_round_trip_preserves_meaning_and_locations() {
        let source = "struct Pair { left:i64, right:i64 } pub fn probe()->i64 { let p=Pair {left:20,right:22}; p.left+p.right }";
        let syntax = crate::syntax::parse(source).unwrap();
        let checked = crate::check::check(crate::elaborate::elaborate(&syntax).unwrap()).unwrap();
        let text = write(checked.program());
        let decoded = read(&text).unwrap();
        assert_eq!(decoded, *checked.program());
        assert!(crate::check::check(decoded).is_ok());
    }

    #[test]
    fn text_can_supply_k_the_elaborator_never_emits() {
        let mut text = write(&Program {
            destructors: Vec::new(),
            types: primitives(),
            functions: Vec::new(),
        });
        text.push_str("fn \"probe\" 0 3\nlocal \"value\" 3 let\n@1 int 0 42\n@1 return 0\n@2 drop 999\nendfn\n");
        let unchecked = read(&text).unwrap();
        assert_eq!(crate::check::check(unchecked).err().unwrap().code, "E_TYPE");
    }

    #[test]
    fn no_allocation_uses_declared_parameter_count() {
        let text = "fn \"bad\" 18446744073709551615 3\nendfn\n";
        let mut decoded = read(text).unwrap();
        decoded.types = primitives();
        assert_eq!(crate::check::check(decoded).err().unwrap().code, "E_K");
    }

    #[test]
    fn malformed_text_gets_a_diagnostic() {
        for text in [
            "type",
            "type \"x\" struct",
            "fn \"x\" 0 3",
            "type \"x\" enum move",
            "type \"x\" int 256 signed",
            "fn \"x\" 0 3\n@1 read 0 move\nendfn",
            "type \"x\\uD800\" unit",
            "type \"x\" unit trailing",
        ] {
            assert_eq!(read(text).err().unwrap().code, "E_K_TEXT", "{text}");
        }
    }

    #[test]
    fn names_and_messages_use_json_string_escapes() {
        let name = "quote\" slash\\ newline\n and 雪";
        let p = Program {
            destructors: Vec::new(),
            types: vec![Type {
                name: name.to_string(),
                kind: TypeKind::World,
            }],
            functions: Vec::new(),
        };
        assert_eq!(read(&write(&p)).unwrap(), p);
        let decoded = read("type \"\\uD83D\\uDE00\" world\n").unwrap();
        assert_eq!(decoded.types[0].name, "😀");
    }

    #[test]
    fn all_instruction_shapes_round_trip() {
        let ops = vec![
            Op::Int { dst: 0, value: -1 },
            Op::Float { dst: 0, bits: 42 },
            Op::Unit { dst: 0 },
            Op::Read {
                dst: 1,
                place: Place {
                    local: 0,
                    projections: vec![
                        Projection::Deref,
                        Projection::Field(1),
                        Projection::Index(2),
                        Projection::VariantField(0, 1),
                    ],
                },
                moving: true,
            },
            Op::Borrow {
                dst: 1,
                place: Place::local(0),
                mutable: true,
            },
            Op::Assign {
                place: Place::local(0),
                src: 1,
            },
            Op::Drop {
                place: Place::local(0),
            },
            Op::Unary {
                dst: 0,
                op: "-".to_string(),
                src: 1,
            },
            Op::Binary {
                dst: 0,
                op: "+".to_string(),
                left: 1,
                right: 2,
            },
            Op::Call {
                dst: 0,
                function: 1,
                args: vec![2, 3],
            },
            Op::Struct {
                dst: 0,
                fields: vec![1, 2],
            },
            Op::Variant {
                dst: 0,
                variant: 1,
                fields: vec![2],
            },
            Op::Discriminant { dst: 0, src: 1 },
            Op::TagTest {
                dst: 0,
                src: 1,
                variant: 2,
            },
            Op::Intrinsic {
                dst: 0,
                name: "vec_new".to_string(),
                args: Vec::new(),
            },
            Op::If { condition: 0 },
            Op::Else,
            Op::Loop,
            Op::Continue,
            Op::Break,
            Op::End,
            Op::Return { src: 0 },
            Op::Trap {
                message: "bad\nvalue".to_string(),
            },
        ];
        let p = Program {
            destructors: Vec::new(),
            types: primitives(),
            functions: vec![Function {
                return_source: None,
                name: "probe".to_string(),
                params: 0,
                result: I64,
                locals: Vec::new(),
                body: ops
                    .into_iter()
                    .map(|op| Instruction { op, line: 7 })
                    .collect(),
                test: true,
            }],
        };
        assert_eq!(read(&write(&p)).unwrap(), p);
    }
    #[test]
    fn arrays_slices_views_and_destructors_round_trip() {
        let source = "struct Guard<'a>{ count:&'a mut i64 } impl<'a> Drop for Guard<'a>{ fn drop(&mut self){*self.count=*self.count+1;} } fn first(values:&[u8])->&u8 { &values[0] } pub fn probe()->i64 { let mut count:i64=0; {let _guard=Guard{count:&mut count};} let bytes:[u8;2]=[20,22]; count+*first(&bytes) as i64 }";
        let syntax = crate::syntax::parse(source).unwrap();
        let checked = crate::check::check(crate::elaborate::elaborate(&syntax).unwrap()).unwrap();
        assert!(!checked.program().destructors.is_empty());
        assert!(
            checked
                .program()
                .functions
                .iter()
                .any(|function| function.return_source.is_some())
        );
        let text = write(checked.program());
        assert!(text.contains("slice-borrow"));
        assert!(text.contains("destructor"));
        let decoded = read(&text).unwrap();
        assert_eq!(decoded, *checked.program());
        assert!(crate::check::check(decoded).is_ok());
    }
}
