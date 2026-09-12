//! Public requests for two ordinary foreground consumers of the transported task library.
use crate::pure_tail_program::Request;
use std::collections::BTreeMap;

fn task(
    r: &mut Request,
    name: &str,
    result: &str,
    body: &str,
    parameters: &[(&str, &str)],
    requirements: &[&str],
) {
    r.text.push_str(&format!("create.function as=${name} module=$module name={name} visibility=private result={result} effect=task body={body}\n"));
    for (index, requirement) in requirements.iter().enumerate() {
        r.text.push_str(&format!(
            "effect.requirement parent=${name} index={index} requirement={requirement}\n"
        ));
    }
    for (name_arg, ty) in parameters {
        r.text.push_str(&format!(
            "add.parameter as=${name}_{name_arg} function=${name} name={name_arg} type={ty}\n"
        ));
    }
}

fn target(
    r: &mut Request,
    name: &str,
    component: &str,
    result: &str,
    row: &str,
    parameters: &[&str],
) {
    r.text.push_str(&format!(
        "type.task-function as=@{name}-port result={result} effect={row}\n"
    ));
    r.types(&format!("@{name}-port"), parameters);
    r.text.push_str(&format!("add.port as=${name}-port component={component} name={name} type=@{name}-port function=${name}\ncreate.target as=${name}-target name={name} component={component} port=${name}-port runner=command\n"));
}

fn binding(r: &mut Request, name: &str, ty: &str, value: &str, body: &str) -> String {
    let expression = r.expression("let", &format!("body={body}"));
    r.text.push_str(&format!("expression.binding parent={expression} index=0 as=${name} name={name} type={ty} value={value}\n"));
    expression
}

fn sequence(r: &mut Request, values: &[String]) -> String {
    let expression = r.expression("sequence", "");
    r.arguments(&expression, values);
    expression
}

fn key(r: &mut Request, s: &BTreeMap<String, String>, text: &str, numbers: &[String]) -> String {
    let text = r.expression("text", &format!("value={text}"));
    let text = r.expression(
        "variant",
        &format!("case={} payload={text}", s["DataKeyPart.Text"]),
    );
    let mut parts = vec![text];
    for number in numbers {
        parts.push(r.expression(
            "variant",
            &format!("case={} payload={number}", s["DataKeyPart.I64"]),
        ));
    }
    let key = r.expression("list", "item=@KeyPart");
    r.arguments(&key, &parts);
    key
}

fn get(r: &mut Request, s: &BTreeMap<String, String>, key: String) -> String {
    let space = r.expression("static-text", "value=foreground");
    r.capability("$data", &s["DataStore.get"], &[space, key])
}

fn write(
    r: &mut Request,
    s: &BTreeMap<String, String>,
    key: String,
    value: String,
    ty: &str,
) -> String {
    let bytes = r.call(&s["json-encode"], &[ty], &[value]);
    r.call("$write", &[], &[key, bytes])
}

pub(super) fn consumer(s: &BTreeMap<String, String>, library: &BTreeMap<String, String>) -> String {
    let mut r = Request::default();
    r.text.push_str(&format!("create.component as=$component module=$module name=application visibility=package\ncreate.component as=$empty-component module=$module name=ungranted visibility=package\ntype.named as=@State declaration={}\ntype.named as=@Output declaration={}\ntype.application as=@Job declaration={}\ntype.argument parent=@Job index=0 type=i64\ntype.named as=@KeyPart declaration={}\ntype.list as=@Key item=@KeyPart\ntype.named as=@Entry declaration={}\ntype.list as=@Entries item=@Entry\neffect.row as=@Empty\n", library["iteration-state"],library["iteration-output"],library["job"],s["DataKeyPart"],s["DataEntry"]));
    for (requirement, interface) in [("config", "Configuration"), ("data", "DataStore")] {
        r.text.push_str(&format!("add.requirement as=${requirement} component=$component name={requirement} interface={}\nrequirement.limit parent=${requirement} index=0 name=maximum_calls maximum=100000 unit=calls\n", s[interface]));
    }
    r.text.push_str(&format!("requirement.operation parent=$config index=0 operation={}\neffect.row as=@Effects\neffect.requirement parent=@Effects index=0 requirement=$config\neffect.requirement parent=@Effects index=1 requirement=$data\neffect.row as=@Data\neffect.requirement parent=@Data index=0 requirement=$data\n",s["Configuration.i64"]));
    for (index, operation) in [
        "schema-read",
        "schema-set",
        "get",
        "scan",
        "put",
        "delete",
        "transaction",
    ]
    .iter()
    .enumerate()
    {
        r.text.push_str(&format!(
            "requirement.operation parent=$data index={index} operation={}\n",
            s[&format!("DataStore.{operation}")]
        ));
    }
    r.text.push_str("create.record as=$report module=$module name=report visibility=public\ntype.named as=@Report declaration=$report\nadd.field as=$report-executions record=$report name=executions type=i64\nadd.field as=$report-result record=$report name=result type=@Output\nadd.field as=$report-payload record=$report name=payload type=@Job\n");

    let selected = r.local("$read-integer_key");
    let entries = get(&mut r, s, selected);
    let prior = r.local("$read-prior");
    let count = r.call(&s["list-length"], &["@Entry"], &[prior]);
    let zero = r.integer(0);
    let empty = r.call(&s["i64-equal"], &[], &[count, zero]);
    let prior = r.local("$read-prior");
    let zero = r.integer(0);
    let entry = r.call(&s["list-get"], &["@Entry"], &[prior, zero]);
    let bytes = r.expression(
        "field",
        &format!("value={entry} field={}", s["DataEntry.value"]),
    );
    let fallback = r.integer(-999);
    let decoded = r.call(&s["json-decode-or"], &["i64"], &[bytes, fallback]);
    let decoded = r.field(&decoded, "value");
    let zero = r.integer(0);
    let result = r.choose(&empty, &zero, &decoded);
    let body = binding(&mut r, "read-prior", "@Entries", &entries, &result);
    task(
        &mut r,
        "read-integer",
        "i64",
        &body,
        &[("key", "@Key")],
        &["$data"],
    );

    let selected = r.local("$write_key");
    let entries = get(&mut r, s, selected);
    let prior = r.local("$write-prior");
    let count = r.call(&s["list-length"], &["@Entry"], &[prior]);
    let zero = r.integer(0);
    let empty = r.call(&s["i64-equal"], &[], &[count, zero]);
    let prior = r.local("$write-prior");
    let zero = r.integer(0);
    let entry = r.call(&s["list-get"], &["@Entry"], &[prior, zero]);
    let revision = r.expression(
        "field",
        &format!("value={entry} field={}", s["DataEntry.revision"]),
    );
    let exact = r.expression(
        "variant",
        &format!("case={} payload={revision}", s["DataExpectation.Exact"]),
    );
    let missing = r.expression("variant", &format!("case={}", s["DataExpectation.Missing"]));
    let expected = r.choose(&empty, &missing, &exact);
    let space = r.expression("static-text", "value=foreground");
    let selected = r.local("$write_key");
    let bytes = r.local("$write_bytes");
    let put = r.capability(
        "$data",
        &s["DataStore.put"],
        &[space, selected, bytes, expected],
    );
    let unit = r.expression("unit", "");
    let body = sequence(&mut r, &[put, unit]);
    let body = binding(&mut r, "write-prior", "@Entries", &entries, &body);
    task(
        &mut r,
        "write",
        "unit",
        &body,
        &[("key", "@Key"), ("bytes", "bytes")],
        &["$data"],
    );

    let tick_key = key(&mut r, s, "tick", &[]);
    let tick = r.call("$read-integer", &[], &[tick_key]);
    let execution = r.local("$observe_execution");
    let position = r.local("$tick");
    let selected = key(&mut r, s, "trace", &[execution, position]);
    let cursor = r.local("$observe_cursor");
    let trace = write(&mut r, s, selected, cursor, "i64");
    let tick_key = key(&mut r, s, "tick", &[]);
    let tick_value = r.local("$tick");
    let one = r.integer(1);
    let next = r.call(&s["add"], &[], &[tick_value, one]);
    let advance = write(&mut r, s, tick_key, next, "i64");
    let body = sequence(&mut r, &[trace, advance]);
    let body = binding(&mut r, "tick", "i64", &tick, &body);
    // Keep the complete small-case trace inside the existing per-transaction mutation bound.
    // Large runs retain this ordered prefix and their independently checked final report.
    let cursor = r.local("$observe_cursor");
    let limit = r.integer(258);
    let traced = r.call(&s["less"], &[], &[cursor, limit]);
    let unit = r.expression("unit", "");
    let body = r.choose(&traced, &body, &unit);
    task(
        &mut r,
        "observe",
        "unit",
        &body,
        &[("execution", "i64"), ("cursor", "i64")],
        &["$data"],
    );

    let selected = key(&mut r, s, "executions", &[]);
    let prior = r.call("$read-integer", &[], &[selected]);
    let one = r.integer(1);
    let executions = r.call(&s["add"], &[], &[prior, one]);
    let selected = key(&mut r, s, "executions", &[]);
    let value = r.local("$execution");
    let increment = write(&mut r, s, selected, value, "i64");
    let selected = key(&mut r, s, "tick", &[]);
    let zero = r.integer(0);
    let reset = write(&mut r, s, selected, zero, "i64");
    let observer = r.function_value("$observe");
    let execution = r.local("$execution");
    let observer = r.bind(&observer, &[execution]);
    let name = r.expression("static-text", "value=stride");
    let stride = r.capability("$config", &s["Configuration.i64"], &[name]);
    let name = r.expression("static-text", "value=offset");
    let offset = r.capability("$config", &s["Configuration.i64"], &[name]);
    let n = r.local("$batch_n");
    let threshold = r.call(&s["add"], &[], &[n, offset]);
    let callback = r.call(
        &library["make-iteration"],
        &[],
        &[stride, threshold, observer],
    );
    r.effects(&callback, &["@Data"]);
    let zero_a = r.integer(0);
    let zero_b = r.integer(0);
    let state = r.expression("record", &format!("type={}", library["iteration-state"]));
    r.text.push_str(&format!("expression.record-field parent={state} index=0 field={} value={zero_a}\nexpression.record-field parent={state} index=1 field={} value={zero_b}\n",library["iteration-state-cursor"],library["iteration-state-sum"]));
    let iterated = r.call(
        &library["task-iterate"],
        &["@State", "@Output"],
        &[state, callback],
    );
    r.effects(&iterated, &["@Data"]);
    let out = r.local("$out");
    let sum = r.expression(
        "field",
        &format!("value={out} field={}", library["iteration-output-total"]),
    );
    let payload = r.expression("record", &format!("type={}", library["payload"]));
    r.types(&payload, &["i64"]);
    r.text.push_str(&format!(
        "expression.record-field parent={payload} index=0 field={} value={sum}\n",
        library["payload-value"]
    ));
    let item = r.expression(
        "variant",
        &format!("case={} payload={payload}", library["job-item"]),
    );
    r.types(&item, &["i64"]);
    let items = r.expression("list", "item=@Job");
    r.arguments(&items, &[item]);
    let payload = r.expression(
        "variant",
        &format!("case={} payload={items}", library["job-children"]),
    );
    r.types(&payload, &["i64"]);
    let execution = r.local("$execution");
    let out = r.local("$out");
    let report = r.expression("record", "type=$report");
    r.text.push_str(&format!("expression.record-field parent={report} index=0 field=$report-executions value={execution}\nexpression.record-field parent={report} index=1 field=$report-result value={out}\nexpression.record-field parent={report} index=2 field=$report-payload value={payload}\n"));
    let selected = key(&mut r, s, "report", &[]);
    let value = r.local("$report-value");
    let stored = write(&mut r, s, selected, value, "@Report");
    let value = r.local("$report-value");
    let body = sequence(&mut r, &[stored, value]);
    let body = binding(&mut r, "report-value", "@Report", &report, &body);
    let body = binding(&mut r, "out", "@Output", &iterated, &body);
    let body = sequence(&mut r, &[increment, reset, body]);
    let body = binding(&mut r, "execution", "i64", &executions, &body);
    task(
        &mut r,
        "batch",
        "@Report",
        &body,
        &[("n", "i64")],
        &["$config", "$data"],
    );

    for name in ["main", "rollback", "trap", "cancel-after-commit"] {
        let n = r.local(&format!("${name}_n"));
        let call = r.call(
            if matches!(name, "trap" | "cancel-after-commit") {
                "$main"
            } else {
                "$batch"
            },
            &[],
            &[n],
        );
        let value = r.local(&format!("${name}-result"));
        let body = if name == "cancel-after-commit" {
            let n = r.integer(i64::MAX);
            let running = r.call("$loop", &[], &[n]);
            sequence(&mut r, &[running, value])
        } else if name == "main" {
            value
        } else {
            let one = r.integer(1);
            let zero = r.integer(0);
            let failure = r.call(&s["divide"], &[], &[one, zero]);
            sequence(&mut r, &[failure, value])
        };
        let body = binding(&mut r, &format!("{name}-result"), "@Report", &call, &body);
        let body = if matches!(name, "main" | "rollback") {
            r.expression(
                "transaction",
                &format!("requirement=$data binding=${name}-tx name=transaction body={body}"),
            )
        } else {
            body
        };
        task(
            &mut r,
            name,
            "@Report",
            &body,
            &[("n", "i64")],
            &["$config", "$data"],
        );
        target(&mut r, name, "$component", "@Report", "@Effects", &["i64"]);
    }
    let selected = key(&mut r, s, "executions", &[]);
    let read = r.call("$read-integer", &[], &[selected]);
    task(&mut r, "count", "i64", &read, &[], &["$data"]);
    target(&mut r, "count", "$component", "i64", "@Data", &[]);

    let unit = r.expression("unit", "");
    task(&mut r, "noop", "unit", &unit, &[("cursor", "i64")], &[]);
    let observer = r.function_value("$noop");
    let stride = r.integer(1);
    let n = r.local("$loop_n");
    let callback = r.call(&library["make-iteration"], &[], &[stride, n, observer]);
    r.effects(&callback, &["@Empty"]);
    let zero_a = r.integer(0);
    let zero_b = r.integer(0);
    let state = r.expression("record", &format!("type={}", library["iteration-state"]));
    r.text.push_str(&format!("expression.record-field parent={state} index=0 field={} value={zero_a}\nexpression.record-field parent={state} index=1 field={} value={zero_b}\n",library["iteration-state-cursor"],library["iteration-state-sum"]));
    let body = r.call(
        &library["task-iterate"],
        &["@State", "@Output"],
        &[state, callback],
    );
    r.effects(&body, &["@Empty"]);
    task(&mut r, "loop", "@Output", &body, &[("n", "i64")], &[]);
    target(
        &mut r,
        "loop",
        "$empty-component",
        "@Output",
        "@Empty",
        &["i64"],
    );
    r.text.push_str("create.variant as=$unsupported-output module=$module name=unsupported-output visibility=private\ntype.named as=@Unsupported declaration=$unsupported-output\ntype.option as=@IntrinsicOption item=unit\nadd.case as=$ready variant=$unsupported-output name=ready\nadd.case as=$unsupported variant=$unsupported-output name=unsupported payload=@IntrinsicOption\n");
    let ready = r.expression("variant", "case=$ready");
    task(&mut r, "unencodable", "@Unsupported", &ready, &[], &[]);
    target(
        &mut r,
        "unencodable",
        "$component",
        "@Unsupported",
        "@Empty",
        &[],
    );
    let zero = r.integer(0);
    r.function("pure-requiring-component", "i64", &zero, &[]);
    r.text.push_str("type.function as=@pure-port result=i64\nadd.port as=$pure-port component=$component name=pure-requiring-component type=@pure-port function=$pure-requiring-component\ncreate.target as=$pure-target name=pure-requiring-component component=$component port=$pure-port runner=command\n");
    let count = r.local("$grow_count");
    let zero = r.integer(0);
    let done = r.call(&s["i64-equal"], &[], &[count, zero]);
    let value = r.local("$grow_value");
    let left = r.local("$grow_value");
    let right = r.local("$grow_value");
    let doubled = r.call(&s["text-concat"], &[], &[left, right]);
    let count = r.local("$grow_count");
    let one = r.integer(1);
    let next = r.call(&s["subtract"], &[], &[count, one]);
    let again = r.call("$grow", &[], &[next, doubled]);
    let body = r.choose(&done, &value, &again);
    task(
        &mut r,
        "grow",
        "text",
        &body,
        &[("count", "i64"), ("value", "text")],
        &[],
    );
    let n = r.integer(0);
    let committed = r.call("$main", &[], &[n]);
    let count = r.integer(21);
    let text = r.expression("text", "value=x");
    let oversized = r.call("$grow", &[], &[count, text]);
    let body = sequence(&mut r, &[committed, oversized]);
    task(
        &mut r,
        "oversized",
        "text",
        &body,
        &[],
        &["$config", "$data"],
    );
    target(&mut r, "oversized", "$component", "text", "@Effects", &[]);
    r.text
}
