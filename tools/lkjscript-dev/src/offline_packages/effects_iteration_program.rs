//! Fresh transported iteration meaning. All looping and validation are ordinary graph records.
use super::effects_program::{parameters, task};
use crate::pure_tail_program::Request;
use std::collections::BTreeMap;

pub(super) fn producer(s: &BTreeMap<String, String>) -> String {
    let mut r = Request::default();
    for (name, fields) in [
        ("iteration-state", ["cursor", "sum"]),
        ("iteration-output", ["position", "total"]),
    ] {
        r.text.push_str(&format!("create.record as=${name} module=$module name={name} visibility=public\ntype.named as=@{name} declaration=${name}\n"));
        for field in fields {
            r.text.push_str(&format!(
                "add.field as=${name}-{field} record=${name} name={field} type=i64\n"
            ));
        }
    }
    r.text.push_str("type.application as=@iteration-transition declaration=$iteration-step\ntype.argument parent=@iteration-transition index=0 type=@iteration-state\ntype.argument parent=@iteration-transition index=1 type=@iteration-output\n");
    for name in ["iteration-advance", "make-iteration"] {
        parameters(&mut r, name, &[]);
        r.text.push_str(&format!("type.task-function as=@{name}-observe result=unit effect=@{name}-E\ntype.argument parent=@{name}-observe index=0 type=i64\n"));
    }
    let state = r.local("$iteration-advance_state");
    let cursor = r.expression(
        "field",
        &format!("value={state} field=$iteration-state-cursor"),
    );
    let limit = r.local("$iteration-advance_threshold");
    let stopped = r.call(&s["less"], &[], &[cursor, limit]);
    let state = r.local("$iteration-advance_state");
    let cursor = r.expression(
        "field",
        &format!("value={state} field=$iteration-state-cursor"),
    );
    let state = r.local("$iteration-advance_state");
    let sum = r.expression(
        "field",
        &format!("value={state} field=$iteration-state-sum"),
    );
    let output = r.expression("record", "type=$iteration-output");
    r.text.push_str(&format!("expression.record-field parent={output} index=0 field=$iteration-output-position value={cursor}\nexpression.record-field parent={output} index=1 field=$iteration-output-total value={sum}\n"));
    let done = r.expression("variant", &format!("case=$iteration-done payload={output}"));
    r.types(&done, &["@iteration-state", "@iteration-output"]);
    let state = r.local("$iteration-advance_state");
    let cursor = r.expression(
        "field",
        &format!("value={state} field=$iteration-state-cursor"),
    );
    let stride = r.local("$iteration-advance_stride");
    let advanced = r.call(&s["add"], &[], &[cursor, stride]);
    let state = r.local("$iteration-advance_state");
    let sum = r.expression(
        "field",
        &format!("value={state} field=$iteration-state-sum"),
    );
    let state = r.local("$iteration-advance_state");
    let cursor = r.expression(
        "field",
        &format!("value={state} field=$iteration-state-cursor"),
    );
    let sum = r.call(&s["add"], &[], &[sum, cursor]);
    let next = r.expression("record", "type=$iteration-state");
    r.text.push_str(&format!("expression.record-field parent={next} index=0 field=$iteration-state-cursor value={advanced}\nexpression.record-field parent={next} index=1 field=$iteration-state-sum value={sum}\n"));
    let next = r.expression(
        "variant",
        &format!("case=$iteration-continue payload={next}"),
    );
    r.types(&next, &["@iteration-state", "@iteration-output"]);
    let decision = r.choose(&stopped, &next, &done);
    let observer = r.local("$iteration-advance_observe");
    let state = r.local("$iteration-advance_state");
    let cursor = r.expression(
        "field",
        &format!("value={state} field=$iteration-state-cursor"),
    );
    let observed = r.invoke(&observer, &[cursor]);
    let body = r.expression("sequence", "");
    r.arguments(&body, &[observed, decision]);
    task(
        &mut r,
        "iteration-advance",
        false,
        "@iteration-transition",
        &body,
        &[
            ("stride", "i64"),
            ("threshold", "i64"),
            ("observe", "@iteration-advance-observe"),
            ("state", "@iteration-state"),
        ],
    );

    r.text.push_str("type.task-function as=@make-iteration-result result=@iteration-transition effect=@make-iteration-E\ntype.argument parent=@make-iteration-result index=0 type=@iteration-state\n");
    let function = r.function_value("$iteration-advance");
    r.effects(&function, &["@make-iteration-E"]);
    let stride = r.local("$make-iteration_stride");
    let threshold = r.local("$make-iteration_threshold");
    let observe = r.local("$make-iteration_observe");
    let bound = r.bind(&function, &[stride, threshold, observe]);
    r.function(
        "make-iteration",
        "@make-iteration-result",
        &bound,
        &[
            ("stride", "i64"),
            ("threshold", "i64"),
            ("observe", "@make-iteration-observe"),
        ],
    );
    r.text.replace("$e", "$iteration-e")
}

pub(super) fn consumer(
    r: &mut Request,
    s: &BTreeMap<String, String>,
    library: &BTreeMap<String, String>,
) -> Vec<(String, String)> {
    use super::effects_consumer::{response, task};
    for name in ["iteration-state", "iteration-output"] {
        r.text.push_str(&format!(
            "type.named as=@{name} declaration={}\n",
            library[name]
        ));
    }
    let tick = r.expression("static-text", "value=tick");
    let tick = r.capability("$config", &s["Configuration.i64"], &[tick]);
    let unit = r.expression("unit", "");
    let body = r.expression("sequence", "");
    r.arguments(&body, &[tick, unit]);
    task(
        r,
        "iteration-tick",
        "unit",
        "$config",
        &body,
        &[("cursor", "i64")],
    );

    for (name, target, fail) in [
        ("iteration-write", "$data-job", false),
        ("iteration-fail", "$data-job", true),
        ("iteration-partial", "$nested-job", true),
    ] {
        let cursor = r.local(&format!("${name}_cursor"));
        let payload = r.expression("record", &format!("type={}", library["payload"]));
        r.types(&payload, &["i64"]);
        r.text.push_str(&format!(
            "expression.record-field parent={payload} index=0 field={} value={cursor}\n",
            library["payload-value"]
        ));
        let job = r.expression(
            "variant",
            &format!("case={} payload={payload}", library["job-item"]),
        );
        r.types(&job, &["i64"]);
        let zero = r.integer(0);
        let write = r.call(target, &[], &[zero, job]);
        let unit = r.expression("unit", "");
        let body = r.expression("sequence", "");
        r.arguments(&body, &[write, unit]);
        let body = if fail {
            let cursor = r.local(&format!("${name}_cursor"));
            let two = r.integer(2);
            let fail = r.call(&s["i64-equal"], &[], &[cursor, two]);
            let one = r.integer(1);
            let zero = r.integer(0);
            let trap = r.call(&s["divide"], &[], &[one, zero]);
            let unit = r.expression("unit", "");
            let trapped = r.expression("sequence", "");
            r.arguments(&trapped, &[trap, unit]);
            r.choose(&fail, &trapped, &body)
        } else {
            body
        };
        task(r, name, "unit", "$data", &body, &[("cursor", "i64")]);
    }
    let mut modes = Vec::new();
    for (name, callback, row, transactional) in [
        ("iterate", "$iteration-tick", "@Config", false),
        ("iterate-transaction", "$iteration-write", "@Data", true),
        ("iterate-rollback", "$iteration-fail", "@Data", true),
        ("iterate-partial", "$iteration-partial", "@Data", false),
    ] {
        let stride = if name == "iterate" {
            let key = r.expression("static-text", "value=stride");
            r.capability("$config", &s["Configuration.i64"], &[key])
        } else {
            r.integer(1)
        };
        let threshold = if name == "iterate" {
            let key = r.expression("static-text", "value=threshold");
            r.capability("$config", &s["Configuration.i64"], &[key])
        } else {
            r.integer(3)
        };
        let stride_value = r.local(&format!("${name}-stride"));
        let threshold_value = r.local(&format!("${name}-threshold"));
        let callback = r.function_value(callback);
        let step = r.call(
            &library["make-iteration"],
            &[],
            &[stride_value, threshold_value, callback],
        );
        r.effects(&step, &[row]);
        let initial = r.expression("record", &format!("type={}", library["iteration-state"]));
        let zero = r.integer(0);
        let sum = r.integer(0);
        r.text.push_str(&format!("expression.record-field parent={initial} index=0 field={} value={zero}\nexpression.record-field parent={initial} index=1 field={} value={sum}\n", library["iteration-state-cursor"], library["iteration-state-sum"]));
        let result = r.call(
            &library["task-iterate"],
            &["@iteration-state", "@iteration-output"],
            &[initial, step],
        );
        r.effects(&result, &[row]);
        let result = if transactional {
            r.expression(
                "transaction",
                &format!("requirement=$data binding=${name}-tx name=iteration body={result}"),
            )
        } else {
            result
        };
        let stride_value = r.local(&format!("${name}-stride"));
        let zero = r.integer(0);
        let positive = r.call(&s["less"], &[], &[zero, stride_value]);
        let one = r.integer(1);
        let zero = r.integer(0);
        let trap = r.call(&s["divide"], &[], &[one, zero]);
        let unreachable = r.expression("record", &format!("type={}", library["iteration-output"]));
        let zero = r.integer(0);
        let total = r.integer(0);
        r.text.push_str(&format!("expression.record-field parent={unreachable} index=0 field={} value={zero}\nexpression.record-field parent={unreachable} index=1 field={} value={total}\n", library["iteration-output-position"], library["iteration-output-total"]));
        let failure = r.expression("sequence", "");
        r.arguments(&failure, &[trap, unreachable]);
        r.function(
            &format!("{name}-invalid"),
            "@iteration-output",
            &failure,
            &[],
        );
        let threshold_value = r.local(&format!("${name}-threshold"));
        let zero = r.integer(0);
        let negative = r.call(&s["less"], &[], &[threshold_value, zero]);
        let failure = r.call(&format!("${name}-invalid"), &[], &[]);
        let validated = r.choose(&negative, &failure, &result);
        let failure = r.call(&format!("${name}-invalid"), &[], &[]);
        let validated = r.choose(&positive, &validated, &failure);
        let body = r.expression("let", &format!("body={validated}"));
        r.text.push_str(&format!("expression.binding parent={body} index=0 as=${name}-stride name=stride type=i64 value={stride}\nexpression.binding parent={body} index=1 as=${name}-threshold name=threshold type=i64 value={threshold}\n"));
        task(
            r,
            name,
            "@iteration-output",
            if name == "iterate" {
                "$config"
            } else {
                "$data"
            },
            &body,
            &[],
        );
        let value = r.call(&format!("${name}"), &[], &[]);
        let response = response(r, s, &value, "@iteration-output");
        modes.push((name.to_owned(), response));
    }
    modes
}
