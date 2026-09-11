//! Public discriminating schemas. All accepted owners are allocated by plan/apply.
use super::*;
use crate::pure_tail_program::Request;

fn declaration(r: &mut Request, name: &str, kind: &str, parameters: &[&str]) {
    r.text.push_str(&format!(
        "create.{kind} as=${name} module=$module name={name} visibility=public\n"
    ));
    for parameter in parameters {
        r.text.push_str(&format!("add.type-parameter as=${name}-{parameter} declaration=${name} name={parameter}\ntype.parameter as=@{name}-{parameter} parameter=${name}-{parameter}\n"));
    }
}
fn application(r: &mut Request, name: &str, declaration: &str, arguments: &[&str]) {
    r.text.push_str(&format!(
        "type.application as=@{name} declaration=${declaration}\n"
    ));
    r.types(&format!("@{name}"), arguments);
}
fn case(r: &mut Request, owner: &str, name: &str, payload: Option<&str>) {
    r.text.push_str(&format!(
        "add.case as=${owner}-{name} variant=${owner} name={name}{}\n",
        payload
            .map(|ty| format!(" payload={ty}"))
            .unwrap_or_default()
    ));
}

pub(super) fn finite() -> String {
    let mut r = Request::default();
    declaration(&mut r, "node", "record", &["T"]);
    application(&mut r, "node-self", "node", &["@node-T"]);
    r.text.push_str("type.list as=@node-children item=@node-self\nadd.field as=$node-value record=$node name=value type=@node-T\nadd.field as=$node-children record=$node name=children type=@node-children\n");
    declaration(&mut r, "uninhabited", "record", &["T"]);
    application(
        &mut r,
        "uninhabited-self",
        "uninhabited",
        &["@uninhabited-T"],
    );
    r.text.push_str(
        "add.field as=$uninhabited-next record=$uninhabited name=next type=@uninhabited-self\n",
    );
    declaration(&mut r, "expr", "variant", &["T"]);
    declaration(&mut r, "group", "record", &["T"]);
    application(&mut r, "expr-group", "group", &["@expr-T"]);
    application(&mut r, "group-expr", "expr", &["@group-T"]);
    r.text.push_str("type.list as=@group-items item=@group-expr\nadd.field as=$group-items record=$group name=items type=@group-items\n");
    case(&mut r, "expr", "atom", Some("@expr-T"));
    case(&mut r, "expr", "group", Some("@expr-group"));
    declaration(&mut r, "flip", "variant", &["A", "B"]);
    application(&mut r, "flipped", "flip", &["@flip-B", "@flip-A"]);
    case(&mut r, "flip", "stop", None);
    case(&mut r, "flip", "next", Some("@flipped"));
    declaration(&mut r, "select-a", "variant", &["T"]);
    declaration(&mut r, "select-b", "record", &["U", "V"]);
    application(
        &mut r,
        "selected-b",
        "select-b",
        &["@select-a-T", "@select-a-T"],
    );
    application(&mut r, "selected-a", "select-a", &["@select-b-V"]);
    case(&mut r, "select-a", "stop", None);
    case(&mut r, "select-a", "next", Some("@selected-b"));
    r.text
        .push_str("add.field as=$select-b-next record=$select-b name=next type=@selected-a\n");
    declaration(&mut r, "reset-a", "variant", &["T"]);
    declaration(&mut r, "reset-b", "record", &["U", "V"]);
    r.text
        .push_str("type.list as=@reset-growth item=@reset-a-T\n");
    application(&mut r, "reset-to-b", "reset-b", &["@reset-growth", "i64"]);
    application(&mut r, "reset-to-a", "reset-a", &["@reset-b-V"]);
    case(&mut r, "reset-a", "stop", None);
    case(&mut r, "reset-a", "next", Some("@reset-to-b"));
    r.text
        .push_str("add.field as=$reset-b-next record=$reset-b name=next type=@reset-to-a\n");
    declaration(&mut r, "wrapper-a", "variant", &["T"]);
    declaration(&mut r, "wrapper", "record", &[]);
    application(&mut r, "fixed-a", "wrapper-a", &["i64"]);
    r.text.push_str("type.named as=@wrapper declaration=$wrapper\nadd.field as=$wrapper-next record=$wrapper name=next type=@fixed-a\n");
    case(&mut r, "wrapper-a", "stop", None);
    case(&mut r, "wrapper-a", "next", Some("@wrapper"));
    declaration(&mut r, "signature", "record", &["T"]);
    application(&mut r, "signature-self", "signature", &["@signature-T"]);
    r.text.push_str("type.function as=@signature-function result=@signature-self\nadd.field as=$signature-next record=$signature name=next type=@signature-function\n");
    r.text
}

pub(super) fn concrete(r: &mut Request, library: &BTreeMap<String, String>) {
    for (name, owner, arguments) in [
        ("node-value", "node", vec!["i64"]),
        ("mutual-value", "expr", vec!["text"]),
        ("flip-value", "flip", vec!["i64", "text"]),
        ("selection-value", "select-a", vec!["i64"]),
        ("reset-value", "reset-a", vec!["text"]),
        ("wrapper-value", "wrapper-a", vec!["text"]),
    ] {
        r.text.push_str(&format!(
            "type.application as=@{name}-type declaration={}\n",
            library[owner]
        ));
        r.types(&format!("@{name}-type"), &arguments);
        let value = r.local(&format!("${name}_value"));
        r.function(
            name,
            &format!("@{name}-type"),
            &value,
            &[("value", &format!("@{name}-type"))],
        );
        r.target(name, &format!("@{name}-type"), &[&format!("@{name}-type")]);
    }
    r.text.push_str(&format!("type.application as=@flip-other declaration={}\ntype.argument parent=@flip-other index=0 type=text\ntype.argument parent=@flip-other index=1 type=i64\n",library["flip"]));
    let value = r.local("$flip-next_value");
    let next = r.expression(
        "variant",
        &format!("case={} payload={value}", library["flip-next"]),
    );
    r.types(&next, &["i64", "text"]);
    r.function(
        "flip-next",
        "@flip-value-type",
        &next,
        &[("value", "@flip-other")],
    );
    r.target("flip-next", "@flip-value-type", &["@flip-other"]);
}

pub(super) fn values() -> Vec<(&'static str, serde_json::Value)> {
    use serde_json::json;
    vec![
        (
            "node-value",
            json!({"value":3,"children":[{"value":7,"children":[]}]}),
        ),
        (
            "mutual-value",
            json!({"case":"group","value":{"items":[{"case":"atom","value":"nested"},{"case":"group","value":{"items":[]}}]}}),
        ),
        (
            "flip-value",
            json!({"case":"next","value":{"case":"next","value":{"case":"stop"}}}),
        ),
        (
            "selection-value",
            json!({"case":"next","value":{"next":{"case":"stop"}}}),
        ),
        (
            "reset-value",
            json!({"case":"next","value":{"next":{"case":"next","value":{"next":{"case":"stop"}}}}}),
        ),
        (
            "wrapper-value",
            json!({"case":"next","value":{"next":{"case":"stop"}}}),
        ),
    ]
}

pub(super) fn rejected() -> Vec<(&'static str, String)> {
    let mut cases = Vec::new();
    for label in ["self", "mutual", "signature", "nominal-argument", "phantom"] {
        let mut r = Request::default();
        r.text.push_str("create.module as=$module name=expanding\n");
        declaration(&mut r, "grow-a", "variant", &["T"]);
        case(&mut r, "grow-a", "stop", None);
        r.text.push_str("type.list as=@larger item=@grow-a-T\n");
        let payload = match label {
            "mutual" => {
                declaration(&mut r, "grow-b", "record", &["U"]);
                application(&mut r, "a-to-b", "grow-b", &["@grow-a-T"]);
                r.text.push_str("type.list as=@larger-b item=@grow-b-U\n");
                application(&mut r, "b-to-a", "grow-a", &["@larger-b"]);
                r.text
                    .push_str("add.field as=$grow-b-next record=$grow-b name=next type=@b-to-a\n");
                "@a-to-b"
            }
            "nominal-argument" | "phantom" => {
                declaration(&mut r, "phantom", "variant", &["P"]);
                case(&mut r, "phantom", "absent", None);
                application(&mut r, "phantom-t", "phantom", &["@grow-a-T"]);
                application(
                    &mut r,
                    "growing",
                    "grow-a",
                    &[if label == "phantom" {
                        "@larger"
                    } else {
                        "@phantom-t"
                    }],
                );
                if label == "phantom" {
                    application(&mut r, "hidden", "phantom", &["@growing"]);
                    "@hidden"
                } else {
                    "@growing"
                }
            }
            _ => {
                application(&mut r, "growing", "grow-a", &["@larger"]);
                if label == "signature" {
                    r.text
                        .push_str("type.function as=@hidden result=@growing\n");
                    "@hidden"
                } else {
                    "@growing"
                }
            }
        };
        case(&mut r, "grow-a", "next", Some(payload));
        cases.push((label, r.text));
    }
    cases
}

pub(super) fn negatives(context: &mut Context) -> Result<(), DevError> {
    let package = context.new_package("recursive-negative")?;
    for (label, changes) in rejected() {
        let path = context
            .evidence
            .join(format!("recursive-expanding-{label}.lkjc"));
        fs::write(
            &path,
            format!("request base={}\n{changes}", package.revision),
        )?;
        for operation in ["plan", "apply"] {
            context.reject(
                &package,
                &[
                    "change",
                    operation,
                    "--input-file",
                    &path.display().to_string(),
                ],
                if operation == "plan" {
                    "kernel_type_nominal_expansion"
                } else {
                    "cli_usage"
                },
            )?;
            // An invalid candidate cannot produce the review token required by public apply.
            // Apply consequently refuses the request at its protocol boundary; plan carries
            // the semantic cycle certificate. Both leave complete accepted authority intact.
            if operation == "apply" {
                continue;
            }
            let command = context
                .receipt
                .commands
                .last()
                .ok_or_else(|| DevError::corrupt("missing rejection command"))?;
            let output = process::read_bounded(
                &context.evidence.join(&command.observation.stdout.path),
                MAXIMUM_OUTPUT_BYTES,
            )?;
            let text = String::from_utf8(output)
                .map_err(|_| DevError::corrupt("diagnostic is not UTF-8"))?;
            require(
                text.contains("expanding")
                    && text.contains("slot")
                    && text.contains("member")
                    && text.contains("argument")
                    && text.contains("occurrence"),
                "recursive rejection omitted owner/slot/path witness",
            )?;
        }
        context.receipt.recursive.schemas.insert(
            if label == "signature" {
                "signature-expanding"
            } else {
                label
            }
            .into(),
            "kernel_type_nominal_expansion".into(),
        );
    }
    fs::remove_dir_all(&package.path)?;
    Ok(())
}
