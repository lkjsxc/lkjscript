use super::*;

#[test]
fn byte_vector_slice_accepts_nll_mutation_move_and_return() {
    let source = ownership_source(
        "let/\nbind/\nb\nnew-byte-vector/\n2\n/new-byte-vector\n/bind\ndo/\nlet/\nbind/\nr\nborrow/\nb\n/borrow\n/bind\nbyte-slice-length/\nr\n/byte-slice-length\n/let\nlet/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\ndo/\nbyte-slice-mut-set-byte/\nm\n0\n65\n/byte-slice-mut-set-byte\nmove/\nb\n/move\n/do\n/let\n/do\n/let",
        "byte-vector",
    );
    let program = compile_source(&source, "owned-valid.lkjscript", &Limits::default())
        .expect("valid byte-vector ownership island");
    let instructions: Vec<_> = program
        .ssa()
        .program()
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .collect();
    let last_end = instructions
        .iter()
        .rposition(|instruction| {
            matches!(
                instruction.kind,
                lkjscript_ir::InstructionKind::EndBorrow { .. }
            )
        })
        .expect("verified loans have explicit end events");
    let moved = instructions
        .iter()
        .rposition(|instruction| {
            matches!(instruction.kind, lkjscript_ir::InstructionKind::Move { .. })
        })
        .expect("fixture transfers its owner");
    assert!(last_end < moved, "borrow must end before owner move");

    let shared_pair = ownership_source(
        "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nlet/\nbind/\nr1\nborrow/\nb\n/borrow\n/bind\nbind/\nr2\nborrow/\nb\n/borrow\n/bind\ndo/\nbyte-slice-length/\nr1\n/byte-slice-length\nbyte-slice-length/\nr2\n/byte-slice-length\n/do\n/let\n/let",
        "i64",
    );
    compile_source(
        &shared_pair,
        "owned-shared-pair.lkjscript",
        &Limits::default(),
    )
    .expect("overlapping shared loans must be accepted");

    let equal_branch = ownership_source(
        "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nif/\ntrue\nmove/\nb\n/move\nmove/\nb\n/move\n/if\n/let",
        "byte-vector",
    );
    compile_source(
        &equal_branch,
        "owned-equal-branch.lkjscript",
        &Limits::default(),
    )
    .expect("equal branch move states must join");

    let conditional_owner = conditional_cleanup_source();
    let conditional = compile_source(
        &conditional_owner,
        "owned-conditional-cleanup.lkjscript",
        &Limits::default(),
    )
    .expect("branch-specific whole-place cleanup must compile");
    assert!(conditional
        .memory_plan()
        .obligations
        .iter()
        .any(|obligation| {
            obligation.drop_class == Some(crate::memory_plan::MemoryDropClass::Conditional)
        }));
    let implicit_drops = conditional
        .ssa()
        .program()
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .filter(|instruction| {
            matches!(
                instruction.kind,
                lkjscript_ir::InstructionKind::Drop {
                    glue: lkjscript_ir::DropGlueIdentity::ByteVector,
                    kind: lkjscript_ir::DropEventKind::ImplicitCleanup,
                    ..
                }
            )
        })
        .count();
    assert_eq!(implicit_drops, 2);

    let branch_local_result = ownership_source(
        "if/\ntrue\nlet/\nbind/\na\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nmove/\na\n/move\n/let\nlet/\nbind/\nb\nnew-byte-vector/\n2\n/new-byte-vector\n/bind\nmove/\nb\n/move\n/let\n/if",
        "byte-vector",
    );
    compile_source(
        &branch_local_result,
        "owned-branch-local-result.lkjscript",
        &Limits::default(),
    )
    .expect("transferred branch-local owners must canonicalize at the result join");

    let constant_false_loop = ownership_source("while/\nfalse\nunit\n/while", "unit");
    compile_source(
        &constant_false_loop,
        "constant-false-loop.lkjscript",
        &Limits::default(),
    )
    .expect("branch simplification must clear a stale loop-header marker");
}

fn conditional_cleanup_source() -> String {
    concat!(
        "def/\nname/\nselect-bytes\n/name\npublic\nfn/\n",
        "sig/\ninputs/\nbool\n/inputs\noutput/\nbyte-vector\n/output\n/sig\n",
        "params/\nflag\nbool\n/params\nlet/\nbind/\nb\nnew-byte-vector/\n1\n",
        "/new-byte-vector\n/bind\nlet/\nbind/\nc\nnew-byte-vector/\n2\n/new-byte-vector\n",
        "/bind\nif/\nflag\nmove/\nb\n/move\nmove/\nc\n/move\n/if\n/let\n/let\n",
        "/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\n",
        "output/\nbyte-vector\n/output\n/sig\nselect-bytes/\ntrue\n/select-bytes\n/main\n"
    )
    .to_owned()
}

#[test]
fn static_byte_cleanup_covers_normal_and_explicit_trap_paths() {
    let normal = ownership_source(
        "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nunit\n/let",
        "unit",
    );
    let program = compile_source(&normal, "drop-normal.lkjscript", &Limits::default())
        .expect("static byte owner cleanup");
    let main = program.ssa().program().functions.last().expect("main SSA");
    assert!(main
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .any(|instruction| {
            matches!(
                instruction.kind,
                lkjscript_ir::InstructionKind::Drop {
                    glue: lkjscript_ir::DropGlueIdentity::ByteVector,
                    kind: lkjscript_ir::DropEventKind::ImplicitCleanup,
                    ..
                }
            )
        }));

    let trapped = ownership_source(
        "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\ntrap/\nstring-literal/\nstop\n/string-literal\n/trap\n/let",
        "unit",
    );
    let program = compile_source(&trapped, "drop-trap.lkjscript", &Limits::default())
        .expect("trap edge byte owner cleanup");
    let main = program.ssa().program().functions.last().expect("main SSA");
    let trap_block = main
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, lkjscript_ir::Terminator::Trap { .. }))
        .expect("trap block");
    assert!(trap_block.instructions.iter().any(|instruction| {
        matches!(instruction.kind, lkjscript_ir::InstructionKind::Drop { .. })
    }));
}

#[test]
fn byte_vector_slice_rejects_affine_and_alias_failures() {
    let cases = [
        ("let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nb\n/let", "byte-vector", "loaded or copied"),
        ("let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\ndo/\nmove/\nb\n/move\nmove/\nb\n/move\n/do\n/let", "byte-vector", "double move"),
        ("let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nlet/\nbind/\nr\nborrow/\nb\n/borrow\n/bind\ndo/\nmove/\nb\n/move\nbyte-slice-length/\nr\n/byte-slice-length\n/do\n/let\n/let", "i64", "while it is borrowed"),
        ("let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nif/\ntrue\ndo/\nmove/\nb\n/move\nunit\n/do\nunit\n/if\n/let", "unit", "branch join"),
        ("borrow/\nnew-byte-vector/\n1\n/new-byte-vector\n/borrow", "unit", "whole byte-vector local"),
        ("let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\ndo/\nmove/\nb\n/move\nbyte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\n/do\n/let", "i64", "after move"),
        ("let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nlet/\nbind/\nr\nborrow/\nb\n/borrow\n/bind\nlet/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\ndo/\nbyte-slice-length/\nr\n/byte-slice-length\nbyte-slice-mut-set-byte/\nm\n0\n1\n/byte-slice-mut-set-byte\n/do\n/let\n/let\n/let", "unit", "conflicting shared and exclusive"),
        ("let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nwhile/\nfalse\nmove/\nb\n/move\n/while\n/let", "unit", "loop-carried"),
        ("let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\ndo/\nsome/\nborrow/\nb\n/borrow\n/some\nunit\n/do\n/let", "unit", "cannot instantiate an enum"),
    ];
    for (body, result, diagnostic) in cases {
        let source = ownership_source(body, result);
        let error = compile_source(&source, "owned-invalid.lkjscript", &Limits::default())
            .expect_err("invalid ownership source must fail")
            .to_string();
        assert!(error.contains(diagnostic), "{diagnostic}: {error}");
    }
}
