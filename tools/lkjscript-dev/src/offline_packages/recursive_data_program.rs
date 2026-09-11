//! Typed recursive persistence requests, interpreted only by the copied executable.
use crate::pure_tail_program::Request;
use std::collections::BTreeMap;

fn stored_key(r: &mut Request, s: &BTreeMap<String, String>) -> String {
    let text = r.expression("text", "value=retained");
    let part = r.expression(
        "variant",
        &format!("case={} payload={text}", s["DataKeyPart.Text"]),
    );
    let result = r.expression("list", "item=@KeyPart");
    r.arguments(&result, &[part]);
    result
}
fn response(r: &mut Request, status: i64, body: &str) -> String {
    let headers = r.expression("list", "item=@Header");
    let status = r.integer(status);
    let value = r.expression("record", "");
    r.text.push_str(&format!("expression.record-field parent={value} index=0 name=body value={body}\nexpression.record-field parent={value} index=1 name=headers value={headers}\nexpression.record-field parent={value} index=2 name=status value={status}\n"));
    value
}
fn empty(r: &mut Request, library: &BTreeMap<String, String>) -> String {
    let children = r.expression("list", "item=@Tree");
    let tree = r.expression(
        "variant",
        &format!("case={} payload={children}", library["branch"]),
    );
    r.types(&tree, &["i64"]);
    tree
}
fn wrapped(r: &mut Request, tree: &str) -> String {
    let value = r.expression("record", "type=$Stored");
    r.text.push_str(&format!(
        "expression.record-field parent={value} index=0 field=$stored-value value={tree}\n"
    ));
    value
}

pub(super) fn program(
    s: &BTreeMap<String, String>,
    b: &BTreeMap<String, String>,
    library: &BTreeMap<String, String>,
) -> String {
    let mut r = Request::default();
    r.http_request_type();
    r.text.push_str(&format!("type.application as=@Tree declaration={}\ntype.argument parent=@Tree index=0 type=i64\ncreate.record as=$Stored module={} name=Stored visibility=private\nadd.field as=$stored-value record=$Stored name=value type=@Tree\ntype.named as=@Stored declaration=$Stored\ntype.named as=@KeyPart declaration={}\ntype.named as=@Entry declaration={}\nadd.requirement as=$data component={} name=data interface={}\nrequirement.limit parent=$data index=0 name=maximum_calls maximum=64 unit=calls\n",library["tree"],b["module"],s["DataKeyPart"],s["DataEntry"],b["component"],s["DataStore"]));
    for (i, name) in [
        "schema-read",
        "schema-set",
        "get",
        "scan",
        "put",
        "delete",
        "transaction",
    ]
    .into_iter()
    .enumerate()
    {
        r.text.push_str(&format!(
            "requirement.operation parent=$data index={i} operation={}\n",
            s[&format!("DataStore.{name}")]
        ));
    }
    r.text.push_str("type.structural-record as=@Input\ntype.field parent=@Input index=0 name=mode type=text\ntype.field parent=@Input index=1 name=value type=@Tree\ntype.structural-record as=@Header\ntype.field parent=@Header index=0 name=name type=text\ntype.field parent=@Header index=1 name=value type=bytes\ntype.list as=@Headers item=@Header\ntype.structural-record as=@Response\ntype.field parent=@Response index=0 name=body type=bytes\ntype.field parent=@Response index=1 name=headers type=@Headers\ntype.field parent=@Response index=2 name=status type=i64\n");
    let value = r.local("$endless_value");
    let body = r.call("$endless", &[], &[value]);
    r.text.push_str(&format!("create.function as=$endless module={} name=endless visibility=private result=@Tree effect=pure body={body}\nadd.parameter as=$endless_value function=$endless name=value type=@Tree\n",b["module"]));
    let space = r.expression("static-text", "value=trees");
    let key = stored_key(&mut r, s);
    let entries = r.capability("$data", &s["DataStore.get"], &[space, key]);
    r.text.push_str("type.list as=@Entries item=@Entry\n");
    let prior = r.local("$prior");
    let length = r.call(&s["list-length"], &["@Entry"], &[prior]);
    let zero = r.integer(0);
    let missing = r.call(&s["i64-equal"], &[], &[length, zero]);
    let zero = r.integer(0);
    let prior = r.local("$prior");
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
    let tree = r.local("$write_value");
    let value = wrapped(&mut r, &tree);
    let bytes = r.call(&s["data-encode"], &["@Stored"], &[value]);
    let space = r.expression("static-text", "value=trees");
    let key = stored_key(&mut r, s);
    let put = r.capability(
        "$data",
        &s["DataStore.put"],
        &[space, key, bytes, expectation],
    );
    let mode = r.local("$write_mode");
    let trap = r.expression("text", "value=trap");
    let is_trap = r.call(&s["text-equal"], &[], &[mode, trap]);
    let one = r.integer(1);
    let zero = r.integer(0);
    let fault = r.call(&s["divide"], &[], &[one, zero]);
    let tree = r.local("$write_value");
    let trap = r.expression("sequence", "");
    r.arguments(&trap, &[fault, tree]);
    let mode = r.local("$write_mode");
    let cancel = r.expression("text", "value=cancel");
    let is_cancel = r.call(&s["text-equal"], &[], &[mode, cancel]);
    let tree = r.local("$write_value");
    let endless = r.call("$endless", &[], &[tree]);
    let tree = r.local("$write_value");
    let result = r.expression(
        "if",
        &format!("condition={is_cancel} when-true={endless} when-false={tree}"),
    );
    let result = r.expression(
        "if",
        &format!("condition={is_trap} when-true={trap} when-false={result}"),
    );
    let steps = r.expression("sequence", "");
    r.arguments(&steps, &[put, result]);
    let body = r.expression("let", &format!("body={steps}"));
    r.text.push_str(&format!("expression.binding parent={body} index=0 as=$prior name=prior value={entries} type=@Entries\n"));
    let transaction = r.expression(
        "transaction",
        &format!("requirement=$data binding=$transaction name=transaction body={body}"),
    );
    r.text.push_str(&format!("create.function as=$write module={} name=write visibility=private result=@Tree effect=task body={transaction}\nadd.parameter as=$write_value function=$write name=value type=@Tree\nadd.parameter as=$write_mode function=$write name=mode type=text\neffect.requirement parent=$write index=0 requirement=$data\n",b["module"]));
    let space = r.expression("static-text", "value=trees");
    let key = stored_key(&mut r, s);
    let entries = r.capability("$data", &s["DataStore.get"], &[space, key]);
    let zero = r.integer(0);
    let entry = r.call(&s["list-get"], &["@Entry"], &[entries, zero]);
    let bytes = r.expression(
        "field",
        &format!("value={entry} field={}", s["DataEntry.value"]),
    );
    let fallback = empty(&mut r, library);
    let fallback = wrapped(&mut r, &fallback);
    let stored_bytes = r.local("$stored-bytes");
    let decoded = r.call(
        &s["data-decode-or"],
        &["@Stored"],
        &[stored_bytes, fallback],
    );
    let value = r.local("$decoded");
    let reencoded = r.call(&s["data-encode"], &["@Stored"], &[value]);
    let bytes_local = r.local("$stored-bytes");
    let matches = r.call(&s["bytes-equal"], &[], &[reencoded, bytes_local]);
    let value = r.local("$decoded");
    let tree = r.expression("field", &format!("value={value} field=$stored-value"));
    let body = r.call(&s["json-encode"], &["@Tree"], &[tree]);
    let success = response(&mut r, 200, &body);
    let mismatch = r.expression("text", "value=layout-mismatch");
    let body = r.call(&s["json-encode"], &["text"], &[mismatch]);
    let mismatch = response(&mut r, 409, &body);
    let body = r.expression(
        "if",
        &format!("condition={matches} when-true={success} when-false={mismatch}"),
    );
    let read = r.expression("let", &format!("body={body}"));
    r.text.push_str(&format!("expression.binding parent={read} index=0 as=$stored-bytes name=stored-bytes value={bytes} type=bytes\nexpression.binding parent={read} index=1 as=$decoded name=decoded value={decoded} type=@Stored\n"));
    let input = r.local(&b["parameter"]);
    let stream = r.field(&input, "body");
    let maximum = r.integer(65536);
    let bytes = r.capability(&b["streams"], &s["ByteStream.read-all"], &[stream, maximum]);
    let mode = r.expression("text", "value=read");
    let tree = empty(&mut r, library);
    let fallback = r.expression("record", "");
    r.text.push_str(&format!("expression.record-field parent={fallback} index=0 name=mode value={mode}\nexpression.record-field parent={fallback} index=1 name=value value={tree}\n"));
    let decoded = r.call(&s["json-decode-or"], &["@Input"], &[bytes, fallback]);
    let data = r.field(&decoded, "value");
    let input = r.local("$request-data");
    let mode = r.field(&input, "mode");
    let reading = r.expression("text", "value=read");
    let is_read = r.call(&s["text-equal"], &[], &[mode, reading]);
    let input = r.local("$request-data");
    let mode = r.field(&input, "mode");
    let input = r.local("$request-data");
    let tree = r.field(&input, "value");
    let written = r.call("$write", &[], &[tree, mode]);
    let body = r.call(&s["json-encode"], &["@Tree"], &[written]);
    let write = response(&mut r, 200, &body);
    let body = r.expression(
        "if",
        &format!("condition={is_read} when-true={read} when-false={write}"),
    );
    let body = r.expression("let", &format!("body={body}"));
    r.text.push_str(&format!("expression.binding parent={body} index=0 as=$request-data name=request-data value={data} type=@Input\nset.function-contract as=%contract function={} result=@Response effect=task\neffect.requirement parent=%contract index=0 requirement={}\neffect.requirement parent=%contract index=1 requirement=$data\nreplace.body function={} body={body}\n",b["function"],b["streams"],b["function"]));
    r.text.push_str(&format!("effect.row as=@http-effects\neffect.requirement parent=@http-effects index=0 requirement={}\neffect.requirement parent=@http-effects index=1 requirement=$data\ntype.task-function as=@http-handler result=@Response effect=@http-effects\ntype.argument parent=@http-handler index=0 type={}\nset.port-contract port={} type=@http-handler\n",b["streams"],"@http-request",b["port"]));
    r.text
}
