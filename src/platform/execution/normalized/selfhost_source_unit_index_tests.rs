//! Direct controls for the canonical trace index and packing boundary.
//!
//! Register beneath selfhost_source_artifact_tests after public adoption. These
//! are internal trace fixtures, not source-admission or native-stage evidence.
//! Both evaluators use the same maintained helper source; expected keys, PCs and
//! packed bytes are constructed independently below.

use super::*;
use crate::platform::execution::normalized::child_storage::Fields;
use crate::platform::execution::normalized::reference::{
    NormalizedReferenceBinding, NormalizedReferenceOwnerRead, NormalizedReferenceRead,
};
use crate::platform::execution::normalized::reference_schema::NormalizedReferenceSchema;
use crate::platform::kernel::{BlobObjectDigest, Name, PackageId};
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

struct CompilerReader {
    source: &'static KernelSnapshot,
    schema: Arc<NormalizedReferenceSchema>,
}

impl NormalizedReferenceRead for CompilerReader {
    fn binding(&self) -> Result<NormalizedReferenceBinding, ExecutionError> {
        NormalizedReferenceRead::binding(self.source)
    }

    fn owner(&self, owner: OwnerKey) -> Result<NormalizedReferenceOwnerRead, ExecutionError> {
        NormalizedReferenceRead::owner(self.source, owner)
    }

    fn schema(&self) -> Result<Arc<NormalizedReferenceSchema>, ExecutionError> {
        Ok(Arc::clone(&self.schema))
    }

    fn blob(&self, digest: BlobObjectDigest) -> Result<Vec<u8>, ExecutionError> {
        NormalizedReferenceRead::blob(self.source, digest)
    }

    fn owner_in_package(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<NormalizedReferenceOwnerRead, ExecutionError> {
        NormalizedReferenceRead::owner_in_package(self.source, package, owner)
    }
}

fn reader() -> Check<&'static CompilerReader> {
    static READER: OnceLock<Check<CompilerReader>> = OnceLock::new();
    READER
        .get_or_init(|| {
            let (source, _) = compiler();
            let schema = NormalizedReferenceRead::schema(source)
                .map_err(|error| format!("index canonical schema: {error:?}"))?;
            Ok(CompilerReader { source, schema })
        })
        .as_ref()
        .map_err(Clone::clone)
}

struct Engine {
    reference: bool,
    reader: &'static CompilerReader,
    program: &'static NormalizedProgram,
}

impl Engine {
    fn new(reference: bool) -> Check<Self> {
        Ok(Self {
            reference,
            reader: reader()?,
            program: &compiler().1,
        })
    }

    fn call(
        &self,
        module: &str,
        name: &str,
        arguments: Vec<NormalizedValue>,
    ) -> Check<NormalizedValue> {
        let entry = function_named(self.reader.source, module, name)?;
        let policy = NormalizedRunPolicy::foreground();
        let control = ExecutionControl::uncancelled();
        let (value, frames) = if self.reference {
            let (value, observed) =
                NormalizedReferenceInterpreter::from_reader(self.reader, self.program, policy)
                    .invoke(entry, arguments, None, &control)
                    .map_err(|error| format!("reference {module}::{name}: {error:?}"))?;
            (value, observed.live_call_frames_after)
        } else {
            let (value, observed) = NormalizedVm::for_test(self.program, policy)
                .invoke(entry, arguments, None, &control)
                .map_err(|error| format!("VM {module}::{name}: {error:?}"))?;
            (value, observed.live_call_frames_after)
        };
        equal(frames, 0, "index invocation frames reclaimed")?;
        Ok(value)
    }

    fn prepare(&self, trace: &[u8], count: i64) -> Check<NormalizedValue> {
        self.call(
            "source-unit-code-index",
            "prepare",
            vec![NormalizedValue::bytes(trace), NormalizedValue::I64(count)],
        )
    }

    fn target(&self, name: &str, trace: &[u8], index: &[u8], owner: i64, label: i64) -> Check<i64> {
        scalar(self.call(
            "source-unit-code-index",
            name,
            vec![
                NormalizedValue::bytes(trace),
                NormalizedValue::bytes(index),
                NormalizedValue::I64(owner),
                NormalizedValue::I64(label),
            ],
        )?)
    }

    fn project(&self, trace: &[u8], count: i64) -> Check<NormalizedValue> {
        let fields = [
            ("code", NormalizedValue::I64(0)),
            ("count", NormalizedValue::I64(count)),
            ("locals", NormalizedValue::I64(2)),
            ("pool", NormalizedValue::bytes(Vec::<u8>::new())),
            ("trace", NormalizedValue::bytes(trace)),
        ]
        .into_iter()
        .map(|(name, value)| {
            Name::new(name.to_owned())
                .map(|name| (name, value))
                .map_err(|error| format!("index field name: {error:?}"))
        })
        .collect::<Check<Vec<_>>>()?;
        let fields = Fields::try_new(fields)
            .map_err(|error| format!("index structural fields: {error:?}"))?;
        self.call(
            "source-unit-code-pack",
            "project",
            vec![
                NormalizedValue::Record(NormalizedRecord::Structural { fields }),
                NormalizedValue::I64(0),
                NormalizedValue::I64(1024),
            ],
        )
    }
}

fn field<'a>(value: &'a NormalizedValue, name: &str) -> Check<&'a NormalizedValue> {
    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = value else {
        return Err("index helper did not return a structural record".to_owned());
    };
    fields
        .iter()
        .find_map(|(key, value)| (key.as_str() == name).then_some(value))
        .ok_or_else(|| format!("index result field missing: {name}"))
}

fn bytes(value: &NormalizedValue) -> Check<&[u8]> {
    match field(value, "bytes")? {
        NormalizedValue::Bytes(value) => Ok(value.as_ref()),
        _ => Err("index result bytes has wrong type".to_owned()),
    }
}

fn code(value: &NormalizedValue, expected: i64) -> Check<()> {
    match field(value, "code")? {
        NormalizedValue::I64(actual) => equal(*actual, expected, "index/packing diagnostic"),
        _ => Err("index result code has wrong type".to_owned()),
    }
}

fn scalar(value: NormalizedValue) -> Check<i64> {
    match value {
        NormalizedValue::I64(value) => Ok(value),
        _ => Err("index scalar result has wrong type".to_owned()),
    }
}

fn words(out: &mut Vec<u8>, values: &[u64]) {
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

fn bits(value: i64) -> u64 {
    u64::from_le_bytes(value.to_le_bytes())
}

fn event(out: &mut Vec<u8>, header: [u64; 6], payload: &[u64]) {
    words(out, &header);
    words(out, payload);
}

// This is a trace/packing fixture, not a type-correct executable program.
// The literal offsets below are a separate oracle for the append sequence.
fn trace() -> Vec<u8> {
    let mut out = Vec::new();
    event(&mut out, [4, 0, 0, bits(-7), 0, 0], &[]);
    event(
        &mut out,
        [20, 256, 0, 0, 0, 3],
        &[0, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0],
    );
    event(&mut out, [0, 256, 0, 2, 0, 3], &[9, 0, 0]);
    event(&mut out, [4, 0, 0, 7, 0, 0], &[]);
    event(&mut out, [11, 256, 3, 0, 0, 0], &[]);
    event(&mut out, [0, 256, 1, 4, 0, 3], &[2, 1, 0]);
    event(&mut out, [4, 0, 0, bits(i64::MIN), 0, 0], &[]);
    event(&mut out, [11, 256, 3, 0, 0, 0], &[]);
    event(&mut out, [0, 256, 2, 6, 0, 3], &[4, 1, 1]);
    event(&mut out, [10, 1, 0, 0, 0, 0], &[]);
    event(&mut out, [4, 0, 0, 23, 0, 0], &[]);
    event(&mut out, [11, 1, 1, 0, 0, 0], &[]);
    event(&mut out, [0, 1, 0, 9, 0, 0], &[]);
    event(&mut out, [4, 0, 0, 29, 0, 0], &[]);
    event(&mut out, [0, 1, 1, 10, 0, 0], &[]);
    event(&mut out, [11, 256, 3, 0, 0, 0], &[]);
    event(&mut out, [0, 256, 3, 11, 0, 0], &[]);
    event(&mut out, [27, 0, 0, 0, 0, 0], &[]);
    out
}

fn expected_pack() -> Vec<u8> {
    let mut out = Vec::new();
    words(&mut out, &[0, 2, 1056, 12]);
    for row in [
        [4, 0, 0, bits(-7), 0, 0],
        [20, 0, 0, 0, 1632, 3],
        [4, 0, 0, 7, 0, 0],
        [11, 11, 0, 0, 0, 0],
        [4, 0, 0, bits(i64::MIN), 0, 0],
        [11, 11, 0, 0, 0, 0],
        [10, 9, 0, 0, 0, 0],
        [4, 0, 0, 23, 0, 0],
        [11, 10, 0, 0, 0, 0],
        [4, 0, 0, 29, 0, 0],
        [11, 11, 0, 0, 0, 0],
        [27, 0, 0, 0, 0, 0],
    ] {
        words(&mut out, &row);
    }
    // Case-table encounter order is independent of numeric table-index order.
    // Presence 1 with local zero remains different from absence 0/local zero.
    words(&mut out, &[9, 2, 0, 0, 2, 4, 1, 0, 4, 6, 1, 1]);
    out
}

fn changed_word(original: &[u8], at: usize, value: u64) -> Check<Vec<u8>> {
    let mut changed = original.to_vec();
    changed
        .get_mut(at..at + 8)
        .ok_or("test word outside trace")?
        .copy_from_slice(&value.to_le_bytes());
    Ok(changed)
}

#[test]
fn selfhost_source_unit_index_preserves_labels_metadata_and_packed_bytes() -> Check<()> {
    let trace = trace();
    equal(trace.len(), 1032, "literal trace extent")?;
    let mut expected = Vec::new();
    for row in [
        [1, 0, 744],
        [1, 1, 840],
        [256, 0, 192],
        [256, 1, 360],
        [256, 2, 528],
        [256, 3, 936],
    ] {
        words(&mut expected, &row);
    }
    for reference in [false, true] {
        let engine = Engine::new(reference)?;
        let indexed = engine.prepare(&trace, 12)?;
        code(&indexed, 0)?;
        let index = bytes(&indexed)?;
        equal_bytes(
            index,
            &expected,
            "numeric index ordering and exact trace offsets",
        )?;
        for (owner, label, pc) in [
            (1, 0, 9),
            (1, 1, 10),
            (256, 0, 2),
            (256, 1, 4),
            (256, 2, 6),
            (256, 3, 11),
        ] {
            equal(
                engine.target("target", &trace, index, owner, label)?,
                pc,
                "indexed target PC",
            )?;
            let old = scalar(engine.call(
                "source-unit-code-pack",
                "label",
                vec![
                    NormalizedValue::bytes(trace.as_slice()),
                    NormalizedValue::I64(owner),
                    NormalizedValue::I64(label),
                    NormalizedValue::I64(0),
                ],
            )?)?;
            equal(old, pc, "preserved direct linear helper contract")?;
        }
        for (label, offset) in [(0, 192), (1, 360), (2, 528)] {
            equal(
                engine.target("arm", &trace, index, 256, label)?,
                offset,
                "arm metadata row",
            )?;
        }
        equal(
            engine.target("arm", &trace, index, 256, 3)?,
            -1,
            "plain label is not arm metadata",
        )?;
        equal(
            engine.target("target", &trace, index, 999, 0)?,
            -1,
            "missing label",
        )?;
        let packed = engine.project(&trace, 12)?;
        code(&packed, 0)?;
        equal_bytes(
            bytes(&packed)?,
            &expected_pack(),
            "exact private code packing",
        )?;
    }
    Ok(())
}

#[test]
fn selfhost_source_unit_index_rejects_invalid_framing_and_bindings() -> Check<()> {
    let original = trace();
    let mut trailing = original.clone();
    trailing.push(0);
    let mut eof_label = original.clone();
    event(&mut eof_label, [0, 777, 0, 12, 0, 0], &[]);
    let duplicate = changed_word(&original, 360 + 16, 0)?;
    // Keep the later label's correct PC and metadata: only its key is duplicated.
    let mut short_metadata = changed_word(&original, 192 + 40, 1)?;
    // Delete the excess payload too, so refusal reaches metadata count, not framing.
    drop(short_metadata.drain(192 + 56..192 + 72));
    let cases = [
        ("short header", original[..47].to_vec(), 12),
        ("short label payload", original[..192 + 71].to_vec(), 12),
        ("trailing byte", trailing, 12),
        (
            "unrepresentable item count",
            changed_word(&original, 40, u64::MAX)?,
            12,
        ),
        ("reserved word", changed_word(&original, 32, 1)?, 12),
        ("duplicate numeric key", duplicate, 12),
        (
            "negative owner",
            changed_word(&original, 192 + 8, bits(-1))?,
            12,
        ),
        (
            "negative label",
            changed_word(&original, 192 + 16, bits(-1))?,
            12,
        ),
        ("wrong label PC", changed_word(&original, 192 + 24, 3)?, 12),
        ("label metadata width", short_metadata, 12),
        (
            "metadata presence",
            changed_word(&original, 192 + 56, 2)?,
            12,
        ),
        (
            "absence with local",
            changed_word(&original, 192 + 64, 1)?,
            12,
        ),
        (
            "negative case index",
            changed_word(&original, 192 + 48, bits(-1))?,
            12,
        ),
        ("label at EOF", eof_label, 12),
        ("instruction count one short", original.clone(), 11),
        ("instruction count one long", original.clone(), 13),
        ("negative instruction count", original.clone(), -1),
        (
            "instruction count beyond u32 domain",
            original.clone(),
            4_294_967_297,
        ),
    ];
    for reference in [false, true] {
        let engine = Engine::new(reference)?;
        for (name, trace, count) in &cases {
            let result = engine
                .prepare(trace, *count)
                .map_err(|error| format!("{name}: {error}"))?;
            code(&result, 3).map_err(|error| format!("{name}: {error}"))?;
            equal_bytes(bytes(&result)?, &[], name)?;
        }
        for (name, trace) in [
            (
                "missing Jump target",
                changed_word(&original, 312 + 8, 777)?,
            ),
            (
                "missing Switch arm",
                changed_word(&original, 48 + 48 + 8, 999)?,
            ),
            (
                "Switch points to plain label",
                changed_word(&original, 48 + 48 + 8, 3)?,
            ),
        ] {
            // A well-framed unique index is accepted first. The exact failure is
            // then lookup/metadata resolution in pack::project, diagnostic 3.
            code(&engine.prepare(&trace, 12)?, 0)?;
            let result = engine
                .project(&trace, 12)
                .map_err(|error| format!("{name}: {error}"))?;
            code(&result, 3).map_err(|error| format!("{name}: {error}"))?;
        }
    }
    Ok(())
}

#[test]
fn selfhost_source_unit_index_handles_wide_numeric_keys() -> Check<()> {
    let mut trace = Vec::new();
    let mut expected_rows = BTreeMap::new();
    for (pc, label) in (0_u64..1025).rev().enumerate() {
        let owner = if label % 2 == 0 { 256 } else { 1 };
        let at = u64::try_from(trace.len()).map_err(|error| error.to_string())?;
        let pc = u64::try_from(pc).map_err(|error| error.to_string())?;
        ensure(
            expected_rows.insert((owner, label), (at, pc)).is_none(),
            "fixture keys unique",
        )?;
        event(&mut trace, [0, owner, label, pc, 0, 0], &[]);
        event(&mut trace, [4, 0, 0, label, 0, 0], &[]);
    }
    event(&mut trace, [27, 0, 0, 0, 0, 0], &[]);
    let mut expected = Vec::new();
    for ((owner, label), (at, _)) in &expected_rows {
        words(&mut expected, &[*owner, *label, *at]);
    }
    for reference in [false, true] {
        let engine = Engine::new(reference)?;
        let empty = engine.prepare(&[], 0)?;
        code(&empty, 0)?;
        equal_bytes(bytes(&empty)?, &[], "empty trace index")?;
        let indexed = engine.prepare(&trace, 1026)?;
        code(&indexed, 0)?;
        equal_bytes(
            bytes(&indexed)?,
            &expected,
            "1025 numeric keys across merge runs",
        )?;
        for key in [(1_u64, 1_u64), (256, 0), (256, 1024)] {
            let (_, pc) = expected_rows.get(&key).ok_or("wide fixture key absent")?;
            equal(
                engine.target(
                    "target",
                    &trace,
                    bytes(&indexed)?,
                    i64::try_from(key.0).map_err(|error| error.to_string())?,
                    i64::try_from(key.1).map_err(|error| error.to_string())?,
                )?,
                i64::try_from(*pc).map_err(|error| error.to_string())?,
                "wide indexed target",
            )?;
        }
    }
    Ok(())
}
