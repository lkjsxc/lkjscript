//! Contributor-only PostgreSQL differential, migration, and resource oracle.

use crate::error::DevError;
use crate::evidence::{self, FileProof, VerificationDigest};
use crate::postgres::PostgresInstance;
use crate::process::{self, ProcessObservation, ProcessSpec, ProcessStatus};
use lkjscript::platform::data::{
    DataCommitOutcome, DataExpectation, DataKey, DataKeyPart, DataLimits, DataScanDirection,
    DataSchema, DataSchemaExpectation, DataStore,
};
use postgres::{Client, NoTls};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const ORACLE_CONTRACT_VERSION: u32 = 1;
const FIXTURE_IDENTITY: &str = "lkjscript-neutral-data-migration-fixture";
const FIXTURE_VERSION: u32 = 1;
const DATA_CONTRACT: &str = "lkjscript-data-store-1";
const SAMPLES: usize = 3;
const MAXIMUM_LOG_BYTES: u64 = 16 * 1024 * 1024;
const SAMPLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, Debug)]
struct Options {
    binary: PathBuf,
    bbs_receipt: PathBuf,
    service_receipt: PathBuf,
    machine: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Workload {
    Bbs,
    Lkjournal,
}

impl Workload {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Bbs => "bbs",
            Self::Lkjournal => "lkjournal",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct NeutralFixture {
    identity: String,
    version: u32,
    bbs: Vec<BbsPost>,
    lkjournal: Vec<JournalFact>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct BbsPost {
    id: String,
    created_at: i64,
    title: String,
    body: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct JournalFact {
    kind: String,
    id: String,
    owner: String,
    sequence: i64,
    payload: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum NeutralPart {
    Bool(bool),
    I64(i64),
    Text(String),
    Bytes(Vec<u8>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CanonicalFact {
    space: String,
    key: Vec<NeutralPart>,
    value: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ChildMetrics {
    workload: Workload,
    backend: String,
    facts: u64,
    operations: u64,
    durable_bytes: u64,
    fsync_publications: u64,
    fact_digest: String,
    cleanup_complete: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ResourceSample {
    wall_nanoseconds: u64,
    cpu_nanoseconds: u64,
    peak_rss_kib: u64,
    durable_bytes: u64,
    fsync_publications: u64,
    operations: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ResourceMedian {
    wall_nanoseconds: u64,
    cpu_nanoseconds: u64,
    peak_rss_kib: u64,
    durable_bytes: u64,
    fsync_publications: u64,
    operations: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WorkloadComparison {
    workload: Workload,
    facts: u64,
    warmup_complete: bool,
    first_party_samples: Vec<ResourceSample>,
    postgres_samples: Vec<ResourceSample>,
    first_party_median: ResourceMedian,
    postgres_median: ResourceMedian,
    wall_ratio_millionths: u64,
    rss_ratio_millionths: u64,
    durable_ratio_millionths: u64,
    thresholds_passed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    contract_version: u32,
    status: String,
    postgres_version: String,
    postgres_image: String,
    postgres_image_config: String,
    data_contract: String,
    fixture: FileProof,
    fixture_digest: VerificationDigest,
    fixture_facts: u64,
    postgres_export_equal: bool,
    first_party_import_equal: bool,
    backup_restore_equal: bool,
    bbs_receipt: FileProof,
    service_receipt: FileProof,
    public_outcomes_equal: bool,
    workloads: Vec<WorkloadComparison>,
    commands: Vec<ProcessObservation>,
    cleanup_complete: bool,
}

pub(crate) fn command(arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let values = arguments.collect::<Vec<_>>();
    if values.first().and_then(|value| value.to_str()) == Some("__sample") {
        return sample_command(&values[1..]);
    }
    let options = parse_options(values.into_iter())?;
    let repository = repository_root()?;
    let binary = resolve_regular_file(&repository, &options.binary, "candidate binary")?;
    let bbs_receipt = resolve_regular_file(&repository, &options.bbs_receipt, "BBS receipt")?;
    let service_receipt =
        resolve_regular_file(&repository, &options.service_receipt, "service receipt")?;
    validate_public_receipts(&binary, &bbs_receipt, &service_receipt)?;

    let run = new_run_directory(&repository)?;
    let mut commands = Vec::new();
    let fixture = deterministic_fixture();
    let fixture_path = run.join("neutral-fixture.json");
    let fixture_published = evidence::publish_json(&fixture_path, &fixture)?;
    let fixture_proof = evidence::proof(
        &fixture_path,
        evidence::relative(&repository, &fixture_path),
    )?;
    let fixture_digest = VerificationDigest::of(&canonical_fixture_bytes(&fixture)?);
    let all_facts = fixture_facts(&fixture, None);

    let mut postgres = PostgresInstance::start(&repository, &run, &mut commands)?;
    let exported = postgres_fixture_round_trip(&mut postgres, &fixture)?;
    if exported != fixture {
        return Err(DevError::corrupt(
            "PostgreSQL neutral migration export differs from its deterministic source",
        ));
    }

    let migration_root = run.join("migration-data");
    let restored_root = run.join("restored-data");
    let backup = run.join("migration-backup.lkjd");
    import_facts(&migration_root, &all_facts)?;
    let imported = export_facts(&migration_root, &all_facts)?;
    if imported != all_facts {
        return Err(DevError::corrupt(
            "first-party imported facts differ from the neutral fixture",
        ));
    }
    let imported_store =
        DataStore::open(&migration_root, "oracle", DataLimits::default()).map_err(data_error)?;
    imported_store.backup(&backup).map_err(data_error)?;
    DataStore::restore(&backup, &restored_root).map_err(data_error)?;
    let restored = export_facts(&restored_root, &all_facts)?;
    if restored != all_facts {
        return Err(DevError::corrupt(
            "restored first-party facts differ from the neutral fixture",
        ));
    }

    let executable = std::env::current_exe()
        .map_err(|error| DevError::infrastructure(format!("resolve oracle executable: {error}")))?;
    let mut workloads = Vec::new();
    for workload in [Workload::Bbs, Workload::Lkjournal] {
        workloads.push(compare_workload(
            &repository,
            &run,
            &executable,
            &mut postgres,
            workload,
            &mut commands,
        )?);
    }
    if workloads.iter().any(|workload| !workload.thresholds_passed) {
        return Err(DevError::corrupt(
            "first-party data resource admission exceeded a PostgreSQL reversal threshold",
        ));
    }
    postgres.stop(&mut commands)?;

    remove_exact_directory(&migration_root)?;
    remove_exact_directory(&restored_root)?;
    let receipt = Receipt {
        contract_version: ORACLE_CONTRACT_VERSION,
        status: "passed".to_owned(),
        postgres_version: crate::postgres::POSTGRES_VERSION.to_owned(),
        postgres_image: PostgresInstance::image().to_owned(),
        postgres_image_config: PostgresInstance::image_config().to_owned(),
        data_contract: DATA_CONTRACT.to_owned(),
        fixture: fixture_proof,
        fixture_digest,
        fixture_facts: u64::try_from(all_facts.len())
            .map_err(|_| DevError::infrastructure("fixture fact count overflowed"))?,
        postgres_export_equal: true,
        first_party_import_equal: true,
        backup_restore_equal: true,
        bbs_receipt: evidence::proof(&bbs_receipt, evidence::relative(&repository, &bbs_receipt))?,
        service_receipt: evidence::proof(
            &service_receipt,
            evidence::relative(&repository, &service_receipt),
        )?,
        public_outcomes_equal: true,
        workloads,
        commands,
        cleanup_complete: !migration_root.exists() && !restored_root.exists(),
    };
    let receipt_path = run.join("receipt.json");
    let published = evidence::publish_json(&receipt_path, &receipt)?;
    if options.machine {
        println!(
            "{}",
            serde_json::json!({
                "status": "passed",
                "receipt": evidence::relative(&repository, &published.path),
                "receipt_bytes": published.bytes,
                "receipt_digest": published.digest,
                "fixture_bytes": fixture_published.bytes,
                "fixture_digest": receipt.fixture_digest,
            })
        );
    } else {
        println!(
            "data oracle passed: receipt={} digest={}",
            evidence::relative(&repository, &published.path),
            published.digest
        );
    }
    Ok(0)
}

fn parse_options(arguments: impl Iterator<Item = OsString>) -> Result<Options, DevError> {
    let mut binary = None;
    let mut bbs_receipt = None;
    let mut service_receipt = None;
    let mut machine = false;
    let mut arguments = arguments;
    while let Some(argument) = arguments.next() {
        let argument = argument
            .into_string()
            .map_err(|_| DevError::usage("data-oracle options must be UTF-8"))?;
        match argument.as_str() {
            "--binary" if binary.is_none() => {
                binary = Some(next_path(&mut arguments, "--binary")?);
            }
            "--bbs-receipt" if bbs_receipt.is_none() => {
                bbs_receipt = Some(next_path(&mut arguments, "--bbs-receipt")?);
            }
            "--service-receipt" if service_receipt.is_none() => {
                service_receipt = Some(next_path(&mut arguments, "--service-receipt")?);
            }
            "--machine" if !machine => machine = true,
            _ => {
                return Err(DevError::usage(format!(
                    "unknown or duplicate data-oracle option '{argument}'"
                )));
            }
        }
    }
    Ok(Options {
        binary: binary.ok_or_else(|| DevError::usage("data-oracle requires --binary PATH"))?,
        bbs_receipt: bbs_receipt
            .ok_or_else(|| DevError::usage("data-oracle requires --bbs-receipt PATH"))?,
        service_receipt: service_receipt
            .ok_or_else(|| DevError::usage("data-oracle requires --service-receipt PATH"))?,
        machine,
    })
}

fn next_path(
    arguments: &mut impl Iterator<Item = OsString>,
    option: &str,
) -> Result<PathBuf, DevError> {
    arguments
        .next()
        .ok_or_else(|| DevError::usage(format!("{option} requires a path")))
        .map(PathBuf::from)
}

fn deterministic_fixture() -> NeutralFixture {
    let bbs = (0..96_i64)
        .map(|index| BbsPost {
            id: format!("post-{index:04}"),
            created_at: 1_800_000_000_000_i64.saturating_add(index),
            title: format!("Title {index:04}"),
            body: format!("Deterministic ordered post body {index:04}"),
        })
        .collect();
    let kinds = [
        "actor",
        "session",
        "resource",
        "immutable-snapshot",
        "object-metadata",
        "lookup",
        "durable-job",
    ];
    let mut lkjournal = Vec::new();
    for sequence in 0..32_i64 {
        for kind in kinds {
            lkjournal.push(JournalFact {
                kind: kind.to_owned(),
                id: format!("{kind}-{sequence:04}"),
                owner: format!("actor-{:04}", sequence % 8),
                sequence,
                payload: format!("{kind} deterministic payload {sequence:04}"),
            });
        }
    }
    lkjournal.sort_by(|left, right| left.kind.cmp(&right.kind).then(left.id.cmp(&right.id)));
    NeutralFixture {
        identity: FIXTURE_IDENTITY.to_owned(),
        version: FIXTURE_VERSION,
        bbs,
        lkjournal,
    }
}

fn canonical_fixture_bytes(fixture: &NeutralFixture) -> Result<Vec<u8>, DevError> {
    serde_json::to_vec(fixture)
        .map_err(|error| DevError::infrastructure(format!("encode neutral fixture: {error}")))
}

fn fixture_facts(fixture: &NeutralFixture, selected: Option<Workload>) -> Vec<CanonicalFact> {
    let mut facts = Vec::new();
    if selected.is_none() || selected == Some(Workload::Bbs) {
        for post in &fixture.bbs {
            facts.push(CanonicalFact {
                space: "bbs.post".to_owned(),
                key: vec![NeutralPart::Text(post.id.clone())],
                value: neutral_value(&[
                    post.id.as_bytes(),
                    &post.created_at.to_be_bytes(),
                    post.title.as_bytes(),
                    post.body.as_bytes(),
                ]),
            });
            facts.push(CanonicalFact {
                space: "bbs.created".to_owned(),
                key: vec![
                    NeutralPart::I64(post.created_at),
                    NeutralPart::Text(post.id.clone()),
                ],
                value: neutral_value(&[post.id.as_bytes()]),
            });
        }
    }
    if selected.is_none() || selected == Some(Workload::Lkjournal) {
        for fact in &fixture.lkjournal {
            facts.push(CanonicalFact {
                space: format!("journal.{}", fact.kind),
                key: vec![
                    NeutralPart::Text(fact.owner.clone()),
                    NeutralPart::I64(fact.sequence),
                    NeutralPart::Text(fact.id.clone()),
                ],
                value: neutral_value(&[
                    fact.kind.as_bytes(),
                    fact.id.as_bytes(),
                    fact.owner.as_bytes(),
                    &fact.sequence.to_be_bytes(),
                    fact.payload.as_bytes(),
                ]),
            });
        }
    }
    facts.sort_by(compare_fact);
    facts
}

fn neutral_value(fields: &[&[u8]]) -> Vec<u8> {
    let mut output = b"LKJNEUT1".to_vec();
    output.extend_from_slice(
        &u16::try_from(fields.len())
            .unwrap_or(u16::MAX)
            .to_be_bytes(),
    );
    for field in fields {
        output.extend_from_slice(&u32::try_from(field.len()).unwrap_or(u32::MAX).to_be_bytes());
        output.extend_from_slice(field);
    }
    output
}

fn compare_fact(left: &CanonicalFact, right: &CanonicalFact) -> Ordering {
    left.space
        .cmp(&right.space)
        .then_with(|| compare_parts(&left.key, &right.key))
        .then_with(|| left.value.cmp(&right.value))
}

fn compare_parts(left: &[NeutralPart], right: &[NeutralPart]) -> Ordering {
    for (left, right) in left.iter().zip(right) {
        let order = compare_part(left, right);
        if order != Ordering::Equal {
            return order;
        }
    }
    left.len().cmp(&right.len())
}

fn compare_part(left: &NeutralPart, right: &NeutralPart) -> Ordering {
    let tag = |part: &NeutralPart| match part {
        NeutralPart::Bool(_) => 0_u8,
        NeutralPart::I64(_) => 1,
        NeutralPart::Text(_) => 2,
        NeutralPart::Bytes(_) => 3,
    };
    tag(left)
        .cmp(&tag(right))
        .then_with(|| match (left, right) {
            (NeutralPart::Bool(left), NeutralPart::Bool(right)) => left.cmp(right),
            (NeutralPart::I64(left), NeutralPart::I64(right)) => left.cmp(right),
            (NeutralPart::Text(left), NeutralPart::Text(right)) => {
                left.as_bytes().cmp(right.as_bytes())
            }
            (NeutralPart::Bytes(left), NeutralPart::Bytes(right)) => left.cmp(right),
            _ => Ordering::Equal,
        })
}

fn postgres_fixture_round_trip(
    postgres: &mut PostgresInstance,
    fixture: &NeutralFixture,
) -> Result<NeutralFixture, DevError> {
    let mut client = postgres.connect()?;
    client
        .batch_execute(
            "DROP SCHEMA IF EXISTS migration_fixture CASCADE;
             CREATE SCHEMA migration_fixture;
             CREATE TABLE migration_fixture.bbs_posts (
               id text PRIMARY KEY, created_at bigint NOT NULL, title text NOT NULL, body text NOT NULL
             );
             CREATE INDEX bbs_posts_created ON migration_fixture.bbs_posts(created_at, id);
             CREATE TABLE migration_fixture.journal_facts (
               kind text NOT NULL, id text NOT NULL, owner_id text NOT NULL,
               sequence bigint NOT NULL, payload text NOT NULL, PRIMARY KEY(kind, id)
             );
             CREATE INDEX journal_owner_sequence ON migration_fixture.journal_facts(owner_id, sequence, id);",
        )
        .map_err(pg_error)?;
    let mut transaction = client.transaction().map_err(pg_error)?;
    for post in &fixture.bbs {
        transaction
            .execute(
                "INSERT INTO migration_fixture.bbs_posts(id,created_at,title,body) VALUES($1,$2,$3,$4)",
                &[&post.id, &post.created_at, &post.title, &post.body],
            )
            .map_err(pg_error)?;
    }
    for fact in &fixture.lkjournal {
        transaction
            .execute(
                "INSERT INTO migration_fixture.journal_facts(kind,id,owner_id,sequence,payload) VALUES($1,$2,$3,$4,$5)",
                &[&fact.kind, &fact.id, &fact.owner, &fact.sequence, &fact.payload],
            )
            .map_err(pg_error)?;
    }
    transaction.commit().map_err(pg_error)?;
    let bbs = client
        .query(
            "SELECT id,created_at,title,body FROM migration_fixture.bbs_posts ORDER BY created_at,id",
            &[],
        )
        .map_err(pg_error)?
        .into_iter()
        .map(|row| BbsPost {
            id: row.get(0),
            created_at: row.get(1),
            title: row.get(2),
            body: row.get(3),
        })
        .collect();
    let lkjournal = client
        .query(
            "SELECT kind,id,owner_id,sequence,payload FROM migration_fixture.journal_facts ORDER BY kind,id",
            &[],
        )
        .map_err(pg_error)?
        .into_iter()
        .map(|row| JournalFact {
            kind: row.get(0),
            id: row.get(1),
            owner: row.get(2),
            sequence: row.get(3),
            payload: row.get(4),
        })
        .collect();
    Ok(NeutralFixture {
        identity: FIXTURE_IDENTITY.to_owned(),
        version: FIXTURE_VERSION,
        bbs,
        lkjournal,
    })
}

fn import_facts(root: &Path, facts: &[CanonicalFact]) -> Result<ChildMetrics, DevError> {
    DataStore::initialize(root).map_err(data_error)?;
    let store = DataStore::open(root, "oracle", DataLimits::default()).map_err(data_error)?;
    let mut transaction = store.begin().map_err(data_error)?;
    let spaces = facts
        .iter()
        .map(|fact| fact.space.as_str())
        .collect::<BTreeSet<_>>();
    for space in spaces {
        let schema = schema_for(space);
        if !transaction
            .schema_set(space, &DataSchemaExpectation::Missing, schema)
            .map_err(data_error)?
        {
            return Err(DevError::corrupt("new data schema expectation failed"));
        }
    }
    for fact in facts {
        let key = data_key(&fact.key)?;
        if !transaction
            .put(
                &fact.space,
                &key,
                fact.value.clone(),
                DataExpectation::Missing,
            )
            .map_err(data_error)?
        {
            return Err(DevError::corrupt("new data fact expectation failed"));
        }
    }
    let (durable, fsyncs) = match transaction.commit().map_err(data_error)? {
        DataCommitOutcome::Committed {
            durable_bytes,
            fsync_publications,
            ..
        } => (durable_bytes as u64, fsync_publications as u64),
        _ => {
            return Err(DevError::corrupt(
                "new data import did not commit one revision",
            ));
        }
    };
    Ok(ChildMetrics {
        workload: Workload::Bbs,
        backend: "first_party".to_owned(),
        facts: facts.len() as u64,
        operations: facts.len() as u64,
        durable_bytes: durable,
        fsync_publications: fsyncs,
        fact_digest: facts_digest(facts),
        cleanup_complete: false,
    })
}

fn export_facts(root: &Path, expected: &[CanonicalFact]) -> Result<Vec<CanonicalFact>, DevError> {
    let store = DataStore::open(root, "oracle", DataLimits::default()).map_err(data_error)?;
    let transaction = store.begin().map_err(data_error)?;
    let spaces = expected
        .iter()
        .map(|fact| fact.space.clone())
        .collect::<BTreeSet<_>>();
    let mut facts = Vec::new();
    for space in spaces {
        if transaction.schema_read(&space).map_err(data_error)? != Some(schema_for(&space)) {
            return Err(DevError::corrupt("first-party schema marker differs"));
        }
        let page = transaction
            .scan(
                &space,
                &[],
                DataScanDirection::Forward,
                DataLimits::default().maximum_scan_items,
                DataLimits::default().maximum_scan_bytes,
                DataLimits::default().maximum_scan_work,
                None,
            )
            .map_err(data_error)?;
        if page.continuation.is_some() {
            return Err(DevError::corrupt(
                "oracle scan unexpectedly required continuation",
            ));
        }
        for item in page.items {
            facts.push(CanonicalFact {
                space: space.clone(),
                key: item.key.parts().iter().map(neutral_part).collect(),
                value: item.value,
            });
        }
    }
    facts.sort_by(compare_fact);
    Ok(facts)
}

fn schema_for(space: &str) -> DataSchema {
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.data-oracle.schema.v1");
    hasher.update(space.as_bytes());
    DataSchema {
        identity: format!("neutral-{space}-1"),
        digest: hasher.finalize().as_bytes().to_vec(),
    }
}

fn data_key(parts: &[NeutralPart]) -> Result<DataKey, DevError> {
    DataKey::new(
        parts
            .iter()
            .map(|part| match part {
                NeutralPart::Bool(value) => DataKeyPart::Bool(*value),
                NeutralPart::I64(value) => DataKeyPart::I64(*value),
                NeutralPart::Text(value) => DataKeyPart::Text(value.clone()),
                NeutralPart::Bytes(value) => DataKeyPart::Bytes(value.clone()),
            })
            .collect(),
        &DataLimits::default(),
    )
    .map_err(data_error)
}

fn neutral_part(part: &DataKeyPart) -> NeutralPart {
    match part {
        DataKeyPart::Bool(value) => NeutralPart::Bool(*value),
        DataKeyPart::I64(value) => NeutralPart::I64(*value),
        DataKeyPart::Text(value) => NeutralPart::Text(value.clone()),
        DataKeyPart::Bytes(value) => NeutralPart::Bytes(value.clone()),
    }
}

fn compare_workload(
    repository: &Path,
    run: &Path,
    executable: &Path,
    postgres: &mut PostgresInstance,
    workload: Workload,
    commands: &mut Vec<ProcessObservation>,
) -> Result<WorkloadComparison, DevError> {
    let fixture = deterministic_fixture();
    let facts = fixture_facts(&fixture, Some(workload));
    let expected_digest = facts_digest(&facts);
    let _ = run_sample(
        repository,
        run,
        executable,
        "data",
        workload,
        "warmup",
        &run.join(format!("warmup-{}-data", workload.as_str())),
        postgres.port(),
        commands,
    )?;
    let _ = run_sample(
        repository,
        run,
        executable,
        "postgres",
        workload,
        "warmup",
        Path::new("unused"),
        postgres.port(),
        commands,
    )?;
    let mut first_party_samples = Vec::new();
    let mut postgres_samples = Vec::new();
    for index in 0..SAMPLES {
        let ordinal = index.to_string();
        let (metrics, observation) = run_sample(
            repository,
            run,
            executable,
            "data",
            workload,
            &ordinal,
            &run.join(format!("sample-{}-{index}-data", workload.as_str())),
            postgres.port(),
            commands,
        )?;
        if metrics.fact_digest != expected_digest || !metrics.cleanup_complete {
            return Err(DevError::corrupt(
                "first-party resource sample fact mismatch",
            ));
        }
        first_party_samples.push(resource_sample(&metrics, &observation, 0)?);

        let (metrics, observation) = run_sample(
            repository,
            run,
            executable,
            "postgres",
            workload,
            &ordinal,
            Path::new("unused"),
            postgres.port(),
            commands,
        )?;
        if metrics.fact_digest != expected_digest || !metrics.cleanup_complete {
            return Err(DevError::corrupt(
                "PostgreSQL resource sample fact mismatch",
            ));
        }
        let server_rss = postgres.sampled_resident_kib(commands)?;
        postgres_samples.push(resource_sample(&metrics, &observation, server_rss)?);
    }
    let first_party_median = median(&first_party_samples)?;
    let postgres_median = median(&postgres_samples)?;
    let wall_ratio = ratio_millionths(
        first_party_median.wall_nanoseconds,
        postgres_median.wall_nanoseconds,
    )?;
    let rss_ratio = ratio_millionths(
        first_party_median.peak_rss_kib,
        postgres_median.peak_rss_kib,
    )?;
    let durable_ratio = ratio_millionths(
        first_party_median.durable_bytes,
        postgres_median.durable_bytes,
    )?;
    Ok(WorkloadComparison {
        workload,
        facts: facts.len() as u64,
        warmup_complete: true,
        first_party_samples,
        postgres_samples,
        first_party_median,
        postgres_median,
        wall_ratio_millionths: wall_ratio,
        rss_ratio_millionths: rss_ratio,
        durable_ratio_millionths: durable_ratio,
        thresholds_passed: wall_ratio <= 5_000_000
            && rss_ratio <= 2_000_000
            && durable_ratio <= 4_000_000,
    })
}

#[allow(clippy::too_many_arguments)]
fn run_sample(
    repository: &Path,
    run: &Path,
    executable: &Path,
    backend: &str,
    workload: Workload,
    ordinal: &str,
    root: &Path,
    port: u16,
    commands: &mut Vec<ProcessObservation>,
) -> Result<(ChildMetrics, ProcessObservation), DevError> {
    let label = format!("sample-{}-{backend}-{ordinal}", workload.as_str());
    let stdout = run.join(format!("{label}.stdout.log"));
    let stderr = run.join(format!("{label}.stderr.log"));
    let command = vec![
        executable.display().to_string(),
        "data-oracle".to_owned(),
        "__sample".to_owned(),
        backend.to_owned(),
        workload.as_str().to_owned(),
        root.display().to_string(),
        port.to_string(),
        ordinal.to_owned(),
    ];
    let observation = process::run(
        &ProcessSpec {
            command,
            cwd: repository.to_path_buf(),
            environment: process::environment(),
            timeout: SAMPLE_TIMEOUT,
            maximum_stdout_bytes: MAXIMUM_LOG_BYTES,
            maximum_stderr_bytes: MAXIMUM_LOG_BYTES,
            stdout_path: stdout.clone(),
            stderr_path: stderr,
            unavailable_exit_code: None,
        },
        repository,
    );
    if observation.status != ProcessStatus::Passed {
        return Err(DevError::infrastructure(format!(
            "data oracle child '{label}' failed ({})",
            observation.reason.as_deref().unwrap_or("child_failed")
        )));
    }
    let bytes = process::read_bounded(&stdout, MAXIMUM_LOG_BYTES)?;
    let metrics: ChildMetrics = serde_json::from_slice(&bytes)
        .map_err(|error| DevError::corrupt(format!("decode data oracle sample: {error}")))?;
    commands.push(observation.clone());
    Ok((metrics, observation))
}

fn sample_command(arguments: &[OsString]) -> Result<u8, DevError> {
    if arguments.len() != 5 {
        return Err(DevError::usage(
            "invalid internal data-oracle sample request",
        ));
    }
    let text = arguments
        .iter()
        .map(|value| {
            value
                .to_str()
                .map(str::to_owned)
                .ok_or_else(|| DevError::usage("data-oracle sample fields must be UTF-8"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let backend = text[0].as_str();
    let workload = match text[1].as_str() {
        "bbs" => Workload::Bbs,
        "lkjournal" => Workload::Lkjournal,
        _ => return Err(DevError::usage("unknown data-oracle workload")),
    };
    let fixture = deterministic_fixture();
    let facts = fixture_facts(&fixture, Some(workload));
    let mut metrics = match backend {
        "data" => benchmark_data(Path::new(&text[2]), workload, &facts)?,
        "postgres" => benchmark_postgres(
            text[3]
                .parse::<u16>()
                .map_err(|_| DevError::usage("invalid PostgreSQL oracle port"))?,
            workload,
            &text[4],
            &facts,
        )?,
        _ => return Err(DevError::usage("unknown data-oracle backend")),
    };
    metrics.cleanup_complete = true;
    println!(
        "{}",
        serde_json::to_string(&metrics)
            .map_err(|error| DevError::infrastructure(format!("encode sample: {error}")))?
    );
    Ok(0)
}

fn benchmark_data(
    root: &Path,
    workload: Workload,
    facts: &[CanonicalFact],
) -> Result<ChildMetrics, DevError> {
    let mut metrics = import_facts(root, facts)?;
    metrics.workload = workload;
    let store = DataStore::open(root, "oracle", DataLimits::default()).map_err(data_error)?;
    let read = store.begin().map_err(data_error)?;
    let spaces = facts
        .iter()
        .map(|fact| fact.space.clone())
        .collect::<BTreeSet<_>>();
    for space in &spaces {
        let page = read
            .scan(
                space,
                &[],
                DataScanDirection::Forward,
                DataLimits::default().maximum_scan_items,
                DataLimits::default().maximum_scan_bytes,
                DataLimits::default().maximum_scan_work,
                None,
            )
            .map_err(data_error)?;
        metrics.operations = metrics.operations.saturating_add(page.items.len() as u64);
    }
    drop(read);
    let first = facts
        .first()
        .ok_or_else(|| DevError::corrupt("empty benchmark fixture"))?;
    let last = facts
        .last()
        .ok_or_else(|| DevError::corrupt("empty benchmark fixture"))?;
    let mut change = store.begin().map_err(data_error)?;
    let first_key = data_key(&first.key)?;
    let first_revision = change
        .get(&first.space, &first_key)
        .map_err(data_error)?
        .ok_or_else(|| DevError::corrupt("benchmark update fact missing"))?
        .revision;
    if !change
        .put(
            &first.space,
            &first_key,
            first.value.clone(),
            DataExpectation::Exact(first_revision),
        )
        .map_err(data_error)?
    {
        return Err(DevError::corrupt("benchmark update expectation failed"));
    }
    let last_key = data_key(&last.key)?;
    let last_revision = change
        .get(&last.space, &last_key)
        .map_err(data_error)?
        .ok_or_else(|| DevError::corrupt("benchmark delete fact missing"))?
        .revision;
    if !change
        .delete(
            &last.space,
            &last_key,
            DataExpectation::Exact(last_revision),
        )
        .map_err(data_error)?
    {
        return Err(DevError::corrupt("benchmark delete expectation failed"));
    }
    if let DataCommitOutcome::Committed {
        durable_bytes,
        fsync_publications,
        ..
    } = change.commit().map_err(data_error)?
    {
        metrics.durable_bytes = metrics.durable_bytes.saturating_add(durable_bytes as u64);
        metrics.fsync_publications = metrics
            .fsync_publications
            .saturating_add(fsync_publications as u64);
    }
    metrics.operations = metrics.operations.saturating_add(4);
    remove_exact_directory(root)?;
    Ok(metrics)
}

fn benchmark_postgres(
    port: u16,
    workload: Workload,
    ordinal: &str,
    facts: &[CanonicalFact],
) -> Result<ChildMetrics, DevError> {
    if !ordinal
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(DevError::usage("invalid PostgreSQL benchmark ordinal"));
    }
    let schema = format!("bench_{}_{}", workload.as_str(), ordinal.replace('-', "_"));
    let mut client = Client::connect(
        &format!("host=127.0.0.1 port={port} user=postgres dbname=oracle connect_timeout=2"),
        NoTls,
    )
    .map_err(pg_error)?;
    client
        .batch_execute(&format!(
            "DROP SCHEMA IF EXISTS {schema} CASCADE;
             CREATE SCHEMA {schema};
             CREATE TABLE {schema}.facts(space text NOT NULL,key bytea NOT NULL,value bytea NOT NULL,PRIMARY KEY(space,key));"
        ))
        .map_err(pg_error)?;
    let before_sync: i64 = client
        .query_one("SELECT wal_sync FROM pg_stat_wal", &[])
        .map_err(pg_error)?
        .get(0);
    let mut transaction = client.transaction().map_err(pg_error)?;
    for fact in facts {
        transaction
            .execute(
                &format!("INSERT INTO {schema}.facts(space,key,value) VALUES($1,$2,$3)"),
                &[&fact.space, &encode_neutral_key(&fact.key), &fact.value],
            )
            .map_err(pg_error)?;
    }
    transaction.commit().map_err(pg_error)?;
    let rows = client
        .query(
            &format!("SELECT space,key,value FROM {schema}.facts ORDER BY space,key"),
            &[],
        )
        .map_err(pg_error)?;
    let first = facts
        .first()
        .ok_or_else(|| DevError::corrupt("empty PostgreSQL benchmark fixture"))?;
    let last = facts
        .last()
        .ok_or_else(|| DevError::corrupt("empty PostgreSQL benchmark fixture"))?;
    client
        .execute(
            &format!("UPDATE {schema}.facts SET value=$1 WHERE space=$2 AND key=$3"),
            &[&first.value, &first.space, &encode_neutral_key(&first.key)],
        )
        .map_err(pg_error)?;
    client
        .execute(
            &format!("DELETE FROM {schema}.facts WHERE space=$1 AND key=$2"),
            &[&last.space, &encode_neutral_key(&last.key)],
        )
        .map_err(pg_error)?;
    let durable: i64 = client
        .query_one(
            &format!("SELECT pg_total_relation_size('{schema}.facts')"),
            &[],
        )
        .map_err(pg_error)?
        .get(0);
    let after_sync: i64 = client
        .query_one("SELECT wal_sync FROM pg_stat_wal", &[])
        .map_err(pg_error)?
        .get(0);
    client
        .batch_execute(&format!("DROP SCHEMA {schema} CASCADE"))
        .map_err(pg_error)?;
    Ok(ChildMetrics {
        workload,
        backend: "postgresql".to_owned(),
        facts: facts.len() as u64,
        operations: facts.len().saturating_add(rows.len()).saturating_add(4) as u64,
        durable_bytes: u64::try_from(durable)
            .map_err(|_| DevError::corrupt("negative PostgreSQL durable bytes"))?,
        fsync_publications: u64::try_from(after_sync.saturating_sub(before_sync))
            .map_err(|_| DevError::corrupt("negative PostgreSQL WAL sync delta"))?,
        fact_digest: facts_digest(facts),
        cleanup_complete: true,
    })
}

fn encode_neutral_key(parts: &[NeutralPart]) -> Vec<u8> {
    let mut output = Vec::new();
    for part in parts {
        match part {
            NeutralPart::Bool(value) => output.extend_from_slice(&[0, u8::from(*value)]),
            NeutralPart::I64(value) => {
                output.push(1);
                output.extend_from_slice(&value.to_be_bytes());
            }
            NeutralPart::Text(value) => {
                output.push(2);
                output.extend_from_slice(&(value.len() as u32).to_be_bytes());
                output.extend_from_slice(value.as_bytes());
            }
            NeutralPart::Bytes(value) => {
                output.push(3);
                output.extend_from_slice(&(value.len() as u32).to_be_bytes());
                output.extend_from_slice(value);
            }
        }
    }
    output
}

fn facts_digest(facts: &[CanonicalFact]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"lkjscript-neutral-facts-1\0");
    for fact in facts {
        digest_field(&mut hasher, fact.space.as_bytes());
        digest_field(&mut hasher, &encode_neutral_key(&fact.key));
        digest_field(&mut hasher, &fact.value);
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn digest_field(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn resource_sample(
    metrics: &ChildMetrics,
    observation: &ProcessObservation,
    additional_rss: u64,
) -> Result<ResourceSample, DevError> {
    Ok(ResourceSample {
        wall_nanoseconds: observation.elapsed_nanoseconds,
        cpu_nanoseconds: observation
            .cpu_nanoseconds
            .ok_or_else(|| DevError::unavailable("resource sample omitted CPU time"))?,
        peak_rss_kib: observation
            .peak_rss_kib
            .ok_or_else(|| DevError::unavailable("resource sample omitted peak RSS"))?
            .saturating_add(additional_rss),
        durable_bytes: metrics.durable_bytes,
        fsync_publications: metrics.fsync_publications,
        operations: metrics.operations,
    })
}

fn median(samples: &[ResourceSample]) -> Result<ResourceMedian, DevError> {
    if samples.len() != SAMPLES {
        return Err(DevError::infrastructure(
            "resource sample count is not exact",
        ));
    }
    Ok(ResourceMedian {
        wall_nanoseconds: median_value(samples.iter().map(|sample| sample.wall_nanoseconds)),
        cpu_nanoseconds: median_value(samples.iter().map(|sample| sample.cpu_nanoseconds)),
        peak_rss_kib: median_value(samples.iter().map(|sample| sample.peak_rss_kib)),
        durable_bytes: median_value(samples.iter().map(|sample| sample.durable_bytes)),
        fsync_publications: median_value(samples.iter().map(|sample| sample.fsync_publications)),
        operations: median_value(samples.iter().map(|sample| sample.operations)),
    })
}

fn median_value(values: impl Iterator<Item = u64>) -> u64 {
    let mut values = values.collect::<Vec<_>>();
    values.sort_unstable();
    values[values.len() / 2]
}

fn ratio_millionths(numerator: u64, denominator: u64) -> Result<u64, DevError> {
    if denominator == 0 {
        return Err(DevError::unavailable("resource ratio denominator is zero"));
    }
    Ok(numerator.saturating_mul(1_000_000) / denominator)
}

fn validate_public_receipts(
    binary: &Path,
    bbs_receipt: &Path,
    service_receipt: &Path,
) -> Result<(), DevError> {
    let binary_sha = sha256_file(binary)?;
    let binary_verification = VerificationDigest::of(&fs::read(binary)?).to_string();
    let bbs: Value = serde_json::from_slice(&fs::read(bbs_receipt)?)?;
    if bbs.pointer("/status").and_then(Value::as_str) != Some("passed")
        || bbs.pointer("/candidate/sha256").and_then(Value::as_str) != Some(&binary_sha)
        || bbs
            .pointer("/result/live/data_contract")
            .and_then(Value::as_str)
            != Some(DATA_CONTRACT)
        || bbs
            .pointer("/result/live/backup_restore_equivalent")
            .and_then(Value::as_bool)
            != Some(true)
        || bbs
            .pointer("/result/live/authority_unchanged")
            .and_then(Value::as_bool)
            != Some(true)
        || bbs
            .pointer("/cleanup/data_cleanup_complete")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Err(DevError::corrupt(
            "BBS receipt does not bind the exact first-party-data public outcome",
        ));
    }
    let service: Value = serde_json::from_slice(&fs::read(service_receipt)?)?;
    if service.pointer("/status").and_then(Value::as_str) != Some("passed")
        || service.pointer("/binary/digest").and_then(Value::as_str)
            != Some(binary_verification.as_str())
        || service.pointer("/data_contract").and_then(Value::as_str) != Some(DATA_CONTRACT)
        || service
            .pointer("/result/data_contract")
            .and_then(Value::as_str)
            != Some(DATA_CONTRACT)
        || service
            .pointer("/result/authority_unchanged")
            .and_then(Value::as_bool)
            != Some(true)
        || service
            .pointer("/result/restart_read_equal")
            .and_then(Value::as_bool)
            != Some(true)
        || service
            .pointer("/result/restored_read_equal")
            .and_then(Value::as_bool)
            != Some(true)
        || service
            .pointer("/result/corrupt_backup_rejected")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Err(DevError::corrupt(
            "service receipt does not bind the exact first-party-data public outcome",
        ));
    }
    Ok(())
}

fn resolve_regular_file(repository: &Path, path: &Path, label: &str) -> Result<PathBuf, DevError> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        repository.join(path)
    };
    let path = path.canonicalize().map_err(|error| {
        DevError::usage(format!("resolve {label} '{}': {error}", path.display()))
    })?;
    let metadata = fs::symlink_metadata(&path).map_err(|error| {
        DevError::usage(format!("inspect {label} '{}': {error}", path.display()))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DevError::usage(format!("{label} is not a regular file")));
    }
    Ok(path)
}

fn repository_root() -> Result<PathBuf, DevError> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|error| DevError::infrastructure(format!("resolve repository root: {error}")))?;
    Ok(root)
}

fn new_run_directory(repository: &Path) -> Result<PathBuf, DevError> {
    let parent = repository.join(".artifacts/lkjscript-dev/data-oracle");
    fs::create_dir_all(&parent)?;
    let directory = parent.join(format!(
        "{}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| DevError::infrastructure(format!("system clock: {error}")))?
            .as_nanos(),
        std::process::id()
    ));
    fs::create_dir(&directory)?;
    Ok(directory)
}

fn remove_exact_directory(path: &Path) -> Result<(), DevError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(DevError::infrastructure("unsafe oracle cleanup path"));
    }
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => Err(
            DevError::infrastructure("oracle cleanup target changed type"),
        ),
        Ok(_) => fs::remove_dir_all(path).map_err(|error| {
            DevError::infrastructure(format!("remove oracle data '{}': {error}", path.display()))
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(DevError::infrastructure(format!(
            "inspect oracle cleanup target '{}': {error}",
            path.display()
        ))),
    }
}

fn sha256_file(path: &Path) -> Result<String, DevError> {
    let bytes = fs::read(path)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn data_error(error: lkjscript::platform::diagnostic::Diagnostic) -> DevError {
    DevError::corrupt(format!("{}: {}", error.code, error.message))
}

fn pg_error(error: postgres::Error) -> DevError {
    DevError::infrastructure(format!("PostgreSQL oracle: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_and_reference_order_are_deterministic() {
        let first = deterministic_fixture();
        let second = deterministic_fixture();
        assert_eq!(first, second);
        let facts = fixture_facts(&first, None);
        assert_eq!(facts.len(), 416);
        assert!(
            facts
                .windows(2)
                .all(|pair| compare_fact(&pair[0], &pair[1]) != Ordering::Greater)
        );
        assert_eq!(facts_digest(&facts).len(), 64);
    }

    #[test]
    fn resource_ratios_and_medians_are_exact() {
        assert_eq!(ratio_millionths(5, 2).expect("ratio"), 2_500_000);
        assert!(ratio_millionths(1, 0).is_err());
        assert_eq!(median_value([9, 1, 5].into_iter()), 5);
    }
}
