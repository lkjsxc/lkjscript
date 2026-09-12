//! Compact authoring requests. The product owns their meaning, checking and publication.
use crate::pure_tail_program::Request;
use std::collections::BTreeMap;

pub(super) fn parameters(r: &mut Request, name: &str, types: &[&str]) {
    for ty in types {
        r.text.push_str(&format!("add.type-parameter as=${name}-{ty} declaration=${name} name={ty}\ntype.parameter as=@{name}-{ty} parameter=${name}-{ty}\n"));
    }
    r.text.push_str(&format!("add.effect-parameter as=${name}-E declaration=${name} name=E\neffect.row as=@{name}-E\neffect.parameter parent=@{name}-E index=0 parameter=${name}-E\n"));
}

pub(super) fn task(
    r: &mut Request,
    name: &str,
    public: bool,
    result: &str,
    body: &str,
    args: &[(&str, &str)],
) {
    let visibility = if public { "public" } else { "private" };
    r.text.push_str(&format!("create.function as=${name} module=$module name={name} visibility={visibility} result={result} effect=task body={body}\neffect.parameter parent=${name} index=0 parameter=${name}-E\n"));
    for (arg, ty) in args {
        r.text.push_str(&format!(
            "add.parameter as=${name}_{arg} function=${name} name={arg} type={ty}\n"
        ));
    }
}

pub(super) fn call(
    r: &mut Request,
    target: &str,
    types: &[&str],
    row: &str,
    args: &[String],
) -> String {
    let value = r.call(target, types, args);
    r.effects(&value, &[row]);
    value
}

/// Graph-owned sequential traversal and data-dependent task iteration.
pub(super) fn standard(standard: &BTreeMap<String, String>) -> String {
    let mut r = Request::default();
    super::effects_traversal::fold(&mut r, standard, None);
    super::effects_traversal::iteration(&mut r);

    for name in ["task-map", "task-map-step"] {
        parameters(&mut r, name, &["Input", "Output"]);
        r.text.push_str(&format!("type.list as=@{name}-inputs item=@{name}-Input\ntype.list as=@{name}-outputs item=@{name}-Output\ntype.task-function as=@{name}-mapper result=@{name}-Output effect=@{name}-E\ntype.argument parent=@{name}-mapper index=0 type=@{name}-Input\n"));
    }
    let mapper = r.local("$task-map-step_mapper");
    let input = r.local("$task-map-step_input");
    let mapped = r.invoke(&mapper, &[input]);
    let output = r.local("$task-map-step_output");
    let body = r.call(
        &standard["list-append"],
        &["@task-map-step-Output"],
        &[output, mapped],
    );
    task(
        &mut r,
        "task-map-step",
        false,
        "@task-map-step-outputs",
        &body,
        &[
            ("mapper", "@task-map-step-mapper"),
            ("output", "@task-map-step-outputs"),
            ("input", "@task-map-step-Input"),
        ],
    );
    let step = r.function_value("$task-map-step");
    r.types(&step, &["@task-map-Input", "@task-map-Output"]);
    r.effects(&step, &["@task-map-E"]);
    let mapper = r.local("$task-map_mapper");
    let bound = r.bind(&step, &[mapper]);
    let inputs = r.local("$task-map_inputs");
    let empty = r.expression("list", "item=@task-map-Output");
    let body = call(
        &mut r,
        "$task-fold-left",
        &["@task-map-Input", "@task-map-outputs"],
        "@task-map-E",
        &[inputs, empty, bound],
    );
    task(
        &mut r,
        "task-map",
        true,
        "@task-map-outputs",
        &body,
        &[
            ("inputs", "@task-map-inputs"),
            ("mapper", "@task-map-mapper"),
        ],
    );
    r.text
}

/// One producer contains traversal policy and a pure factory, with no consumer requirements.
pub(super) fn library(standard_names: &BTreeMap<String, String>) -> String {
    // Separate symbol spaces avoid sharing structural expression identities between requests.
    let mut r = Request::default();
    r.text.push_str("create.record as=$payload module=$module name=payload visibility=public\nadd.type-parameter as=$payload-T declaration=$payload name=T\ntype.parameter as=@payload-T parameter=$payload-T\nadd.field as=$payload-value record=$payload name=value type=@payload-T\ncreate.variant as=$job module=$module name=job visibility=public\nadd.type-parameter as=$job-T declaration=$job name=T\ntype.parameter as=@job-T parameter=$job-T\ntype.application as=@job-payload declaration=$payload\ntype.argument parent=@job-payload index=0 type=@job-T\ntype.application as=@job-self declaration=$job\ntype.argument parent=@job-self index=0 type=@job-T\ntype.list as=@job-children item=@job-self\nadd.case as=$job-item variant=$job name=item payload=@job-payload\nadd.case as=$job-children variant=$job name=children payload=@job-children\n");
    parameters(&mut r, "configure-task", &["Prefix", "Input", "Output"]);
    r.text.push_str("set.type-parameter-constraint parameter=$configure-task-Prefix constraint=capture-safe\ntype.task-function as=@configure-task-source result=@configure-task-Output effect=@configure-task-E\ntype.argument parent=@configure-task-source index=0 type=@configure-task-Prefix\ntype.argument parent=@configure-task-source index=1 type=@configure-task-Input\ntype.task-function as=@configure-task-bound result=@configure-task-Output effect=@configure-task-E\ntype.argument parent=@configure-task-bound index=0 type=@configure-task-Input\ntype.application as=@configure-task-package declaration=$payload\ntype.argument parent=@configure-task-package index=0 type=@configure-task-bound\n");
    let source = r.local("$configure-task_source");
    let prefix = r.local("$configure-task_prefix");
    let descriptor = r.bind(&source, &[prefix]);
    let body = r.expression("record", "type=$payload");
    r.types(&body, &["@configure-task-bound"]);
    r.text.push_str(&format!(
        "expression.record-field parent={body} index=0 field=$payload-value value={descriptor}\n"
    ));
    r.function(
        "configure-task",
        "@configure-task-package",
        &body,
        &[
            ("prefix", "@configure-task-Prefix"),
            ("source", "@configure-task-source"),
        ],
    );
    // The traversal builder uses its own symbols; prefixes preserve their exact disjoint scope.
    let traversal = standard(standard_names).replace("$e", "$traversal-e");
    r.text.push_str(&traversal);
    r.text
        .push_str(&super::effects_iteration_program::producer(standard_names));
    r.text
}
