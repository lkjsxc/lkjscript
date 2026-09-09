//! Finite compact requests for the copied-executable pure-tail oracle.
//! These strings are requests only; the public executable owns validation and publication.

use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Request {
    pub text: String,
    next: usize,
}

impl Request {
    fn expression(&mut self, form: &str, fields: &str) -> String {
        let symbol = format!("$e{}", self.next);
        self.next += 1;
        self.text
            .push_str(&format!("expression.{form} as={symbol} {fields}\n"));
        symbol
    }

    fn local(&mut self, name: &str) -> String {
        self.expression("local", &format!("value={name}"))
    }

    fn integer(&mut self, value: i64) -> String {
        self.expression("i64", &format!("value={value}"))
    }

    fn arguments(&mut self, symbol: &str, arguments: &[String]) {
        for (index, argument) in arguments.iter().enumerate() {
            self.text.push_str(&format!(
                "expression.argument parent={symbol} index={index} expression={argument}\n"
            ));
        }
    }

    fn types(&mut self, symbol: &str, types: &[&str]) {
        for (index, ty) in types.iter().enumerate() {
            self.text.push_str(&format!(
                "type.argument parent={symbol} index={index} type={ty}\n"
            ));
        }
    }

    fn call(&mut self, function: &str, types: &[&str], arguments: &[String]) -> String {
        let symbol = self.expression("call", &format!("function={function}"));
        self.types(&symbol, types);
        self.arguments(&symbol, arguments);
        symbol
    }

    fn function_value(&mut self, function: &str) -> String {
        self.expression("function-value", &format!("function={function}"))
    }

    fn bind(&mut self, callee: &str, captures: &[String]) -> String {
        let result = self.expression("bind", &format!("callee={callee}"));
        self.arguments(&result, captures);
        result
    }

    fn invoke(&mut self, callee: &str, arguments: &[String]) -> String {
        let result = self.expression("invoke", &format!("function={callee}"));
        self.arguments(&result, arguments);
        result
    }

    fn function(&mut self, name: &str, result: &str, body: &str, parameters: &[(&str, &str)]) {
        self.text.push_str(&format!("create.function as=${name} module=$module name={name} visibility=public result={result} effect=pure body={body}\n"));
        for (parameter, ty) in parameters {
            self.text.push_str(&format!(
                "add.parameter as=${name}_{parameter} function=${name} name={parameter} type={ty}\n"
            ));
        }
    }

    fn target(&mut self, name: &str, result: &str, parameters: &[&str]) {
        self.text
            .push_str(&format!("type.function as=@{name} result={result}\n"));
        self.types(&format!("@{name}"), parameters);
        self.text.push_str(&format!("add.port as=${name}_port component=$component name={name} type=@{name} function=${name}\ncreate.target as=${name}_target name={name} component=$component port=${name}_port runner=command\n"));
    }

    fn test_zero(&mut self, standard: &BTreeMap<String, String>, parameter: &str) -> String {
        let value = self.local(parameter);
        let zero = self.integer(0);
        self.call(&standard["i64-equal"], &[], &[value, zero])
    }

    fn decrement(&mut self, standard: &BTreeMap<String, String>, parameter: &str) -> String {
        let value = self.local(parameter);
        let one = self.integer(1);
        self.call(&standard["subtract"], &[], &[value, one])
    }

    fn choose(&mut self, condition: &str, yes: &str, no: &str) -> String {
        self.expression(
            "if",
            &format!("condition={condition} when-true={yes} when-false={no}"),
        )
    }

    fn field(&mut self, value: &str, name: &str) -> String {
        self.expression("field", &format!("value={value} name={name}"))
    }

    fn capability(&mut self, requirement: &str, operation: &str, arguments: &[String]) -> String {
        let value = self.expression(
            "capability-call",
            &format!("requirement={requirement} operation={operation}"),
        );
        self.arguments(&value, arguments);
        value
    }
}

pub(super) fn http(
    standard: &BTreeMap<String, String>,
    bindings: &BTreeMap<String, String>,
    helper: &str,
) -> String {
    let mut request = Request::default();
    request.text.push_str(&format!("type.list as=@items item=i64\ntype.named as=@key-part declaration={}\nadd.requirement as=$data component={} name=data interface={}\nrequirement.limit parent=$data index=0 name=maximum_calls maximum=16 unit=calls\n",standard["DataKeyPart"],bindings["component"],standard["DataStore"]));
    for (index, name) in [
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
        request.text.push_str(&format!(
            "requirement.operation parent=$data index={index} operation={}\n",
            standard[&format!("DataStore.{name}")]
        ));
    }
    let space = request.expression("static-text", "value=tail");
    let key = request.local("$write-key");
    let part = request.expression(
        "variant",
        &format!("case={} payload={key}", standard["DataKeyPart.Text"]),
    );
    let key = request.expression("list", "item=@key-part");
    request.arguments(&key, &[part]);
    let written = request.expression("text", "value=written");
    let value = request.call(&standard["bytes-from-text"], &[], &[written]);
    let missing = request.expression(
        "variant",
        &format!("case={}", standard["DataExpectation.Missing"]),
    );
    let put = request.capability(
        "$data",
        &standard["DataStore.put"],
        &[space, key, value, missing],
    );
    let task_items = request.local("$task-items");
    let task_scale = request.local("$task-scale");
    let task_bias = request.local("$task-bias");
    let pure_fold = request.call(helper, &[], &[task_items, task_scale, task_bias]);
    request.text.push_str(&format!("create.function as=$task-fold module={} name=task-fold visibility=private result=@items effect=task body={pure_fold}\nadd.parameter as=$task-items function=$task-fold name=items type=@items\nadd.parameter as=$task-scale function=$task-fold name=scale type=i64\nadd.parameter as=$task-bias function=$task-fold name=bias type=i64\n", bindings["module"]));
    let items = request.local("$write-items");
    let scale = request.local("$write-scale");
    let bias = request.local("$write-bias");
    let folded = request.call("$task-fold", &[], &[items, scale, bias]);
    let space = request.expression("static-text", "value=tail");
    let key = request.local("$write-key");
    let part = request.expression(
        "variant",
        &format!("case={} payload={key}", standard["DataKeyPart.Text"]),
    );
    let key = request.expression("list", "item=@key-part");
    request.arguments(&key, &[part]);
    let value = request.local("$mapped-output");
    let bytes = request.call(&standard["json-encode"], &["@items"], &[value]);
    request.text.push_str(&format!(
        "type.named as=@data-entry declaration={}\n",
        standard["DataEntry"]
    ));
    let read_space = request.expression("static-text", "value=tail");
    let read_key = request.local("$write-key");
    let read_part = request.expression(
        "variant",
        &format!("case={} payload={read_key}", standard["DataKeyPart.Text"]),
    );
    let read_key = request.expression("list", "item=@key-part");
    request.arguments(&read_key, &[read_part]);
    let entries = request.capability("$data", &standard["DataStore.get"], &[read_space, read_key]);
    let zero = request.integer(0);
    let entry = request.call(&standard["list-get"], &["@data-entry"], &[entries, zero]);
    let revision = request.expression(
        "field",
        &format!("value={entry} field={}", standard["DataEntry.revision"]),
    );
    let exact = request.expression(
        "variant",
        &format!(
            "case={} payload={revision}",
            standard["DataExpectation.Exact"]
        ),
    );
    let save = request.capability(
        "$data",
        &standard["DataStore.put"],
        &[space, key, bytes, exact],
    );
    let value = request.local("$mapped-output");
    let saved = request.expression("sequence", "");
    request.arguments(&saved, &[save, value]);
    let returned = request.expression("let", &format!("body={saved}"));
    request.text.push_str(&format!("expression.binding parent={returned} index=0 as=$mapped-output name=mapped value={folded} type=@items\n"));
    let steps = request.expression("sequence", "");
    request.arguments(&steps, &[put, returned]);
    let transaction = request.expression(
        "transaction",
        &format!("requirement=$data binding=$transaction name=transaction body={steps}"),
    );
    request.text.push_str(&format!("create.function as=$write-fold module={} name=write-fold visibility=private result=@items effect=task body={transaction}\neffect.requirement parent=$write-fold index=0 requirement=$data\nadd.parameter as=$write-items function=$write-fold name=items type=@items\nadd.parameter as=$write-key function=$write-fold name=key type=text\nadd.parameter as=$write-scale function=$write-fold name=scale type=i64\nadd.parameter as=$write-bias function=$write-fold name=bias type=i64\n",bindings["module"]));

    let input = request.local(&bindings["parameter"]);
    let stream = request.field(&input, "body");
    let maximum = request.integer(65536);
    let bytes = request.capability(
        &bindings["streams"],
        &standard["ByteStream.read-all"],
        &[stream, maximum],
    );
    let empty = request.expression("list", "item=i64");
    request.text.push_str("type.structural-record as=@configured-input\ntype.field parent=@configured-input index=0 name=bias type=i64\ntype.field parent=@configured-input index=1 name=items type=@items\ntype.field parent=@configured-input index=2 name=scale type=i64\n");
    let one = request.integer(1);
    let zero = request.integer(0);
    let fallback = request.expression("record", "");
    for (index, (name, value)) in [("bias", zero), ("items", empty), ("scale", one)]
        .iter()
        .enumerate()
    {
        request.text.push_str(&format!(
            "expression.record-field parent={fallback} index={index} name={name} value={value}\n"
        ));
    }
    let decoded = request.call(
        &standard["json-decode-or"],
        &["@configured-input"],
        &[bytes, fallback],
    );
    let configured = request.field(&decoded, "value");
    let input_items = request.local("$request-data");
    let items = request.field(&input_items, "items");
    let input_scale = request.local("$request-data");
    let scale = request.field(&input_scale, "scale");
    let input_bias = request.local("$request-data");
    let bias = request.field(&input_bias, "bias");
    let input = request.local(&bindings["parameter"]);
    let key = request.field(&input, "query");
    let sum = request.call("$write-fold", &[], &[items, key, scale, bias]);
    let body = request.call(&standard["json-encode"], &["@items"], &[sum]);
    request.text.push_str("type.structural-record as=@header\ntype.field parent=@header index=0 name=name type=text\ntype.field parent=@header index=1 name=value type=bytes\n");
    let headers = request.expression("list", "item=@header");
    let status = request.integer(200);
    let response = request.expression("record", "");
    for (index, (name, value)) in [("body", body), ("headers", headers), ("status", status)]
        .iter()
        .enumerate()
    {
        request.text.push_str(&format!(
            "expression.record-field parent={response} index={index} name={name} value={value}\n"
        ));
    }
    request.text.push_str("type.list as=@headers item=@header\ntype.structural-record as=@response\ntype.field parent=@response index=0 name=body type=bytes\ntype.field parent=@response index=1 name=headers type=@headers\ntype.field parent=@response index=2 name=status type=i64\n");
    let retained = request.expression("let", &format!("body={response}"));
    request.text.push_str(&format!("expression.binding parent={retained} index=0 as=$request-data name=request-data value={configured} type=@configured-input\n"));
    request.text.push_str(&format!("set.function-contract as=%contract function={} result=@response effect=task\neffect.requirement parent=%contract index=0 requirement={}\neffect.requirement parent=%contract index=1 requirement=$data\nreplace.body function={} body={retained}\n",bindings["function"],bindings["streams"],bindings["function"]));
    request.text
}

pub(super) fn library(standard: &BTreeMap<String, String>) -> String {
    let mut request = Request::default();
    request.text.push_str("create.module as=$module name=library\ntype.parameter as=@A parameter=$A\ntype.parameter as=@B parameter=$B\ncreate.variant as=$branch module=$module name=Branch visibility=private\nadd.case as=$selected variant=$branch name=selected\n");
    let condition = request.test_zero(standard, "$keep_n");
    let retained = request.local("$saved");
    let next = request.decrement(standard, "$keep_n");
    let a = request.local("$saved");
    let b = request.local("$keep_b");
    let recurse = request.call("$keep", &["@A", "@B"], &[next, a, b]);
    let branch = request.choose(&condition, &retained, &recurse);
    let selected = request.expression("variant", "case=$selected");
    let matched = request.expression("match", &format!("value={selected}"));
    request.text.push_str(&format!(
        "expression.match-arm parent={matched} index=0 case=$selected body={branch}\n"
    ));
    let first = request.expression("unit", "");
    let sequence = request.expression("sequence", "");
    request.arguments(&sequence, &[first, matched]);
    let a = request.local("$keep_a");
    let body = request.expression("let", &format!("body={sequence}"));
    request.text.push_str(&format!(
        "expression.binding parent={body} index=0 as=$saved name=saved value={a} type=@A\n"
    ));
    request.function(
        "keep",
        "@A",
        &body,
        &[("n", "i64"), ("a", "@A"), ("b", "@B")],
    );
    request.text.push_str("add.type-parameter as=$A function=$keep name=A\nadd.type-parameter as=$B function=$keep name=B\n");
    request.text.push_str("type.function as=@reducer result=i64\ntype.argument parent=@reducer index=0 type=i64\ntype.argument parent=@reducer index=1 type=i64\n");
    let scale = request.local("$configured-step_scale");
    let item = request.local("$configured-step_item");
    let scaled = request.call(&standard["multiply"], &[], &[scale, item]);
    let state = request.local("$configured-step_state");
    let sum = request.call(&standard["add"], &[], &[state, scaled]);
    let bias = request.local("$configured-step_bias");
    let step = request.call(&standard["add"], &[], &[sum, bias]);
    request.text.push_str(&format!("create.function as=$configured-step module=$module name=configured-step visibility=private result=i64 effect=pure body={step}\n"));
    for name in ["scale", "bias", "state", "item"] {
        request.text.push_str(&format!("add.parameter as=$configured-step_{name} function=$configured-step name={name} type=i64\n"));
    }
    let target = request.function_value("$configured-step");
    let scale = request.local("$reducer-factory_scale");
    let bias = request.local("$reducer-factory_bias");
    let bound = request.bind(&target, &[scale, bias]);
    request.function(
        "reducer-factory",
        "@reducer",
        &bound,
        &[("scale", "i64"), ("bias", "i64")],
    );
    request.text
}

pub(super) fn consumer(
    standard: &BTreeMap<String, String>,
    library: &str,
    factory: &str,
) -> String {
    let mut request = Request::default();
    request.text.push_str("create.module as=$module name=application\ncreate.component as=$component module=$module name=application visibility=package\ntype.list as=@items item=i64\n");
    for (name, operator) in [("sum-step", "add"), ("ordered-step", "subtract")] {
        let state = request.local(&format!("${name}_state"));
        let item = request.local(&format!("${name}_item"));
        let arguments = if name == "ordered-step" {
            vec![item, state]
        } else {
            vec![state, item]
        };
        let body = request.call(&standard[operator], &[], &arguments);
        request.function(name, "i64", &body, &[("state", "i64"), ("item", "i64")]);
    }
    for (name, step) in [("sum", "$sum-step"), ("ordered", "$ordered-step")] {
        let items = request.local(&format!("${name}_items"));
        let initial = request.integer(0);
        let step = request.function_value(step);
        let body = request.call(
            &standard["list-fold-left"],
            &["i64", "i64"],
            &[items, initial, step],
        );
        request.function(name, "i64", &body, &[("items", "@items")]);
        request.target(name, "i64", &["@items"]);
    }
    for (name, other, boolean, indirect) in [
        ("count", "count", None, false),
        ("even", "odd", Some(true), true),
        ("odd", "even", Some(false), true),
        ("non-tail", "non-tail", None, false),
    ] {
        let parameter = format!("${name}_n");
        let condition = request.test_zero(standard, &parameter);
        let base = match boolean {
            Some(value) => request.expression("bool", &format!("value={value}")),
            None => request.integer(0),
        };
        let argument = request.decrement(standard, &parameter);
        let mut recursion = if indirect {
            let callee = request.function_value(&format!("${other}"));
            let invoke = request.expression("invoke", &format!("function={callee}"));
            request.arguments(&invoke, &[argument]);
            invoke
        } else {
            request.call(&format!("${other}"), &[], &[argument])
        };
        if name == "non-tail" {
            let one = request.integer(1);
            recursion = request.call(&standard["add"], &[], &[one, recursion]);
        }
        let body = request.choose(&condition, &base, &recursion);
        let result = if boolean.is_some() { "bool" } else { "i64" };
        request.function(name, result, &body, &[("n", "i64")]);
        request.target(name, result, &["i64"]);
    }
    let forever = request.call("$forever", &[], &[]);
    request.function("forever", "i64", &forever, &[]);
    request.target("forever", "i64", &[]);

    for (name, first, second) in [
        ("generic-i64", "i64", "bool"),
        ("generic-bool", "bool", "i64"),
    ] {
        let n = request.local(&format!("${name}_n"));
        let a = request.local(&format!("${name}_a"));
        let b = request.local(&format!("${name}_b"));
        let body = request.call(library, &[first, second], &[n, a, b]);
        request.function(
            name,
            first,
            &body,
            &[("n", "i64"), ("a", first), ("b", second)],
        );
        request.target(name, first, &["i64", first, second]);
    }
    for name in ["allocate", "pending-record", "pending-sequence"] {
        let parameter = format!("${name}_n");
        let condition = request.test_zero(standard, &parameter);
        let zero = request.integer(0);
        let next = request.decrement(standard, &parameter);
        let recur = request.call(&format!("${name}"), &[], &[next]);
        let branch = match name {
            "allocate" => {
                let one = request.integer(1);
                let two = request.integer(2);
                let list = request.expression("list", "item=i64");
                request.arguments(&list, &[one, two]);
                let seq = request.expression("sequence", "");
                request.arguments(&seq, &[list, recur]);
                seq
            }
            "pending-record" => {
                let record = request.expression("record", "");
                request.text.push_str(&format!(
                    "expression.record-field parent={record} index=0 name=value value={recur}\n"
                ));
                request.expression("field", &format!("value={record} name=value"))
            }
            _ => {
                let result = request.integer(123);
                let seq = request.expression("sequence", "");
                request.arguments(&seq, &[recur, result]);
                seq
            }
        };
        let body = request.choose(&condition, &zero, &branch);
        request.function(name, "i64", &body, &[("n", "i64")]);
        request.target(name, "i64", &["i64"]);
    }
    for name in ["unselected-trap", "argument-order", "callee-order"] {
        let one = request.integer(1);
        let zero = request.integer(0);
        let division = request.call(&standard["divide"], &[], &[one, zero]);
        let body = if name == "unselected-trap" {
            let condition = request.expression("bool", "value=true");
            let seven = request.integer(7);
            request.choose(&condition, &seven, &division)
        } else {
            let maximum = request.integer(i64::MAX);
            let one = request.integer(1);
            let overflow = request.call(&standard["add"], &[], &[maximum, one]);
            if name == "argument-order" {
                request.call(&standard["add"], &[], &[division, overflow])
            } else {
                let zero = request.integer(0);
                let condition = request.call(&standard["i64-equal"], &[], &[division, zero]);
                let yes = request.function_value("$count");
                let no = request.function_value("$count");
                let callee = request.choose(&condition, &yes, &no);
                let invoke = request.expression("invoke", &format!("function={callee}"));
                request.arguments(&invoke, &[overflow]);
                invoke
            }
        };
        request.function(name, "i64", &body, &[]);
        request.target(name, "i64", &[]);
    }
    let condition = request.test_zero(standard, "$forward_n");
    let items = request.local("$forward_items");
    let length = request.call(&standard["list-length"], &["i64"], &[items]);
    let next = request.decrement(standard, "$forward_n");
    let items = request.local("$forward_items");
    let recurse = request.call("$forward", &[], &[next, items]);
    let body = request.choose(&condition, &length, &recurse);
    request.function(
        "forward",
        "i64",
        &body,
        &[("n", "i64"), ("items", "@items")],
    );
    request.target("forward", "i64", &["i64", "@items"]);
    let callee = request.function_value(library);
    request.types(&callee, &["@items", "bool"]);
    let n = request.local("$forward-generic_n");
    let items = request.local("$forward-generic_items");
    let flag = request.expression("bool", "value=true");
    let invocation = request.expression("invoke", &format!("function={callee}"));
    request.arguments(&invocation, &[n, items, flag]);
    let body = request.call(&standard["list-length"], &["i64"], &[invocation]);
    request.function(
        "forward-generic",
        "i64",
        &body,
        &[("n", "i64"), ("items", "@items")],
    );
    request.target("forward-generic", "i64", &["i64", "@items"]);
    binding_consumer(&mut request, standard, factory);
    mapping_consumer(&mut request, standard, factory);
    request.text
}

fn mapping_consumer(request: &mut Request, standard: &BTreeMap<String, String>, factory: &str) {
    // The imported factory retains a private producer helper; further binding makes
    // its reducer a unary mapper, using only the ordinary public language forms.
    let scale = request.local("$map_scale");
    let bias = request.local("$map_bias");
    let returned = request.call(factory, &[], &[scale, bias]);
    let zero = request.integer(0);
    let mapper = request.bind(&returned, &[zero]);
    let input = request.local("$map_items");
    let body = request.call(&standard["list-map"], &["i64", "i64"], &[input, mapper]);
    request.function(
        "map",
        "@items",
        &body,
        &[("items", "@items"), ("scale", "i64"), ("bias", "i64")],
    );
    request.target("map", "@items", &["@items", "i64", "i64"]);

    request.text.push_str("type.structural-record as=@aliases\ntype.field parent=@aliases index=0 name=left type=@items\ntype.field parent=@aliases index=1 name=mapped type=@items\ntype.field parent=@aliases index=2 name=original type=@items\ntype.field parent=@aliases index=3 name=right type=@items\n");
    let items = request.local("$map-aliases_items");
    let three = request.integer(3);
    let five = request.integer(5);
    let mapped = request.call("$map", &[], &[items, three, five]);
    let mut values = Vec::new();
    for (name, extra) in [
        ("left", Some(99)),
        ("mapped", None),
        ("original", None),
        ("right", Some(-7)),
    ] {
        let base = request.local(if name == "original" {
            "$map-aliases_items"
        } else {
            "$mapped-list"
        });
        let value = if let Some(extra) = extra {
            let extra = request.integer(extra);
            request.call(&standard["list-append"], &["i64"], &[base, extra])
        } else {
            base
        };
        values.push((name, value));
    }
    let record = request.expression("record", "");
    for (index, (name, value)) in values.iter().enumerate() {
        request.text.push_str(&format!(
            "expression.record-field parent={record} index={index} name={name} value={value}\n"
        ));
    }
    let body = request.expression("let", &format!("body={record}"));
    request.text.push_str(&format!("expression.binding parent={body} index=0 as=$mapped-list name=mapped value={mapped} type=@items\n"));
    request.function("map-aliases", "@aliases", &body, &[("items", "@items")]);
    request.target("map-aliases", "@aliases", &["@items"]);

    request.text.push_str("type.list as=@texts item=text\n");
    let label = request.function_value("$binding-label");
    let equal = request.function_value(&standard["i64-equal"]);
    let answer = request.integer(42);
    let predicate = request.bind(&equal, &[answer]);
    let items = request.local("$map-types_items");
    let booleans = request.call(&standard["list-map"], &["i64", "bool"], &[items, predicate]);
    let text = request.call(&standard["list-map"], &["bool", "text"], &[booleans, label]);
    request.function("map-types", "@texts", &text, &[("items", "@items")]);
    request.target("map-types", "@texts", &["@items"]);

    let equal = request.function_value(&standard["i64-equal"]);
    let answer = request.integer(42);
    let inner = request.bind(&equal, &[answer]);
    let outer = request.function_value("$binding-label");
    let mapper = request.call(
        &standard["function-compose"],
        &["i64", "bool", "text"],
        &[outer, inner],
    );
    let items = request.local("$map-composed_items");
    let body = request.call(&standard["list-map"], &["i64", "text"], &[items, mapper]);
    request.function("map-composed", "@texts", &body, &[("items", "@items")]);
    request.target("map-composed", "@texts", &["@items"]);
}

fn binding_consumer(request: &mut Request, standard: &BTreeMap<String, String>, factory: &str) {
    request.text.push_str("type.function as=@thunk result=i64\ntype.function as=@reducer result=i64\ntype.argument parent=@reducer index=0 type=i64\ntype.argument parent=@reducer index=1 type=i64\ntype.structural-record as=@totals\ntype.field parent=@totals index=0 name=first type=i64\ntype.field parent=@totals index=1 name=second type=i64\n");
    let mut closures = Vec::new();
    for name in ["first", "second"] {
        let scale = request.local(&format!("$configured_{name}-scale"));
        let bias = request.local(&format!("$configured_{name}-bias"));
        closures.push(request.call(factory, &[], &[scale, bias]));
    }
    let mut totals = Vec::new();
    for name in ["first", "second"] {
        let items = request.local("$configured_items");
        let zero = request.integer(0);
        let reducer = request.local(&format!("$retained-{name}"));
        totals.push(request.call(
            &standard["list-fold-left"],
            &["i64", "i64"],
            &[items, zero, reducer],
        ));
    }
    let result = request.expression("record", "");
    for (index, name) in ["first", "second"].iter().enumerate() {
        request.text.push_str(&format!(
            "expression.record-field parent={result} index={index} name={name} value={}\n",
            totals[index]
        ));
    }
    let body = request.expression("let", &format!("body={result}"));
    for (index, name) in ["first", "second"].iter().enumerate() {
        request.text.push_str(&format!("expression.binding parent={body} index={index} as=$retained-{name} name={name} value={} type=@reducer\n", closures[index]));
    }
    request.function(
        "configured",
        "@totals",
        &body,
        &[
            ("first-scale", "i64"),
            ("first-bias", "i64"),
            ("second-scale", "i64"),
            ("second-bias", "i64"),
            ("items", "@items"),
        ],
    );
    request.target(
        "configured",
        "@totals",
        &["i64", "i64", "i64", "i64", "@items"],
    );

    let items = request.local("$retained-length_items");
    let length = request.call(&standard["list-length"], &["i64"], &[items]);
    request.function("retained-length", "i64", &length, &[("items", "@items")]);
    let condition = request.test_zero(standard, "$forward-bound_n");
    let closure = request.local("$forward-bound_closure");
    let value = request.invoke(&closure, &[]);
    let closure = request.local("$forward-bound_closure");
    let measured = request.invoke(&closure, &[]);
    let next = request.decrement(standard, "$forward-bound_n");
    let closure = request.local("$forward-bound_closure");
    let transfer = request.call("$forward-bound", &[], &[next, closure]);
    let sequence = request.expression("sequence", "");
    request.arguments(&sequence, &[measured, transfer]);
    let body = request.choose(&condition, &value, &sequence);
    request.function(
        "forward-bound",
        "i64",
        &body,
        &[("n", "i64"), ("closure", "@thunk")],
    );
    let function = request.function_value("$retained-length");
    let items = request.local("$bound-forward_items");
    let closure = request.bind(&function, &[items]);
    let n = request.local("$bound-forward_n");
    let body = request.call("$forward-bound", &[], &[n, closure]);
    request.function(
        "bound-forward",
        "i64",
        &body,
        &[("n", "i64"), ("items", "@items")],
    );
    request.target("bound-forward", "i64", &["i64", "@items"]);

    // This consumer returns and rebinds an exported factory's private implementation.
    let scale = request.local("$configured-fold_scale");
    let bias = request.local("$configured-fold_bias");
    let reducer = request.call(factory, &[], &[scale, bias]);
    let items = request.local("$configured-fold_items");
    let zero = request.integer(0);
    let body = request.call(
        &standard["list-fold-left"],
        &["i64", "i64"],
        &[items, zero, reducer],
    );
    request.function(
        "configured-fold",
        "i64",
        &body,
        &[("items", "@items"), ("scale", "i64"), ("bias", "i64")],
    );

    for shape in ["empty", "partial", "repeated", "complete", "factory"] {
        let body = if shape == "factory" {
            let three = request.integer(3);
            let five = request.integer(5);
            let reducer = request.call(factory, &[], &[three, five]);
            let one = request.integer(1);
            let rebound = request.bind(&reducer, &[one]);
            let two = request.integer(2);
            request.invoke(&rebound, &[two])
        } else {
            let target = request.function_value(&standard["add"]);
            let four = request.integer(4);
            let five = request.integer(5);
            match shape {
                "empty" => {
                    let bound = request.bind(&target, &[]);
                    request.invoke(&bound, &[four, five])
                }
                "partial" => {
                    let bound = request.bind(&target, &[four]);
                    request.invoke(&bound, &[five])
                }
                "repeated" => {
                    let first = request.bind(&target, &[four]);
                    let second = request.bind(&first, &[five]);
                    request.invoke(&second, &[])
                }
                _ => {
                    let bound = request.bind(&target, &[four, five]);
                    request.invoke(&bound, &[])
                }
            }
        };
        let name = format!("binding-{shape}");
        request.function(&name, "i64", &body, &[]);
        request.target(&name, "i64", &[]);
    }
    for reverse in [false, true] {
        let name = if reverse {
            "compose-reverse"
        } else {
            "compose-forward"
        };
        let add = request.function_value(&standard["add"]);
        let bias = request.local(&format!("${name}_bias"));
        let outer = request.bind(&add, &[bias]);
        let multiply = request.function_value(&standard["multiply"]);
        let scale = request.local(&format!("${name}_scale"));
        let inner = request.bind(&multiply, &[scale]);
        let arguments = if reverse {
            [inner, outer]
        } else {
            [outer, inner]
        };
        let composed = request.call(
            &standard["function-compose"],
            &["i64", "i64", "i64"],
            &arguments,
        );
        let input = request.local(&format!("${name}_input"));
        let complete = request.bind(&composed, &[input]);
        let body = request.invoke(&complete, &[]);
        request.function(
            name,
            "i64",
            &body,
            &[("input", "i64"), ("scale", "i64"), ("bias", "i64")],
        );
        request.target(name, "i64", &["i64", "i64", "i64"]);
    }
    let condition = request.local("$binding-label_value");
    let yes = request.expression("text", "value=true");
    let no = request.expression("text", "value=false");
    let label = request.choose(&condition, &yes, &no);
    request.function("binding-label", "text", &label, &[("value", "bool")]);
    let outer = request.function_value("$binding-label");
    let equal = request.function_value(&standard["i64-equal"]);
    let fixed = request.integer(42);
    let inner = request.bind(&equal, &[fixed]);
    let composed = request.call(
        &standard["function-compose"],
        &["i64", "bool", "text"],
        &[outer, inner],
    );
    let input = request.local("$compose-types_input");
    let body = request.invoke(&composed, &[input]);
    request.function("compose-types", "text", &body, &[("input", "i64")]);
    request.target("compose-types", "text", &["i64"]);

    for invoke in [false, true] {
        let division = request.function_value(&standard["divide"]);
        let one = request.integer(1);
        let zero = request.integer(0);
        let thunk = request.bind(&division, &[one, zero]);
        let body = if invoke {
            request.invoke(&thunk, &[])
        } else {
            let forty_two = request.integer(42);
            let sequence = request.expression("sequence", "");
            request.arguments(&sequence, &[thunk, forty_two]);
            sequence
        };
        let name = if invoke {
            "binding-thunk-trap"
        } else {
            "binding-thunk-created"
        };
        request.function(name, "i64", &body, &[]);
        request.target(name, "i64", &[]);
    }
    let target = request.function_value(&standard["add"]);
    let one = request.integer(1);
    let zero = request.integer(0);
    let first = request.call(&standard["divide"], &[], &[one, zero]);
    let maximum = request.integer(i64::MAX);
    let one = request.integer(1);
    let second = request.call(&standard["add"], &[], &[maximum, one]);
    let bound = request.bind(&target, &[first, second]);
    let body = request.invoke(&bound, &[]);
    request.function("binding-capture-order", "i64", &body, &[]);
    request.target("binding-capture-order", "i64", &[]);
    let one = request.integer(1);
    let zero = request.integer(0);
    let first = request.call(&standard["divide"], &[], &[one, zero]);
    let target = request.function_value(&standard["add"]);
    let callee = request.expression("sequence", "");
    request.arguments(&callee, &[first, target]);
    let maximum = request.integer(i64::MAX);
    let one = request.integer(1);
    let capture = request.call(&standard["add"], &[], &[maximum, one]);
    let zero = request.integer(0);
    let bound = request.bind(&callee, &[capture, zero]);
    let body = request.invoke(&bound, &[]);
    request.function("binding-callee-order", "i64", &body, &[]);
    request.target("binding-callee-order", "i64", &[]);
    let target = request.function_value(&standard["add"]);
    let mut captures = Vec::new();
    for value in [1, 2] {
        let value = request.integer(value);
        let three = request.integer(3);
        captures.push(request.call(&standard["add"], &[], &[value, three]));
    }
    let bound = request.bind(&target, &captures);
    let body = request.invoke(&bound, &[]);
    request.function("binding-capture-once", "i64", &body, &[]);
    request.target("binding-capture-once", "i64", &[]);
    let target = request.function_value(&standard["add"]);
    let four = request.integer(4);
    let five = request.integer(5);
    let bound = request.bind(&target, &[four, five]);
    request.text.push_str(&format!("create.constant as=$bound-constant module=$module name=bound-constant visibility=private type=@thunk value={bound}\n"));
    let constant = request.expression("constant", "declaration=$bound-constant");
    let body = request.invoke(&constant, &[]);
    request.function("binding-constant", "i64", &body, &[]);
    request.target("binding-constant", "i64", &[]);
}
