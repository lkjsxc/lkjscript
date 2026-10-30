//! Test authoring only: reconstruct and replay maintained public proposals.
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
    include_str!("../../tools/selfhost/wire.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/pack.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/unit-map.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/unit-wire.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/artifact-order.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/artifact-manifest.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/artifact.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/artifact-input.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/artifact-check.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-budget.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-read.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-shape.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-fields.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-expr.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-owner.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-map.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-state.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-history.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-object.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-index.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-graph.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-link-ref.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-link-owner.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-link-expr.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-links.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-table.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-sort.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-edges.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-containment.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-members.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-forest.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-scope.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-namespace-key.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-namespace-sort.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-namespace.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-type-table.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-type-uses.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-type-dag.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-type-scope.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-nominal-queue.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-nominal-cycle.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-nominal-flow.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-nominal.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-effect-shape.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-capability-shape.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-effect.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-capability.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-function.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-scalar-profile.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-scalar-infer.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-scalar.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-scalar-pack.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-scalar-plan.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-scalar-cursor.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-scalar-state.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-scalar-trace.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-scalar-emit.lkjc"),
    "\n",
    include_str!("../../tools/selfhost/source-elf.lkjc"),
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
// Compose structural records from the existing unrestricted fixture leaves.
// The explicit stack follows finite type syntax without a recursive depth guard.
fn record_stub(ty: &Form) -> Result<String, String> {
    enum Part<'a> {
        Value(&'a Form),
        Field(&'a Form),
        Close,
    }
    let mut pending = vec![Part::Value(ty)];
    let mut result = String::new();
    while let Some(part) = pending.pop() {
        match part {
            Part::Value(Form::Atom(name)) => result.push_str(match name.as_str() {
                "Bytes" => "(call ci::text-bytes (text \"\"))",
                "I64" => "(i64 0)",
                "Bool" => "(bool false)",
                other => return Err(format!("unsupported bootstrap record field {other}")),
            }),
            Part::Value(ty) if ty.head() == Some("record") => {
                let Form::List(fields) = ty else {
                    unreachable!()
                };
                result.push_str("(record structural");
                pending.push(Part::Close);
                pending.extend(fields[1..].iter().rev().map(Part::Field));
            }
            Part::Value(other) => {
                return Err(format!(
                    "unsupported bootstrap record field {}",
                    other.text()
                ));
            }
            Part::Field(field) => {
                let Form::List(field) = field else {
                    return Err("invalid bootstrap record field".to_owned());
                };
                if field.len() != 2 {
                    return Err("invalid bootstrap record field".to_owned());
                }
                result.push_str(" (field ");
                result.push_str(&field[0].text());
                result.push(' ');
                pending.push(Part::Close);
                pending.push(Part::Value(&field[1]));
            }
            Part::Close => result.push(')'),
        }
    }
    Ok(result)
}

fn stub(function: &mut [Form]) -> Result<(), String> {
    let result = function
        .iter()
        .find(|x| x.head() == Some("returns"))
        .unwrap();
    let Form::List(result) = result else {
        unreachable!()
    };
    let body = if result[1].head() == Some("record") {
        record_stub(&result[1])?
    } else {
        match result[1].text().as_str() {
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
            other => return Err(format!("unsupported test bootstrap result {other}")),
        }
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
        Some(
            "module"
                | "function"
                | "external"
                | "parameter"
                | "type-parameter"
                | "component"
                | "port"
                | "target"
        )
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

fn proposal_batch_size(module: &str) -> usize {
    // Incremental reconstruction retains small independently validated transitions.
    // These fixture groups do not limit the complete compiler inventory.
    if module == "source-owner" { 2 } else { 4 }
}

/// Declare typed interfaces one module at a time, then replace small groups of
/// bodies through tx. Every intermediate graph passes ordinary typed publication.
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
    let mut bodies = vec![];
    for (unit, header) in units.iter().zip(headers.iter_mut()) {
        if unit.head() != Some("module") {
            continue;
        }
        let (Form::List(module), Form::List(header)) = (unit, header) else {
            unreachable!()
        };
        let name = module[2].text();
        let mut members = vec![];
        for (member, head) in module.iter().skip(3).zip(header.iter_mut().skip(3)) {
            if member.head() != Some("function") {
                continue;
            }
            let Form::List(head) = head else {
                unreachable!()
            };
            stub(head)?;
            members.push(upsert(member, &name).text());
        }
        for group in members.chunks(proposal_batch_size(&name)) {
            bodies.push(format!(
                "(tx (base @head) (upsert-module {name} {}))",
                group.join(" ")
            ));
        }
    }
    let first = headers.first().ok_or("empty compiler proposal")?;
    let mut changes = headers
        .iter()
        .skip(1)
        .map(|header| {
            let Form::List(items) = header else {
                unreachable!()
            };
            let name = items[2].text();
            format!("(tx (base @head) {})", upsert(header, &name).text())
        })
        .collect::<Vec<_>>();
    changes.extend(bodies);
    Ok((
        format!(
            "declarations.begin\n(units {})\ndeclarations.end\n",
            first.text()
        ),
        changes,
    ))
}

/// Install every fixture header before replacing bodies in an existing project.
#[allow(
    dead_code,
    reason = "shared helper is also compiled by the public CLI test harness"
)]
pub fn updates(source: &str) -> Result<Vec<String>, String> {
    let (initial, mut changes) = steps(source)?;
    let units = parse(&initial)
        .into_iter()
        .find(|form| form.head() == Some("units"))
        .ok_or("missing proposal units")?;
    let Form::List(units) = units else {
        unreachable!()
    };
    let Form::List(header) = &units[1] else {
        return Err("invalid proposal header".to_owned());
    };
    changes.insert(
        0,
        format!(
            "(tx (base @head) {})",
            upsert(&units[1], &header[2].text()).text()
        ),
    );
    Ok(changes)
}

/// Reapply every final definition to an existing graph in one public transaction.
/// Existing interfaces already resolve forward references; installing temporary
/// stubs or publishing partial groups is unnecessary for this final-state check.
#[allow(
    dead_code,
    reason = "shared helper is also compiled by the public CLI test harness"
)]
pub fn replay(source: &str) -> Vec<String> {
    let mut units_to_replay = vec![];
    for form in parse(source) {
        if form.head() != Some("units") {
            continue;
        }
        let Form::List(units) = form else {
            unreachable!()
        };
        for unit in &units[1..] {
            let Form::List(items) = unit else {
                unreachable!()
            };
            let name = items[2].text();
            units_to_replay.push(upsert(unit, &name).text());
        }
    }
    if units_to_replay.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "(tx (base @head)\n  {}\n)",
            units_to_replay.join("\n  ")
        )]
    }
}
