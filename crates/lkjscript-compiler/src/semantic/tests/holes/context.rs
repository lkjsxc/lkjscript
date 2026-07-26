use super::*;
use crate::semantic::schema::{BlockerCode, ExpectedTypeFact, OwnershipAccess, SemanticEffect};

#[test]
fn ambiguity_is_retained_and_ranked_deterministically() {
    let directory = case_dir("hole-ranking");
    let root = directory.join("main.lkjscript");
    std::fs::write(&root, function_source(&hole("body", Some("choose value"))))
        .expect("write hole source");
    let first = context(&root);
    let second = context(&root);
    let first_json = serde_json::to_vec(&first).expect("encode first context");
    let second_json = serde_json::to_vec(&second).expect("encode second context");
    assert_eq!(first_json, second_json);
    assert!(matches!(first.expected_type, ExpectedTypeFact::Available {
        ref canonical, instantiated: true,
    } if canonical == "I64"));
    let visible_entities: Vec<_> = first
        .scope_entities
        .iter()
        .filter(|entity| entity.kind == crate::semantic::schema::ScopeEntityKind::Parameter)
        .collect();
    assert_eq!(visible_entities.len(), 2);
    assert_ne!(visible_entities[0].identity, visible_entities[1].identity);
    let visible: Vec<_> = first
        .candidates
        .iter()
        .filter(|candidate| {
            candidate.category == crate::semantic::schema::CandidateCategory::VisibleBinding
        })
        .map(|candidate| candidate.snippets[0].source.trim())
        .collect();
    assert_eq!(visible, ["x", "y"]);
    assert!(first
        .candidates
        .windows(2)
        .all(|pair| pair[0].rank.category <= pair[1].rank.category));
    assert!(first
        .blockers
        .iter()
        .any(|blocker| { blocker.code == BlockerCode::QualificationUnsupported }));
}

#[test]
fn ownership_and_effect_candidates_are_checker_derived() {
    let directory = case_dir("hole-ownership");
    let root = directory.join("main.lkjscript");
    let source = concat!(
        "def/\nname/\nf\n/name\nfn/\nsig/\nOwned\nBuf\n->\nOwned\nBuf\n/sig\n",
        "params/\nx\nOwned/\nBuf\n/Owned\n/params\nhole/\nname/\nowned\n/name\n/hole\n/fn\n/def\n",
        "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n",
    );
    std::fs::write(&root, source).expect("write ownership hole");
    let ownership = context(&root);
    assert_eq!(
        ownership.constraints.ownership.expected_access,
        OwnershipAccess::Move
    );
    assert!(ownership.constraints.ownership.checker_validated);
    assert!(ownership.candidates.iter().any(|candidate| {
        candidate.snippets[0].source == "move/\nx\n/move\n"
            && candidate.ownership == OwnershipAccess::Move
    }));

    let effect_root = directory.join("effect.lkjscript");
    let effect_source = concat!(
        "def/\nname/\ng\n/name\nfn/\nsig/\n->\nUnit\n/sig\nparams/\n/params\n",
        "print/\nstr/\nx\n/str\n/print\n/fn\n/def\n",
        "def/\nname/\nf\n/name\nfn/\nsig/\n->\nUnit\n/sig\nparams/\n/params\n",
        "hole/\nname/\neffect\n/name\n/hole\n/fn\n/def\n",
        "main/\nsig/\n->\nUnit\n/sig\nf/\n/f\n/main\n",
    );
    std::fs::write(&effect_root, effect_source).expect("write effect hole");
    let effects = context(&effect_root);
    let call = effects
        .candidates
        .iter()
        .find(|candidate| candidate.snippets[0].source == "g/\n/g\n")
        .expect("effectful direct call");
    assert!(call.effects.contains(&SemanticEffect::HostIo));
    assert!(call.effects.contains(&SemanticEffect::MayTrap));
}

#[test]
fn exact_conversion_candidates_are_complete_and_checker_validated() {
    let directory = case_dir("hole-conversion");
    let root = directory.join("main.lkjscript");
    let source = format!("main/\nsig/\n->\nStr\n/sig\n{}/main\n", hole("text", None),);
    std::fs::write(&root, source).expect("write conversion hole");
    let context = context(&root);
    let conversion = context
        .candidates
        .iter()
        .find(|candidate| {
            candidate.category == crate::semantic::schema::CandidateCategory::ExactConversion
                && candidate.snippets[0].source.starts_with("str-from-i64/\n")
        })
        .expect("exact conversion candidate");
    assert!(conversion.snippets[0].complete);
    assert!(conversion.edits.iter().any(|edit| matches!(
        edit,
        crate::semantic::schema::ExactSemanticEdit::ReplaceHole { .. }
    )));
}

#[test]
fn edition2_numeric_conversion_candidates_use_canonical_operations() {
    for (name, ty, operation) in [
        ("rounded", "F64", "f64-from-i64-rounded"),
        (
            "exact",
            "Result/\nF64\nNumericError\n/Result",
            "f64-from-i64-exact",
        ),
    ] {
        let directory = case_dir(&format!("hole-numeric-conversion-{name}"));
        let root = directory.join("main.lkjscript");
        let source = format!(
            "edition/\n2\n/edition\nmain/\nsig/\n->\n{ty}\n/sig\n{}/main\n",
            hole("numeric", None),
        );
        std::fs::write(&root, source).expect("write numeric conversion hole");
        let context = context(&root);
        assert!(
            context.candidates.iter().any(|candidate| {
                candidate.category == crate::semantic::schema::CandidateCategory::ExactConversion
                    && candidate.snippets[0]
                        .source
                        .starts_with(&format!("{operation}/\n"))
            }),
            "missing {name} numeric conversion candidate"
        );
    }
}

#[test]
fn typed_hole_entity_roundtrips_and_legal_actions_are_closed() {
    let directory = case_dir("hole-roundtrip");
    let root = directory.join("main.lkjscript");
    std::fs::write(&root, function_source(&hole("body", None))).expect("write source");
    let (revision, snapshot) = snapshot(&root);
    let declaration = snapshot
        .declarations
        .iter()
        .find(|item| item.name == "f")
        .expect("function declaration");
    let operation = format!(
        concat!(
            "{{\"kind\":\"read_entity\",\"revision\":{revision:?},",
            "\"declaration_key\":{:?},\"entity_fingerprint\":null}}",
        ),
        declaration.key,
        revision = revision
    );
    let encoded = crate::semantic::execute(&request(&root, &operation)).expect("read entity");
    let ResponseResult::ReadEntity { entity } = response(&encoded).result else {
        panic!("expected entity")
    };
    assert!(entity.canonical_subtree.contains("hole/\n"));
    assert!(entity
        .descendants
        .iter()
        .any(|node| { node.kind == crate::semantic::schema::SemanticNodeKind::TypedHole }));
    let actions = actions(&root);
    assert!(actions.coverage.supported);
    assert!(actions
        .transaction_kinds
        .contains(&crate::semantic::schema::HoleTransactionKind::FillHole,));
    assert!(!actions
        .transaction_kinds
        .contains(&crate::semantic::schema::HoleTransactionKind::InsertHole,));
}
