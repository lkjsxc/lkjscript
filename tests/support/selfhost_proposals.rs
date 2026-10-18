//! Test authoring only: split maintained proposals into bounded public changes.
//! This rewrites proposal syntax, never repository objects or compiler output.
pub const SOURCE: &str = concat!(
    include_str!("../../tools/selfhost/input.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/elf.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/flow.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/machine.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/asm.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/types.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/custody.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/profile.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/values.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/heap.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/objects.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/hash.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/stream.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/runtime.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/aggregate.lkjc"),
    "\n",
);

#[derive(Clone)]
enum Form {
    Atom(String),
    List(Vec<Form>),
}
impl Form {
    fn text(&self) -> String {
        match self {
            Self::Atom(a) => a.clone(),
            Self::List(v) => format!(
                "({})",
                v.iter().map(Self::text).collect::<Vec<_>>().join(" ")
            ),
        }
    }
    fn head(&self) -> Option<&str> {
        match self {
            Self::List(v) => match v.first() {
                Some(Self::Atom(a)) => Some(a),
                _ => None,
            },
            _ => None,
        }
    }
}
fn parse(source: &str) -> Vec<Form> {
    fn one(source: &str, at: &mut usize) -> Option<Form> {
        let bytes = source.as_bytes();
        while *at < bytes.len() {
            if bytes[*at].is_ascii_whitespace() {
                *at += 1;
            } else if bytes[*at] == b';' {
                while *at < bytes.len() && bytes[*at] != b'\n' {
                    *at += 1;
                }
            } else {
                break;
            }
        }
        if *at == bytes.len() {
            return None;
        }
        if bytes[*at] == b'(' {
            *at += 1;
            let mut values = vec![];
            loop {
                while *at < bytes.len() && bytes[*at].is_ascii_whitespace() {
                    *at += 1;
                }
                assert!(*at < bytes.len(), "unclosed maintained proposal");
                if bytes[*at] == b')' {
                    *at += 1;
                    break;
                }
                values.push(one(source, at).unwrap());
            }
            Some(Form::List(values))
        } else {
            let start = *at;
            if bytes[*at] == b'"' {
                *at += 1;
                while *at < bytes.len() && bytes[*at] != b'"' {
                    *at += if bytes[*at] == b'\\' { 2 } else { 1 };
                }
                assert!(*at < bytes.len(), "unclosed proposal string");
                *at += 1;
            } else {
                while *at < bytes.len()
                    && !bytes[*at].is_ascii_whitespace()
                    && !b"()".contains(&bytes[*at])
                {
                    *at += 1;
                }
                assert!(*at > start, "unexpected proposal delimiter");
            }
            Some(Form::Atom(source[start..*at].to_owned()))
        }
    }
    let mut at = 0;
    let mut forms = vec![];
    while let Some(form) = one(source, &mut at) {
        forms.push(form);
    }
    forms
}
fn stub(function: &mut [Form]) -> Result<(), String> {
    let result = function
        .iter()
        .find(|x| x.head() == Some("returns"))
        .unwrap();
    let Form::List(result) = result else {
        unreachable!()
    };
    let body = match result[1].text().as_str() {
        "I64" => "(i64 0)".to_owned(),
        "Bool" => "(bool false)".to_owned(),
        "Unit" => "(unit)".to_owned(),
        "Text" => "(text \"\")".to_owned(),
        "Bytes" => "(call ci::text-bytes (text \"\"))".to_owned(),
        "(list I64)" => "(list I64)".to_owned(),
        "ByteBuffer" => {
            let owner = function.iter().find_map(|f| {
                let Form::List(p) = f else {
                    return None;
                };
                (f.head() == Some("parameter")
                    && p.iter().any(|x| x.text() == "(type ByteBuffer)")
                    && p.iter().any(|x| x.text() == "(use consume)"))
                .then(|| p[2].text())
            });
            owner.map_or_else(|| "(call ci::empty)".to_owned(), |p| format!("(local {p})"))
        }
        "(record (diagnostic I64) (image Bytes))" => concat!(
            "(record structural (field diagnostic (i64 0)) ",
            "(field image (call ci::text-bytes (text \"\"))))"
        )
        .to_owned(),
        other => return Err(format!("unsupported test bootstrap result {other}")),
    };
    let original = function
        .iter_mut()
        .find(|x| x.head() == Some("body"))
        .unwrap();
    *original = parse(&format!("(body {body})")).pop().unwrap();
    Ok(())
}
fn upsert(form: &Form, module: &str) -> Form {
    let Form::List(items) = form else {
        return form.clone();
    };
    let mut result = items.clone();
    if matches!(
        form.head(),
        Some("function" | "parameter" | "type-parameter")
    ) && items[1].text() == "create"
    {
        result.remove(1);
        result[0] = Form::Atom(format!("upsert-{}", items[0].text()));
    }
    if form.head() == Some("call") && !items[1].text().contains("::") {
        result[1] = Form::Atom(format!("{module}::{}", items[1].text()));
    }
    Form::List(result.iter().map(|x| upsert(x, module)).collect())
}

/// First declare typed interfaces with trivial valid bodies; then replace each body
/// through tx. Every intermediate graph is admitted under the default request bounds.
pub fn steps(source: &str) -> Result<(String, Vec<String>), String> {
    let mut units = vec![];
    for form in parse(source) {
        if form.head() == Some("units") {
            let Form::List(mut values) = form else {
                unreachable!()
            };
            units.extend(values.drain(1..));
        }
    }
    let mut headers = units.clone();
    let mut changes = vec![];
    for (unit, header) in units.iter().zip(headers.iter_mut()) {
        if unit.head() != Some("module") {
            continue;
        }
        let (Form::List(module), Form::List(header)) = (unit, header) else {
            unreachable!()
        };
        let name = module[2].text();
        for (member, head) in module.iter().skip(3).zip(header.iter_mut().skip(3)) {
            if member.head() != Some("function") {
                continue;
            }
            let Form::List(head) = head else {
                unreachable!()
            };
            stub(head)?;
            changes.push(format!(
                "(tx (base @head) (upsert-module {name} {}))",
                upsert(member, &name).text()
            ));
        }
    }
    Ok((
        format!(
            "declarations.begin\n(units {})\ndeclarations.end\n",
            headers
                .iter()
                .map(Form::text)
                .collect::<Vec<_>>()
                .join("\n")
        ),
        changes,
    ))
}
