//! Ordinary graph requests shared by maintained adoption and the transported producer.
use super::effects_program::{call, parameters, task};
use crate::pure_tail_program::Request;
use std::collections::BTreeMap;

pub(super) fn iteration(r: &mut Request) {
    r.text.push_str("create.variant as=$iteration-step module=$module name=iteration-step visibility=public\nadd.type-parameter as=$iteration-State declaration=$iteration-step name=State\nadd.type-parameter as=$iteration-Output declaration=$iteration-step name=Output\ntype.parameter as=@iteration-State parameter=$iteration-State\ntype.parameter as=@iteration-Output parameter=$iteration-Output\nadd.case as=$iteration-continue variant=$iteration-step name=continue payload=@iteration-State\nadd.case as=$iteration-done variant=$iteration-step name=done payload=@iteration-Output\n");
    parameters(r, "task-iterate", &["State", "Output"]);
    r.text.push_str("type.application as=@task-iterate-result declaration=$iteration-step\ntype.argument parent=@task-iterate-result index=0 type=@task-iterate-State\ntype.argument parent=@task-iterate-result index=1 type=@task-iterate-Output\ntype.task-function as=@task-iterate-step result=@task-iterate-result effect=@task-iterate-E\ntype.argument parent=@task-iterate-step index=0 type=@task-iterate-State\n");
    let step = r.local("$task-iterate_step");
    let initial = r.local("$task-iterate_initial");
    let result = r.invoke(&step, &[initial]);
    let next = r.local("$iterate-next");
    let step = r.local("$task-iterate_step");
    let again = call(
        r,
        "$task-iterate",
        &["@task-iterate-State", "@task-iterate-Output"],
        "@task-iterate-E",
        &[next, step],
    );
    let done = r.local("$iterate-output");
    let body = r.expression("match", &format!("value={result}"));
    r.text.push_str(&format!("expression.match-arm parent={body} index=0 case=$iteration-continue as=$iterate-next name=next type=@task-iterate-State body={again}\nexpression.match-arm parent={body} index=1 case=$iteration-done as=$iterate-output name=output type=@task-iterate-Output body={done}\n"));
    task(
        r,
        "task-iterate",
        true,
        "@task-iterate-Output",
        &body,
        &[
            ("initial", "@task-iterate-State"),
            ("step", "@task-iterate-step"),
        ],
    );
}

/// Keep the public fold's exact contract while replacing only its body and private traversal.
pub(super) fn fold(
    r: &mut Request,
    s: &BTreeMap<String, String>,
    existing: Option<&BTreeMap<String, String>>,
) {
    for name in ["task-fold-left", "task-fold-left-loop"] {
        if name == "task-fold-left"
            && let Some(old) = existing
        {
            for parameter in ["Item", "State"] {
                r.text.push_str(&format!(
                    "type.parameter as=@{name}-{parameter} parameter={}\n",
                    old[parameter]
                ));
            }
            r.text.push_str(&format!(
                "effect.row as=@{name}-E\neffect.parameter parent=@{name}-E index=0 parameter={}\n",
                old["E"]
            ));
        } else {
            parameters(r, name, &["Item", "State"]);
        }
        r.text.push_str(&format!("type.list as=@{name}-items item=@{name}-Item\ntype.task-function as=@{name}-step result=@{name}-State effect=@{name}-E\ntype.argument parent=@{name}-step index=0 type=@{name}-State\ntype.argument parent=@{name}-step index=1 type=@{name}-Item\n"));
    }
    let parameter = |name: &str| {
        existing.map_or_else(
            || format!("$task-fold-left_{name}"),
            |old| old[name].clone(),
        )
    };
    let items = r.local(&parameter("items"));
    let state = r.local(&parameter("state"));
    let step = r.local(&parameter("step"));
    let index = r.integer(0);
    let source = r.local(&parameter("items"));
    let length = r.call(&s["list-length"], &["@task-fold-left-Item"], &[source]);
    let body = call(
        r,
        "$task-fold-left-loop",
        &["@task-fold-left-Item", "@task-fold-left-State"],
        "@task-fold-left-E",
        &[items, state, step, index, length],
    );
    if let Some(old) = existing {
        r.text.push_str(&format!(
            "replace.body function={} body={body}\ndelete.owner owner={} policy=owned-closure\n",
            old["function"], old["range"]
        ));
    } else {
        task(
            r,
            "task-fold-left",
            true,
            "@task-fold-left-State",
            &body,
            &[
                ("items", "@task-fold-left-items"),
                ("state", "@task-fold-left-State"),
                ("step", "@task-fold-left-step"),
            ],
        );
    }
    let index = r.local("$task-fold-left-loop_index");
    let length = r.local("$task-fold-left-loop_length");
    let end = r.call(&s["i64-equal"], &[], &[index, length]);
    let final_state = r.local("$task-fold-left-loop_state");
    let items = r.local("$task-fold-left-loop_items");
    let step = r.local("$task-fold-left-loop_step");
    let state = r.local("$task-fold-left-loop_state");
    let source = r.local("$task-fold-left-loop_items");
    let index = r.local("$task-fold-left-loop_index");
    let item = r.call(
        &s["list-get"],
        &["@task-fold-left-loop-Item"],
        &[source, index],
    );
    let next = r.invoke(&step, &[state, item]);
    let step = r.local("$task-fold-left-loop_step");
    let index = r.local("$task-fold-left-loop_index");
    let one = r.integer(1);
    let index = r.call(&s["add"], &[], &[index, one]);
    let length = r.local("$task-fold-left-loop_length");
    let again = call(
        r,
        "$task-fold-left-loop",
        &["@task-fold-left-loop-Item", "@task-fold-left-loop-State"],
        "@task-fold-left-loop-E",
        &[items, next, step, index, length],
    );
    let body = r.expression(
        "if",
        &format!("condition={end} when-true={final_state} when-false={again}"),
    );
    task(
        r,
        "task-fold-left-loop",
        false,
        "@task-fold-left-loop-State",
        &body,
        &[
            ("items", "@task-fold-left-loop-items"),
            ("state", "@task-fold-left-loop-State"),
            ("step", "@task-fold-left-loop-step"),
            ("index", "i64"),
            ("length", "i64"),
        ],
    );
}
