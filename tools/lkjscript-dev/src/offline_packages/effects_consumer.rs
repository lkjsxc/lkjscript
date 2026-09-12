//! Consumer-owned callbacks and an HTTP contract, authored through the public request grammar.
use crate::pure_tail_program::Request;
use std::collections::BTreeMap;

pub(super) fn response(
    r: &mut Request,
    s: &BTreeMap<String, String>,
    value: &str,
    ty: &str,
) -> String {
    let body = r.call(&s["json-encode"], &[ty], &[value.to_owned()]);
    let headers = r.expression("list", "item=@Header");
    let status = r.integer(200);
    let response = r.expression("record", "");
    r.text.push_str(&format!("expression.record-field parent={response} index=0 name=body value={body}\nexpression.record-field parent={response} index=1 name=headers value={headers}\nexpression.record-field parent={response} index=2 name=status value={status}\n"));
    response
}

fn key(r: &mut Request, s: &BTreeMap<String, String>) -> String {
    let text = r.expression("text", "value=repeated");
    let part = r.expression(
        "variant",
        &format!("case={} payload={text}", s["DataKeyPart.Text"]),
    );
    let key = r.expression("list", "item=@KeyPart");
    r.arguments(&key, &[part]);
    key
}

fn read(r: &mut Request, s: &BTreeMap<String, String>) -> String {
    let space = r.expression("static-text", "value=effects");
    let key = key(r, s);
    r.capability("$data", &s["DataStore.get"], &[space, key])
}

pub(super) fn task(
    r: &mut Request,
    name: &str,
    result: &str,
    requirement: &str,
    body: &str,
    args: &[(&str, &str)],
) {
    r.text.push_str(&format!("create.function as=${name} module=$module name={name} visibility=private result={result} effect=task body={body}\neffect.requirement parent=${name} index=0 requirement={requirement}\n"));
    for (arg, ty) in args {
        r.text.push_str(&format!(
            "add.parameter as=${name}_{arg} function=${name} name={arg} type={ty}\n"
        ));
    }
}

fn mode(r: &mut Request, s: &BTreeMap<String, String>, name: &str, yes: &str, no: &str) -> String {
    let input = r.local("$input");
    let actual = r.field(&input, "mode");
    let expected = r.expression("text", &format!("value={name}"));
    let matches = r.call(&s["text-equal"], &[], &[actual, expected]);
    r.expression(
        "if",
        &format!("condition={matches} when-true={yes} when-false={no}"),
    )
}

fn mapped(
    r: &mut Request,
    library: &BTreeMap<String, String>,
    callback: &str,
    row: &str,
    input_symbol: &str,
) -> String {
    let source = r.function_value(callback);
    let input = r.local(input_symbol);
    let prefix = r.field(&input, "prefix");
    let configured = r.call(
        &library["configure-task"],
        &["i64", "@Job", "i64"],
        &[prefix, source],
    );
    r.effects(&configured, &[row]);
    let mapper = r.expression(
        "field",
        &format!("value={configured} field={}", library["payload-value"]),
    );
    let input = r.local(input_symbol);
    let items = r.field(&input, "values");
    let result = r.call(&library["task-map"], &["@Job", "i64"], &[items, mapper]);
    r.effects(&result, &[row]);
    result
}

fn concrete_comparison(r: &mut Request, s: &BTreeMap<String, String>, fold: bool) {
    let start = r.text.len();
    let state_type = if fold { "i64" } else { "@Numbers" };
    let input = r.local("$concrete-jobs_input");
    let values = r.field(&input, "values");
    let input = r.local("$concrete-jobs_input");
    let prefix = r.field(&input, "prefix");
    let state = if fold {
        r.integer(0)
    } else {
        r.expression("list", "item=i64")
    };
    let lo = r.integer(0);
    let input = r.local("$concrete-jobs_input");
    let values_for_length = r.field(&input, "values");
    let hi = r.call(&s["list-length"], &["@Job"], &[values_for_length]);
    let body = r.call("$concrete-range", &[], &[values, prefix, state, lo, hi]);
    task(
        r,
        "concrete-jobs",
        state_type,
        "$config",
        &body,
        &[("input", "@Input")],
    );

    let lo = r.local("$concrete-range_lo");
    let hi = r.local("$concrete-range_hi");
    let empty = r.call(&s["i64-equal"], &[], &[lo, hi]);
    let state = r.local("$concrete-range_state");
    let hi = r.local("$concrete-range_hi");
    let lo = r.local("$concrete-range_lo");
    let width = r.call(&s["subtract"], &[], &[hi, lo]);
    let one = r.integer(1);
    let single = r.call(&s["i64-equal"], &[], &[width, one]);
    let values = r.local("$concrete-range_values");
    let index = r.local("$concrete-range_lo");
    let item = r.call(&s["list-get"], &["@Job"], &[values, index]);
    let prefix = r.local("$concrete-range_prefix");
    let mapped = r.call("$config-job", &[], &[prefix, item]);
    let accumulated = r.local("$concrete-range_state");
    let singleton = if fold {
        r.call(&s["add"], &[], &[accumulated, mapped])
    } else {
        r.call(&s["list-append"], &["i64"], &[accumulated, mapped])
    };
    let hi = r.local("$concrete-range_hi");
    let lo = r.local("$concrete-range_lo");
    let width = r.call(&s["subtract"], &[], &[hi, lo]);
    let two = r.integer(2);
    let half = r.call(&s["divide"], &[], &[width, two]);
    let lo = r.local("$concrete-range_lo");
    let mid = r.call(&s["add"], &[], &[lo, half]);
    let values = r.local("$concrete-range_values");
    let prefix = r.local("$concrete-range_prefix");
    let accumulated = r.local("$concrete-range_state");
    let lo = r.local("$concrete-range_lo");
    let middle = r.local("$concrete-mid");
    let left = r.call(
        "$concrete-range",
        &[],
        &[values, prefix, accumulated, lo, middle],
    );
    let values = r.local("$concrete-range_values");
    let prefix = r.local("$concrete-range_prefix");
    let advanced = r.local("$concrete-left");
    let middle = r.local("$concrete-mid");
    let hi = r.local("$concrete-range_hi");
    let right = r.call(
        "$concrete-range",
        &[],
        &[values, prefix, advanced, middle, hi],
    );
    let split = r.expression("let", &format!("body={right}"));
    r.text.push_str(&format!("expression.binding parent={split} index=0 as=$concrete-mid name=mid type=i64 value={mid}\nexpression.binding parent={split} index=1 as=$concrete-left name=left type={state_type} value={left}\n"));
    let nonempty = r.expression(
        "if",
        &format!("condition={single} when-true={singleton} when-false={split}"),
    );
    let body = r.expression(
        "if",
        &format!("condition={empty} when-true={state} when-false={nonempty}"),
    );
    task(
        r,
        "concrete-range",
        state_type,
        "$config",
        &body,
        &[
            ("values", "@Jobs"),
            ("prefix", "i64"),
            ("state", state_type),
            ("lo", "i64"),
            ("hi", "i64"),
        ],
    );
    if fold {
        let code = r.text.split_off(start);
        r.text.push_str(
            &code
                .replace("$concrete-", "$comparison-fold-")
                .replace("name=concrete-", "name=comparison-fold-"),
        );
    }
}

fn result_values(
    r: &mut Request,
    s: &BTreeMap<String, String>,
    library: &BTreeMap<String, String>,
) -> String {
    // Ordinary graph-owned Result data; either case is an element, not a traversal command.
    r.text.push_str("create.variant as=$callback-result module=$module name=callback-result visibility=private\nadd.type-parameter as=$result-Value declaration=$callback-result name=Value\ntype.parameter as=@result-Value parameter=$result-Value\nadd.case as=$result-ok variant=$callback-result name=ok payload=@result-Value\nadd.case as=$result-error variant=$callback-result name=error payload=@result-Value\ntype.application as=@Result declaration=$callback-result\ntype.argument parent=@Result index=0 type=i64\ntype.list as=@Results item=@Result\n");
    let prefix = r.local("$result-job_prefix");
    let job = r.local("$result-job_job");
    let value = r.call("$config-job", &[], &[prefix, job]);
    let stored = r.local("$result-value");
    let fifteen = r.integer(15);
    let failed = r.call(&s["i64-equal"], &[], &[stored, fifteen]);
    let stored = r.local("$result-value");
    let error = r.expression("variant", &format!("case=$result-error payload={stored}"));
    r.types(&error, &["i64"]);
    let stored = r.local("$result-value");
    let ok = r.expression("variant", &format!("case=$result-ok payload={stored}"));
    r.types(&ok, &["i64"]);
    let body = r.expression(
        "if",
        &format!("condition={failed} when-true={error} when-false={ok}"),
    );
    let body = r.expression("let", &format!("body={body}"));
    r.text.push_str(&format!("expression.binding parent={body} index=0 as=$result-value name=value type=i64 value={value}\n"));
    task(
        r,
        "result-job",
        "@Result",
        "$config",
        &body,
        &[("prefix", "i64"), ("job", "@Job")],
    );
    let source = r.function_value("$result-job");
    let input = r.local("$results-jobs_input");
    let prefix = r.field(&input, "prefix");
    let configured = r.call(
        &library["configure-task"],
        &["i64", "@Job", "@Result"],
        &[prefix, source],
    );
    r.effects(&configured, &["@Config"]);
    let mapper = r.expression(
        "field",
        &format!("value={configured} field={}", library["payload-value"]),
    );
    let input = r.local("$results-jobs_input");
    let items = r.field(&input, "values");
    let mapped = r.call(&library["task-map"], &["@Job", "@Result"], &[items, mapper]);
    r.effects(&mapped, &["@Config"]);
    task(
        r,
        "results-jobs",
        "@Results",
        "$config",
        &mapped,
        &[("input", "@Input")],
    );
    let input = r.local("$input");
    let result = r.call("$results-jobs", &[], &[input]);
    response(r, s, &result, "@Results")
}

fn nested_map(
    r: &mut Request,
    s: &BTreeMap<String, String>,
    library: &BTreeMap<String, String>,
) -> String {
    let callback = r.function_value("$config-job");
    let prefix = r.local("$nested-map-job_prefix");
    let callback = r.bind(&callback, &[prefix]);
    let item = r.local("$nested-map-job_job");
    let items = r.expression("list", "item=@Job");
    r.arguments(&items, &[item]);
    let inner = r.call(&library["task-map"], &["@Job", "i64"], &[items, callback]);
    r.effects(&inner, &["@Config"]);
    let zero = r.integer(0);
    let result = r.call(&s["list-get"], &["i64"], &[inner, zero]);
    task(
        r,
        "nested-map-job",
        "i64",
        "$config",
        &result,
        &[("prefix", "i64"), ("job", "@Job")],
    );
    let result = mapped(r, library, "$nested-map-job", "@Config", "$input");
    response(r, s, &result, "@Numbers")
}

pub(super) fn program(
    s: &BTreeMap<String, String>,
    b: &BTreeMap<String, String>,
    library: &BTreeMap<String, String>,
) -> String {
    let mut r = Request::default();
    r.http_request_type();
    r.text.push_str(&format!("type.application as=@Job declaration={}\ntype.argument parent=@Job index=0 type=i64\ntype.application as=@Payload declaration={}\ntype.argument parent=@Payload index=0 type=i64\ntype.list as=@Jobs item=@Job\ntype.list as=@Numbers item=i64\ntype.named as=@KeyPart declaration={}\ntype.named as=@Entry declaration={}\ntype.list as=@Entries item=@Entry\nadd.requirement as=$config component={} name=config interface={}\nrequirement.operation parent=$config index=0 operation={}\nrequirement.limit parent=$config index=0 name=maximum_calls maximum=50000 unit=calls\neffect.row as=@Config\neffect.requirement parent=@Config index=0 requirement=$config\nadd.requirement as=$data component={} name=data interface={}\nrequirement.limit parent=$data index=0 name=maximum_calls maximum=50000 unit=calls\neffect.row as=@Data\neffect.requirement parent=@Data index=0 requirement=$data\n",library["job"],library["payload"],s["DataKeyPart"],s["DataEntry"],b["component"],s["Configuration"],s["Configuration.i64"],b["component"],s["DataStore"]));
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
    r.text.push_str("type.structural-record as=@Input\ntype.field parent=@Input index=0 name=mode type=text\ntype.field parent=@Input index=1 name=prefix type=i64\ntype.field parent=@Input index=2 name=values type=@Jobs\ntype.structural-record as=@Header\ntype.field parent=@Header index=0 name=name type=text\ntype.field parent=@Header index=1 name=value type=bytes\ntype.list as=@Headers item=@Header\ntype.structural-record as=@Response\ntype.field parent=@Response index=0 name=body type=bytes\ntype.field parent=@Response index=1 name=headers type=@Headers\ntype.field parent=@Response index=2 name=status type=i64\n");

    let payload = r.local("$sum-item");
    let leaf = r.expression(
        "field",
        &format!("value={payload} field={}", library["payload-value"]),
    );
    let children = r.local("$sum-children");
    let callback = r.function_value("$sum-job");
    let values = r.call(&s["list-map"], &["@Job", "i64"], &[children, callback]);
    let zero = r.integer(0);
    let plus = r.function_value(&s["add"]);
    let total = r.call(&s["list-fold-left"], &["i64", "i64"], &[values, zero, plus]);
    let value = r.local("$sum-job_value");
    let body = r.expression("match", &format!("value={value}"));
    r.text.push_str(&format!("expression.match-arm parent={body} index=0 case={} as=$sum-item name=item type=@Payload body={leaf}\nexpression.match-arm parent={body} index=1 case={} as=$sum-children name=children type=@Jobs body={total}\n", library["job-item"], library["job-children"]));
    r.function("sum-job", "i64", &body, &[("value", "@Job")]);

    let job = r.local("$config-job_job");
    let value = r.call("$sum-job", &[], &[job]);
    let name = r.expression("static-text", "value=scale");
    let scale = r.capability("$config", &s["Configuration.i64"], &[name]);
    let product = r.call(&s["multiply"], &[], &[value, scale]);
    let prefix = r.local("$config-job_prefix");
    let biased = r.call(&s["add"], &[], &[prefix, product]);
    let name = r.expression("static-text", "value=bias");
    let bias = r.capability("$config", &s["Configuration.i64"], &[name]);
    let result = r.call(&s["add"], &[], &[biased, bias]);
    task(
        &mut r,
        "config-job",
        "i64",
        "$config",
        &result,
        &[("prefix", "i64"), ("job", "@Job")],
    );
    let state = r.local("$config-step_state");
    let prefix = r.local("$config-step_prefix");
    let job = r.local("$config-step_job");
    let value = r.call("$config-job", &[], &[prefix, job]);
    let result = r.call(&s["add"], &[], &[state, value]);
    task(
        &mut r,
        "config-step",
        "i64",
        "$config",
        &result,
        &[("prefix", "i64"), ("state", "i64"), ("job", "@Job")],
    );

    let entries = read(&mut r, s);
    let prior = r.local("$prior");
    let length = r.call(&s["list-length"], &["@Entry"], &[prior]);
    let zero = r.integer(0);
    let missing = r.call(&s["i64-equal"], &[], &[length, zero]);
    let prior = r.local("$prior");
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
    let absent = r.expression("variant", &format!("case={}", s["DataExpectation.Missing"]));
    let expectation = r.expression(
        "if",
        &format!("condition={missing} when-true={absent} when-false={exact}"),
    );
    let value = r.local("$data-value");
    let bytes = r.call(&s["data-encode"], &["i64"], &[value]);
    let space = r.expression("static-text", "value=effects");
    let key = key(&mut r, s);
    let put = r.capability(
        "$data",
        &s["DataStore.put"],
        &[space, key, bytes, expectation],
    );
    let value = r.local("$data-value");
    let sequence = r.expression("sequence", "");
    r.arguments(&sequence, &[put, value]);
    let writing = r.expression("let", &format!("body={sequence}"));
    r.text.push_str(&format!("expression.binding parent={writing} index=0 as=$prior name=prior type=@Entries value={entries}\n"));
    let value = r.local("$data-sum");
    let negative = r.integer(-1);
    let fails = r.call(&s["i64-equal"], &[], &[value, negative]);
    let one = r.integer(1);
    let zero = r.integer(0);
    let failure = r.call(&s["divide"], &[], &[one, zero]);
    let guarded = r.expression(
        "if",
        &format!("condition={fails} when-true={failure} when-false={writing}"),
    );
    let job = r.local("$data-job_job");
    let sum = r.call("$sum-job", &[], &[job]);
    let prefix = r.local("$data-job_prefix");
    let value = r.local("$data-sum");
    let value = r.call(&s["add"], &[], &[prefix, value]);
    let body = r.expression("let", &format!("body={guarded}"));
    r.text.push_str(&format!("expression.binding parent={body} index=0 as=$data-sum name=sum type=i64 value={sum}\nexpression.binding parent={body} index=1 as=$data-value name=value type=i64 value={value}\n"));
    task(
        &mut r,
        "data-job",
        "i64",
        "$data",
        &body,
        &[("prefix", "i64"), ("job", "@Job")],
    );
    let prefix = r.local("$nested-job_prefix");
    let job = r.local("$nested-job_job");
    let result = r.call("$data-job", &[], &[prefix, job]);
    let transaction = r.expression(
        "transaction",
        &format!("requirement=$data binding=$nested-tx name=nested body={result}"),
    );
    task(
        &mut r,
        "nested-job",
        "i64",
        "$data",
        &transaction,
        &[("prefix", "i64"), ("job", "@Job")],
    );

    let configured = mapped(
        &mut r,
        library,
        "$config-job",
        "@Config",
        "$mapped-jobs_input",
    );
    task(
        &mut r,
        "mapped-jobs",
        "@Numbers",
        "$config",
        &configured,
        &[("input", "@Input")],
    );
    let input = r.local("$input");
    let configured = r.call("$mapped-jobs", &[], &[input]);
    let map_response = response(&mut r, s, &configured, "@Numbers");
    let input = r.local("$folded-jobs_input");
    let prefix = r.field(&input, "prefix");
    let step = r.function_value("$config-step");
    let step = r.bind(&step, &[prefix]);
    let input = r.local("$folded-jobs_input");
    let items = r.field(&input, "values");
    let zero = r.integer(0);
    let fold = r.call(
        &library["task-fold-left"],
        &["@Job", "i64"],
        &[items, zero, step],
    );
    r.effects(&fold, &["@Config"]);
    task(
        &mut r,
        "folded-jobs",
        "i64",
        "$config",
        &fold,
        &[("input", "@Input")],
    );
    let input = r.local("$input");
    let fold = r.call("$folded-jobs", &[], &[input]);
    let fold_response = response(&mut r, s, &fold, "i64");
    concrete_comparison(&mut r, s, false);
    concrete_comparison(&mut r, s, true);
    let results_response = result_values(&mut r, s, library);
    // The traversal has no enclosing transaction; each callback owns and commits its write.
    let written = mapped(&mut r, library, "$nested-job", "@Data", "$input");
    let write_response = response(&mut r, s, &written, "@Numbers");
    let written = mapped(
        &mut r,
        library,
        "$data-job",
        "@Data",
        "$transaction-jobs_input",
    );
    let transaction = r.expression(
        "transaction",
        &format!("requirement=$data binding=$outer-tx name=outer body={written}"),
    );
    task(
        &mut r,
        "transaction-jobs",
        "@Numbers",
        "$data",
        &transaction,
        &[("input", "@Input")],
    );
    let input = r.local("$input");
    let transaction = r.call("$transaction-jobs", &[], &[input]);
    let transaction_response = response(&mut r, s, &transaction, "@Numbers");
    let written = mapped(&mut r, library, "$nested-job", "@Data", "$input");
    let transaction = r.expression(
        "transaction",
        &format!("requirement=$data binding=$outer-nested-tx name=outer body={written}"),
    );
    let nested_response = response(&mut r, s, &transaction, "@Numbers");
    let entries = read(&mut r, s);
    let zero = r.integer(0);
    let entry = r.call(&s["list-get"], &["@Entry"], &[entries, zero]);
    let bytes = r.expression(
        "field",
        &format!("value={entry} field={}", s["DataEntry.value"]),
    );
    let fallback = r.integer(-999);
    let value = r.call(&s["data-decode-or"], &["i64"], &[bytes, fallback]);
    let read_response = response(&mut r, s, &value, "i64");
    let body = mode(&mut r, s, "read", &read_response, &map_response);
    let body = mode(&mut r, s, "fold", &fold_response, &body);
    let body = mode(&mut r, s, "write", &write_response, &body);
    let body = mode(&mut r, s, "transaction", &transaction_response, &body);
    let body = mode(&mut r, s, "nested", &nested_response, &body);
    let body = mode(&mut r, s, "results", &results_response, &body);
    let nested_map_response = nested_map(&mut r, s, library);
    let body = mode(&mut r, s, "nested-map", &nested_map_response, &body);
    let mut body = body;
    for (name, response) in super::effects_iteration_program::consumer(&mut r, s, library) {
        body = mode(&mut r, s, &name, &response, &body);
    }
    let input = r.local(&b["parameter"]);
    let stream = r.field(&input, "body");
    let maximum = r.integer(1_048_576);
    let bytes = r.capability(&b["streams"], &s["ByteStream.read-all"], &[stream, maximum]);
    let mode = r.expression("text", "value=map");
    let prefix = r.integer(0);
    let values = r.expression("list", "item=@Job");
    let fallback = r.expression("record", "");
    r.text.push_str(&format!("expression.record-field parent={fallback} index=0 name=mode value={mode}\nexpression.record-field parent={fallback} index=1 name=prefix value={prefix}\nexpression.record-field parent={fallback} index=2 name=values value={values}\n"));
    let input = r.call(&s["json-decode-or"], &["@Input"], &[bytes, fallback]);
    let input = r.field(&input, "value");
    let body = r.expression("let", &format!("body={body}"));
    r.text.push_str(&format!("expression.binding parent={body} index=0 as=$input name=input type=@Input value={input}\nset.function-contract as=%contract function={} result=@Response effect=task\neffect.requirement parent=%contract index=0 requirement={}\neffect.requirement parent=%contract index=1 requirement=$config\neffect.requirement parent=%contract index=2 requirement=$data\nreplace.body function={} body={body}\neffect.row as=@Handler\neffect.requirement parent=@Handler index=0 requirement={}\neffect.requirement parent=@Handler index=1 requirement=$config\neffect.requirement parent=@Handler index=2 requirement=$data\ntype.task-function as=@Handler result=@Response effect=@Handler\ntype.argument parent=@Handler index=0 type={}\nset.port-contract port={} type=@Handler\n",b["function"],b["streams"],b["function"],b["streams"],"@http-request",b["port"]));
    r.text
        .replace("module=$module", &format!("module={}", b["module"]))
}
