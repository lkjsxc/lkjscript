use super::{
    CanonicalReleaseTest, DecodedRelease, MAXIMUM_RELEASE_ARTIFACT_BYTES,
    MAXIMUM_RELEASE_COORDINATE_BYTES, MAXIMUM_RELEASE_DEPENDENCIES, MAXIMUM_RELEASE_EXPORTS,
    MAXIMUM_RELEASE_IMPORTS, MAXIMUM_RELEASE_ITEMS, MAXIMUM_RELEASE_NAME_BYTES,
    MAXIMUM_RELEASE_SLOT_BYTES, MAXIMUM_RELEASE_TESTS, MAXIMUM_RELEASE_VERSION_BYTES,
    ReleaseContentDigest, ReleaseDependency, ReleaseExport, ReleaseExportKind, ReleaseId,
    ReleaseImport, ReleaseItemId, ReleaseTestExpectation, ReleaseTrap, ReleaseTrapCode, canonical,
};
use crate::artifact;
use crate::codec::{CodecError, CodecErrorKind, Reader, Writer};
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::{NodeId, Revision};
use crate::interpret::{self, RunPolicy, RuntimeValue};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const RELEASE_MAGIC: [u8; 8] = *b"LKJREL\0\x02";
pub(super) const RELEASE_FORMAT_VERSION: u16 = 2;
const RELEASE_ID_DOMAIN: &str = "lkjscript.reusable-release.identity.v2";
const RELEASE_CONTENT_DOMAIN: &str = "lkjscript.reusable-release.content.v2";
const MAXIMUM_RUNTIME_VALUE_JSON_BYTES: usize = 64 * 1024 * 1024;

pub(super) fn encode(release: &DecodedRelease) -> Result<Vec<u8>> {
    let payload = encode_payload(release)?;
    if payload.len() > MAXIMUM_RELEASE_ARTIFACT_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "reusable release payload exceeds byte policy",
        ));
    }
    let id = release_id(&payload);
    let content = content_digest(&payload);
    let mut writer = Writer::with_capacity(
        RELEASE_MAGIC.len() + 2 + artifact::SCHEMA_ID.0.len() + 8 + payload.len() + 64,
    );
    writer.fixed(&RELEASE_MAGIC);
    writer.u16(RELEASE_FORMAT_VERSION);
    writer.fixed(&artifact::SCHEMA_ID.0);
    put_count(&mut writer, payload.len())?;
    writer.fixed(&payload);
    writer.fixed(&id.as_bytes());
    writer.fixed(&content.as_bytes());
    let bytes = writer.finish();
    if bytes.len() > MAXIMUM_RELEASE_ARTIFACT_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "reusable release artifact exceeds byte policy",
        ));
    }
    Ok(bytes)
}

pub(super) fn decode(bytes: &[u8]) -> Result<DecodedRelease> {
    if bytes.len() > MAXIMUM_RELEASE_ARTIFACT_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "reusable release artifact exceeds decoder byte policy",
        ));
    }
    let mut reader = Reader::new(bytes);
    if reader.fixed(RELEASE_MAGIC.len()).map_err(codec)? != RELEASE_MAGIC {
        return Err(corrupt("reusable release artifact magic is invalid"));
    }
    if reader.u16().map_err(codec)? != RELEASE_FORMAT_VERSION {
        return Err(corrupt(
            "reusable release artifact format version is unsupported",
        ));
    }
    if reader.fixed(artifact::SCHEMA_ID.0.len()).map_err(codec)? != artifact::SCHEMA_ID.0 {
        return Err(corrupt(
            "reusable release semantic schema identity is unsupported",
        ));
    }
    let payload_length = usize::try_from(reader.u64().map_err(codec)?)
        .map_err(|_| corrupt("reusable release payload length overflows host indexes"))?;
    if payload_length > MAXIMUM_RELEASE_ARTIFACT_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "reusable release payload exceeds decoder byte policy",
        ));
    }
    let payload = reader.fixed(payload_length).map_err(codec)?;
    let encoded_id = read_digest(&mut reader)?;
    let encoded_content = read_digest(&mut reader)?;
    reader.finish().map_err(codec)?;
    let id = release_id(payload);
    let content = content_digest(payload);
    if encoded_id != id.as_bytes() || encoded_content != content.as_bytes() {
        return Err(corrupt(
            "reusable release identity or content digest is invalid",
        ));
    }
    let mut release = decode_payload(payload)?;
    release.bytes = bytes.to_vec();
    release.id = id;
    release.content_digest = content;
    canonical::validate_release_model(&release, true).map_err(decoded_error)?;
    if encode(&release)? != bytes {
        return Err(corrupt(
            "reusable release artifact encoding is not canonical",
        ));
    }
    Ok(release)
}

fn encode_payload(release: &DecodedRelease) -> Result<Vec<u8>> {
    let mut writer = Writer::new();
    writer.string(&release.coordinate).map_err(codec)?;
    writer.string(&release.user_version).map_err(codec)?;
    put_item(&mut writer, release.unit_root);
    put_count(&mut writer, release.dependencies.len())?;
    for dependency in &release.dependencies {
        writer.string(&dependency.slot).map_err(codec)?;
        writer.fixed(&dependency.release.as_bytes());
    }
    put_count(&mut writer, release.imports.len())?;
    for import in &release.imports {
        put_item(&mut writer, import.local);
        writer.string(&import.dependency_slot).map_err(codec)?;
        put_item(&mut writer, import.target);
    }
    put_count(&mut writer, release.exports.len())?;
    for export in &release.exports {
        writer.string(&export.name).map_err(codec)?;
        writer.u8(export.kind.stable_tag());
        put_item(&mut writer, export.target);
    }
    put_count(&mut writer, release.tests.len())?;
    for test in &release.tests {
        put_test(&mut writer, test)?;
    }
    put_count(&mut writer, release.snapshot.node_count())?;
    for (id, node) in release.snapshot.nodes() {
        writer.u64(id.serial());
        artifact::put_node(&mut writer, node)?;
    }
    Ok(writer.finish())
}

fn decode_payload(payload: &[u8]) -> Result<DecodedRelease> {
    let mut reader = Reader::new(payload);
    let coordinate = reader
        .string(MAXIMUM_RELEASE_COORDINATE_BYTES)
        .map_err(codec)?;
    let user_version = reader
        .string(MAXIMUM_RELEASE_VERSION_BYTES)
        .map_err(codec)?;
    let unit_root = read_item(&mut reader)?;

    let dependency_count = reader.count(MAXIMUM_RELEASE_DEPENDENCIES).map_err(codec)?;
    let mut dependencies = Vec::with_capacity(dependency_count);
    let mut prior_slot: Option<String> = None;
    for _ in 0..dependency_count {
        let slot = reader.string(MAXIMUM_RELEASE_SLOT_BYTES).map_err(codec)?;
        require_strict(prior_slot.as_deref(), &slot, "dependency slots")?;
        prior_slot = Some(slot.clone());
        dependencies.push(ReleaseDependency {
            slot,
            release: ReleaseId::from_bytes(read_digest(&mut reader)?),
        });
    }

    let import_count = reader.count(MAXIMUM_RELEASE_IMPORTS).map_err(codec)?;
    let mut imports = Vec::with_capacity(import_count);
    let mut prior_import = None;
    for _ in 0..import_count {
        let local = read_item(&mut reader)?;
        if prior_import.is_some_and(|prior| prior >= local) {
            return Err(corrupt(
                "reusable release imports are not in strict local-item order",
            ));
        }
        prior_import = Some(local);
        imports.push(ReleaseImport {
            local,
            dependency_slot: reader.string(MAXIMUM_RELEASE_SLOT_BYTES).map_err(codec)?,
            target: read_item(&mut reader)?,
        });
    }

    let export_count = reader.count(MAXIMUM_RELEASE_EXPORTS).map_err(codec)?;
    let mut exports = Vec::with_capacity(export_count);
    let mut prior_export: Option<String> = None;
    for _ in 0..export_count {
        let name = reader.string(MAXIMUM_RELEASE_NAME_BYTES).map_err(codec)?;
        require_strict(prior_export.as_deref(), &name, "export names")?;
        prior_export = Some(name.clone());
        let tag = reader.u8().map_err(codec)?;
        let kind = ReleaseExportKind::from_stable_tag(tag)
            .ok_or_else(|| corrupt("reusable release export kind tag is unknown"))?;
        exports.push(ReleaseExport {
            name,
            kind,
            target: read_item(&mut reader)?,
        });
    }

    let test_count = reader.count(MAXIMUM_RELEASE_TESTS).map_err(codec)?;
    let mut tests = Vec::with_capacity(test_count);
    let mut prior_test: Option<String> = None;
    for _ in 0..test_count {
        let test = read_test(&mut reader)?;
        require_strict(prior_test.as_deref(), &test.name, "release test names")?;
        prior_test = Some(test.name.clone());
        tests.push(test);
    }

    let node_count = reader.count(MAXIMUM_RELEASE_ITEMS).map_err(codec)?;
    if node_count == 0 {
        return Err(corrupt("reusable release semantic closure is empty"));
    }
    let mut nodes = BTreeMap::new();
    let mut prior = None;
    for _ in 0..node_count {
        let id = NodeId::from_encoded(
            canonical::RELEASE_LOCAL_WORKSPACE,
            reader.u64().map_err(codec)?,
        )
        .map_err(|error| {
            corrupt(&format!(
                "reusable release contains an invalid local item identity: {error}"
            ))
        })?;
        if prior.is_some_and(|prior| prior >= id) {
            return Err(corrupt(
                "reusable release nodes are not in strict identity order",
            ));
        }
        prior = Some(id);
        let node = artifact::read_node(
            &mut reader,
            canonical::RELEASE_LOCAL_WORKSPACE,
            artifact::DecodePolicy {
                maximum_artifact_bytes: MAXIMUM_RELEASE_ARTIFACT_BYTES,
                maximum_name_bytes: MAXIMUM_RELEASE_NAME_BYTES,
            },
        )?;
        nodes.insert(id, node);
    }
    reader.finish().map_err(codec)?;
    let maximum = nodes
        .keys()
        .filter(|id| id.is_durable())
        .map(|id| id.serial())
        .max()
        .ok_or_else(|| corrupt("reusable release contains no durable semantic items"))?;
    let durable = nodes
        .keys()
        .filter(|id| id.is_durable())
        .map(|id| id.serial())
        .collect::<BTreeSet<_>>();
    if durable != (1..=maximum).collect::<BTreeSet<_>>() {
        return Err(corrupt(
            "reusable release durable item identities are not contiguous",
        ));
    }
    let root = NodeId::new(canonical::RELEASE_LOCAL_WORKSPACE, 1).map_err(|error| {
        corrupt(&format!(
            "reusable release root identity is invalid: {error}"
        ))
    })?;
    let snapshot = Snapshot::from_parts(
        canonical::RELEASE_LOCAL_WORKSPACE,
        Revision::INITIAL,
        root,
        maximum
            .checked_add(1)
            .ok_or_else(|| corrupt("reusable release identity frontier overflows"))?,
        BTreeSet::new(),
        nodes,
    )
    .map_err(decoded_error)?;
    Ok(DecodedRelease {
        bytes: Vec::new(),
        id: ReleaseId::from_bytes([0; 32]),
        content_digest: ReleaseContentDigest::from_bytes([0; 32]),
        coordinate,
        user_version,
        unit_root,
        dependencies,
        imports,
        exports,
        tests,
        snapshot,
    })
}

fn put_test(writer: &mut Writer, test: &CanonicalReleaseTest) -> Result<()> {
    writer.string(&test.name).map_err(codec)?;
    put_item(writer, test.target);
    put_policy(writer, test.policy);
    put_count(writer, test.arguments.len())?;
    for argument in &test.arguments {
        put_runtime_value(writer, argument)?;
    }
    match &test.expected {
        ReleaseTestExpectation::Value(value) => {
            writer.u8(1);
            put_runtime_value(writer, value)?;
        }
        ReleaseTestExpectation::Trap(trap) => {
            writer.u8(2);
            writer.u8(trap.code.stable_tag());
            put_optional_node(writer, trap.target);
        }
    }
    Ok(())
}

fn read_test(reader: &mut Reader<'_>) -> Result<CanonicalReleaseTest> {
    let name = reader.string(MAXIMUM_RELEASE_NAME_BYTES).map_err(codec)?;
    let target = read_item(reader)?;
    let policy = read_policy(reader)?;
    let argument_count = reader.count(interpret::MAX_RUN_ARGUMENTS).map_err(codec)?;
    let mut arguments = Vec::with_capacity(argument_count);
    for _ in 0..argument_count {
        arguments.push(read_runtime_value(reader)?);
    }
    let expected = match reader.u8().map_err(codec)? {
        1 => ReleaseTestExpectation::Value(read_runtime_value(reader)?),
        2 => {
            let tag = reader.u8().map_err(codec)?;
            let code = ReleaseTrapCode::from_stable_tag(tag)
                .ok_or_else(|| corrupt("reusable release trap tag is unknown"))?;
            ReleaseTestExpectation::Trap(ReleaseTrap {
                code,
                target: read_optional_node(reader)?,
            })
        }
        _ => return Err(corrupt("reusable release test expectation tag is unknown")),
    };
    Ok(CanonicalReleaseTest {
        name,
        target,
        arguments,
        expected,
        policy,
    })
}

fn put_runtime_value(writer: &mut Writer, value: &RuntimeValue) -> Result<()> {
    let encoded = serde_json::to_vec(value).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("cannot encode reusable release test value: {error}"),
        )
    })?;
    if encoded.len() > MAXIMUM_RUNTIME_VALUE_JSON_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "reusable release test value exceeds byte policy",
        ));
    }
    writer.bytes(&encoded).map_err(codec)
}

fn read_runtime_value(reader: &mut Reader<'_>) -> Result<RuntimeValue> {
    let encoded = reader
        .bytes(MAXIMUM_RUNTIME_VALUE_JSON_BYTES)
        .map_err(codec)?;
    let value = serde_json::from_slice::<RuntimeValue>(encoded).map_err(|error| {
        corrupt(&format!(
            "reusable release test value JSON is malformed: {error}"
        ))
    })?;
    if serde_json::to_vec(&value).map_err(|error| {
        corrupt(&format!(
            "reusable release test value cannot be canonicalized: {error}"
        ))
    })? != encoded
    {
        return Err(corrupt("reusable release test value JSON is not canonical"));
    }
    Ok(value)
}

fn put_policy(writer: &mut Writer, policy: RunPolicy) {
    writer.u64(policy.fuel);
    writer.u32(policy.maximum_frames);
}

fn read_policy(reader: &mut Reader<'_>) -> Result<RunPolicy> {
    let policy = RunPolicy {
        fuel: reader.u64().map_err(codec)?,
        maximum_frames: reader.u32().map_err(codec)?,
    };
    interpret::validate_policy(policy).map_err(decoded_error)?;
    Ok(policy)
}

fn put_item(writer: &mut Writer, item: ReleaseItemId) {
    writer.u64(item.get());
}

fn read_item(reader: &mut Reader<'_>) -> Result<ReleaseItemId> {
    ReleaseItemId::new(reader.u64().map_err(codec)?).map_err(decoded_error)
}

fn put_optional_node(writer: &mut Writer, item: Option<NodeId>) {
    writer.bool(item.is_some());
    if let Some(item) = item {
        writer.u64(item.serial());
    }
}

fn read_optional_node(reader: &mut Reader<'_>) -> Result<Option<NodeId>> {
    if reader.bool().map_err(codec)? {
        NodeId::from_encoded(
            canonical::RELEASE_LOCAL_WORKSPACE,
            reader.u64().map_err(codec)?,
        )
        .map(Some)
        .map_err(|error| corrupt(&format!("release trap target is invalid: {error}")))
    } else {
        Ok(None)
    }
}

fn put_count(writer: &mut Writer, count: usize) -> Result<()> {
    writer.u64(u64::try_from(count).map_err(|_| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "reusable release collection length does not fit canonical u64",
        )
    })?);
    Ok(())
}

fn read_digest(reader: &mut Reader<'_>) -> Result<[u8; 32]> {
    let mut value = [0_u8; 32];
    value.copy_from_slice(reader.fixed(32).map_err(codec)?);
    Ok(value)
}

fn release_id(payload: &[u8]) -> ReleaseId {
    let mut hasher = blake3::Hasher::new_derive_key(RELEASE_ID_DOMAIN);
    hasher.update(payload);
    ReleaseId::from_bytes(*hasher.finalize().as_bytes())
}

fn content_digest(payload: &[u8]) -> ReleaseContentDigest {
    let mut hasher = blake3::Hasher::new_derive_key(RELEASE_CONTENT_DOMAIN);
    hasher.update(payload);
    ReleaseContentDigest::from_bytes(*hasher.finalize().as_bytes())
}

fn require_strict(prior: Option<&str>, current: &str, label: &str) -> Result<()> {
    if prior.is_some_and(|prior| prior >= current) {
        Err(corrupt(&format!(
            "reusable release {label} are not in strict canonical order"
        )))
    } else {
        Ok(())
    }
}

fn codec(error: CodecError) -> LkError {
    LkError::new(
        if error.kind == CodecErrorKind::PolicyExceeded {
            ErrorCode::PolicyExceeded
        } else {
            ErrorCode::ArtifactCorrupt
        },
        format!("canonical reusable release decoding failed: {error}"),
    )
}

fn decoded_error(mut error: LkError) -> LkError {
    if error.code != ErrorCode::PolicyExceeded {
        error.code = ErrorCode::ArtifactCorrupt;
    }
    error
}

fn corrupt(message: &str) -> LkError {
    LkError::new(ErrorCode::ArtifactCorrupt, message)
}
