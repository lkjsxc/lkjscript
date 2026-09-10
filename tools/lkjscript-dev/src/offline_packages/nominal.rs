//! Applied nominal meaning through the copied public package and execution paths.
use super::*;
use serde_json::{Value, json};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InspectionRecord {
    operation: String,
    fields: BTreeMap<String, String>,
}

fn inspected(records: &[CompactRecord]) -> Vec<InspectionRecord> {
    records
        .iter()
        .filter(|record| {
            matches!(
                record.operation.as_str(),
                "owner"
                    | "declaration"
                    | "type-parameter"
                    | "field"
                    | "case"
                    | "parameter"
                    | "type"
                    | "definition.function"
                    | "definition.parameter"
                    | "definition.type-parameter"
            )
        })
        .map(|record| InspectionRecord {
            operation: record.operation.clone(),
            fields: record
                .fields
                .iter()
                .map(|field| (field.name.clone(), field.value.clone()))
                .collect(),
        })
        .collect()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NominalReceipt {
    pub library_package: String,
    pub library_revision: String,
    pub generic_declarations: Vec<String>,
    pub producer_removed_before_execution: bool,
    pub results: BTreeMap<String, Value>,
    pub maximum_map_items: usize,
    pub maximum_map_sum: i64,
    pub changed_body_result: Value,
    pub replacement_library_revision: String,
    pub replacement_result: Value,
    pub replacement_keep_result: Value,
    pub session: crate::service::nominal::Observation,
    pub state_steps: Vec<Value>,
    pub retained_states: Vec<Value>,
    pub session_rejections: Vec<String>,
    pub inspections: BTreeMap<String, Vec<InspectionRecord>>,
}

pub(super) fn validate(receipt: &NominalReceipt) -> Result<(), DevError> {
    validate_inspections(receipt)?;
    crate::service::nominal::validate(&receipt.session)?;
    require(
        receipt.session_rejections
            == [
                "mismatched-application",
                "phantom-secret",
                "phantom-callable",
                "nested-secret",
            ],
        "nominal session omitted a required complete-type rejection",
    )?;
    require(
        receipt.state_steps
            == [
                json!({"first":1,"second":"a"}),
                json!({"first":2,"second":"ab"}),
            ],
        "nominal source-bound state steps missing",
    )?;
    require(
        receipt.retained_states
            == [
                json!({"first":0,"second":""}),
                json!({"first":1,"second":"a"}),
                json!({"first":2,"second":"ab"}),
            ],
        "nominal independently inspected retained session state missing",
    )?;
    require(
        receipt.library_package.starts_with("pkg_")
            && receipt.library_revision.starts_with("rev_")
            && receipt.generic_declarations.len() == 2
            && receipt.producer_removed_before_execution,
        "nominal package inventory or producer deletion witness missing",
    )?;
    for (name, items) in [
        ("i64", json!([1, 2, 4])),
        ("text", json!(["a", "bc"])),
        ("nested", json!([[1, 2], [], [4]])),
        ("owned", json!([{"text":"one"},{"text":"two"}])),
    ] {
        let original = json!({"revision":7,"items":items});
        require(
            receipt.results.get(&format!("{name}-keep")) == Some(&original)
                && receipt.results.get(&format!("{name}-snapshot")) == Some(&original)
                && receipt.results.get(&format!("{name}-identity")) == Some(&original),
            "nominal keep, map or retained snapshot differs from the fixed complete value",
        )?;
        let changed = match name {
            "i64" => json!([-2, 0, 9]),
            "text" => json!(["changed"]),
            "nested" => json!([[9], []]),
            _ => json!([{"text":"changed"}]),
        };
        require(
            receipt.results.get(&format!("{name}-retention"))
                == Some(&json!({"changed":{"revision":8,"items":changed},"snapshot":original})),
            "nominal snapshot was not retained across a distinct applied replacement",
        )?;
    }
    for (name, expected) in [
        ("i64-replace", json!({"revision":8,"items":[-2,0,9]})),
        ("map-first", json!({"revision":7,"items":[8,11,17]})),
        ("map-second", json!({"revision":7,"items":[2,0,-4]})),
        ("map-empty", json!({"revision":7,"items":[]})),
        ("pair-first", json!(1)),
        ("pair-second", json!("a")),
        ("pair-reordered-arguments", json!(1)),
        (
            "heterogeneous",
            json!({"revision":7,"items":[{"text":"1"},{"text":"2"},{"text":"4"}]}),
        ),
    ] {
        require(
            receipt.results.get(name) == Some(&expected),
            "nominal fixed result missing or changed",
        )?;
    }
    require(
        receipt.maximum_map_items == 8192
            && receipt.maximum_map_sum == 100_691_968
            && receipt.changed_body_result == json!({"revision":9,"items":[1,5,9]}),
        "nominal full-list or changed-body observation missing",
    )?;
    require(
        receipt.replacement_library_revision.starts_with("rev_")
            && receipt.replacement_library_revision != receipt.library_revision
            && receipt.replacement_result == json!({"revision":"next","items":[1,5,9]})
            && receipt.replacement_keep_result == json!({"revision":"retained","items":[1,2,4]}),
        "nominal exact dependency field, case and bound replacement was not revalidated",
    )
}

fn validate_inspections(receipt: &NominalReceipt) -> Result<(), DevError> {
    require(
        receipt.generic_declarations.len() == 2,
        "nominal declarations omitted",
    )?;
    require(
        receipt
            .inspections
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            == ["batch", "edit", "pair", "snapshot", "snapshot-definition"],
        "nominal inspection inventory changed",
    )?;
    let has = |name: &str, operation: &str, fields: &[(&str, &str)]| {
        receipt.inspections.get(name).is_some_and(|records| {
            records.iter().any(|record| {
                record.operation == operation
                    && fields.iter().all(|(key, value)| {
                        record.fields.get(*key).map(String::as_str) == Some(*value)
                    })
            })
        })
    };
    for (name, kind, parameter, bound) in [
        ("batch", "record", "T", "none"),
        ("edit", "variant", "T", "none"),
        ("snapshot", "pure_function", "T", "capture-safe"),
    ] {
        require(
            has(name, "declaration", &[("kind", kind), ("name", name)]),
            "nominal declaration projection missing",
        )?;
        require(
            has(
                name,
                "type-parameter",
                &[("index", "0"), ("name", parameter), ("constraint", bound)],
            ),
            "nominal ordered parameter/bound projection missing",
        )?;
        require(
            receipt.inspections[name]
                .iter()
                .filter(|record| record.operation == "type-parameter")
                .count()
                == 1,
            "nominal parameter count changed",
        )?;
    }
    for (index, name) in [("0", "First"), ("1", "Second")] {
        require(
            has(
                "pair",
                "type-parameter",
                &[("index", index), ("name", name), ("constraint", "none")],
            ),
            "standard pair parameter order changed",
        )?;
    }
    for (name, path, form) in [
        ("batch", "field.revision", "i64"),
        ("batch", "field.items", "list"),
        ("batch", "field.items.item", "parameter"),
        ("edit", "case.replace", "application"),
        ("edit", "case.replace.argument.0", "parameter"),
        ("snapshot", "parameter.batch", "application"),
        ("snapshot", "result", "function"),
        ("snapshot", "result.parameter.0", "unit"),
        ("snapshot", "result.result", "application"),
        ("snapshot", "result.result.argument.0", "parameter"),
        ("pair", "field.first", "parameter"),
        ("pair", "field.second", "parameter"),
    ] {
        require(
            has(name, "type", &[("path", path), ("form", form)]),
            "nominal full type projection missing",
        )?;
    }
    for (name, path) in [
        ("edit", "case.replace"),
        ("snapshot", "parameter.batch"),
        ("snapshot", "result.result"),
    ] {
        require(
            has(
                name,
                "type",
                &[
                    ("path", path),
                    ("reference", &receipt.generic_declarations[0]),
                ],
            ),
            "nominal projected application targets another declaration",
        )?;
    }
    require(
        has("edit", "case", &[("name", "keep"), ("payload", "false")])
            && has("edit", "case", &[("name", "replace"), ("payload", "true")]),
        "nominal complete case projection missing",
    )?;
    for (name, path, parameter_name) in [
        ("batch", "field.items.item", "T"),
        ("edit", "case.replace.argument.0", "T"),
        ("snapshot", "parameter.batch.argument.0", "T"),
        ("snapshot", "result.result.argument.0", "T"),
        ("pair", "field.first", "First"),
        ("pair", "field.second", "Second"),
    ] {
        let parameter = receipt.inspections[name]
            .iter()
            .find(|record| {
                record.operation == "owner"
                    && record.fields.get("kind").map(String::as_str) == Some("type_parameter")
                    && record.fields.get("name").map(String::as_str) == Some(parameter_name)
            })
            .and_then(|record| record.fields.get("id"))
            .ok_or_else(|| DevError::corrupt("nominal parameter identity omitted"))?;
        require(
            has(name, "type", &[("path", path), ("parameter", parameter)]),
            "nominal type expression changed its exact parameter identity",
        )?;
    }
    let snapshot_type = |path: &str| {
        receipt.inspections["snapshot"]
            .iter()
            .find(|record| {
                record.operation == "type"
                    && record.fields.get("path").map(String::as_str) == Some(path)
            })
            .and_then(|record| record.fields.get("digest"))
            .map(String::as_str)
            .ok_or_else(|| DevError::corrupt("snapshot type digest omitted"))
    };
    require(
        has(
            "snapshot-definition",
            "definition.parameter",
            &[("index", "0"), ("type", snapshot_type("parameter.batch")?)],
        ) && has(
            "snapshot-definition",
            "definition.function",
            &[("result", snapshot_type("result")?)],
        ) && has(
            "snapshot-definition",
            "definition.type-parameter",
            &[("index", "0"), ("constraint", "capture-safe")],
        ),
        "function definition did not preserve the exact applied signature",
    )
}

fn output(
    context: &mut Context,
    package: &Package,
    name: &str,
    arguments: Value,
    expected: &Value,
) -> Result<Value, DevError> {
    let records = context.cli(
        Some(&package.path),
        &["run", name, "--arguments", &arguments.to_string()],
        true,
    )?;
    require(
        field(&records, "execution", "differential")? == "equal",
        "nominal production/reference results differ",
    )?;
    let value: Value = serde_json::from_str(&field(&records, "execution", "value")?)?;
    require(
        &value == expected,
        "nominal result differs from independently fixed ordered fields/items",
    )?;
    Ok(value)
}

pub(super) fn workflow(context: &mut Context, standard: &mut Package) -> Result<(), DevError> {
    // The preceding offline HTTP witness removes all of its own inputs. Obtain the exact
    // built-in dependency for this independently authored workload through public export.
    standard.container = context.root.join("nominal-standard.lkjp");
    let exported = context.cli(
        None,
        &[
            "package",
            "builtin",
            "export",
            "--kind",
            "transport",
            "--output",
            &standard.container.display().to_string(),
        ],
        true,
    )?;
    require(
        field(&exported, "package", "transport")? == standard.transport,
        "nominal standard export changed its exact dependency",
    )?;
    for (name, kind) in [
        ("function-constant", "pure_function"),
        ("pair", "record"),
        ("pair-new", "pure_function"),
        ("pair-first", "pure_function"),
        ("pair-second", "pure_function"),
        ("i64-to-text", "external"),
    ] {
        let records = context.cli(
            None,
            &[
                "package", "builtin", "query", "owners", "--kind", kind, "--name", name,
            ],
            true,
        )?;
        standard
            .symbols
            .insert(name.into(), field(&records, "owner", "reference")?);
    }
    let mut library = context.new_package("nominal-library")?;
    context.stage(&library, standard)?;
    context.apply(
        &mut library,
        &format!(
            "{}{}{}",
            binding("add", standard),
            module(),
            library_request(standard)
        ),
    )?;
    let batch = reference(&library, "$batch")?;
    let edit = reference(&library, "$edit")?;
    context.receipt.nominal.library_package = library.id.clone();
    context.receipt.nominal.library_revision = library.revision.clone();
    context.receipt.nominal.generic_declarations = vec![batch.clone(), edit.clone()];
    for (name, kind) in [("batch", "record"), ("edit", "variant")] {
        let owners = context.cli(
            Some(&library.path),
            &[
                "query", "owners", "--kind", kind, "--limit", "1000", "--bytes", "1048576",
            ],
            true,
        )?;
        require(
            owners
                .iter()
                .filter(|record| {
                    record.operation == "owner"
                        && record_field(record, "name").is_ok_and(|value| value == name)
                })
                .count()
                == 1,
            "generic template was expanded into hidden monomorphic declarations",
        )?;
    }
    context.export(&mut library)?;
    let mut consumer = context.new_package("nominal-consumer")?;
    context.stage(&consumer, &library)?;
    context.stage(&consumer, standard)?;
    for (name, kind, symbol) in [
        ("batch", "record", "$batch"),
        ("edit", "variant", "$edit"),
        ("snapshot", "pure_function", "$snapshot"),
    ] {
        let records = context.cli(
            Some(&consumer.path),
            &[
                "package",
                "dependency",
                "inspect",
                "owner",
                kind,
                &library.symbols[symbol],
                "--package-revision",
                &library.logical,
            ],
            true,
        )?;
        context
            .receipt
            .nominal
            .inspections
            .insert(name.into(), inspected(&records));
    }
    let pair = standard.symbols["pair"]
        .rsplit('/')
        .next()
        .ok_or_else(|| DevError::corrupt("pair identity omitted"))?
        .to_owned();
    let records = context.cli(
        None,
        &["package", "builtin", "inspect", "owner", "record", &pair],
        true,
    )?;
    context
        .receipt
        .nominal
        .inspections
        .insert("pair".into(), inspected(&records));
    let records = context.cli(
        Some(&library.path),
        &[
            "inspect",
            "owner",
            "pure_function",
            &library.symbols["$snapshot"],
            "--detail",
            "definition",
            "--limit",
            "1000",
            "--bytes",
            "1048576",
        ],
        true,
    )?;
    require(
        field(&records, "page", "complete")? == "true",
        "nominal snapshot definition was not complete",
    )?;
    context
        .receipt
        .nominal
        .inspections
        .insert("snapshot-definition".into(), inspected(&records));
    context.apply(
        &mut consumer,
        &format!(
            "{}{}{}{}",
            binding("add", &library),
            binding("add", standard),
            module(),
            consumer_request(&library, standard)?
        ),
    )?;
    context.export(&mut consumer)?;
    let previous_library = library.clone();
    let mut replacement = format!(
        "set.field-type field={} type=text\nset.case-payload case={} payload=unit\n",
        library.symbols["$revision"], library.symbols["$keep"]
    );
    for parameter in ["$T", "$E", "$A", "$In", "$Out"] {
        replacement.push_str(&format!(
            "set.type-parameter-constraint parameter={} constraint=capture-safe\n",
            library.symbols[parameter]
        ));
    }
    replacement.push_str(&format!("type.parameter as=@A parameter={}\ntype.application as=@Batch declaration={}\ntype.argument parent=@Batch index=0 type=@A\nexpression.local as=$change value={}\nexpression.local as=$original value={}\nexpression.local as=$replacement value=$payload\nexpression.match as=$matched value=$change\nexpression.match-arm parent=$matched index=0 case={} as=$ignored name=ignored type=unit body=$original\nexpression.match-arm parent=$matched index=1 case={} as=$payload name=payload type=@Batch body=$replacement\nreplace.body function={} body=$matched\n",
        library.symbols["$A"],library.symbols["$batch"],library.symbols["$change"],library.symbols["$original"],
        reference(&library,"$keep")?,reference(&library,"$replace")?,library.symbols["$apply"]));
    context.apply(&mut library, &replacement)?;
    context.export(&mut library)?;
    context.stage(&consumer, &library)?;
    context.receipt.nominal.replacement_library_revision = library.revision.clone();
    fs::remove_dir_all(&library.path)?;
    fs::remove_file(&previous_library.container)?;
    fs::remove_file(&library.container)?;
    context.receipt.nominal.producer_removed_before_execution =
        !library.path.exists() && !library.container.exists();
    context.build(&consumer, "nominal-after-producer-removal")?;
    context.check(&consumer, 33, 3)?;
    for (name, original_items, changed_items) in [
        ("i64", json!([1, 2, 4]), json!([-2, 0, 9])),
        ("text", json!(["a", "bc"]), json!(["changed"])),
        ("nested", json!([[1, 2], [], [4]]), json!([[9], []])),
        (
            "owned",
            json!([{"text":"one"},{"text":"two"}]),
            json!([{"text":"changed"}]),
        ),
    ] {
        let original = json!({"revision":7,"items":original_items});
        let changed = json!({"revision":8,"items":changed_items});
        let expected = json!({"changed":changed,"snapshot":original});
        let label = format!("{name}-retention");
        let value = output(
            context,
            &consumer,
            &label,
            json!([original,{"case":"replace","value":changed}]),
            &expected,
        )?;
        context.receipt.nominal.results.insert(label, value);
    }
    for (name, items) in [
        ("i64", json!([1, 2, 4])),
        ("text", json!(["a", "bc"])),
        ("nested", json!([[1, 2], [], [4]])),
        ("owned", json!([{"text":"one"},{"text":"two"}])),
    ] {
        let original = json!({"revision":7,"items":items});
        for (suffix, arguments) in [
            ("keep", json!([original,{"case":"keep"}])),
            ("snapshot", json!([original])),
            ("identity", json!([original])),
        ] {
            let label = format!("{name}-{suffix}");
            let value = output(context, &consumer, &label, arguments, &original)?;
            context.receipt.nominal.results.insert(label, value);
        }
    }
    let replacement = json!({"revision":8,"items":[-2,0,9]});
    let value = output(
        context,
        &consumer,
        "i64-keep",
        json!([{"revision":7,"items":[1,2,4]},{"case":"replace","value":replacement}]),
        &replacement,
    )?;
    context
        .receipt
        .nominal
        .results
        .insert("i64-replace".into(), value);
    for (name, scale, bias, items, expected) in [
        ("map-first", 3, 5, json!([1, 2, 4]), json!([8, 11, 17])),
        ("map-second", -2, 4, json!([1, 2, 4]), json!([2, 0, -4])),
        ("map-empty", 3, 5, json!([]), json!([])),
    ] {
        let expected = json!({"revision":7,"items":expected});
        let value = output(
            context,
            &consumer,
            "map",
            json!([{"revision":7,"items":items},scale,bias]),
            &expected,
        )?;
        context.receipt.nominal.results.insert(name.into(), value);
    }
    for (name, expected) in [("pair-first", json!(1)), ("pair-second", json!("a"))] {
        let value = output(context, &consumer, name, json!([1, "a"]), &expected)?;
        context.receipt.nominal.results.insert(name.into(), value);
    }
    let heterogeneous = json!({"revision":7,"items":[{"text":"1"},{"text":"2"},{"text":"4"}]});
    let value = output(
        context,
        &consumer,
        "heterogeneous",
        json!([{"revision":7,"items":[1,2,4]}]),
        &heterogeneous,
    )?;
    context
        .receipt
        .nominal
        .results
        .insert("heterogeneous".into(), value);
    for n in [256, 1024, 4096, 8192] {
        let input = (0..n).collect::<Vec<i64>>();
        let expected = input.iter().map(|i| 3 * i + 5).collect::<Vec<_>>();
        output(
            context,
            &consumer,
            "map",
            json!([{"revision":7,"items":input},3,5]),
            &json!({"revision":7,"items":expected}),
        )?;
        context.receipt.nominal.maximum_map_items = n as usize;
        context.receipt.nominal.maximum_map_sum = expected.iter().sum();
    }
    let changed = format!(
        "expression.local as=$value value={}\nexpression.local as=$scale value={}\nexpression.local as=$bias value={}\n{}{}replace.body function={} body=$changed\n",
        consumer.symbols["$step-value"],
        consumer.symbols["$step-scale"],
        consumer.symbols["$step-bias"],
        call(
            "$product",
            &standard.symbols["multiply"],
            &["$value", "$scale"]
        ),
        call(
            "$changed",
            &standard.symbols["subtract"],
            &["$product", "$bias"]
        ),
        consumer.symbols["$step"]
    );
    context.apply(&mut consumer, &changed)?;
    let expected = json!({"revision":9,"items":[1,5,9]});
    context.receipt.nominal.changed_body_result = output(
        context,
        &consumer,
        "map",
        json!([{"revision":9,"items":[2,3,4]},4,7]),
        &expected,
    )?;
    context.cache_recovery(&consumer, "nominal-changed-body")?;
    let reorder = format!(
        "expression.local as=$a value={}\nexpression.local as=$b value={}\nexpression.call as=$pair function={}\ntype.argument parent=$pair index=0 type=text\ntype.argument parent=$pair index=1 type=i64\nexpression.argument parent=$pair index=0 expression=$b\nexpression.argument parent=$pair index=1 expression=$a\nexpression.call as=$selected function={}\ntype.argument parent=$selected index=0 type=text\ntype.argument parent=$selected index=1 type=i64\nexpression.argument parent=$selected index=0 expression=$pair\nreplace.body function={} body=$selected\n",
        consumer.symbols["$pair-first-a"],
        consumer.symbols["$pair-first-b"],
        standard.symbols["pair-new"],
        standard.symbols["pair-second"],
        consumer.symbols["$pair-first"]
    );
    context.apply(&mut consumer, &reorder)?;
    let reordered = output(context, &consumer, "pair-first", json!([1, "a"]), &json!(1))?;
    context
        .receipt
        .nominal
        .results
        .insert("pair-reordered-arguments".into(), reordered);
    context.cache_recovery(&consumer, "nominal-reordered-arguments")?;
    context.apply(&mut consumer, &binding("replace", &library))?;
    let expected = json!({"revision":"next","items":[1,5,9]});
    context.receipt.nominal.replacement_result = output(
        context,
        &consumer,
        "map",
        json!([{"revision":"next","items":[2,3,4]},4,7]),
        &expected,
    )?;
    let expected = json!({"revision":"retained","items":[1,2,4]});
    context.receipt.nominal.replacement_keep_result = output(
        context,
        &consumer,
        "i64-keep",
        json!([expected,{"case":"keep","value":null}]),
        &expected,
    )?;
    context.cache_recovery(&consumer, "nominal-replaced-template-case-bound")?;
    fs::remove_dir_all(&consumer.path)?;
    super::nominal_session::workflow(context, standard)?;
    validate(&context.receipt.nominal)?;
    Ok(())
}

fn library_request(standard: &Package) -> String {
    r#"
create.record as=$batch module=$module name=batch visibility=public
add.type-parameter as=$T declaration=$batch name=T
type.parameter as=@T parameter=$T
type.list as=@Ts item=@T
add.field as=$revision record=$batch name=revision type=i64
add.field as=$items record=$batch name=items type=@Ts
create.variant as=$edit module=$module name=edit visibility=public
add.type-parameter as=$E declaration=$edit name=T
type.parameter as=@E parameter=$E
type.application as=@EBatch declaration=$batch
type.argument parent=@EBatch index=0 type=@E
add.case as=$keep variant=$edit name=keep
add.case as=$replace variant=$edit name=replace payload=@EBatch
create.function as=$apply module=$module name=apply-edit visibility=public result=@ABatch effect=pure body=$matched
add.type-parameter as=$A declaration=$apply name=T
type.parameter as=@A parameter=$A
type.application as=@ABatch declaration=$batch
type.argument parent=@ABatch index=0 type=@A
type.application as=@AEdit declaration=$edit
type.argument parent=@AEdit index=0 type=@A
add.parameter as=$original function=$apply name=original type=@ABatch
add.parameter as=$change function=$apply name=change type=@AEdit
expression.local as=$change-value value=$change
expression.local as=$original-value value=$original
expression.local as=$replacement-value value=$replacement
expression.match as=$matched value=$change-value
expression.match-arm parent=$matched index=0 case=$keep body=$original-value
expression.match-arm parent=$matched index=1 case=$replace as=$replacement name=replacement type=@ABatch body=$replacement-value
create.function as=$map module=$module name=map-batch visibility=public result=@OutBatch effect=pure body=$mapped-batch
add.type-parameter as=$In declaration=$map name=Input
add.type-parameter as=$Out declaration=$map name=Output
type.parameter as=@In parameter=$In
type.parameter as=@Out parameter=$Out
type.application as=@InBatch declaration=$batch
type.argument parent=@InBatch index=0 type=@In
type.application as=@OutBatch declaration=$batch
type.argument parent=@OutBatch index=0 type=@Out
type.function as=@Mapper result=@Out
type.argument parent=@Mapper index=0 type=@In
add.parameter as=$map-input function=$map name=batch type=@InBatch
add.parameter as=$map-callback function=$map name=callback type=@Mapper
expression.local as=$map-source value=$map-input
expression.field as=$map-revision value=$map-source field=$revision
expression.local as=$map-source2 value=$map-input
expression.field as=$map-items value=$map-source2 field=$items
expression.local as=$mapper value=$map-callback
expression.call as=$mapped-items function=LIST_MAP
type.argument parent=$mapped-items index=0 type=@In
type.argument parent=$mapped-items index=1 type=@Out
expression.argument parent=$mapped-items index=0 expression=$map-items
expression.argument parent=$mapped-items index=1 expression=$mapper
expression.record as=$mapped-batch type=$batch
type.argument parent=$mapped-batch index=0 type=@Out
expression.record-field parent=$mapped-batch index=0 field=$revision value=$map-revision
expression.record-field parent=$mapped-batch index=1 field=$items value=$mapped-items
create.function as=$snapshot module=$module name=snapshot visibility=public result=@Snapshot effect=pure body=$retained
add.type-parameter as=$S declaration=$snapshot name=T constraint=capture-safe
type.parameter as=@S parameter=$S
type.application as=@SBatch declaration=$batch
type.argument parent=@SBatch index=0 type=@S
type.function as=@Snapshot result=@SBatch
type.argument parent=@Snapshot index=0 type=unit
add.parameter as=$snapshot-input function=$snapshot name=batch type=@SBatch
expression.local as=$saved value=$snapshot-input
expression.call as=$retained function=FUNCTION_CONSTANT
type.argument parent=$retained index=0 type=@SBatch
type.argument parent=$retained index=1 type=unit
expression.argument parent=$retained index=0 expression=$saved
"#.replace("LIST_MAP",&standard.symbols["list-map"]).replace("FUNCTION_CONSTANT",&standard.symbols["function-constant"])
}

fn consumer_request(library: &Package, standard: &Package) -> Result<String, DevError> {
    let batch = reference(library, "$batch")?;
    let edit = reference(library, "$edit")?;
    let apply = reference(library, "$apply")?;
    let map = reference(library, "$map")?;
    let snapshot = reference(library, "$snapshot")?;
    let mut text = "create.component as=$component module=$module name=commands visibility=private\ncreate.record as=$owned module=$module name=Owned visibility=public\nadd.field as=$owned-text record=$owned name=text type=text\ntype.named as=@Owned declaration=$owned\ntype.list as=@Nested item=i64\n".to_owned();
    for (label, ty) in [
        ("i64", "i64"),
        ("text", "text"),
        ("nested", "@Nested"),
        ("owned", "@Owned"),
    ] {
        text.push_str(&format!(r#"
type.application as=@{label}-batch declaration={batch}
type.argument parent=@{label}-batch index=0 type={ty}
type.application as=@{label}-edit declaration={edit}
type.argument parent=@{label}-edit index=0 type={ty}
create.function as=${label}-keep module=$module name={label}-keep visibility=public result=@{label}-batch effect=pure body=${label}-applied
add.parameter as=${label}-original function=${label}-keep name=original type=@{label}-batch
add.parameter as=${label}-edit-value function=${label}-keep name=edit type=@{label}-edit
expression.local as=${label}-original-value value=${label}-original
expression.local as=${label}-change-value value=${label}-edit-value
expression.call as=${label}-applied function={apply}
type.argument parent=${label}-applied index=0 type={ty}
expression.argument parent=${label}-applied index=0 expression=${label}-original-value
expression.argument parent=${label}-applied index=1 expression=${label}-change-value
create.function as=${label}-snapshot module=$module name={label}-snapshot visibility=public result=@{label}-batch effect=pure body=${label}-recalled
add.parameter as=${label}-save function=${label}-snapshot name=original type=@{label}-batch
expression.local as=${label}-save-value value=${label}-save
expression.call as=${label}-retained function={snapshot}
type.argument parent=${label}-retained index=0 type={ty}
expression.argument parent=${label}-retained index=0 expression=${label}-save-value
expression.unit as=${label}-unit
expression.invoke as=${label}-recalled function=${label}-retained
expression.argument parent=${label}-recalled index=0 expression=${label}-unit
create.function as=${label}-id module=$module name={label}-id visibility=private result={ty} effect=pure body=${label}-id-value
add.parameter as=${label}-id-input function=${label}-id name=value type={ty}
expression.local as=${label}-id-value value=${label}-id-input
create.function as=${label}-identity module=$module name={label}-identity visibility=public result=@{label}-batch effect=pure body=${label}-mapped
add.parameter as=${label}-identity-input function=${label}-identity name=batch type=@{label}-batch
expression.local as=${label}-identity-value value=${label}-identity-input
expression.function-value as=${label}-identity-callback function=${label}-id
expression.call as=${label}-mapped function={map}
type.argument parent=${label}-mapped index=0 type={ty}
type.argument parent=${label}-mapped index=1 type={ty}
expression.argument parent=${label}-mapped index=0 expression=${label}-identity-value
expression.argument parent=${label}-mapped index=1 expression=${label}-identity-callback
type.function as=@{label}-thunk result=@{label}-batch
type.argument parent=@{label}-thunk index=0 type=unit
type.structural-record as=@{label}-retention
type.field parent=@{label}-retention index=0 name=changed type=@{label}-batch
type.field parent=@{label}-retention index=1 name=snapshot type=@{label}-batch
create.function as=${label}-retention module=$module name={label}-retention visibility=public result=@{label}-retention effect=pure body=${label}-retention-body
add.parameter as=${label}-retention-input function=${label}-retention name=original type=@{label}-batch
add.parameter as=${label}-retention-change function=${label}-retention name=edit type=@{label}-edit
expression.local as=${label}-capture-value value=${label}-retention-input
expression.call as=${label}-capture function={snapshot}
type.argument parent=${label}-capture index=0 type={ty}
expression.argument parent=${label}-capture index=0 expression=${label}-capture-value
expression.local as=${label}-change-original value=${label}-retention-input
expression.local as=${label}-change-input value=${label}-retention-change
expression.call as=${label}-changed function={apply}
type.argument parent=${label}-changed index=0 type={ty}
expression.argument parent=${label}-changed index=0 expression=${label}-change-original
expression.argument parent=${label}-changed index=1 expression=${label}-change-input
expression.local as=${label}-thunk-value value=${label}-thunk
expression.unit as=${label}-recall-unit
expression.invoke as=${label}-recall function=${label}-thunk-value
expression.argument parent=${label}-recall index=0 expression=${label}-recall-unit
expression.local as=${label}-changed-value value=${label}-changed-binding
expression.record as=${label}-retention-record
expression.record-field parent=${label}-retention-record index=0 name=changed value=${label}-changed-value
expression.record-field parent=${label}-retention-record index=1 name=snapshot value=${label}-recall
expression.let as=${label}-retention-body body=${label}-retention-record
expression.binding parent=${label}-retention-body index=0 as=${label}-thunk name=snapshot value=${label}-capture type=@{label}-thunk
expression.binding parent=${label}-retention-body index=1 as=${label}-changed-binding name=changed value=${label}-changed type=@{label}-batch
"#));
        for (suffix, args) in [
            (
                "keep",
                vec![format!("@{label}-batch"), format!("@{label}-edit")],
            ),
            ("snapshot", vec![format!("@{label}-batch")]),
            ("identity", vec![format!("@{label}-batch")]),
        ] {
            target_request(
                &mut text,
                &format!("{label}-{suffix}"),
                &format!("@{label}-batch"),
                &args,
            );
        }
        target_request(
            &mut text,
            &format!("{label}-retention"),
            &format!("@{label}-retention"),
            &[format!("@{label}-batch"), format!("@{label}-edit")],
        );
    }
    text.push_str("create.function as=$step module=$module name=step visibility=private result=i64 effect=pure body=$stepped\nadd.parameter as=$step-scale function=$step name=scale type=i64\nadd.parameter as=$step-bias function=$step name=bias type=i64\nadd.parameter as=$step-value function=$step name=value type=i64\nexpression.local as=$scale value=$step-scale\nexpression.local as=$bias value=$step-bias\nexpression.local as=$value value=$step-value\n");
    text.push_str(&call(
        "$product",
        &standard.symbols["multiply"],
        &["$value", "$scale"],
    ));
    text.push_str(&call(
        "$stepped",
        &standard.symbols["add"],
        &["$product", "$bias"],
    ));
    text.push_str(&format!(r#"
create.function as=$map module=$module name=map visibility=public result=@i64-batch effect=pure body=$mapped
add.parameter as=$map-original function=$map name=original type=@i64-batch
add.parameter as=$map-scale function=$map name=scale type=i64
add.parameter as=$map-bias function=$map name=bias type=i64
expression.local as=$map-value value=$map-original
expression.local as=$map-scale-value value=$map-scale
expression.local as=$map-bias-value value=$map-bias
expression.function-value as=$step-value function=$step
expression.bind as=$mapper callee=$step-value
expression.argument parent=$mapper index=0 expression=$map-scale-value
expression.argument parent=$mapper index=1 expression=$map-bias-value
expression.call as=$mapped function={map}
type.argument parent=$mapped index=0 type=i64
type.argument parent=$mapped index=1 type=i64
expression.argument parent=$mapped index=0 expression=$map-value
expression.argument parent=$mapped index=1 expression=$mapper
"#).replace("expression.function-value as=$step-value", "expression.function-value as=$step-callee").replace("callee=$step-value", "callee=$step-callee"));
    target_request(
        &mut text,
        "map",
        "@i64-batch",
        &["@i64-batch".into(), "i64".into(), "i64".into()],
    );
    text.push_str(&format!(r#"
create.function as=$wrap module=$module name=wrap visibility=private result=@Owned effect=pure body=$wrapped
add.parameter as=$wrap-input function=$wrap name=input type=i64
expression.local as=$wrap-value value=$wrap-input
expression.call as=$wrap-text function={}
expression.argument parent=$wrap-text index=0 expression=$wrap-value
expression.record as=$wrapped type=$owned
expression.record-field parent=$wrapped index=0 field=$owned-text value=$wrap-text
create.function as=$heterogeneous module=$module name=heterogeneous visibility=public result=@owned-batch effect=pure body=$heterogeneous-result
add.parameter as=$heterogeneous-input function=$heterogeneous name=input type=@i64-batch
expression.local as=$heterogeneous-value value=$heterogeneous-input
expression.function-value as=$heterogeneous-callback function=$wrap
expression.call as=$heterogeneous-result function={map}
type.argument parent=$heterogeneous-result index=0 type=i64
type.argument parent=$heterogeneous-result index=1 type=@Owned
expression.argument parent=$heterogeneous-result index=0 expression=$heterogeneous-value
expression.argument parent=$heterogeneous-result index=1 expression=$heterogeneous-callback
"#,standard.symbols["i64-to-text"]));
    target_request(
        &mut text,
        "heterogeneous",
        "@owned-batch",
        &["@i64-batch".into()],
    );
    for (name, result) in [("pair-first", "i64"), ("pair-second", "text")] {
        text.push_str(&format!("create.function as=${name} module=$module name={name} visibility=public result={result} effect=pure body=${name}-out\nadd.parameter as=${name}-a function=${name} name=first type=i64\nadd.parameter as=${name}-b function=${name} name=second type=text\nexpression.local as=${name}-av value=${name}-a\nexpression.local as=${name}-bv value=${name}-b\nexpression.call as=${name}-pair function={}\ntype.argument parent=${name}-pair index=0 type=i64\ntype.argument parent=${name}-pair index=1 type=text\nexpression.argument parent=${name}-pair index=0 expression=${name}-av\nexpression.argument parent=${name}-pair index=1 expression=${name}-bv\nexpression.call as=${name}-out function={}\ntype.argument parent=${name}-out index=0 type=i64\ntype.argument parent=${name}-out index=1 type=text\nexpression.argument parent=${name}-out index=0 expression=${name}-pair\n",standard.symbols["pair-new"],standard.symbols[name]));
        target_request(&mut text, name, result, &["i64".into(), "text".into()]);
    }
    Ok(text)
}

fn target_request(text: &mut String, name: &str, result: &str, arguments: &[String]) {
    text.push_str(&format!("type.function as=@port-{name} result={result}\n"));
    for (index, ty) in arguments.iter().enumerate() {
        text.push_str(&format!(
            "type.argument parent=@port-{name} index={index} type={ty}\n"
        ));
    }
    text.push_str(&format!("add.port as=$port-{name} component=$component name={name} type=@port-{name} function=${name}\ncreate.target as=$target-{name} name={name} component=$component port=$port-{name} runner=command\n"));
}
