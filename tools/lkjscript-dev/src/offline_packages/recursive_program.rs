//! Public compact requests for a separately authored reusable tree library.
use crate::pure_tail_program::Request;
use std::collections::BTreeMap;

fn parameters(r: &mut Request, function: &str, names: &[&str]) {
    for name in names {
        r.text.push_str(&format!("add.type-parameter as=${function}-{name} declaration=${function} name={name}\ntype.parameter as=@{function}-{name} parameter=${function}-{name}\n"));
    }
}

fn tree_type(r: &mut Request, alias: &str, declaration: &str, argument: &str) {
    r.text.push_str(&format!("type.application as=@{alias} declaration={declaration}\ntype.argument parent=@{alias} index=0 type={argument}\n"));
}

fn variant(r: &mut Request, case: &str, argument: &str, value: &str) -> String {
    let expression = r.expression("variant", &format!("case={case} payload={value}"));
    r.types(&expression, &[argument]);
    expression
}

pub(super) fn library(standard: &BTreeMap<String, String>) -> String {
    let mut r = Request::default();
    r.text.push_str(&super::recursive_schemas::finite());
    r.text.push_str("create.variant as=$tree module=$module name=tree visibility=public\nadd.type-parameter as=$TreeT declaration=$tree name=T\ntype.parameter as=@TreeT parameter=$TreeT\ntype.application as=@Tree declaration=$tree\ntype.argument parent=@Tree index=0 type=@TreeT\ntype.list as=@Trees item=@Tree\nadd.case as=$leaf variant=$tree name=leaf payload=@TreeT\nadd.case as=$branch variant=$tree name=branch payload=@Trees\n");
    for name in ["tree-map", "tree-map-children"] {
        parameters(&mut r, name, &["T", "U"]);
        tree_type(
            &mut r,
            &format!("{name}-input"),
            "$tree",
            &format!("@{name}-T"),
        );
        tree_type(
            &mut r,
            &format!("{name}-output"),
            "$tree",
            &format!("@{name}-U"),
        );
        r.text.push_str(&format!("type.list as=@{name}-inputs item=@{name}-input\ntype.list as=@{name}-outputs item=@{name}-output\ntype.function as=@{name}-callback result=@{name}-U\ntype.argument parent=@{name}-callback index=0 type=@{name}-T\n"));
    }
    let value = r.local("$map-leaf");
    let callback = r.local("$tree-map_callback");
    let mapped = r.invoke(&callback, &[value]);
    let leaf = variant(&mut r, "$leaf", "@tree-map-U", &mapped);
    let children = r.local("$map-branch");
    let callback = r.local("$tree-map_callback");
    let zero = r.integer(0);
    let empty = r.expression("list", "item=@tree-map-output");
    let children = r.call(
        "$tree-map-children",
        &["@tree-map-T", "@tree-map-U"],
        &[children, callback, zero, empty],
    );
    let branch = variant(&mut r, "$branch", "@tree-map-U", &children);
    let value = r.local("$tree-map_tree");
    let body = r.expression("match", &format!("value={value}"));
    r.text.push_str(&format!("expression.match-arm parent={body} index=0 case=$leaf as=$map-leaf name=leaf type=@tree-map-T body={leaf}\nexpression.match-arm parent={body} index=1 case=$branch as=$map-branch name=children type=@tree-map-inputs body={branch}\n"));
    r.function(
        "tree-map",
        "@tree-map-output",
        &body,
        &[
            ("tree", "@tree-map-input"),
            ("callback", "@tree-map-callback"),
        ],
    );

    let items = r.local("$tree-map-children_items");
    let length = r.call(
        &standard["list-length"],
        &["@tree-map-children-input"],
        &[items],
    );
    let index = r.local("$tree-map-children_index");
    let done = r.call(&standard["i64-equal"], &[], &[index, length]);
    let result = r.local("$tree-map-children_result");
    let items = r.local("$tree-map-children_items");
    let index = r.local("$tree-map-children_index");
    let item = r.call(
        &standard["list-get"],
        &["@tree-map-children-input"],
        &[items, index],
    );
    let callback = r.local("$tree-map-children_callback");
    let mapped = r.call(
        "$tree-map",
        &["@tree-map-children-T", "@tree-map-children-U"],
        &[item, callback],
    );
    let accumulated = r.local("$tree-map-children_result");
    let appended = r.call(
        &standard["list-append"],
        &["@tree-map-children-output"],
        &[accumulated, mapped],
    );
    let items = r.local("$tree-map-children_items");
    let callback = r.local("$tree-map-children_callback");
    let index = r.local("$tree-map-children_index");
    let one = r.integer(1);
    let next = r.call(&standard["add"], &[], &[index, one]);
    let recur = r.call(
        "$tree-map-children",
        &["@tree-map-children-T", "@tree-map-children-U"],
        &[items, callback, next, appended],
    );
    let body = r.expression(
        "if",
        &format!("condition={done} when-true={result} when-false={recur}"),
    );
    r.function(
        "tree-map-children",
        "@tree-map-children-outputs",
        &body,
        &[
            ("items", "@tree-map-children-inputs"),
            ("callback", "@tree-map-children-callback"),
            ("index", "i64"),
            ("result", "@tree-map-children-outputs"),
        ],
    );

    for name in ["tree-fold", "tree-fold-children"] {
        parameters(&mut r, name, &["T", "A"]);
        tree_type(
            &mut r,
            &format!("{name}-tree"),
            "$tree",
            &format!("@{name}-T"),
        );
        r.text.push_str(&format!("type.list as=@{name}-trees item=@{name}-tree\ntype.function as=@{name}-callback result=@{name}-A\ntype.argument parent=@{name}-callback index=0 type=@{name}-A\ntype.argument parent=@{name}-callback index=1 type=@{name}-T\n"));
    }
    let callback = r.local("$tree-fold_callback");
    let accumulated = r.local("$tree-fold_accumulator");
    let value = r.local("$fold-leaf");
    let leaf = r.invoke(&callback, &[accumulated, value]);
    let children = r.local("$fold-branch");
    let callback = r.local("$tree-fold_callback");
    let zero = r.integer(0);
    let accumulated = r.local("$tree-fold_accumulator");
    let branch = r.call(
        "$tree-fold-children",
        &["@tree-fold-T", "@tree-fold-A"],
        &[children, callback, zero, accumulated],
    );
    let value = r.local("$tree-fold_tree");
    let body = r.expression("match", &format!("value={value}"));
    r.text.push_str(&format!("expression.match-arm parent={body} index=0 case=$leaf as=$fold-leaf name=leaf type=@tree-fold-T body={leaf}\nexpression.match-arm parent={body} index=1 case=$branch as=$fold-branch name=children type=@tree-fold-trees body={branch}\n"));
    r.function(
        "tree-fold",
        "@tree-fold-A",
        &body,
        &[
            ("tree", "@tree-fold-tree"),
            ("callback", "@tree-fold-callback"),
            ("accumulator", "@tree-fold-A"),
        ],
    );
    let items = r.local("$tree-fold-children_items");
    let length = r.call(
        &standard["list-length"],
        &["@tree-fold-children-tree"],
        &[items],
    );
    let index = r.local("$tree-fold-children_index");
    let done = r.call(&standard["i64-equal"], &[], &[index, length]);
    let result = r.local("$tree-fold-children_accumulator");
    let items = r.local("$tree-fold-children_items");
    let index = r.local("$tree-fold-children_index");
    let item = r.call(
        &standard["list-get"],
        &["@tree-fold-children-tree"],
        &[items, index],
    );
    let callback = r.local("$tree-fold-children_callback");
    let accumulated = r.local("$tree-fold-children_accumulator");
    let accumulated = r.call(
        "$tree-fold",
        &["@tree-fold-children-T", "@tree-fold-children-A"],
        &[item, callback, accumulated],
    );
    let items = r.local("$tree-fold-children_items");
    let callback = r.local("$tree-fold-children_callback");
    let index = r.local("$tree-fold-children_index");
    let one = r.integer(1);
    let next = r.call(&standard["add"], &[], &[index, one]);
    let recur = r.call(
        "$tree-fold-children",
        &["@tree-fold-children-T", "@tree-fold-children-A"],
        &[items, callback, next, accumulated],
    );
    let body = r.expression(
        "if",
        &format!("condition={done} when-true={result} when-false={recur}"),
    );
    r.function(
        "tree-fold-children",
        "@tree-fold-children-A",
        &body,
        &[
            ("items", "@tree-fold-children-trees"),
            ("callback", "@tree-fold-children-callback"),
            ("index", "i64"),
            ("accumulator", "@tree-fold-children-A"),
        ],
    );

    parameters(&mut r, "append-leaf", &["T"]);
    r.text
        .push_str("type.list as=@append-leaf-items item=@append-leaf-T\n");
    let items = r.local("$append-leaf_items");
    let value = r.local("$append-leaf_value");
    let body = r.call(
        &standard["list-append"],
        &["@append-leaf-T"],
        &[items, value],
    );
    r.function(
        "append-leaf",
        "@append-leaf-items",
        &body,
        &[("items", "@append-leaf-items"), ("value", "@append-leaf-T")],
    );
    parameters(&mut r, "tree-flatten", &["T"]);
    tree_type(&mut r, "tree-flatten-tree", "$tree", "@tree-flatten-T");
    r.text
        .push_str("type.list as=@tree-flatten-items item=@tree-flatten-T\n");
    let tree = r.local("$tree-flatten_tree");
    let callback = r.function_value("$append-leaf");
    r.types(&callback, &["@tree-flatten-T"]);
    let empty = r.expression("list", "item=@tree-flatten-T");
    let body = r.call(
        "$tree-fold",
        &["@tree-flatten-T", "@tree-flatten-items"],
        &[tree, callback, empty],
    );
    r.function(
        "tree-flatten",
        "@tree-flatten-items",
        &body,
        &[("tree", "@tree-flatten-tree")],
    );

    r.text.push_str("add.type-parameter as=$snapshot-T declaration=$snapshot name=T constraint=capture-safe\ntype.parameter as=@snapshot-T parameter=$snapshot-T\n");
    tree_type(&mut r, "snapshot-tree", "$tree", "@snapshot-T");
    r.text.push_str("type.function as=@snapshot-callable result=@snapshot-tree\ntype.argument parent=@snapshot-callable index=0 type=unit\n");
    let tree = r.local("$snapshot_tree");
    let body = r.call(
        &standard["function-constant"],
        &["@snapshot-tree", "unit"],
        &[tree],
    );
    r.function(
        "snapshot",
        "@snapshot-callable",
        &body,
        &[("tree", "@snapshot-tree")],
    );
    tree_type(&mut r, "sum-tree", "$tree", "i64");
    let tree = r.local("$tree-sum_tree");
    let callback = r.function_value(&standard["add"]);
    let zero = r.integer(0);
    let body = r.call("$tree-fold", &["i64", "i64"], &[tree, callback, zero]);
    r.function("tree-sum", "i64", &body, &[("tree", "@sum-tree")]);
    for name in ["tree-shape", "tree-shape-children"] {
        parameters(&mut r, name, &["T"]);
        tree_type(
            &mut r,
            &format!("{name}-tree"),
            "$tree",
            &format!("@{name}-T"),
        );
        r.text
            .push_str(&format!("type.list as=@{name}-trees item=@{name}-tree\n"));
    }
    r.text.push_str("type.list as=@shape item=i64\n");
    let result = r.local("$tree-shape_result");
    let zero = r.integer(0);
    let leaf = r.call(&standard["list-append"], &["i64"], &[result, zero]);
    let children = r.local("$shape-children");
    let length = r.call(&standard["list-length"], &["@tree-shape-tree"], &[children]);
    let one = r.integer(1);
    let tag = r.call(&standard["add"], &[], &[length, one]);
    let result = r.local("$tree-shape_result");
    let result = r.call(&standard["list-append"], &["i64"], &[result, tag]);
    let children = r.local("$shape-children");
    let zero = r.integer(0);
    let branch = r.call(
        "$tree-shape-children",
        &["@tree-shape-T"],
        &[children, zero, result],
    );
    let tree = r.local("$tree-shape_tree");
    let body = r.expression("match", &format!("value={tree}"));
    r.text.push_str(&format!("expression.match-arm parent={body} index=0 case=$leaf as=$shape-leaf name=leaf type=@tree-shape-T body={leaf}\nexpression.match-arm parent={body} index=1 case=$branch as=$shape-children name=children type=@tree-shape-trees body={branch}\n"));
    r.function(
        "tree-shape",
        "@shape",
        &body,
        &[("tree", "@tree-shape-tree"), ("result", "@shape")],
    );
    let trees = r.local("$tree-shape-children_trees");
    let index = r.local("$tree-shape-children_index");
    let length = r.call(
        &standard["list-length"],
        &["@tree-shape-children-tree"],
        &[trees],
    );
    let done = r.call(&standard["i64-equal"], &[], &[index, length]);
    let result = r.local("$tree-shape-children_result");
    let trees = r.local("$tree-shape-children_trees");
    let index = r.local("$tree-shape-children_index");
    let child = r.call(
        &standard["list-get"],
        &["@tree-shape-children-tree"],
        &[trees, index],
    );
    let previous = r.local("$tree-shape-children_result");
    let advanced = r.call(
        "$tree-shape",
        &["@tree-shape-children-T"],
        &[child, previous],
    );
    let trees = r.local("$tree-shape-children_trees");
    let index = r.local("$tree-shape-children_index");
    let one = r.integer(1);
    let next = r.call(&standard["add"], &[], &[index, one]);
    let remaining = r.call(
        "$tree-shape-children",
        &["@tree-shape-children-T"],
        &[trees, next, advanced],
    );
    let body = r.expression(
        "if",
        &format!("condition={done} when-true={result} when-false={remaining}"),
    );
    r.function(
        "tree-shape-children",
        "@shape",
        &body,
        &[
            ("trees", "@tree-shape-children-trees"),
            ("index", "i64"),
            ("result", "@shape"),
        ],
    );
    r.text
}

pub(super) fn consumer(
    standard: &BTreeMap<String, String>,
    library: &BTreeMap<String, String>,
) -> String {
    let mut r = Request::default();
    r.text.push_str("create.component as=$component module=$module name=commands visibility=private\ncreate.record as=$owned module=$module name=Element visibility=public\nadd.field as=$owned-text record=$owned name=text type=text\ntype.named as=@Owned declaration=$owned\n");
    super::recursive_schemas::concrete(&mut r, library);
    for (label, ty) in [("i64", "i64"), ("text", "text"), ("owned", "@Owned")] {
        tree_type(&mut r, &format!("{label}-tree"), &library["tree"], ty);
        r.text
            .push_str(&format!("type.list as=@{label}-leaves item={ty}\n"));
        let tree = r.local(&format!("${label}-flatten_tree"));
        let result = r.call(&library["tree-flatten"], &[ty], &[tree]);
        r.function(
            &format!("{label}-flatten"),
            &format!("@{label}-leaves"),
            &result,
            &[("tree", &format!("@{label}-tree"))],
        );
        r.target(
            &format!("{label}-flatten"),
            &format!("@{label}-leaves"),
            &[&format!("@{label}-tree")],
        );
        let value = r.local(&format!("${label}-identity_value"));
        r.function(&format!("{label}-identity"), ty, &value, &[("value", ty)]);
        let tree = r.local(&format!("${label}-map_tree"));
        let callback = r.function_value(&format!("${label}-identity"));
        let result = r.call(&library["tree-map"], &[ty, ty], &[tree, callback]);
        r.function(
            &format!("{label}-map"),
            &format!("@{label}-tree"),
            &result,
            &[("tree", &format!("@{label}-tree"))],
        );
        r.target(
            &format!("{label}-map"),
            &format!("@{label}-tree"),
            &[&format!("@{label}-tree")],
        );
    }
    for (name, scale, bias) in [("affine", 3, 5), ("plus-two", 1, 2), ("composition", 3, 11)] {
        let value = r.local(&format!("${name}_value"));
        let scale = r.integer(scale);
        let product = r.call(&standard["multiply"], &[], &[scale, value]);
        let bias = r.integer(bias);
        let body = r.call(&standard["add"], &[], &[product, bias]);
        r.function(name, "i64", &body, &[("value", "i64")]);
    }
    let tree = r.local("$mapped_tree");
    let callback = r.function_value("$affine");
    let mapped = r.call(&library["tree-map"], &["i64", "i64"], &[tree, callback]);
    r.function("mapped", "@i64-tree", &mapped, &[("tree", "@i64-tree")]);
    r.target("mapped", "@i64-tree", &["@i64-tree"]);
    let tree = r.local("$composed_tree");
    let callback = r.function_value("$plus-two");
    let first = r.call(&library["tree-map"], &["i64", "i64"], &[tree, callback]);
    let callback = r.function_value("$affine");
    let body = r.call(&library["tree-map"], &["i64", "i64"], &[first, callback]);
    r.function("composed", "@i64-tree", &body, &[("tree", "@i64-tree")]);
    r.target("composed", "@i64-tree", &["@i64-tree"]);
    let tree = r.local("$composition-map_tree");
    let callback = r.function_value("$composition");
    let body = r.call(&library["tree-map"], &["i64", "i64"], &[tree, callback]);
    r.function(
        "composition-map",
        "@i64-tree",
        &body,
        &[("tree", "@i64-tree")],
    );
    r.target("composition-map", "@i64-tree", &["@i64-tree"]);

    let accumulator = r.local("$decimal_accumulator");
    let ten = r.integer(10);
    let prefix = r.call(&standard["multiply"], &[], &[ten, accumulator]);
    let value = r.local("$decimal_value");
    let body = r.call(&standard["add"], &[], &[prefix, value]);
    r.function(
        "decimal",
        "i64",
        &body,
        &[("accumulator", "i64"), ("value", "i64")],
    );
    let tree = r.local("$ordered-fold_tree");
    let callback = r.function_value("$decimal");
    let zero = r.integer(0);
    let body = r.call(
        &library["tree-fold"],
        &["i64", "i64"],
        &[tree, callback, zero],
    );
    r.function("ordered-fold", "i64", &body, &[("tree", "@i64-tree")]);
    r.target("ordered-fold", "i64", &["@i64-tree"]);
    let tree = r.local("$sum_tree");
    let body = r.call(&library["tree-sum"], &[], &[tree]);
    r.function("sum", "i64", &body, &[("tree", "@i64-tree")]);
    r.target("sum", "i64", &["@i64-tree"]);
    let tree = r.local("$count_tree");
    let leaves = r.call(&library["tree-flatten"], &["i64"], &[tree]);
    let body = r.call(&standard["list-length"], &["i64"], &[leaves]);
    r.function("count", "i64", &body, &[("tree", "@i64-tree")]);
    r.target("count", "i64", &["@i64-tree"]);

    r.text.push_str("type.function as=@Saved result=@i64-tree\ntype.argument parent=@Saved index=0 type=unit\ntype.structural-record as=@Retained\ntype.field parent=@Retained index=0 name=changed type=@i64-tree\ntype.field parent=@Retained index=1 name=original type=@i64-tree\n");
    let original = r.local("$retained_tree");
    let saved = r.call(&library["snapshot"], &["i64"], &[original]);
    let original = r.local("$retained_tree");
    let mapped = r.call("$mapped", &[], &[original]);
    let changed = r.local("$changed");
    let saved_value = r.local("$saved");
    let unit = r.expression("unit", "");
    let original = r.invoke(&saved_value, &[unit]);
    let body = r.expression("record", "");
    r.text.push_str(&format!("expression.record-field parent={body} index=0 name=changed value={changed}\nexpression.record-field parent={body} index=1 name=original value={original}\n"));
    let body = r.expression("let", &format!("body={body}"));
    r.text.push_str(&format!("expression.binding parent={body} index=0 as=$saved name=saved value={saved} type=@Saved\nexpression.binding parent={body} index=1 as=$changed name=changed value={mapped} type=@i64-tree\n"));
    r.function("retained", "@Retained", &body, &[("tree", "@i64-tree")]);
    r.target("retained", "@Retained", &["@i64-tree"]);
    let one = r.integer(1);
    let zero = r.integer(0);
    let body = r.call(&standard["divide"], &[], &[one, zero]);
    let value = r.local("$trap_value");
    let two = r.integer(2);
    let second = r.call(&standard["i64-equal"], &[], &[value, two]);
    let value = r.local("$trap_value");
    let four = r.integer(4);
    let fourth = r.call(&standard["i64-equal"], &[], &[value, four]);
    let maximum = r.integer(i64::MAX);
    let one = r.integer(1);
    let overflow = r.call(&standard["add"], &[], &[maximum, one]);
    let value = r.local("$trap_value");
    let later = r.expression(
        "if",
        &format!("condition={fourth} when-true={overflow} when-false={value}"),
    );
    let body = r.expression(
        "if",
        &format!("condition={second} when-true={body} when-false={later}"),
    );
    r.function("trap", "i64", &body, &[("value", "i64")]);
    let tree = r.local("$trapped_tree");
    let callback = r.function_value("$trap");
    let body = r.call(&library["tree-map"], &["i64", "i64"], &[tree, callback]);
    r.function("trapped", "@i64-tree", &body, &[("tree", "@i64-tree")]);
    r.target("trapped", "@i64-tree", &["@i64-tree"]);
    // Construct the actual balanced recursive value inside the language. Its complete
    // shape is checked against the independently constructed Rust input before traversal.
    // Only the two small integer seeds cross argv, whose per-argument OS bound is smaller
    // than the 4,096-leaf JSON value on the admitted Linux target.
    let start = r.local("$balanced_start");
    let leaf = r.expression(
        "variant",
        &format!("case={} payload={start}", library["leaf"]),
    );
    r.types(&leaf, &["i64"]);
    let count = r.local("$balanced_count");
    let two = r.integer(2);
    let half = r.call(&standard["divide"], &[], &[count, two]);
    let start = r.local("$balanced_start");
    let half_value = r.local("$half");
    let left = r.call("$balanced", &[], &[start, half_value]);
    let start = r.local("$balanced_start");
    let half_value = r.local("$half");
    let next_start = r.call(&standard["add"], &[], &[start, half_value]);
    let count = r.local("$balanced_count");
    let half_value = r.local("$half");
    let remaining = r.call(&standard["subtract"], &[], &[count, half_value]);
    let right = r.call("$balanced", &[], &[next_start, remaining]);
    let children = r.expression("list", "item=@i64-tree");
    r.arguments(&children, &[left, right]);
    let branch = r.expression(
        "variant",
        &format!("case={} payload={children}", library["branch"]),
    );
    r.types(&branch, &["i64"]);
    let split = r.expression("let", &format!("body={branch}"));
    r.text.push_str(&format!(
        "expression.binding parent={split} index=0 as=$half name=half value={half} type=i64\n"
    ));
    let count = r.local("$balanced_count");
    let one = r.integer(1);
    let single = r.call(&standard["i64-equal"], &[], &[count, one]);
    let body = r.expression(
        "if",
        &format!("condition={single} when-true={leaf} when-false={split}"),
    );
    let count = r.local("$balanced_count");
    let zero = r.integer(0);
    let is_empty = r.call(&standard["i64-equal"], &[], &[count, zero]);
    let children = r.expression("list", "item=@i64-tree");
    let empty = r.expression(
        "variant",
        &format!("case={} payload={children}", library["branch"]),
    );
    r.types(&empty, &["i64"]);
    let body = r.expression(
        "if",
        &format!("condition={is_empty} when-true={empty} when-false={body}"),
    );
    r.function(
        "balanced",
        "@i64-tree",
        &body,
        &[("start", "i64"), ("count", "i64")],
    );
    for (name, result) in [
        ("scale-shape", "@i64-leaves"),
        ("scale-leaves", "@i64-leaves"),
        ("scale-sum", "i64"),
        ("scale-mapped-shape", "@i64-leaves"),
        ("scale-mapped-sum", "i64"),
    ] {
        let start = r.local(&format!("${name}_start"));
        let count = r.local(&format!("${name}_count"));
        let tree = r.call("$balanced", &[], &[start, count]);
        let tree = if name.contains("mapped") {
            r.call("$mapped", &[], &[tree])
        } else {
            tree
        };
        let body = if name.ends_with("sum") {
            r.call("$sum", &[], &[tree])
        } else if name.ends_with("leaves") {
            r.call("$i64-flatten", &[], &[tree])
        } else if name.ends_with("shape") {
            let empty = r.expression("list", "item=i64");
            r.call(&library["tree-shape"], &["i64"], &[tree, empty])
        } else {
            tree
        };
        r.function(name, result, &body, &[("start", "i64"), ("count", "i64")]);
        r.target(name, result, &["i64", "i64"]);
    }
    r.text
}
