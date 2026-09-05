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
    let pure_fold = request.call(helper, &[], &[task_items]);
    request.text.push_str(&format!("create.function as=$task-fold module={} name=task-fold visibility=private result=i64 effect=task body={pure_fold}\nadd.parameter as=$task-items function=$task-fold name=items type=@items\n", bindings["module"]));
    let items = request.local("$write-items");
    let folded = request.call("$task-fold", &[], &[items]);
    let steps = request.expression("sequence", "");
    request.arguments(&steps, &[put, folded]);
    let transaction = request.expression(
        "transaction",
        &format!("requirement=$data binding=$transaction name=transaction body={steps}"),
    );
    request.text.push_str(&format!("create.function as=$write-fold module={} name=write-fold visibility=private result=i64 effect=task body={transaction}\neffect.requirement parent=$write-fold index=0 requirement=$data\nadd.parameter as=$write-items function=$write-fold name=items type=@items\nadd.parameter as=$write-key function=$write-fold name=key type=text\n",bindings["module"]));

    let input = request.local(&bindings["parameter"]);
    let stream = request.field(&input, "body");
    let maximum = request.integer(65536);
    let bytes = request.capability(
        &bindings["streams"],
        &standard["ByteStream.read-all"],
        &[stream, maximum],
    );
    let empty = request.expression("list", "item=i64");
    let decoded = request.call(&standard["json-decode-or"], &["@items"], &[bytes, empty]);
    let items = request.field(&decoded, "value");
    let input = request.local(&bindings["parameter"]);
    let key = request.field(&input, "query");
    let sum = request.call("$write-fold", &[], &[items, key]);
    let body = request.call(&standard["json-encode"], &["i64"], &[sum]);
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
    request.text.push_str(&format!("set.function-contract as=%contract function={} result=@response effect=task\neffect.requirement parent=%contract index=0 requirement={}\neffect.requirement parent=%contract index=1 requirement=$data\nreplace.body function={} body={response}\n",bindings["function"],bindings["streams"],bindings["function"]));
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
    request.text
}

pub(super) fn consumer(standard: &BTreeMap<String, String>, library: &str) -> String {
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
    request.text
}
