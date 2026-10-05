//! Digest repair never authorizes erased handoffs or counterfeit result relationships.
use super::*;
use crate::platform::kernel::KernelSnapshot;

const SOURCE: &str = r#"
declarations.begin
(units (module create borrowed-results
  (external create length (visibility private) (implementation core.buffer.length)
    (parameter create value (type ByteBuffer) (use borrow)) (returns I64))
  (function create root-view (visibility private) (effect pure)
    (parameter create source (type ByteBuffer) (use borrow))
    (parameter create other (type ByteBuffer) (use borrow))
    (returns ByteBuffer (borrow-from source)) (body (local source)))
  (function create project-view (visibility private) (effect pure)
    (parameter create source (type (owned-product (field payload ByteBuffer))) (use borrow))
    (returns ByteBuffer (borrow-from source))
    (body (borrow-owned-field (type (owned-product (field payload ByteBuffer))) (local source)
      (field payload (binding view (type ByteBuffer))) (in (local view)))))
  (function create consume-view (visibility private) (effect pure)
    (parameter create source (type ByteBuffer) (use borrow))
    (parameter create other (type ByteBuffer) (use borrow))
    (returns I64)
    (body (if (bool true) (i64 7)
      (borrow-call (call root-view (local source) (local other))
        (binding view (type ByteBuffer)) (in (call length (local view)))))))))
declarations.end
"#;

fn fixture() -> (KernelSnapshot, LoadedArtifact) {
    let source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE)
            .expect("borrowed results authored through the public native surface");
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let dir = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&dir.path().join("borrowed-results"), &source, None)
        .unwrap()
        .repository;
    let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let artifact = link_artifact(&repository, compilation.manifest_digest, &[]).unwrap();
    let loaded =
        load_artifact(&artifact.artifact.bytes).expect("complete borrowed-result meaning loads");
    (source, loaded)
}

fn unit(
    source: &KernelSnapshot,
    loaded: &LoadedArtifact,
    name: &str,
) -> (ObjectKey, CompilationUnit) {
    let owner = OwnerKey::Declaration(declaration_named(source, name));
    loaded
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .map(|(key, bytes)| (*key, CompilationUnit::decode(bytes, *key).unwrap()))
        .find(|(_, unit)| unit.source.owner == owner)
        .unwrap()
}

fn code(unit: &mut CompilationUnit) -> &mut CompiledCode {
    let CompilationPayload::Function { code, .. } = &mut unit.payload else {
        panic!("function fixture")
    };
    code
}

fn reject(loaded: &LoadedArtifact, key: ObjectKey, changed: &CompilationUnit, attack: &str) {
    let bytes = match changed.validate() {
        Ok(()) => effect_tests::replace_unit(loaded, key, changed, vec![]),
        Err(error) => effect_tests::replace_rejected_unit(loaded, key, changed, &error.code),
    };
    let error = load_artifact(&bytes)
        .expect_err("consistently rehashed borrowed-result forgery must reject");
    println!("borrowed result {attack}: {}", error.code);
}

#[test]
fn borrowed_result_artifact_rejects_counterfeit_source_and_return_mode() {
    let (source, loaded) = fixture();
    let (key, original) = unit(&source, &loaded, "root-view");
    load_artifact(&effect_tests::replace_unit(&loaded, key, &original, vec![]))
        .expect("neutral rehash");
    for attack in ["source", "owning-mode", "owning-return", "consume-result"] {
        let mut changed = original.clone();
        let CompilationPayload::Function { signature, code } = &mut changed.payload else {
            unreachable!()
        };
        match attack {
            "source" => signature.result_borrow = Some(signature.parameters[1].parameter),
            "owning-mode" => {
                signature.result_borrow = None;
                *code.instructions.last_mut().unwrap() = CompiledInstruction::Return;
            }
            "owning-return" => *code.instructions.last_mut().unwrap() = CompiledInstruction::Return,
            "consume-result" => {
                code.instructions[0] = CompiledInstruction::LoadLocal {
                    local: 0,
                    use_mode: crate::platform::kernel::ParameterUse::Consume,
                }
            }
            _ => unreachable!(),
        }
        reject(&loaded, key, &changed, attack);
    }
}

#[test]
fn borrowed_result_artifact_rejects_erased_or_mismatched_handoff_in_untaken_code() {
    let (source, loaded) = fixture();
    let (key, original) = unit(&source, &loaded, "consume-view");
    let mut inspection = original.clone();
    let instructions = &code(&mut inspection).instructions;
    let begin = instructions
        .iter()
        .position(|i| matches!(i, CompiledInstruction::BeginBorrowCall { .. }))
        .unwrap();
    let end = instructions
        .iter()
        .position(|i| matches!(i, CompiledInstruction::EndOwnedBorrow { .. }))
        .unwrap();
    for attack in [
        "source",
        "position",
        "erase-begin",
        "erase-adoption",
        "erase-end",
        "binding-type",
        "binding-custody",
        "branch-inside",
    ] {
        let mut changed = original.clone();
        let i64_type = changed
            .tables
            .types
            .iter()
            .position(|ty| {
                source
                    .types
                    .get(ty)
                    .is_some_and(|t| matches!(t.form, TypeForm::I64))
            })
            .unwrap() as u32;
        let instructions = &mut code(&mut changed).instructions;
        match attack {
            "source" => {
                let CompiledInstruction::BeginBorrowCall { source_local, .. } =
                    &mut instructions[begin]
                else {
                    unreachable!()
                };
                *source_local = 1;
                let CompiledInstruction::AdoptBorrowResult { source_local, .. } =
                    &mut instructions[begin + 2]
                else {
                    unreachable!()
                };
                *source_local = 1;
            }
            "position" => {
                let CompiledInstruction::BeginBorrowCall {
                    source_position, ..
                } = &mut instructions[begin]
                else {
                    unreachable!()
                };
                *source_position = 1;
            }
            "erase-begin" => instructions[begin] = CompiledInstruction::Jump(begin as u32 + 1),
            "erase-adoption" => instructions[begin + 2] = CompiledInstruction::Drop,
            "erase-end" => instructions[end] = CompiledInstruction::Jump(end as u32 + 1),
            "binding-type" => {
                let CompiledInstruction::AdoptBorrowResult { binding_type, .. } =
                    &mut instructions[begin + 2]
                else {
                    unreachable!()
                };
                *binding_type = i64_type;
            }
            "binding-custody" => {
                let CompiledInstruction::AdoptBorrowResult { binding_local, .. } =
                    &mut instructions[begin + 2]
                else {
                    unreachable!()
                };
                *binding_local = 0;
            }
            "branch-inside" => {
                let branch = instructions
                    .iter()
                    .position(|i| matches!(i, CompiledInstruction::JumpIfFalse(_)))
                    .unwrap();
                instructions[branch] = CompiledInstruction::JumpIfFalse(begin as u32 + 1);
            }
            _ => unreachable!(),
        }
        reject(&loaded, key, &changed, attack);
    }
}

#[test]
fn borrowed_result_artifact_rejects_erased_projected_guard() {
    let (source, loaded) = fixture();
    let (key, mut changed) = unit(&source, &loaded, "project-view");
    let instructions = &mut code(&mut changed).instructions;
    let end = instructions
        .iter()
        .position(|i| matches!(i, CompiledInstruction::EndOwnedBorrow { .. }))
        .unwrap();
    instructions[end] = CompiledInstruction::Jump(end as u32 + 1);
    reject(&loaded, key, &changed, "erased projected ancestor guard");
}
