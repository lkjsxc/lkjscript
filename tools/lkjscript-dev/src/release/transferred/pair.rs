//! Within-invocation binding, never cross-run reuse or public acquisition authentication.
use super::*;
use std::io::Read;
use std::os::unix::fs::MetadataExt;

mod installation;
mod lifecycle;
mod recovery;
use installation::Acquisition;

const PAIR_SCHEMA: &str = "lkjscript-transferred-pair";
const PAIR_VERSION: u32 = 2;
const POLICY: &str = "static-installed-pair-cleared-environment-private-state-2";

#[derive(Clone, Debug)]
struct PairOptions {
    verify: bool,
    exact_assets: PathBuf,
    latest_assets: PathBuf,
    tag: String,
    commit: String,
    publication: PublicationMode,
    acquisition: Acquisition,
    evidence_root: PathBuf,
    verifier_identity: PathBuf,
    expected_verifier_sha256: String,
    expected_verifier_bytes: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Route {
    Exact,
    Latest,
}
impl Route {
    fn name(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Latest => "latest",
        }
    }
    fn assets(self, options: &PairOptions) -> &Path {
        match self {
            Self::Exact => &options.exact_assets,
            Self::Latest => &options.latest_assets,
        }
    }
    fn root(self, options: &PairOptions) -> PathBuf {
        options.evidence_root.join(self.name())
    }
    fn extraction(self, options: &PairOptions) -> PathBuf {
        self.root(options).join("extracted")
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Scope {
    PublicFormatValidation,
    LocalPairRehearsal,
}
fn scope(acquisition: Acquisition) -> Scope {
    match acquisition {
        Acquisition::Anonymous => Scope::PublicFormatValidation,
        Acquisition::Simulated => Scope::LocalPairRehearsal,
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FileBinding {
    file: ExternalEvidence,
    mode: u32,
}
fn binding(path: &Path) -> Result<FileBinding, DevError> {
    let file = external(path)?;
    require(
        path.as_os_str() == path.canonicalize()?.as_os_str(),
        "pair file path is an alias",
    )?;
    let metadata = fs::symlink_metadata(path)?;
    require(metadata.nlink() == 1, "pair input has a shared-file alias")?;
    Ok(FileBinding {
        file,
        mode: metadata.permissions().mode() & 0o7777,
    })
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Inputs {
    archive: FileBinding,
    checksums: FileBinding,
    installer: FileBinding,
}
fn inputs(path: &Path) -> Result<Inputs, DevError> {
    directory(path)?;
    let mut names = fs::read_dir(path)?
        .take(4)
        .map(|e| e.map(|e| e.file_name()))
        .collect::<Result<Vec<_>, _>>()?;
    names.sort();
    let mut expected = [
        OsString::from(archive::ARCHIVE_NAME),
        OsString::from(archive::CHECKSUM_NAME),
        OsString::from(super::super::bootstrap::NAME),
    ];
    expected.sort();
    require(
        names == expected,
        "pair asset directory must contain exactly the target archive, SHA256SUMS and install.sh",
    )?;
    Ok(Inputs {
        archive: binding(&path.join(archive::ARCHIVE_NAME))?,
        checksums: binding(&path.join(archive::CHECKSUM_NAME))?,
        installer: binding(&path.join(super::super::bootstrap::NAME))?,
    })
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Extraction {
    path: String,
    mode: u32,
    members: Vec<FileBinding>,
    candidate: target::BuiltCandidate,
}
fn extraction(path: &Path, admitted: &archive::VerifiedArchive) -> Result<Extraction, DevError> {
    directory(path)?;
    let mut names = fs::read_dir(path)?
        .take(5)
        .map(|e| e.map(|e| e.file_name()))
        .collect::<Result<Vec<_>, _>>()?;
    names.sort();
    let expected = [
        "LICENSE",
        "RELEASE-MANIFEST.json",
        "THIRD-PARTY-LICENSES.html",
        "lkjscript",
    ];
    require(
        names == expected.map(OsString::from),
        "pair extraction inventory changed",
    )?;
    let mut members = Vec::new();
    for member in &admitted.members {
        if member.name == archive::TOP_DIRECTORY {
            continue;
        }
        let name = member
            .name
            .strip_prefix(archive::TOP_DIRECTORY)
            .ok_or_else(|| DevError::corrupt("foreign admitted member"))?;
        require(expected.contains(&name), "foreign admitted payload")?;
        let observed = binding(&path.join(name))?;
        require(
            observed.mode == member.mode
                && observed.file.byte_length == member.byte_length
                && Some(&observed.file.sha256) == member.sha256.as_ref(),
            "admitted payload bytes or modes changed",
        )?;
        members.push(observed);
    }
    require(members.len() == 4, "incomplete admitted payloads")?;
    let mode = fs::metadata(path)?.permissions().mode() & 0o7777;
    require(mode == 0o755, "extraction directory mode changed")?;
    let candidate = target::observe_candidate(&path.join("lkjscript"))?;
    require(
        candidate.elf == admitted.manifest.executable.elf,
        "pair executable linkage changed",
    )?;
    Ok(Extraction {
        path: path.display().to_string(),
        mode,
        members,
        candidate,
    })
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RouteReceipt {
    route: Route,
    inputs: Inputs,
    admission: Option<archive::VerifiedArchive>,
    extraction: Option<Extraction>,
    admission_nanoseconds: u64,
    lifecycle: Option<lifecycle::Lifecycle>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Comparison {
    name: String,
    exact: FileBinding,
    latest: FileBinding,
    compared_bytes: u64,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Equality {
    comparisons: Vec<Comparison>,
    admitted_modes_equal: bool,
    execution_policy: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Disposition {
    FreshExecution,
    BoundWithinSamePair,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SuiteBinding {
    exact: Disposition,
    latest: Disposition,
    source_aggregate: ReceiptIdentity,
    equality_digest: evidence::VerificationDigest,
    pair_root: String,
    scope: Scope,
    hosted_context: HostedContext,
    started_unix_nanoseconds: u128,
    completed_unix_nanoseconds: u128,
    expensive_executions: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Environment {
    policy: String,
    candidate_environment: BTreeMap<String, String>,
    scratch_policy: String,
    architecture: String,
    kernel: String,
}
fn environment() -> Result<Environment, DevError> {
    // procfs reports a zero file length. Bound the stream itself, not a regular-file stat length.
    let mut kernel = String::new();
    fs::File::open("/proc/sys/kernel/osrelease")?
        .take(4097)
        .read_to_string(&mut kernel)?;
    require(
        !kernel.is_empty() && kernel.len() <= 4096,
        "kernel identity exceeds its bound",
    )?;
    Ok(Environment {
        policy: POLICY.to_owned(),
        candidate_environment: BTreeMap::from([("LANG".to_owned(), "C".to_owned()), ("PATH".to_owned(), "/usr/bin:/bin".to_owned())]),
        scratch_policy: "create-new-private-cwd-and-TMPDIR-per-lifecycle-and-child;no-inherited-home-source-credentials-or-sidecars".to_owned(),
        architecture: std::env::consts::ARCH.to_owned(),
        kernel,
    })
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PairReceipt {
    schema: SchemaIdentity,
    status: Status,
    phase: String,
    scope: Scope,
    tag: String,
    source_commit: String,
    publication: PublicationMode,
    acquisition: Acquisition,
    evidence_root: String,
    target: String,
    target_policy_sha256: String,
    verifier: FileBinding,
    verifier_handoff: FileBinding,
    environment: Environment,
    hosted_context: HostedContext,
    started_unix_nanoseconds: u128,
    completed_unix_nanoseconds: Option<u128>,
    elapsed_nanoseconds: u64,
    equality_nanoseconds: u64,
    receipt_verification_nanoseconds: u64,
    routes: [RouteReceipt; 2],
    equality: Option<Equality>,
    suite: Option<SuiteBinding>,
    recovery: Option<recovery::Recovery>,
    cleanup_complete: bool,
    failure: Option<String>,
}

pub(super) fn command(
    operation: &str,
    arguments: impl Iterator<Item = OsString>,
) -> Result<u8, DevError> {
    let options = parse_options(operation, arguments)?;
    let verifier = std::env::current_exe()?.canonicalize()?;
    validate_options(&options, &verifier)?;
    let cancellation = Cancellation::new()?;
    if options.verify {
        let receipt = read_controlled(&options, &verifier, Some(&cancellation.control))?;
        summary(&options, &receipt)?;
        return Ok(0);
    }
    let receipt = run(&options, &verifier, &cancellation.control)?;
    summary(&options, &receipt)?;
    Ok(if receipt.status == Status::FreshPassed {
        0
    } else {
        1
    })
}

fn run(
    options: &PairOptions,
    verifier: &Path,
    control: &process::ProcessControl,
) -> Result<PairReceipt, DevError> {
    super::super::require_absolute_extraction_output(&options.evidence_root)?;
    fs::create_dir(&options.evidence_root)?;
    fs::set_permissions(&options.evidence_root, fs::Permissions::from_mode(0o700))?;
    let started = Instant::now();
    let mut receipt = PairReceipt {
        schema: SchemaIdentity {
            identity: PAIR_SCHEMA.to_owned(),
            version: PAIR_VERSION,
        },
        status: Status::NotRun,
        phase: "created".to_owned(),
        scope: scope(options.acquisition),
        tag: options.tag.clone(),
        source_commit: options.commit.clone(),
        publication: options.publication,
        acquisition: options.acquisition,
        evidence_root: options.evidence_root.display().to_string(),
        target: target::TARGET_TRIPLE.to_owned(),
        target_policy_sha256: target::policy_sha256()?,
        verifier: binding(verifier)?,
        verifier_handoff: binding(&options.verifier_identity)?,
        environment: environment()?,
        hosted_context: context()?,
        started_unix_nanoseconds: super::super::unix_nanoseconds()?,
        completed_unix_nanoseconds: None,
        elapsed_nanoseconds: 0,
        equality_nanoseconds: 0,
        receipt_verification_nanoseconds: 0,
        routes: [
            RouteReceipt {
                route: Route::Exact,
                inputs: inputs(&options.exact_assets)?,
                admission: None,
                extraction: None,
                admission_nanoseconds: 0,
                lifecycle: None,
            },
            RouteReceipt {
                route: Route::Latest,
                inputs: inputs(&options.latest_assets)?,
                admission: None,
                extraction: None,
                admission_nanoseconds: 0,
                lifecycle: None,
            },
        ],
        equality: None,
        suite: None,
        recovery: None,
        cleanup_complete: false,
        failure: None,
    };
    persist(options, &receipt)?;
    let result = execute_pair(options, verifier, &mut receipt, control);
    receipt.completed_unix_nanoseconds = Some(super::super::unix_nanoseconds()?);
    receipt.elapsed_nanoseconds = elapsed(started)?;
    receipt.cleanup_complete = clean(options, &receipt)?;
    match result {
        Ok(()) if receipt.cleanup_complete && !control.cancelled() => {
            receipt.status = Status::FreshPassed;
            receipt.phase = "complete".to_owned();
        }
        result => {
            receipt.status = Status::Failed;
            receipt.failure = Some(match result {
                Err(error) => error.to_string(),
                Ok(()) => "pair cancelled or cleanup incomplete".to_owned(),
            });
        }
    }
    if receipt.status == Status::FreshPassed {
        let validation_started = Instant::now();
        if let Err(error) = validate(&receipt, options, verifier) {
            receipt.status = Status::Failed;
            receipt.failure = Some(error.to_string());
        }
        receipt.receipt_verification_nanoseconds = elapsed(validation_started)?;
    }
    receipt.completed_unix_nanoseconds = Some(super::super::unix_nanoseconds()?);
    receipt.elapsed_nanoseconds = elapsed(started)?;
    persist(options, &receipt)?;
    if receipt.status == Status::FreshPassed {
        // Returning success always includes a strict read of the atomically published bytes.
        if let Err(error) = read_controlled(options, verifier, Some(control))
            .and_then(|_| checkpoint(options, verifier, &receipt, control))
        {
            receipt.status = Status::Failed;
            receipt.failure = Some(error.to_string());
            persist(options, &receipt)?;
        }
    }
    Ok(receipt)
}

fn execute_pair(
    options: &PairOptions,
    verifier: &Path,
    receipt: &mut PairReceipt,
    control: &process::ProcessControl,
) -> Result<(), DevError> {
    for index in 0..2 {
        checkpoint(options, verifier, receipt, control)?;
        let route = receipt.routes[index].route;
        receipt.phase = format!("{}-admission", route.name());
        persist(options, receipt)?;
        let root = route.root(options);
        fs::create_dir(&root)?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        let started = Instant::now();
        let assets = route.assets(options);
        let admitted = archive::admit_archive(
            &assets.join(archive::ARCHIVE_NAME),
            &assets.join(archive::CHECKSUM_NAME),
            &root,
            &route.extraction(options),
            control,
        )?;
        validate_admission(&admitted, options)?;
        super::super::bootstrap::verify(&assets.join(super::super::bootstrap::NAME), &admitted)?;
        let observed = extraction(&route.extraction(options), &admitted)?;
        receipt.routes[index].admission = Some(admitted);
        receipt.routes[index].extraction = Some(observed);
        receipt.routes[index].admission_nanoseconds = elapsed(started)?;
        persist(options, receipt)?;
    }
    checkpoint(options, verifier, receipt, control)?;
    let started = Instant::now();
    receipt.phase = "equality".to_owned();
    persist(options, receipt)?;
    receipt.equality = Some(compare(options, &receipt.routes)?);
    receipt.equality_nanoseconds = elapsed(started)?;
    persist(options, receipt)?;
    for index in 0..2 {
        checkpoint(options, verifier, receipt, control)?;
        let route = receipt.routes[index].route;
        receipt.phase = format!("{}-lifecycle", route.name());
        persist(options, receipt)?;
        let admitted = receipt.routes[index]
            .admission
            .clone()
            .ok_or_else(|| DevError::corrupt("route admission missing"))?;
        receipt.routes[index].lifecycle = Some(lifecycle::run(options, route, &admitted, control)?);
        persist(options, receipt)?;
        lifecycle::validate(
            options,
            route,
            &admitted,
            receipt.routes[index]
                .lifecycle
                .as_ref()
                .ok_or_else(|| DevError::corrupt("lifecycle missing"))?,
        )?;
        checkpoint(options, verifier, receipt, control)?;
    }
    receipt.phase = "installation-recovery".to_owned();
    persist(options, receipt)?;
    receipt.recovery = Some(recovery::run(
        options,
        receipt.routes[0]
            .admission
            .as_ref()
            .ok_or_else(|| DevError::corrupt("missing recovery archive"))?,
        control,
    )?);
    persist(options, receipt)?;
    receipt.phase = "full-suite".to_owned();
    persist(options, receipt)?;
    #[cfg(test)]
    tests::phase_hook("before-suite", control);
    let frozen = receipt.clone();
    let single = single_options(options);
    let manifest = load_manifest(&single)?;
    let suite_started = super::super::unix_nanoseconds()?;
    let aggregate = execute(&single, verifier, &manifest, control, &mut || {
        checkpoint(options, verifier, &frozen, control)
    })?;
    require(
        aggregate.status == Status::FreshPassed,
        "pair behavioral aggregate failed",
    )?;
    let equality = receipt
        .equality
        .as_ref()
        .ok_or_else(|| DevError::corrupt("equality missing"))?;
    receipt.suite = Some(SuiteBinding {
        exact: Disposition::FreshExecution,
        latest: Disposition::BoundWithinSamePair,
        source_aggregate: receipt_identity(&single.evidence_root.join("receipt.json"))?,
        equality_digest: evidence::VerificationDigest::of(&evidence::encode_json(equality)?),
        pair_root: receipt.evidence_root.clone(),
        scope: receipt.scope,
        hosted_context: receipt.hosted_context.clone(),
        started_unix_nanoseconds: suite_started,
        completed_unix_nanoseconds: super::super::unix_nanoseconds()?,
        expensive_executions: aggregate.children.len(),
    });
    checkpoint(options, verifier, receipt, control)
}

fn checkpoint(
    options: &PairOptions,
    verifier: &Path,
    receipt: &PairReceipt,
    control: &process::ProcessControl,
) -> Result<(), DevError> {
    #[cfg(test)]
    tests::phase_hook(&receipt.phase, control);
    require(!control.cancelled(), "pair cancelled")?;
    frozen_inputs(options, verifier, receipt)
}
fn frozen_inputs(
    options: &PairOptions,
    verifier: &Path,
    receipt: &PairReceipt,
) -> Result<(), DevError> {
    validate_options(options, verifier)?;
    require(
        binding(verifier)? == receipt.verifier
            && binding(&options.verifier_identity)? == receipt.verifier_handoff
            && environment()? == receipt.environment
            && context()? == receipt.hosted_context,
        "pair verifier, handoff, environment or hosted context changed",
    )?;
    require(
        receipt.routes[0].route == Route::Exact && receipt.routes[1].route == Route::Latest,
        "pair routes omitted, duplicated or reordered",
    )?;
    for route in &receipt.routes {
        require(
            inputs(route.route.assets(options))? == route.inputs,
            "pair asset bytes or modes changed",
        )?;
        match (&route.admission, &route.extraction) {
            (Some(admitted), Some(observed)) => {
                validate_admission(admitted, options)?;
                require(
                    admitted.archive_byte_length == route.inputs.archive.file.byte_length
                        && admitted.archive_sha256 == route.inputs.archive.file.sha256,
                    "route admission is foreign",
                )?;
                require(
                    extraction(&route.route.extraction(options), admitted)? == *observed,
                    "pair extraction changed",
                )?;
                let mut single = single_options(options);
                single.candidate = route.route.extraction(options).join("lkjscript");
                single.manifest = route
                    .route
                    .extraction(options)
                    .join("RELEASE-MANIFEST.json");
                require(
                    load_manifest(&single)? == admitted.manifest,
                    "admission manifest differs from actual extraction",
                )?;
            }
            (None, None) => (),
            _ => return Err(DevError::corrupt("partial route admission")),
        }
    }
    if let Some(equality) = &receipt.equality {
        require(
            compare(options, &receipt.routes)? == *equality,
            "pair equality changed",
        )?;
    }
    Ok(())
}

fn validate_admission(
    admitted: &archive::VerifiedArchive,
    options: &PairOptions,
) -> Result<(), DevError> {
    super::super::validate_manifest(&admitted.manifest)?;
    require(
        admitted.manifest.source.expected_release_tag == options.tag
            && admitted.manifest.source.tagged_commit_sha == options.commit
            && admitted.manifest.publication_mode == options.publication,
        "pair tag, source or publication mismatch",
    )?;
    require(
        archive::sha256_bytes(&archive::canonical_json(&admitted.manifest)?)?
            == admitted.manifest_sha256
            && admitted.source_timestamp_unix_seconds
                == admitted.manifest.source.commit_timestamp_unix_seconds,
        "admitted manifest identity changed",
    )?;
    require(
        admitted
            .members
            .iter()
            .map(|m| (&m.name, m.mode))
            .eq(archive::manifest_members()
                .iter()
                .map(|m| (&m.name, m.mode))),
        "admitted member inventory changed",
    )
}

fn compare(options: &PairOptions, routes: &[RouteReceipt; 2]) -> Result<Equality, DevError> {
    for route in routes {
        require(
            route.admission.is_some() && route.extraction.is_some(),
            "both strict admissions must precede sharing",
        )?;
    }
    let pairs = [
        (
            "archive",
            options.exact_assets.join(archive::ARCHIVE_NAME),
            options.latest_assets.join(archive::ARCHIVE_NAME),
        ),
        (
            "checksums",
            options.exact_assets.join(archive::CHECKSUM_NAME),
            options.latest_assets.join(archive::CHECKSUM_NAME),
        ),
        (
            "installer",
            options.exact_assets.join(super::super::bootstrap::NAME),
            options.latest_assets.join(super::super::bootstrap::NAME),
        ),
        (
            "manifest",
            Route::Exact
                .extraction(options)
                .join("RELEASE-MANIFEST.json"),
            Route::Latest
                .extraction(options)
                .join("RELEASE-MANIFEST.json"),
        ),
        (
            "executable",
            Route::Exact.extraction(options).join("lkjscript"),
            Route::Latest.extraction(options).join("lkjscript"),
        ),
    ];
    let mut comparisons = Vec::new();
    for (name, left, right) in pairs {
        let exact = binding(&left)?;
        let latest = binding(&right)?;
        require(exact.mode == latest.mode, "pair input modes differ")?;
        let compared_bytes = stream_equal(&left, &right)?;
        comparisons.push(Comparison {
            name: name.to_owned(),
            exact,
            latest,
            compared_bytes,
        });
    }
    require(
        routes[0].admission == routes[1].admission,
        "admitted route content or policy differs",
    )?;
    Ok(Equality {
        comparisons,
        admitted_modes_equal: true,
        execution_policy: POLICY.to_owned(),
    })
}

fn stream_equal(left: &Path, right: &Path) -> Result<u64, DevError> {
    let mut left_file = fs::File::open(left)?;
    let mut right_file = fs::File::open(right)?;
    let length = left_file.metadata()?.len();
    require(
        length <= 384 * 1024 * 1024 && right_file.metadata()?.len() == length,
        "pair byte lengths differ or exceed bound",
    )?;
    let mut left_buffer = [0_u8; 64 * 1024];
    let mut right_buffer = [0_u8; 64 * 1024];
    let mut remaining = length;
    while remaining > 0 {
        let count = remaining.min(left_buffer.len() as u64) as usize;
        left_file.read_exact(&mut left_buffer[..count])?;
        right_file.read_exact(&mut right_buffer[..count])?;
        require(
            left_buffer[..count] == right_buffer[..count],
            "pair full-content bytes differ",
        )?;
        remaining -= count as u64;
    }
    require(
        left_file.read(&mut left_buffer[..1])? == 0
            && right_file.read(&mut right_buffer[..1])? == 0,
        "pair input grew during comparison",
    )?;
    Ok(length)
}

fn single_options(options: &PairOptions) -> Options {
    Options {
        verify: true,
        candidate: installation::candidate(options, Route::Exact),
        manifest: installation::candidate(options, Route::Exact)
            .with_file_name("RELEASE-MANIFEST.json"),
        tag: options.tag.clone(),
        commit: options.commit.clone(),
        publication: options.publication,
        boundary: match options.acquisition {
            Acquisition::Anonymous => Boundary::ExactDownload,
            Acquisition::Simulated => Boundary::PrePublication,
        },
        verifier_identity: options.verifier_identity.clone(),
        expected_verifier_sha256: options.expected_verifier_sha256.clone(),
        expected_verifier_bytes: options.expected_verifier_bytes,
        evidence_root: options.evidence_root.join("full-suite"),
    }
}

#[cfg(test)]
fn read(options: &PairOptions, verifier: &Path) -> Result<PairReceipt, DevError> {
    read_controlled(options, verifier, None)
}

fn read_controlled(
    options: &PairOptions,
    verifier: &Path,
    control: Option<&process::ProcessControl>,
) -> Result<PairReceipt, DevError> {
    require(
        control.is_none_or(|c| !c.cancelled()),
        "pair reread cancelled",
    )?;
    let path = options.evidence_root.join("receipt.json");
    regular(&path)?;
    let bytes = process::read_bounded(&path, MAXIMUM_RECEIPT_BYTES)?;
    let receipt: PairReceipt = serde_json::from_slice(&bytes)?;
    require(
        evidence::encode_json(&receipt)? == bytes,
        "pair receipt is noncanonical",
    )?;
    validate(&receipt, options, verifier)?;
    // Re-establish V from the containers, without executing either application or creating a
    // replacement admitted extraction. This also rejects forged typed admission summaries.
    for route in &receipt.routes {
        let work = tempfile::Builder::new()
            .prefix(".pair-reread-")
            .tempdir_in(&options.evidence_root)?;
        let observed = archive::verify_archive_controlled(
            &route.route.assets(options).join(archive::ARCHIVE_NAME),
            work.path(),
            None,
            control,
        );
        let closed = work.close();
        closed?;
        require(
            Some(observed?) == route.admission,
            "strict admission reread differs",
        )?;
        super::super::verify_checksum_bytes(
            &process::read_bounded(
                &route.route.assets(options).join(archive::CHECKSUM_NAME),
                1024,
            )?,
            &route.inputs.archive.file.sha256,
        )?;
    }
    require(
        control.is_none_or(|c| !c.cancelled()),
        "pair reread cancelled",
    )?;
    frozen_inputs(options, verifier, &receipt)?;
    Ok(receipt)
}
fn validate(receipt: &PairReceipt, options: &PairOptions, verifier: &Path) -> Result<(), DevError> {
    require(
        receipt.schema.identity == PAIR_SCHEMA
            && receipt.schema.version == PAIR_VERSION
            && receipt.status == Status::FreshPassed
            && receipt.phase == "complete"
            && receipt.failure.is_none()
            && receipt.cleanup_complete
            && receipt.acquisition == options.acquisition
            && receipt.scope == scope(options.acquisition)
            && receipt.publication == options.publication
            && receipt.tag == options.tag
            && receipt.source_commit == options.commit
            && receipt.evidence_root == options.evidence_root.display().to_string()
            && receipt.target == target::TARGET_TRIPLE
            && receipt.target_policy_sha256 == target::policy_sha256()?
            && receipt.started_unix_nanoseconds > 0
            && receipt
                .completed_unix_nanoseconds
                .is_some_and(|v| v >= receipt.started_unix_nanoseconds),
        "pair receipt is foreign, incomplete or non-passing",
    )?;
    frozen_inputs(options, verifier, receipt)?;
    for route in &receipt.routes {
        let admitted = route
            .admission
            .as_ref()
            .ok_or_else(|| DevError::corrupt("missing independent route admission"))?;
        let lifecycle = route
            .lifecycle
            .as_ref()
            .ok_or_else(|| DevError::corrupt("missing independent route lifecycle"))?;
        lifecycle::validate(options, route.route, admitted, lifecycle)?;
        require(
            lifecycle.started_unix_nanoseconds >= receipt.started_unix_nanoseconds
                && Some(lifecycle.completed_unix_nanoseconds) <= receipt.completed_unix_nanoseconds,
            "route lifecycle is outside this pair",
        )?;
    }
    recovery::validate(
        options,
        receipt.routes[0]
            .admission
            .as_ref()
            .ok_or_else(|| DevError::corrupt("missing recovery admission"))?,
        receipt
            .recovery
            .as_ref()
            .ok_or_else(|| DevError::corrupt("missing two-version recovery proof"))?,
    )?;
    let equality = receipt
        .equality
        .as_ref()
        .ok_or_else(|| DevError::corrupt("missing complete content equality"))?;
    let suite = receipt
        .suite
        .as_ref()
        .ok_or_else(|| DevError::corrupt("missing source aggregate binding"))?;
    let single = single_options(options);
    require(
        suite.exact == Disposition::FreshExecution
            && suite.latest == Disposition::BoundWithinSamePair
            && suite.expensive_executions == ORACLES.len()
            && suite.pair_root == receipt.evidence_root
            && suite.scope == receipt.scope
            && suite.hosted_context == receipt.hosted_context
            && suite.equality_digest
                == evidence::VerificationDigest::of(&evidence::encode_json(equality)?)
            && suite.source_aggregate
                == receipt_identity(&single.evidence_root.join("receipt.json"))?,
        "pair shared disposition or source reference is foreign",
    )?;
    let manifest = load_manifest(&single)?;
    let aggregate = read_receipt(&single, verifier, &manifest)?;
    require(
        aggregate.hosted_context == receipt.hosted_context
            && aggregate.children.len() == suite.expensive_executions
            && suite.started_unix_nanoseconds >= receipt.started_unix_nanoseconds
            && aggregate.started_unix_nanoseconds >= suite.started_unix_nanoseconds
            && aggregate
                .completed_unix_nanoseconds
                .is_some_and(|v| v <= suite.completed_unix_nanoseconds)
            && Some(suite.completed_unix_nanoseconds) <= receipt.completed_unix_nanoseconds
            && receipt.routes.iter().all(|r| {
                r.lifecycle
                    .as_ref()
                    .is_some_and(|l| l.completed_unix_nanoseconds <= suite.started_unix_nanoseconds)
            }),
        "source aggregate is outside this pair invocation",
    )?;
    require(clean(options, receipt)?, "pair cleanup incomplete")
}

fn clean(options: &PairOptions, receipt: &PairReceipt) -> Result<bool, DevError> {
    for route in &receipt.routes {
        if !route.lifecycle.as_ref().is_some_and(|l| l.cleanup_complete) {
            return Ok(false);
        }
        if route.route.root(options).join("runtime").try_exists()? {
            return Ok(false);
        }
        if fs::read_dir(route.route.root(options))?
            .any(|entry| entry.map_or(true, |e| e.file_name().to_string_lossy().starts_with('.')))
        {
            return Ok(false);
        }
    }
    let single = options.evidence_root.join("full-suite");
    if !single.is_dir() {
        return Ok(false);
    }
    if fs::read_dir(single)?.any(|entry| {
        entry.map_or(true, |e| {
            e.file_name().to_string_lossy().starts_with(".child-state-")
        })
    }) {
        return Ok(false);
    }
    Ok(true)
}
fn persist(options: &PairOptions, receipt: &PairReceipt) -> Result<(), DevError> {
    evidence::publish_json(&options.evidence_root.join("receipt.json"), receipt)?;
    Ok(())
}
fn elapsed(started: Instant) -> Result<u64, DevError> {
    u64::try_from(started.elapsed().as_nanos())
        .map_err(|_| DevError::corrupt("pair elapsed overflow"))
}
fn directory(path: &Path) -> Result<(), DevError> {
    archive::ensure_directory(path, "pair directory")?;
    require(
        path.is_absolute() && path.canonicalize()?.as_os_str() == path.as_os_str(),
        "pair directory must be canonical, absolute and nonsymlink",
    )
}
fn context() -> Result<HostedContext, DevError> {
    let context = super::super::hosted_context();
    let values = [
        &context.github_actions,
        &context.repository,
        &context.workflow,
        &context.job,
        &context.run_id,
        &context.run_attempt,
        &context.run_url,
        &context.runner_os,
        &context.runner_architecture,
        &context.runner_image_os,
        &context.runner_image_version,
    ];
    require(
        values
            .iter()
            .all(|v| v.as_ref().is_none_or(|v| !v.is_empty())),
        "blank hosted context is not an identity",
    )?;
    if values.iter().any(|v| v.is_some()) {
        require(
            context.github_actions.as_deref() == Some("true")
                && context.repository.as_deref() == Some("lkjsxc/lkjscript")
                && context.workflow.is_some()
                && context.job.is_some()
                && context
                    .run_id
                    .as_ref()
                    .is_some_and(|s| s.parse::<u64>().is_ok_and(|n| n > 0))
                && context
                    .run_attempt
                    .as_ref()
                    .is_some_and(|s| s.parse::<u64>().is_ok_and(|n| n > 0)),
            "incomplete hosted run identity",
        )?;
    }
    Ok(context)
}
fn validate_options(options: &PairOptions, verifier: &Path) -> Result<(), DevError> {
    directory(&options.exact_assets)?;
    directory(&options.latest_assets)?;
    require(
        options.evidence_root.is_absolute(),
        "pair evidence root must be absolute",
    )?;
    let parent = options
        .evidence_root
        .parent()
        .ok_or_else(|| DevError::usage("pair evidence parent missing"))?;
    directory(parent)?;
    require(
        options
            .evidence_root
            .file_name()
            .is_some_and(|name| parent.join(name).as_os_str() == options.evidence_root.as_os_str()),
        "pair evidence path is an alias",
    )?;
    if options.evidence_root.try_exists()? {
        directory(&options.evidence_root)?;
    }
    require(
        !options.evidence_root.starts_with(
            options
                .verifier_identity
                .parent()
                .ok_or_else(|| DevError::usage("verifier parent missing"))?,
        ) && !verifier.starts_with(&options.evidence_root),
        "pair output overlaps verifier handoff",
    )?;
    require(
        options.exact_assets != options.latest_assets
            && !options.exact_assets.starts_with(&options.latest_assets)
            && !options.latest_assets.starts_with(&options.exact_assets),
        "pair routes must have separate canonical directories",
    )?;
    for input in [&options.exact_assets, &options.latest_assets] {
        require(
            !options.evidence_root.starts_with(input) && !input.starts_with(&options.evidence_root),
            "pair input and output roots overlap",
        )?;
        inputs(input)?;
    }
    let mut identities = std::collections::BTreeSet::new();
    for directory in [&options.exact_assets, &options.latest_assets] {
        for name in [
            archive::ARCHIVE_NAME,
            archive::CHECKSUM_NAME,
            super::super::bootstrap::NAME,
        ] {
            let metadata = fs::symlink_metadata(directory.join(name))?;
            require(
                identities.insert((metadata.dev(), metadata.ino())),
                "pair inputs alias the same file",
            )?;
        }
    }
    regular(&options.verifier_identity)?;
    verifier::validate_handoff(
        verifier,
        &options.verifier_identity,
        &options.tag,
        &options.commit,
        &options.expected_verifier_sha256,
        options.expected_verifier_bytes,
    )?;
    context()?;
    Ok(())
}
fn parse_options(
    operation: &str,
    arguments: impl Iterator<Item = OsString>,
) -> Result<PairOptions, DevError> {
    let mut values = verifier::parse_values(
        arguments,
        &[
            "--exact-assets",
            "--latest-assets",
            "--tag",
            "--commit",
            "--publication",
            "--acquisition",
            "--evidence-root",
            "--verifier-identity",
            "--expected-verifier-sha256",
            "--expected-verifier-bytes",
        ],
    )?;
    let exact_assets = PathBuf::from(verifier::required(&mut values, "--exact-assets")?);
    let latest_assets = PathBuf::from(verifier::required(&mut values, "--latest-assets")?);
    let tag = verifier::required(&mut values, "--tag")?;
    verifier::validate_tag(&tag)?;
    let commit = verifier::required(&mut values, "--commit")?;
    super::super::validate_git_sha(&commit, "pair source")?;
    let publication =
        super::super::parse_publication(verifier::required(&mut values, "--publication")?)?;
    let acquisition = match verifier::required(&mut values, "--acquisition")?.as_str() {
        "simulated" => Acquisition::Simulated,
        "anonymous" if publication == PublicationMode::Release => Acquisition::Anonymous,
        _ => {
            return Err(DevError::usage(
                "pair acquisition must be simulated, or anonymous with release publication",
            ));
        }
    };
    let expected_verifier_sha256 = verifier::required(&mut values, "--expected-verifier-sha256")?;
    Sha256Digest::new(expected_verifier_sha256.clone()).map_err(DevError::usage)?;
    let expected_verifier_bytes = verifier::required(&mut values, "--expected-verifier-bytes")?
        .parse::<u64>()
        .ok()
        .filter(|n| *n > 0)
        .ok_or_else(|| DevError::usage("expected verifier bytes must be positive"))?;
    Ok(PairOptions {
        verify: operation == "pair-verify",
        exact_assets,
        latest_assets,
        tag,
        commit,
        publication,
        acquisition,
        evidence_root: PathBuf::from(verifier::required(&mut values, "--evidence-root")?),
        verifier_identity: PathBuf::from(verifier::required(&mut values, "--verifier-identity")?),
        expected_verifier_sha256,
        expected_verifier_bytes,
    })
}
fn summary(options: &PairOptions, receipt: &PairReceipt) -> Result<(), DevError> {
    let identity = receipt_identity(&options.evidence_root.join("receipt.json"))?;
    println!(
        "{}",
        serde_json::json!({"status": if receipt.status == Status::FreshPassed {"passed"} else {"failed"}, "scope":receipt.scope, "tag":receipt.tag,"source_commit":receipt.source_commit,"receipt":identity,"suite":receipt.suite,"cleanup_complete":receipt.cleanup_complete,"failure":receipt.failure,"elapsed_nanoseconds":receipt.elapsed_nanoseconds})
    );
    Ok(())
}

#[cfg(test)]
pub(super) mod tests;
