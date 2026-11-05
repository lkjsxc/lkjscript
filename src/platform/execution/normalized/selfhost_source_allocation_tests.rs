//! Direct projection controls for allocation-path rewrites.
//!
//! These invoke maintained source helpers in the VM and canonical reference,
//! bypassing E1 deliberately. Their derived parent argument is not a certificate:
//! each helper must independently check the relation it consumes.

use super::*;
use crate::platform::execution::normalized::reference::{
    NormalizedReferenceBinding, NormalizedReferenceOwnerRead, NormalizedReferenceRead,
};
use crate::platform::execution::normalized::reference_schema::NormalizedReferenceSchema;
use crate::platform::kernel::{BlobObjectDigest, PackageId};
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
                .map_err(|error| format!("projection canonical schema: {error:?}"))?;
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
        equal(frames, 0, "projection invocation frames reclaimed")?;
        Ok(value)
    }

    fn parents(&self, carrier: &[u8]) -> Check<Vec<u8>> {
        let value = self.call(
            "source-members",
            "prepare",
            vec![NormalizedValue::bytes(carrier)],
        )?;
        code(&value, 0)?;
        Ok(bytes(&value, "rows")?.to_vec())
    }

    fn inventory(&self, name: &str, carrier: &[u8], parents: &[u8]) -> Check<NormalizedValue> {
        self.call(
            "source-artifact-owners",
            name,
            vec![
                NormalizedValue::bytes(carrier),
                NormalizedValue::bytes(parents),
            ],
        )
    }
}

fn field<'a>(value: &'a NormalizedValue, name: &str) -> Check<&'a NormalizedValue> {
    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = value else {
        return Err("projection did not return a structural record".to_owned());
    };
    fields
        .iter()
        .find_map(|(key, value)| (key.as_str() == name).then_some(value))
        .ok_or_else(|| format!("projection field missing: {name}"))
}

fn bytes<'a>(value: &'a NormalizedValue, name: &str) -> Check<&'a [u8]> {
    match field(value, name)? {
        NormalizedValue::Bytes(bytes) => Ok(bytes.as_ref()),
        _ => Err(format!("projection field is not Bytes: {name}")),
    }
}

fn integer(value: &NormalizedValue, name: &str) -> Check<i64> {
    match field(value, name)? {
        NormalizedValue::I64(value) => Ok(*value),
        _ => Err(format!("projection field is not I64: {name}")),
    }
}

fn code(value: &NormalizedValue, expected: i64) -> Check<()> {
    equal(
        integer(value, "code")?,
        expected,
        "direct projection diagnostic",
    )
}

fn row(source: &KernelSnapshot, key: OwnerKey) -> Check<usize> {
    source
        .owners
        .keys()
        .position(|candidate| *candidate == key)
        .ok_or_else(|| format!("fixture owner absent: {key:?}"))
}

fn word(bytes: &[u8], at: usize) -> Check<u64> {
    let array: [u8; 8] = bytes
        .get(at..at + 8)
        .ok_or("projection word extent")?
        .try_into()
        .map_err(|_| "projection word width")?;
    Ok(u64::from_le_bytes(array))
}

fn function_key(source: &KernelSnapshot, name: &str) -> Check<OwnerKey> {
    Ok(OwnerKey::Declaration(
        function_named(source, "artifact-controls", name)?.declaration,
    ))
}

fn callable_needs_runtime(
    source: &KernelSnapshot,
    mut parameters: impl Iterator<Item = OwnerKey>,
    result: TypeObjectDigest,
) -> bool {
    source
        .types
        .get(&result)
        .is_some_and(|object| matches!(object.form, kernel::TypeForm::ByteBuffer))
        || parameters.any(|key| {
            matches!(
                source.owners.get(&key),
                Some(OwnerRecord::Parameter(parameter))
                    if parameter.use_mode != kernel::ParameterUse::Unrestricted
            )
        })
}

// The expected relation comes from canonical declaration arrays, not from a
// second byte parser or the old source helper.
fn expected_inventory(source: &KernelSnapshot) -> Check<(Vec<u8>, Vec<u8>)> {
    let positions = source
        .owners
        .keys()
        .enumerate()
        .map(|(index, key)| (*key, index))
        .collect::<BTreeMap<_, _>>();
    let mut members = Vec::new();
    let mut classes = vec![0; positions.len()];
    for (index, record) in source.owners.values().enumerate() {
        let mut children = Vec::new();
        let included = match record {
            OwnerRecord::Declaration(declaration) => match &declaration.payload {
                kernel::DeclarationPayload::External(function) => {
                    children.extend(
                        function
                            .type_parameters
                            .iter()
                            .map(|id| OwnerKey::TypeParameter(*id)),
                    );
                    children.extend(
                        function
                            .parameters
                            .iter()
                            .map(|id| OwnerKey::Parameter(*id)),
                    );
                    callable_needs_runtime(
                        source,
                        function
                            .parameters
                            .iter()
                            .map(|id| OwnerKey::Parameter(*id)),
                        function.result,
                    )
                }
                kernel::DeclarationPayload::Function(function) => {
                    children.extend(
                        function
                            .type_parameters
                            .iter()
                            .map(|id| OwnerKey::TypeParameter(*id)),
                    );
                    children.extend(
                        function
                            .parameters
                            .iter()
                            .map(|id| OwnerKey::Parameter(*id)),
                    );
                    callable_needs_runtime(
                        source,
                        function
                            .parameters
                            .iter()
                            .map(|id| OwnerKey::Parameter(*id)),
                        function.result.ty,
                    )
                }
                kernel::DeclarationPayload::Component { ports, .. } => {
                    children.extend(ports.iter().map(|id| OwnerKey::Port(*id)));
                    true
                }
                _ => false,
            },
            OwnerRecord::Target(_) => true,
            _ => false,
        };
        if included {
            classes[index] = 1;
        }
        for child in children {
            let child_row = *positions.get(&child).ok_or("canonical child absent")?;
            let kind = source
                .owners
                .get(&child)
                .ok_or("canonical child absent")?
                .kind()
                .tag();
            members.push([child_row as u64, index as u64, u64::from(kind)]);
            classes[child_row] = 1;
        }
    }
    members.sort_unstable();
    ensure(
        members.windows(2).all(|pair| pair[0][0] < pair[1][0]),
        "canonical fixture membership is not unique",
    )?;
    let mut encoded = Vec::new();
    for triple in members {
        for value in triple {
            encoded.extend_from_slice(&value.to_le_bytes());
        }
    }
    Ok((encoded, classes))
}

#[test]
fn selfhost_source_allocation_membership_matches_canonical_inventory() -> Check<()> {
    let case = fixture_case()?;
    let (expected_members, expected_classes) = expected_inventory(&case.source)?;
    let kinds = case
        .source
        .owners
        .values()
        .map(|owner| owner.kind().tag())
        .collect::<BTreeSet<_>>();
    ensure(
        [5, 6, 9, 11, 15, 19, 20]
            .iter()
            .all(|kind| kinds.contains(kind)),
        "fixture misses a callable/child/component/target family",
    )?;
    ensure(
        expected_classes.contains(&0) && expected_classes.contains(&1),
        "classification control is trivial",
    )?;
    for reference in [false, true] {
        let engine = Engine::new(reference)?;
        let parents = engine.parents(&case.carrier)?;
        for (name, expected) in [
            ("members", expected_members.as_slice()),
            ("classifications", expected_classes.as_slice()),
        ] {
            let actual = engine.inventory(name, &case.carrier, &parents)?;
            code(&actual, 0)?;
            equal_bytes(bytes(&actual, "rows")?, expected, name)?;
        }
    }
    Ok(())
}

fn owner_image(source: &KernelSnapshot, key: OwnerKey) -> Check<Vec<u8>> {
    let record = source.owners.get(&key).ok_or("fixture owner absent")?;
    kernel::encode_owner(record)
        .map(|pair| pair.1)
        .map_err(|error| format!("fixture owner encoding: {error:?}"))
}

// Source fixtures rebind the object digest, owner-map commitment, semantic state,
// and synthetic revision. Rebuild the inner envelope too, so these negatives do
// not depend on a stale checksum. They test direct projection, not E1 admission.
fn changed_owner(source: &KernelSnapshot, key: OwnerKey, image: &[u8]) -> Check<Vec<u8>> {
    let end = image.len().checked_sub(32).ok_or("owner envelope footer")?;
    let payload = image.get(18..end).ok_or("owner envelope payload")?;
    let image = object_envelope(
        &kernel::contract::OWNER_MAGIC,
        kernel::contract::OWNER_ENVELOPE_DOMAIN,
        payload,
    );
    Ok(source_fixture::raw_object_packet(
        source,
        Some((key, &image)),
        None,
    ))
}

fn exact_window(bytes: &[u8], pattern: &[u8]) -> Check<usize> {
    let mut matches = bytes
        .windows(pattern.len())
        .enumerate()
        .filter_map(|(index, window)| (window == pattern).then_some(index));
    let found = matches.next().ok_or("fixture reference window absent")?;
    ensure(
        matches.next().is_none(),
        "fixture reference window is ambiguous",
    )?;
    Ok(found)
}

fn source_reference(key: OwnerKey, namespace: u8) -> Vec<u8> {
    let mut encoded = kernel::owner_map_key(key).to_vec();
    encoded[0] = namespace;
    encoded
}

#[test]
fn selfhost_source_allocation_membership_rechecks_independent_parent_inputs() -> Check<()> {
    let case = fixture_case()?;
    let sum = function_key(&case.source, "sum")?;
    let sum_record = match case.source.owners.get(&sum) {
        Some(OwnerRecord::Declaration(record)) => record,
        _ => return Err("fixture sum declaration missing".to_owned()),
    };
    let parameters = match &sum_record.payload {
        kernel::DeclarationPayload::External(function) => &function.parameters,
        _ => return Err("fixture sum is not external".to_owned()),
    };
    equal(parameters.len(), 2, "fixture parameter pair")?;
    let first = OwnerKey::Parameter(parameters[0]);
    let second = OwnerKey::Parameter(parameters[1]);
    let original = owner_image(&case.source, sum)?;
    let first_at = exact_window(&original, &source_reference(first, 7))?;
    let second_at = exact_window(&original, &source_reference(second, 7))?;
    let first_image = owner_image(&case.source, first)?;
    let declared_at = exact_window(&first_image, &source_reference(sum, 3))?;
    let mut controls = Vec::new();

    let mut changed = original.clone();
    changed[first_at] = 17;
    controls.push((
        "child-namespace",
        changed_owner(&case.source, sum, &changed)?,
        12,
    ));

    let mut changed = first_image.clone();
    changed[declared_at] = 7;
    controls.push((
        "declared-parent-namespace",
        changed_owner(&case.source, first, &changed)?,
        12,
    ));

    let missing = [0xa5; 16];
    ensure(
        !case
            .source
            .owners
            .keys()
            .any(|key| kernel::owner_map_key(*key)[1..] == missing),
        "missing endpoint control collides with a fixture identity",
    )?;
    let mut changed = original.clone();
    changed[first_at + 1..first_at + 17].copy_from_slice(&missing);
    controls.push((
        "missing-child",
        changed_owner(&case.source, sum, &changed)?,
        14,
    ));

    let mut changed = original.clone();
    changed[first_at + 1..first_at + 17].copy_from_slice(&kernel::owner_map_key(sum)[1..]);
    controls.push((
        "wrong-child-domain",
        changed_owner(&case.source, sum, &changed)?,
        14,
    ));

    let mut changed = original.clone();
    changed[second_at..second_at + 17].copy_from_slice(&source_reference(first, 7));
    controls.push((
        "duplicate-child",
        changed_owner(&case.source, sum, &changed)?,
        15,
    ));

    let other = function_key(&case.source, "list-get")?;
    let mut changed = first_image.clone();
    changed[declared_at..declared_at + 17].copy_from_slice(&source_reference(other, 3));
    controls.push((
        "conflicting-declared-parent",
        changed_owner(&case.source, first, &changed)?,
        15,
    ));

    for reference in [false, true] {
        let engine = Engine::new(reference)?;
        // Only the valid input goes through links/containment preparation. Every malformed carrier reaches
        // the new helper directly with the original, explicitly untrusted rows.
        let parents = engine.parents(&case.carrier)?;
        for (label, carrier, diagnostic) in &controls {
            ensure(carrier != &case.carrier, "negative carrier unchanged")?;
            for name in ["members", "classifications"] {
                let actual = engine.inventory(name, carrier, &parents)?;
                code(&actual, *diagnostic).map_err(|error| format!("{label}/{name}: {error}"))?;
                equal_bytes(
                    bytes(&actual, "rows")?,
                    &[],
                    "refusal publishes no partial rows",
                )?;
            }
        }
        for (label, at, value) in [
            (
                "parent-row-identity",
                row(&case.source, first)? * 12,
                row(&case.source, second)?,
            ),
            (
                "parent-row-parent",
                row(&case.source, first)? * 12 + 4,
                row(&case.source, other)?,
            ),
        ] {
            let mut changed = parents.clone();
            let value = u32::try_from(value).map_err(|_| "parent row domain")?;
            changed
                .get_mut(at..at + 4)
                .ok_or("parent row extent")?
                .copy_from_slice(&value.to_le_bytes());
            for name in ["members", "classifications"] {
                let actual = engine.inventory(name, &case.carrier, &changed)?;
                code(&actual, 15).map_err(|error| format!("{label}/{name}: {error}"))?;
                equal_bytes(
                    bytes(&actual, "rows")?,
                    &[],
                    "parent refusal publishes no partial rows",
                )?;
            }
        }
    }
    Ok(())
}

#[test]
fn selfhost_source_allocation_visibility_and_signature_prefixes_are_exact() -> Check<()> {
    let case = fixture_case()?;
    let public = function_key(&case.source, "text-id")?;
    let private = function_key(&case.source, "sum")?;
    let original = owner_image(&case.source, public)?;
    equal(original.get(56), Some(&7), "fixture compact name length")?;
    equal_bytes(
        original.get(57..64).ok_or("fixture name extent")?,
        b"text-id",
        "fixture name",
    )?;
    equal(original.get(64), Some(&2), "fixture public visibility")?;
    let mut visibility = Vec::new();
    for (label, use_mode) in [("private", 0), ("package", 1)] {
        let mut image = original.clone();
        image[64] = use_mode;
        visibility.push((label, changed_owner(&case.source, public, &image)?));
    }
    let mut image = original.clone();
    image[57] = b'!';
    visibility.push(("invalid-name", changed_owner(&case.source, public, &image)?));
    let mut image = original.clone();
    image[56] = 0;
    image[57] = 2;
    visibility.push((
        "empty-name-followed-by-public",
        changed_owner(&case.source, public, &image)?,
    ));
    let mut image = original.clone();
    // A complete valid name with no following visibility byte in its payload.
    image.truncate(64);
    image.extend_from_slice(&[0; 32]);
    visibility.push((
        "missing-visibility",
        changed_owner(&case.source, public, &image)?,
    ));

    for reference in [false, true] {
        let engine = Engine::new(reference)?;
        for (label, carrier, owner, expected) in [
            ("public", case.carrier.as_slice(), public, true),
            ("private-original", case.carrier.as_slice(), private, false),
        ]
        .into_iter()
        .chain(
            visibility
                .iter()
                .map(|(label, carrier)| (*label, carrier.as_slice(), public, false)),
        ) {
            let actual = engine.call(
                "source-interface-record",
                "public",
                vec![
                    NormalizedValue::bytes(carrier),
                    NormalizedValue::I64(row(&case.source, owner)? as i64),
                ],
            )?;
            ensure(
                matches!(actual, NormalizedValue::Bool(value) if value == expected),
                &format!("checked visibility/name predicate: {label}"),
            )?;
        }
        // Generic binders and owned-parameter use bytes both survive the same
        // prefix resume path. Compare resumed bytes with the full helper output.
        for name in ["list-get", "push"] {
            let owner = row(&case.source, function_key(&case.source, name)?)?;
            let complete = engine.call(
                "source-unit-tables-signature",
                "prepare",
                vec![
                    NormalizedValue::bytes(case.carrier.as_slice()),
                    NormalizedValue::I64(owner as i64),
                ],
            )?;
            code(&complete, 0)?;
            equal(
                integer(&complete, "count")?,
                2,
                "signature fixture parameter count",
            )?;
            let rows = bytes(&complete, "parameters")?;
            let slots = bytes(&complete, "slots")?;
            let prefix_end = 40_usize
                .checked_add(usize::try_from(word(rows, 32)?).map_err(|_| "signature name extent")?)
                .ok_or("signature prefix extent overflow")?;
            let resumed = engine.call(
                "source-unit-tables-signature",
                "parameters",
                vec![
                    NormalizedValue::bytes(case.carrier.as_slice()),
                    NormalizedValue::I64(owner as i64),
                    NormalizedValue::bytes(bytes(&complete, "pool")?),
                    NormalizedValue::bytes(bytes(&complete, "binders")?),
                    NormalizedValue::I64(1),
                    NormalizedValue::I64(2),
                    NormalizedValue::bytes(rows.get(..prefix_end).ok_or("signature row prefix")?),
                    NormalizedValue::bytes(slots.get(..16).ok_or("signature slot prefix")?),
                ],
            )?;
            code(&resumed, 0)?;
            for field_name in ["pool", "binders", "parameters", "slots"] {
                equal_bytes(
                    bytes(&resumed, field_name)?,
                    bytes(&complete, field_name)?,
                    field_name,
                )?;
            }
            equal(
                integer(&resumed, "count")?,
                integer(&complete, "count")?,
                "resumed count",
            )?;
            equal(
                integer(&resumed, "result")?,
                integer(&complete, "result")?,
                "resumed result type",
            )?;
        }
    }
    Ok(())
}
