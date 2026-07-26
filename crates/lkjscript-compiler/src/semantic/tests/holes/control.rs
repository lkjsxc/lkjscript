use super::*;
use crate::semantic::schema::{BlockerCode, CandidateCategory, LegalChildKind, SemanticNodeKind};

#[test]
fn edition2_holes_expose_checked_never_and_control_forms() {
    let directory = case_dir("hole-control-edition2");
    let root = directory.join("main.lkjscript");
    let source = format!(
        "edition/\n2\n/edition\nmain/\nsig/\n->\nI64\n/sig\n{}/main\n",
        hole("control", None),
    );
    std::fs::write(&root, source).expect("write Edition 2 control hole");
    let context = context(&root);
    assert!(context.constraints.never_admissible);
    assert_eq!(context.constraints.control.function_return, "I64");
    assert!(context
        .constraints
        .control
        .available_forms
        .iter()
        .any(|form| form == "return"));
    for form in ["loop/", "return/", "trap/", "exit/"] {
        assert!(context
            .candidates
            .iter()
            .any(|candidate| candidate.snippets[0].source.starts_with(form)));
    }
    assert!(context.candidates.iter().any(|candidate| {
        candidate.category == CandidateCategory::NeverForm && candidate.result_type == "Never"
    }));
    assert!(!context
        .blockers
        .iter()
        .any(|blocker| blocker.code == BlockerCode::UnsupportedEditionOneForm));
    let actions = actions(&root);
    for kind in [
        LegalChildKind::Loop,
        LegalChildKind::Return,
        LegalChildKind::Trap,
        LegalChildKind::Exit,
    ] {
        assert!(actions.legal_child_kinds.contains(&kind));
    }
    let never_root = directory.join("never.lkjscript");
    std::fs::write(
        &never_root,
        "edition/\n2\n/edition\nmain/\nsig/\n->\nNever\n/sig\ntrap/\nstr/\nx\n/str\n/trap\n/main\n",
    )
    .expect("write Never schema source");
    let (_, snapshot) = snapshot(&never_root);
    assert!(snapshot
        .nodes
        .iter()
        .any(|node| node.kind == SemanticNodeKind::TypeNever));
    assert!(snapshot
        .nodes
        .iter()
        .any(|node| node.kind == SemanticNodeKind::Trap));
}

#[test]
fn nearest_loop_constraints_expose_break_and_continue_only_in_loop() {
    let directory = case_dir("hole-control-loop");
    let root = directory.join("main.lkjscript");
    let source = format!(
        "edition/\n2\n/edition\nmain/\nsig/\n->\nI64\n/sig\nloop/\ntype/\nI64\n/type\n{}\n/loop\n/main\n",
        hole("loop-body", None),
    );
    std::fs::write(&root, source).expect("write loop hole");
    let context = context(&root);
    assert_eq!(context.constraints.control.loop_depth, 1);
    assert_eq!(
        context.constraints.control.loop_result.as_deref(),
        Some("I64")
    );
    for form in ["break/", "continue/"] {
        assert!(context
            .candidates
            .iter()
            .any(|candidate| candidate.snippets[0].source.starts_with(form)));
    }
    let actions = actions(&root);
    assert!(actions.legal_child_kinds.contains(&LegalChildKind::Break));
    assert!(actions
        .legal_child_kinds
        .contains(&LegalChildKind::Continue));
}
