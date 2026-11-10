//! Independent profile-nine wire expectations and checked canonical fixtures.
//! These controls do not execute the future source consumer or claim a seal.

use super::*;
use crate::platform::compiler::{OptimizationPolicy, build_clean, link_artifact, load_artifact};
use crate::platform::execution::ExecutionControl;
use crate::platform::execution::normalized::{
    NormalizedCommandPolicy, run_pure_command_differential as run_pure_command,
};
use crate::platform::kernel::{KernelSnapshot, Name, OwnerKey, OwnerRecord, PackageId};
use crate::platform::publication::GraphRepository;
use crate::platform::semantic_id::{CaseId, DeclarationId, FieldId};
use std::sync::Arc;

type Check<T> = std::result::Result<T, String>;

fn word(data: &[u8], at: usize) -> u64 {
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&data[at..at + 8]);
    u64::from_le_bytes(bytes)
}

fn append_words(out: &mut Vec<u8>, values: &[u64]) {
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

fn declaration(serial: u64) -> DeclarationReference {
    DeclarationReference {
        package: PackageId::migrate(b"nominal-mode-zero", 0),
        declaration: DeclarationId::migrate(b"nominal-mode-zero", serial),
    }
}

fn record() -> NominalRecord {
    let declaration = declaration(1);
    NominalRecord {
        declaration,
        fields: vec![(
            FieldReference {
                package: declaration.package,
                field: FieldId::migrate(b"nominal-mode-zero", 0),
            },
            1,
        )],
    }
}

fn variant() -> NominalVariant {
    let declaration = declaration(2);
    let mut cases = [
        CaseId::migrate(b"nominal-mode-zero", 0),
        CaseId::migrate(b"nominal-mode-zero", 1),
        CaseId::migrate(b"nominal-mode-zero", 2),
    ];
    cases.sort();
    NominalVariant {
        declaration,
        cases: cases
            .into_iter()
            .zip([None, Some(3), Some(8)])
            .map(|(case, ty)| {
                (
                    CaseReference {
                        package: declaration.package,
                        case,
                    },
                    ty,
                )
            })
            .collect(),
    }
}

#[test]
fn selfhost_nominal_mode0_matches_independent_descriptor_golden() -> Result<(), Diagnostic> {
    let record = record();
    let variant = variant();
    let function = Function {
        result: 1,
        parameters: 0,
        locals: vec![],
        external: 0,
        nodes: vec![
            Node {
                tag: 1,
                immediate: 42,
                ..Node::default()
            },
            Node {
                tag: 11,
                stack: vec![ValueType::owned(1)],
                ..Node::default()
            },
        ],
    };
    let bytes = encode_descriptors(
        &[function],
        &[
            Descriptor::NominalRecord(record.clone()),
            Descriptor::NominalVariant(variant.clone()),
        ],
        0,
    )?;
    assert_eq!(&bytes[8..12], &9_u32.to_le_bytes());
    assert_eq!(&bytes[12..16], &0_u32.to_le_bytes());
    assert_eq!(word(&bytes, 16), 896);
    assert_eq!(word(&bytes, 80), 10);
    assert_eq!(word(&bytes, 88), 480);
    assert_eq!(word(&bytes, 104), 560);
    assert_eq!(word(&bytes, 128), 688);
    let mut expected = Vec::new();
    append_words(&mut expected, &[10, 1, 688, 0, 11, 3, 752, 0, 9, 0, 880, 1]);
    assert_eq!(&bytes[384..480], expected.as_slice());
    expected.clear();
    expected.extend_from_slice(&record.declaration.package.bytes());
    expected.extend_from_slice(&record.declaration.declaration.bytes());
    expected.extend_from_slice(&record.fields[0].0.field.bytes());
    append_words(&mut expected, &[1, 0]);
    expected.extend_from_slice(&variant.declaration.package.bytes());
    expected.extend_from_slice(&variant.declaration.declaration.bytes());
    for ((case, _), words) in variant.cases.iter().zip([[0, 0], [1, 3], [1, 8]]) {
        expected.extend_from_slice(&case.case.bytes());
        append_words(&mut expected, &words);
    }
    assert_eq!(&bytes[688..880], expected.as_slice());
    assert_eq!(word(&bytes, 480 + 64), 10);
    Ok(())
}

#[test]
fn selfhost_nominal_mode0_interner_rejects_affine_forward_and_conflicting_layouts()
-> Result<(), Diagnostic> {
    let mut types = Types::default();
    let record = record();
    assert_eq!(types.nominal_record(record.clone())?, 8);
    assert_eq!(types.nominal_record(record.clone())?, 8);
    let mut conflict = record.clone();
    conflict.fields[0].1 = 2;
    assert!(types.nominal_record(conflict).is_err());
    let mut hidden_buffer = variant();
    hidden_buffer.cases[2].1 = Some(6);
    assert!(types.nominal_variant(hidden_buffer).is_err());
    let mut forward = variant();
    forward.cases[2].1 = Some(9);
    assert!(types.nominal_variant(forward).is_err());
    assert_eq!(types.nominal_variant(variant())?, 9);
    assert!(
        types.fields(8).is_err(),
        "nominal record must not satisfy structural filter result"
    );
    let mut foreign_member = record;
    foreign_member.fields[0].0.package = PackageId::migrate(b"foreign-member", 0);
    assert!(types.nominal_record(foreign_member).is_err());
    Ok(())
}

const SOURCE: &str = r#"declarations.begin
(units
  (module create nominal-producer
    (external create add (visibility private) (implementation core.i64.add)
      (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
    (external create sub (visibility private) (implementation core.i64.subtract)
      (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
    (external create empty (visibility private) (implementation core.buffer.empty) (returns ByteBuffer))
    (record create Point (visibility private)
      (field create left (type I64)) (field create right (type I64)))
    (variant create Choice (visibility private)
      (case create point (payload Point)) (case create absent)
      (case create marker (payload Unit)) (case create number (payload I64)))
    (function create identity (visibility private) (effect pure)
      (parameter create value (type I64)) (returns I64) (body (local value)))
    (function create inspect (visibility private) (effect pure)
      (parameter create value (type Choice)) (returns I64)
      (body (let (binding scratch (type ByteBuffer) (call empty))
        (in (match (local value)
          (arm Choice::point (payload p Point)
            (call identity (call sub (field (local p) Point::right) (field (local p) Point::left))))
          (arm Choice::absent (call identity (i64 7)))
          (arm Choice::marker (payload u Unit) (sequence (local u) (call identity (i64 11))))
          (arm Choice::number (payload n I64) (call identity (local n))))))))
    (function create main (visibility public) (effect pure) (returns I64)
      (body (call add
        (call add
          (call inspect (variant Choice::point
            (record Point (field Point::right (i64 49)) (field Point::left (i64 7)))))
          (call inspect (variant Choice::absent)))
        (call add
          (call inspect (variant Choice::marker (unit)))
          (call inspect (variant Choice::number (i64 -17)))))))
    (component create app (visibility private)
      (port create run (type (function () I64)) (function main))))
  (target create nominal-producer (component nominal-producer::app)
    (runner command) (port nominal-producer::app::run)))
declarations.end"#;

const RESOURCE_SOURCE: &str = r#"declarations.begin
(units
  (module create resource-producer
    (interface create Handle (visibility private)
      (operation create noop (returns Unit) (idempotency idempotent) (external-visibility none)))
    (variant create Envelope (visibility private)
      (case create ordinary (payload I64)) (case create held (payload (resource Handle))))
    (function create main (visibility public) (effect pure) (returns I64)
      (body (sequence (variant Envelope::ordinary (i64 41)) (i64 42))))
    (component create app (visibility private)
      (port create run (type (function () I64)) (function main))))
  (target create resource-producer (component resource-producer::app)
    (runner command) (port resource-producer::app::run)))
declarations.end"#;

const NOMINAL_FILTER: &str = r#"declarations.begin
(units
  (module create named-filter
    (record create Reply (visibility public)
      (field create diagnostic (type I64)) (field create image (type Bytes)))
    (function create main (visibility public) (effect pure)
      (parameter create input (type Bytes)) (returns Reply)
      (body (record Reply (field Reply::diagnostic (i64 0)) (field Reply::image (local input)))))
    (component create app (visibility private)
      (port create run (type (function (Bytes) Reply)) (function main))))
  (target create named-filter (component named-filter::app)
    (runner command) (port named-filter::app::run)))
declarations.end"#;

struct Fixture {
    source: KernelSnapshot,
    repository: GraphRepository,
    program: Arc<NormalizedProgram>,
}

fn fixture(text: &str) -> Check<Fixture> {
    let source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(text)?;
    let memory =
        crate::platform::MemProjectStore::new(None).map_err(|error| format!("store: {error:?}"))?;
    let repository = GraphRepository::create_memory(&memory, &source, None)
        .map_err(|error| format!("repository: {error:?}"))?
        .repository;
    let compiled = build_clean(&repository, OptimizationPolicy::DeterministicBaseline)
        .map_err(|error| format!("compile: {error:?}"))?;
    let artifact = link_artifact(&repository, compiled.manifest_digest, &[])
        .map_err(|error| format!("link: {error:?}"))?;
    let loaded =
        load_artifact(&artifact.artifact.bytes).map_err(|error| format!("load: {error:?}"))?;
    let program = Arc::new(
        NormalizedProgram::prepare(loaded).map_err(|error| format!("prepare: {error:?}"))?,
    );
    Ok(Fixture {
        source,
        repository,
        program,
    })
}

fn main_index(fixture: &Fixture) -> Check<usize> {
    let id = fixture
        .source
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(record))
                if record.name.as_str() == "main" =>
            {
                Some(*id)
            }
            _ => None,
        })
        .ok_or_else(|| "main is absent".to_owned())?;
    let reference = DeclarationReference {
        package: fixture.source.root.package_id,
        declaration: id,
    };
    fixture
        .program
        .function_by_declaration
        .get(&reference)
        .map(|index| index.0 as usize)
        .ok_or_else(|| "prepared main is absent".to_owned())
}

fn oracle(fixture: &Fixture, target: &str, expected: &[u8]) -> Check<()> {
    let view = fixture
        .repository
        .view_current()
        .map_err(|error| format!("view: {error:?}"))?;
    let target = Name::new(target).map_err(|error| format!("target: {error:?}"))?;
    let result = run_pure_command(
        &view,
        &fixture.program,
        &target,
        b"[]",
        NormalizedCommandPolicy::default(),
        &ExecutionControl::uncancelled(),
    )
    .map_err(|error| format!("oracle: {error:?}"))?;
    assert_eq!(result.differential, "equal");
    assert_eq!(result.result_json, expected);
    Ok(())
}

#[test]
fn selfhost_nominal_mode0_source_match_has_arm_specific_state_and_unit_presence() -> Check<()> {
    let fixture = fixture(SOURCE)?;
    oracle(&fixture, "nominal-producer", b"43")?;
    let bytes = export(&fixture.program, main_index(&fixture)?)
        .map_err(|error| format!("export: {error:?}"))?;
    let functions = word(&bytes, 88) as usize;
    let function_count = word(&bytes, 96) as usize;
    let nodes = word(&bytes, 104) as usize;
    let mut matches = 0;
    let mut absent_constructors = 0;
    let mut unit_constructors = 0;
    for fi in 0..function_count {
        let function = functions + fi * 80;
        let locals = word(&bytes, function + 24) as usize;
        let local_types = word(&bytes, function + 56) as usize;
        let first = word(&bytes, function + 32) as usize;
        let count = word(&bytes, function + 40) as usize;
        for ip in 0..count {
            let node = nodes + (first + ip) * 64;
            let tag = word(&bytes, node);
            if tag == 22 {
                let ty = word(&bytes, node + 8) as usize;
                let descriptor = 160 + (ty - 1) * 32;
                let member = word(&bytes, descriptor + 16) as usize
                    + 32
                    + word(&bytes, node + 16) as usize * 32;
                let presence = word(&bytes, member + 16);
                assert_eq!(word(&bytes, node + 24), presence);
                let before = word(&bytes, node + 56);
                let after = word(&bytes, node + 64 + 56);
                if presence == 0 {
                    absent_constructors += 1;
                    assert_eq!(after, before + 1);
                } else if word(&bytes, member + 24) == 3 {
                    unit_constructors += 1;
                    assert_eq!(after, before);
                }
            }
            if tag != 23 {
                continue;
            }
            matches += 1;
            let ty = word(&bytes, node + 8) as usize;
            let descriptor = 160 + (ty - 1) * 32;
            assert_eq!(word(&bytes, descriptor), 11);
            let cases = word(&bytes, descriptor + 8) as usize;
            let case_rows = word(&bytes, descriptor + 16) as usize + 32;
            assert_eq!(word(&bytes, node + 16) as usize, cases);
            assert_eq!(cases, 4);
            let arms = word(&bytes, node + 24) as usize;
            let before = word(&bytes, node + 48) as usize;
            let depth = word(&bytes, node + 56) as usize;
            let mut live_buffers = 0;
            for local in 0..locals {
                if word(&bytes, local_types + local * 16) == 6 && bytes[before + local] == 1 {
                    live_buffers += 1;
                }
            }
            assert_eq!(live_buffers, 1);
            for ordinal in 0..cases {
                let row = arms + ordinal * 32;
                let member = case_rows + ordinal * 32;
                assert_eq!(word(&bytes, row) as usize, ordinal);
                let target = word(&bytes, row + 8) as usize;
                assert!(target > ip && target < count);
                let presence = word(&bytes, row + 16);
                assert_eq!(presence, word(&bytes, member + 16));
                let local = word(&bytes, row + 24) as usize;
                let next = nodes + (first + target) * 64;
                assert_eq!(word(&bytes, next + 56) as usize, depth - 1);
                let after = word(&bytes, next + 48) as usize;
                assert_eq!(
                    &bytes[before + locals..before + locals + (depth - 1) * 16],
                    &bytes[after + locals..after + locals + (depth - 1) * 16]
                );
                if presence == 1 {
                    assert_eq!(bytes[before + local], 0);
                    assert_eq!(bytes[after + local], 1);
                    assert_eq!(
                        word(&bytes, local_types + local * 16),
                        word(&bytes, member + 24)
                    );
                    assert_eq!(word(&bytes, local_types + local * 16 + 8), 0);
                } else {
                    assert_eq!(local, 0);
                }
                for slot in 0..locals {
                    if presence == 1 && slot == local {
                        continue;
                    }
                    if word(&bytes, local_types + slot * 16) == 6 {
                        assert_eq!(bytes[after + slot], bytes[before + slot]);
                    } else {
                        assert!(bytes[after + slot] <= bytes[before + slot]);
                    }
                }
            }
        }
    }
    assert_eq!(matches, 1);
    assert_eq!(absent_constructors, 1);
    assert_eq!(unit_constructors, 1);
    Ok(())
}

#[test]
fn selfhost_nominal_mode0_rejects_unselected_resource_case_after_independent_oracle() -> Check<()> {
    let fixture = fixture(RESOURCE_SOURCE)?;
    oracle(&fixture, "resource-producer", b"42")?;
    let result = export(&fixture.program, main_index(&fixture)?);
    let error = match result {
        Ok(_) => return Err("export accepted resource-bearing variant".to_owned()),
        Err(error) => error,
    };
    assert_eq!(error.code, "compiler_input_unsupported");
    Ok(())
}

#[test]
fn selfhost_nominal_mode0_keeps_binary_filter_result_structural() -> Check<()> {
    let fixture = fixture(NOMINAL_FILTER)?;
    let result = export(&fixture.program, main_index(&fixture)?);
    let error = match result {
        Ok(_) => return Err("export accepted nominal filter result".to_owned()),
        Err(error) => error,
    };
    assert_eq!(error.code, "compiler_input_unsupported");
    Ok(())
}
