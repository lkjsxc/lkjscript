use crate::semantic::schema::Expression;

#[test]
fn semantic_source_match_and_enum_types_are_closed_and_exact() {
    let encoded = concat!(
        "{\"kind\":\"match\",\"scrutinee\":{\"kind\":\"i64\",\"value\":7},",
        "\"arms\":[{\"pattern\":{\"kind\":\"binding\",\"name\":\"x\"},",
        "\"body\":{\"kind\":\"name-reference\",\"name\":\"x\"}}]}"
    );
    let expression: Expression = serde_json::from_str(encoded).expect("decode closed match");
    let source = expression
        .to_source(crate::source::SourceSpan::zero())
        .expect("project exact match");
    let text = crate::source::format_node_source(&source);
    assert!(text.contains("match/\n7\narms/\narm/\nbinding/"), "{text}");
    assert!(serde_json::from_str::<Expression>(
        &encoded.replace("\"kind\":\"binding\"", "\"kind\":\"guard\"",)
    )
    .is_err());

    let enum_type = concat!(
        "{\"kind\":\"variant-value\",\"value_type\":",
        "{\"kind\":\"enum\",\"name\":\"maybe\",\"arguments\":[{\"kind\":\"i64\"}]},",
        "\"variant\":\"none\",\"fields\":[]}"
    );
    let expression: Expression = serde_json::from_str(enum_type).expect("decode enum type");
    let text = crate::source::format_node_source(
        &expression
            .to_source(crate::source::SourceSpan::zero())
            .expect("enum source"),
    );
    assert!(text.contains("type/\nmaybe/\ni64\n/maybe\n/type"), "{text}");
}

#[test]
fn semantic_match_nodes_roundtrip_closed_pattern_kinds() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "match/\ntrue\narms/\n",
        "arm/\nbool-pattern/\nfalse\n/bool-pattern\n0\n/arm\n",
        "arm/\nwildcard/\n/wildcard\n1\n/arm\n",
        "/arms\n/match\n/main\n",
    );
    let tree = crate::source::validate(source, "semantic-match.lkjscript")
        .expect("validate semantic match source");
    let records = crate::semantic::tree::node_records(&tree);
    for expected in [
        crate::semantic::schema::SemanticNodeKind::Match,
        crate::semantic::schema::SemanticNodeKind::MatchArms,
        crate::semantic::schema::SemanticNodeKind::MatchArm,
        crate::semantic::schema::SemanticNodeKind::BoolPattern,
        crate::semantic::schema::SemanticNodeKind::WildcardPattern,
    ] {
        assert!(records.iter().any(|record| record.kind == expected));
    }
    let main = tree
        .declarations()
        .iter()
        .find(|item| item.kind() == crate::source::DeclarationKind::Main)
        .expect("main declaration");
    let subtree = crate::semantic::tree::subtree_record(&tree, main.node().index())
        .expect("closed match subtree");
    let rebuilt = subtree.to_source().expect("rebuild match subtree");
    assert_eq!(
        crate::source::format_node_source(&rebuilt),
        source
            .split_once("main/\n")
            .map(|(_, rest)| format!("main/\n{rest}"))
            .expect("main source marker")
    );
}
