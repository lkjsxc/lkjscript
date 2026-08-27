mod archive;
mod model;

use crate::error::DevError;
use crate::process::{self, ProcessObservation, ProcessSpec, ProcessStatus};
use archive::{ARCHIVE_NAME, CHECKSUM_NAME, RECEIPT_NAME};
use model::{
    ArtifactIdentity, ElfIdentity, EvidenceClassification, ExecutableIdentity, ExternalEvidence,
    HostedContext, MANIFEST_SCHEMA, MANIFEST_SCHEMA_VERSION, NoticeIdentity, PackageIdentity,
    PackagingIdentity, PayloadIdentity, PublicationMode, RECEIPT_SCHEMA, RECEIPT_SCHEMA_VERSION,
    ReleaseManifest, ReleaseReceipt, SchemaIdentity, Sha256Digest, SourceIdentity,
    ToolchainIdentity, VerificationClassification,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const REPOSITORY_IDENTITY: &str = "lkjsxc/lkjscript";
const PACKAGE_NAME: &str = "lkjscript";
const TARGET_TRIPLE: &str = "x86_64-unknown-linux-gnu";
const TOOLCHAIN_CHANNEL: &str = "1.98.0";
const CLI_CONTRACT: u16 = 10;
const CARGO_ABOUT_VERSION: &str = "0.9.2";
const CARGO_ABOUT_ARCHIVE_SHA256: &str =
    "9099a59e820c38a68b9d65f300662a567d56562f9a10f6aa4c7e86c17c2566af";
const CARGO_ABOUT_EXECUTABLE_SHA256: &str =
    "b06bd6a8bfd726cffb90e3e0588e3e0b1cfbb582bf6a34f4c1c2692ba8f2e7b8";
const EXPECTED_CLASSIFICATIONS: [&str; 10] = [
    "source_identity",
    "toolchain",
    "cargo_about",
    "notice_generation",
    "candidate_capabilities",
    "candidate_lifecycle",
    "full_verification",
    "deterministic_packaging",
    "archive_verification",
    "checksum_integrity",
];

#[derive(Debug)]
struct PrepareOptions {
    candidate: PathBuf,
    cargo_about: PathBuf,
    cargo_about_archive: PathBuf,
    output: PathBuf,
    tag: String,
    publication_mode: PublicationMode,
    full_verification_receipt: Option<PathBuf>,
    require_full_verification: bool,
}

#[derive(Debug)]
struct VerifyOptions {
    archive: PathBuf,
    checksums: PathBuf,
    candidate: Option<PathBuf>,
    receipt: Option<PathBuf>,
    expected_tag: Option<String>,
    expected_publication_mode: Option<PublicationMode>,
}

#[derive(Debug)]
struct SourceFacts {
    package_version: String,
    tag: String,
    commit_sha: String,
    commit_timestamp_unix_seconds: u64,
    tag_object_sha: Option<String>,
}

#[derive(Debug)]
struct CapabilitiesFacts {
    cli_contract: u16,
    registry_digest: String,
}

#[derive(Debug)]
struct FullVerificationFacts {
    evidence: ExternalEvidence,
    selected_gates: usize,
}

pub(crate) fn command(mut arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let subcommand = crate::next_utf8(&mut arguments, "release subcommand")?
        .ok_or_else(|| DevError::usage("release subcommand is required"))?;
    match subcommand.as_str() {
        "prepare" => prepare(parse_prepare(arguments)?),
        "verify" => verify(parse_verify(arguments)?),
        value => Err(DevError::usage(format!(
            "unknown release subcommand '{value}'"
        ))),
    }
}

fn prepare(options: PrepareOptions) -> Result<u8, DevError> {
    let started = Instant::now();
    let started_unix_nanoseconds = unix_nanoseconds()?;
    let repository = repository_root()?;
    require_absolute_regular_executable(&options.candidate, "release candidate")?;
    require_absolute_regular_executable(&options.cargo_about, "cargo-about executable")?;
    require_absolute_regular(&options.cargo_about_archive, "cargo-about archive")?;
    require_absolute_output(&options.output)?;
    if let Some(path) = &options.full_verification_receipt {
        require_absolute_regular(path, "full verification receipt")?;
    }
    ensure_clean_checkout(&repository)?;
    let source = source_facts(&repository, &options.tag, options.publication_mode)?;
    let toolchain = toolchain_facts(&repository)?;
    let tar_version = command_version("tar", &["--version"], &repository)?;
    let gzip_version = command_version("gzip", &["--version"], &repository)?;
    let readelf_version = command_version("readelf", &["--version"], &repository)?;
    let _sha256sum_version = command_version("sha256sum", &["--version"], &repository)?;
    let (cargo_lock_sha256, _) = archive::sha256_file(&repository.join("Cargo.lock"))?;
    let (license_sha256, license_bytes) = archive::sha256_file(&repository.join("LICENSE"))?;
    let candidate_metadata = archive::ensure_regular(&options.candidate, "release candidate")?;
    if candidate_metadata.permissions().mode() & 0o111 == 0 {
        return Err(DevError::infrastructure(
            "release candidate does not have an executable mode",
        ));
    }
    let (candidate_sha256, candidate_bytes) = archive::sha256_file(&options.candidate)?;
    let elf = inspect_elf(&options.candidate, &repository, readelf_version)?;
    let capabilities = inspect_capabilities(&options.candidate, &repository)?;
    if capabilities.cli_contract != CLI_CONTRACT {
        return Err(DevError::corrupt(format!(
            "release candidate CLI contract is {}, expected {CLI_CONTRACT}",
            capabilities.cli_contract
        )));
    }
    validate_registry_digest(&capabilities.registry_digest)?;

    let (about_archive_digest, about_archive_bytes) =
        archive::sha256_file(&options.cargo_about_archive)?;
    if about_archive_digest.as_str() != CARGO_ABOUT_ARCHIVE_SHA256 {
        return Err(DevError::corrupt(format!(
            "cargo-about archive digest mismatch: observed {} for {about_archive_bytes} bytes",
            about_archive_digest.as_str()
        )));
    }
    let (about_executable_digest, _) = archive::sha256_file(&options.cargo_about)?;
    if about_executable_digest.as_str() != CARGO_ABOUT_EXECUTABLE_SHA256 {
        return Err(DevError::corrupt(
            "cargo-about executable does not match the pinned verified archive",
        ));
    }
    let about_version = command_version(
        options
            .cargo_about
            .to_str()
            .ok_or_else(|| DevError::usage("cargo-about path must be portable UTF-8"))?,
        &["--version"],
        &repository,
    )?;
    if about_version != format!("cargo-about {CARGO_ABOUT_VERSION}") {
        return Err(DevError::corrupt(format!(
            "cargo-about version mismatch: observed '{about_version}'"
        )));
    }

    let full_verification = options
        .full_verification_receipt
        .as_deref()
        .map(|path| inspect_full_verification(path, &source.commit_sha))
        .transpose()?;
    if options.require_full_verification && full_verification.is_none() {
        return Err(DevError::usage(
            "--require-full-verification requires --full-verification-receipt",
        ));
    }

    let parent = options
        .output
        .parent()
        .ok_or_else(|| DevError::usage("release output must have an existing parent directory"))?;
    archive::ensure_directory(parent, "release output parent")?;
    let work = tempfile::Builder::new()
        .prefix(".lkjscript-release-work-")
        .tempdir_in(parent)
        .map_err(|error| {
            DevError::infrastructure(format!("create release work directory: {error}"))
        })?;
    let notice_one = work.path().join("THIRD-PARTY-LICENSES-1.html");
    let notice_two = work.path().join("THIRD-PARTY-LICENSES-2.html");
    generate_notices(
        &repository,
        &options.cargo_about,
        &notice_one,
        work.path(),
        "notice-one",
    )?;
    generate_notices(
        &repository,
        &options.cargo_about,
        &notice_two,
        work.path(),
        "notice-two",
    )?;
    require_equal_files(&notice_one, &notice_two, "third-party notice generation")?;
    let (notice_sha256, notice_bytes) = archive::sha256_file(&notice_one)?;
    if notice_bytes == 0 {
        return Err(DevError::corrupt("third-party notice output is empty"));
    }
    audit_notice(&notice_one)?;

    let candidate_lifecycle =
        run_candidate_lifecycle(&repository, &options.candidate, work.path())?;
    let candidate_after = archive::ensure_regular(&options.candidate, "release candidate")?;
    let (candidate_sha256_after, candidate_bytes_after) = archive::sha256_file(&options.candidate)?;
    if candidate_sha256 != candidate_sha256_after
        || candidate_bytes != candidate_bytes_after
        || candidate_after.permissions().mode() & 0o111 == 0
    {
        return Err(DevError::infrastructure(
            "release candidate changed during exact-candidate acceptance",
        ));
    }

    let manifest = ReleaseManifest {
        schema: SchemaIdentity {
            identity: MANIFEST_SCHEMA.to_owned(),
            version: MANIFEST_SCHEMA_VERSION,
        },
        publication_mode: options.publication_mode,
        package: PackageIdentity {
            name: PACKAGE_NAME.to_owned(),
            version: source.package_version.clone(),
        },
        source: SourceIdentity {
            repository: REPOSITORY_IDENTITY.to_owned(),
            expected_release_tag: source.tag.clone(),
            tagged_commit_sha: source.commit_sha.clone(),
            commit_timestamp_unix_seconds: source.commit_timestamp_unix_seconds,
            annotated_tag_object_sha: source.tag_object_sha.clone(),
        },
        target_triple: TARGET_TRIPLE.to_owned(),
        toolchain,
        cargo_lock_sha256,
        executable: ExecutableIdentity {
            archive_mode: 0o755,
            byte_length: candidate_bytes,
            sha256: candidate_sha256,
            elf,
            cli_contract: capabilities.cli_contract,
            executable_registry_digest: capabilities.registry_digest,
        },
        root_license: PayloadIdentity {
            archive_mode: 0o644,
            byte_length: license_bytes,
            sha256: license_sha256,
        },
        third_party_notices: NoticeIdentity {
            generator: "cargo-about".to_owned(),
            generator_version: CARGO_ABOUT_VERSION.to_owned(),
            downloaded_archive_sha256: about_archive_digest,
            executable_sha256: about_executable_digest,
            invocation: normalized_notice_invocation(),
            archive_mode: 0o644,
            byte_length: notice_bytes,
            sha256: notice_sha256,
        },
        packaging: PackagingIdentity {
            tar_format: "posix-ustar".to_owned(),
            tar_version,
            tar_invocation: archive::normalized_tar_invocation(
                source.commit_timestamp_unix_seconds,
            ),
            gzip_version,
            gzip_level: 9,
            gzip_name_header: false,
            gzip_time_header: false,
            gzip_invocation: archive::normalized_gzip_invocation(),
            numeric_owner: 0,
            numeric_group: 0,
            source_timestamp_unix_seconds: source.commit_timestamp_unix_seconds,
            members: archive::manifest_members(),
        },
    };
    validate_manifest(&manifest)?;
    let manifest_bytes = archive::canonical_json(&manifest)?;
    let payload_parent = work.path().join("payload");
    fs::create_dir(&payload_parent)
        .map_err(|error| DevError::infrastructure(format!("create payload parent: {error}")))?;
    archive::stage_payload(
        &payload_parent.join("lkjscript"),
        &options.candidate,
        &repository.join("LICENSE"),
        &notice_one,
        &manifest_bytes,
    )?;
    let package_one = work.path().join("package-one");
    let package_two = work.path().join("package-two");
    fs::create_dir(&package_one)
        .and_then(|()| fs::create_dir(&package_two))
        .map_err(|error| {
            DevError::infrastructure(format!("create packaging directories: {error}"))
        })?;
    let archive_one = archive::create_archive(
        &payload_parent,
        &package_one.join("lkjscript-x86_64-unknown-linux-gnu.tar"),
        source.commit_timestamp_unix_seconds,
    )?;
    let archive_two = archive::create_archive(
        &payload_parent,
        &package_two.join("lkjscript-x86_64-unknown-linux-gnu.tar"),
        source.commit_timestamp_unix_seconds,
    )?;
    require_equal_files(
        &archive_one,
        &archive_two,
        "deterministic release packaging",
    )?;
    let verify_one = work.path().join("verify-one");
    let verify_two = work.path().join("verify-two");
    fs::create_dir(&verify_one)
        .and_then(|()| fs::create_dir(&verify_two))
        .map_err(|error| {
            DevError::infrastructure(format!("create archive verification directories: {error}"))
        })?;
    let verified_one =
        archive::verify_archive(&archive_one, &verify_one, Some(&options.candidate))?;
    let verified_two =
        archive::verify_archive(&archive_two, &verify_two, Some(&options.candidate))?;
    if verified_one.archive_sha256 != verified_two.archive_sha256
        || verified_one.archive_byte_length != verified_two.archive_byte_length
        || verified_one.manifest_sha256 != verified_two.manifest_sha256
    {
        return Err(DevError::corrupt(
            "repeated release preparations produced different identities",
        ));
    }
    validate_manifest(&verified_one.manifest)?;
    let checksum_bytes = checksum_bytes(&verified_one.archive_sha256);
    verify_checksum_bytes(&checksum_bytes, &verified_one.archive_sha256)?;
    let checksum_sha256 = archive::sha256_bytes(&checksum_bytes)?;
    let completed_unix_nanoseconds = unix_nanoseconds()?;
    let classifications = classifications(&full_verification, verified_one.members.len());
    let receipt = ReleaseReceipt {
        schema: SchemaIdentity {
            identity: RECEIPT_SCHEMA.to_owned(),
            version: RECEIPT_SCHEMA_VERSION,
        },
        publication_mode: options.publication_mode,
        release_tag: source.tag.clone(),
        commit_sha: source.commit_sha.clone(),
        started_unix_nanoseconds,
        completed_unix_nanoseconds,
        elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
        hosted_context: hosted_context(),
        manifest_sha256: verified_one.manifest_sha256.clone(),
        archive: ArtifactIdentity {
            name: ARCHIVE_NAME.to_owned(),
            byte_length: verified_one.archive_byte_length,
            sha256: verified_one.archive_sha256.clone(),
        },
        checksum_file: ArtifactIdentity {
            name: CHECKSUM_NAME.to_owned(),
            byte_length: checksum_bytes.len() as u64,
            sha256: checksum_sha256,
        },
        full_verification_receipt: full_verification.map(|facts| facts.evidence),
        candidate_lifecycle,
        classifications,
    };
    validate_receipt(&receipt, &verified_one)?;
    let receipt_bytes = archive::canonical_json(&receipt)?;

    let output_stage = tempfile::Builder::new()
        .prefix(".lkjscript-release-output-")
        .tempdir_in(parent)
        .map_err(|error| {
            DevError::infrastructure(format!("create release output stage: {error}"))
        })?;
    archive::copy_new(&archive_one, &output_stage.path().join(ARCHIVE_NAME), 0o644)?;
    archive::write_new(
        &output_stage.path().join(CHECKSUM_NAME),
        &checksum_bytes,
        0o644,
    )?;
    archive::write_new(
        &output_stage.path().join(RECEIPT_NAME),
        &receipt_bytes,
        0o644,
    )?;
    fs::set_permissions(output_stage.path(), fs::Permissions::from_mode(0o755)).map_err(
        |error| DevError::infrastructure(format!("set release output directory mode: {error}")),
    )?;
    archive::synchronize_directory(output_stage.path())?;
    publish_directory_no_replace(output_stage.path(), &options.output)?;
    archive::synchronize_directory(parent)?;

    #[derive(Serialize)]
    struct Summary<'a> {
        status: &'static str,
        publication_mode: &'static str,
        tag: &'a str,
        commit_sha: &'a str,
        output: String,
        archive: &'static str,
        archive_bytes: u64,
        archive_sha256: &'a str,
        checksums: &'static str,
        receipt: &'static str,
        receipt_sha256: String,
    }
    let receipt_sha256 = archive::sha256_file(&options.output.join(RECEIPT_NAME))?.0;
    let summary = Summary {
        status: "passed",
        publication_mode: options.publication_mode.as_str(),
        tag: &source.tag,
        commit_sha: &source.commit_sha,
        output: options.output.to_string_lossy().into_owned(),
        archive: ARCHIVE_NAME,
        archive_bytes: verified_one.archive_byte_length,
        archive_sha256: verified_one.archive_sha256.as_str(),
        checksums: CHECKSUM_NAME,
        receipt: RECEIPT_NAME,
        receipt_sha256: receipt_sha256.as_str().to_owned(),
    };
    println!(
        "{}",
        serde_json::to_string(&summary).map_err(|error| DevError::infrastructure(format!(
            "encode release summary: {error}"
        )))?
    );
    Ok(0)
}

fn verify(options: VerifyOptions) -> Result<u8, DevError> {
    require_absolute_regular(&options.archive, "release archive")?;
    require_absolute_regular(&options.checksums, "release checksum file")?;
    if let Some(candidate) = &options.candidate {
        require_absolute_regular_executable(candidate, "release candidate")?;
    }
    if let Some(receipt) = &options.receipt {
        require_absolute_regular(receipt, "release receipt")?;
    }
    let parent = options
        .archive
        .parent()
        .ok_or_else(|| DevError::usage("release archive path must have a parent directory"))?;
    let work = tempfile::Builder::new()
        .prefix(".lkjscript-release-verify-")
        .tempdir_in(parent)
        .map_err(|error| {
            DevError::infrastructure(format!("create release verifier work: {error}"))
        })?;
    let verified =
        archive::verify_archive(&options.archive, work.path(), options.candidate.as_deref())?;
    validate_manifest(&verified.manifest)?;
    if let Some(tag) = &options.expected_tag
        && verified.manifest.source.expected_release_tag != *tag
    {
        return Err(DevError::corrupt(format!(
            "release tag mismatch: expected '{tag}'"
        )));
    }
    if let Some(mode) = options.expected_publication_mode
        && verified.manifest.publication_mode != mode
    {
        return Err(DevError::corrupt(format!(
            "publication mode mismatch: expected '{}'",
            mode.as_str()
        )));
    }
    let checksum = fs::read(&options.checksums).map_err(|error| {
        DevError::infrastructure(format!("read release checksum file: {error}"))
    })?;
    verify_checksum_bytes(&checksum, &verified.archive_sha256)?;
    if let Some(path) = &options.receipt {
        let bytes = fs::read(path)
            .map_err(|error| DevError::infrastructure(format!("read release receipt: {error}")))?;
        let receipt: ReleaseReceipt = serde_json::from_slice(&bytes)
            .map_err(|error| DevError::corrupt(format!("decode release receipt: {error}")))?;
        if archive::canonical_json(&receipt)? != bytes {
            return Err(DevError::corrupt(
                "release receipt is not in canonical first-party encoding",
            ));
        }
        validate_receipt(&receipt, &verified)?;
        let checksum_digest = archive::sha256_bytes(&checksum)?;
        if receipt.checksum_file.sha256 != checksum_digest
            || receipt.checksum_file.byte_length != checksum.len() as u64
        {
            return Err(DevError::corrupt(
                "release receipt checksum-file binding mismatch",
            ));
        }
    }
    #[derive(Serialize)]
    struct Summary<'a> {
        status: &'static str,
        tag: &'a str,
        commit_sha: &'a str,
        publication_mode: &'static str,
        archive_bytes: u64,
        archive_sha256: &'a str,
        manifest_sha256: &'a str,
        source_timestamp_unix_seconds: u64,
        members: usize,
    }
    let summary = Summary {
        status: "passed",
        tag: &verified.manifest.source.expected_release_tag,
        commit_sha: &verified.manifest.source.tagged_commit_sha,
        publication_mode: verified.manifest.publication_mode.as_str(),
        archive_bytes: verified.archive_byte_length,
        archive_sha256: verified.archive_sha256.as_str(),
        manifest_sha256: verified.manifest_sha256.as_str(),
        source_timestamp_unix_seconds: verified.source_timestamp_unix_seconds,
        members: verified.members.len(),
    };
    println!(
        "{}",
        serde_json::to_string(&summary)
            .map_err(|error| DevError::infrastructure(format!("encode verify summary: {error}")))?
    );
    Ok(0)
}

fn parse_prepare(
    mut arguments: impl Iterator<Item = OsString>,
) -> Result<PrepareOptions, DevError> {
    let mut values = BTreeMap::new();
    let mut require_full_verification = false;
    while let Some(argument) = crate::next_utf8(&mut arguments, "release prepare option")? {
        if argument == "--require-full-verification" {
            if require_full_verification {
                return Err(DevError::usage(
                    "duplicate --require-full-verification option",
                ));
            }
            require_full_verification = true;
            continue;
        }
        let name = match argument.as_str() {
            "--candidate"
            | "--cargo-about"
            | "--cargo-about-archive"
            | "--output"
            | "--tag"
            | "--publication"
            | "--full-verification-receipt" => argument,
            value => {
                return Err(DevError::usage(format!(
                    "unknown release prepare option '{value}'"
                )));
            }
        };
        let value = crate::next_utf8(&mut arguments, &format!("value for {name}"))?
            .ok_or_else(|| DevError::usage(format!("{name} requires a value")))?;
        if values.insert(name.clone(), value).is_some() {
            return Err(DevError::usage(format!("duplicate option '{name}'")));
        }
    }
    let publication_mode = parse_publication(required(&mut values, "--publication")?)?;
    let full_verification_receipt = values
        .remove("--full-verification-receipt")
        .map(PathBuf::from);
    let options = PrepareOptions {
        candidate: PathBuf::from(required(&mut values, "--candidate")?),
        cargo_about: PathBuf::from(required(&mut values, "--cargo-about")?),
        cargo_about_archive: PathBuf::from(required(&mut values, "--cargo-about-archive")?),
        output: PathBuf::from(required(&mut values, "--output")?),
        tag: required(&mut values, "--tag")?,
        publication_mode,
        full_verification_receipt,
        require_full_verification,
    };
    if !values.is_empty() {
        return Err(DevError::usage("unconsumed release prepare options"));
    }
    Ok(options)
}

fn parse_verify(mut arguments: impl Iterator<Item = OsString>) -> Result<VerifyOptions, DevError> {
    let mut values = BTreeMap::new();
    while let Some(argument) = crate::next_utf8(&mut arguments, "release verify option")? {
        let name = match argument.as_str() {
            "--archive"
            | "--checksums"
            | "--candidate"
            | "--receipt"
            | "--expected-tag"
            | "--expected-publication" => argument,
            value => {
                return Err(DevError::usage(format!(
                    "unknown release verify option '{value}'"
                )));
            }
        };
        let value = crate::next_utf8(&mut arguments, &format!("value for {name}"))?
            .ok_or_else(|| DevError::usage(format!("{name} requires a value")))?;
        if values.insert(name.clone(), value).is_some() {
            return Err(DevError::usage(format!("duplicate option '{name}'")));
        }
    }
    let candidate = values.remove("--candidate").map(PathBuf::from);
    let receipt = values.remove("--receipt").map(PathBuf::from);
    let expected_tag = values.remove("--expected-tag");
    let expected_publication_mode = values
        .remove("--expected-publication")
        .map(parse_publication)
        .transpose()?;
    let options = VerifyOptions {
        archive: PathBuf::from(required(&mut values, "--archive")?),
        checksums: PathBuf::from(required(&mut values, "--checksums")?),
        candidate,
        receipt,
        expected_tag,
        expected_publication_mode,
    };
    if !values.is_empty() {
        return Err(DevError::usage("unconsumed release verify options"));
    }
    Ok(options)
}

fn required(values: &mut BTreeMap<String, String>, name: &str) -> Result<String, DevError> {
    values
        .remove(name)
        .ok_or_else(|| DevError::usage(format!("required option '{name}' is missing")))
}

fn parse_publication(value: String) -> Result<PublicationMode, DevError> {
    match value.as_str() {
        "dry-run" => Ok(PublicationMode::DryRun),
        "release" => Ok(PublicationMode::Release),
        _ => Err(DevError::usage(format!(
            "publication mode must be 'dry-run' or 'release', observed '{value}'"
        ))),
    }
}

fn repository_root() -> Result<PathBuf, DevError> {
    let current = env::current_dir()
        .map_err(|error| DevError::infrastructure(format!("read current directory: {error}")))?;
    archive::ensure_regular(&current.join("Cargo.toml"), "root Cargo.toml")?;
    archive::ensure_directory(&current.join(".git"), "Git metadata directory")?;
    Ok(current)
}

fn require_absolute_regular(path: &Path, label: &str) -> Result<(), DevError> {
    if !path.is_absolute() {
        return Err(DevError::usage(format!(
            "{label} path '{}' must be absolute",
            path.display()
        )));
    }
    archive::ensure_regular(path, label)?;
    Ok(())
}

fn require_absolute_regular_executable(path: &Path, label: &str) -> Result<(), DevError> {
    require_absolute_regular(path, label)?;
    let metadata = archive::ensure_regular(path, label)?;
    if metadata.permissions().mode() & 0o111 == 0 {
        return Err(DevError::usage(format!(
            "{label} '{}' must be executable",
            path.display()
        )));
    }
    Ok(())
}

fn require_absolute_output(path: &Path) -> Result<(), DevError> {
    if !path.is_absolute() {
        return Err(DevError::usage(format!(
            "release output '{}' must be absolute",
            path.display()
        )));
    }
    archive::reject_existing(path, "release output")
}

fn ensure_clean_checkout(repository: &Path) -> Result<(), DevError> {
    let status = command_text(
        "git",
        &["status", "--porcelain=v1", "--untracked-files=all"],
        repository,
        4 * 1024 * 1024,
    )?;
    if !status.is_empty() {
        return Err(DevError::infrastructure(
            "release preparation requires a clean checkout",
        ));
    }
    Ok(())
}

fn source_facts(
    repository: &Path,
    tag: &str,
    publication_mode: PublicationMode,
) -> Result<SourceFacts, DevError> {
    let (package_name, package_version) = package_identity(&repository.join("Cargo.toml"))?;
    if package_name != PACKAGE_NAME {
        return Err(DevError::corrupt(format!(
            "root package name is '{package_name}', expected '{PACKAGE_NAME}'"
        )));
    }
    validate_strict_tag(tag, &package_version)?;
    let commit_sha = command_text("git", &["rev-parse", "HEAD"], repository, 1024)?;
    validate_git_sha(&commit_sha, "HEAD commit")?;
    let timestamp = command_text(
        "git",
        &["show", "-s", "--format=%ct", "HEAD"],
        repository,
        1024,
    )?;
    let commit_timestamp_unix_seconds = timestamp
        .parse::<u64>()
        .map_err(|_| DevError::corrupt(format!("Git commit timestamp '{timestamp}' is invalid")))?;
    let origin = command_text(
        "git",
        &["config", "--get", "remote.origin.url"],
        repository,
        4096,
    )?;
    if !origin_matches_repository(&origin) {
        return Err(DevError::corrupt(format!(
            "origin '{origin}' does not identify {REPOSITORY_IDENTITY}"
        )));
    }
    match publication_mode {
        PublicationMode::DryRun => run_git_success(
            repository,
            &["merge-base", "--is-ancestor", "origin/main", "HEAD"],
            "dry-run commit must be a fast-forward descendant of origin/main",
        )?,
        PublicationMode::Release => run_git_success(
            repository,
            &["merge-base", "--is-ancestor", "HEAD", "origin/main"],
            "release commit must be reachable from origin/main",
        )?,
    }
    let tag_object_sha = match publication_mode {
        PublicationMode::DryRun => None,
        PublicationMode::Release => {
            let reference = format!("refs/tags/{tag}");
            let kind = command_text("git", &["cat-file", "-t", &reference], repository, 1024)?;
            if kind != "tag" {
                return Err(DevError::corrupt(format!(
                    "release tag '{tag}' is not annotated"
                )));
            }
            let tagged_commit = command_text(
                "git",
                &["rev-parse", &format!("{reference}^{{commit}}")],
                repository,
                1024,
            )?;
            if tagged_commit != commit_sha {
                return Err(DevError::corrupt(format!(
                    "release tag '{tag}' does not target current HEAD"
                )));
            }
            let object = command_text("git", &["rev-parse", &reference], repository, 1024)?;
            validate_git_sha(&object, "annotated tag object")?;
            Some(object)
        }
    };
    Ok(SourceFacts {
        package_version,
        tag: tag.to_owned(),
        commit_sha,
        commit_timestamp_unix_seconds,
        tag_object_sha,
    })
}

fn package_identity(path: &Path) -> Result<(String, String), DevError> {
    let text = fs::read_to_string(path)
        .map_err(|error| DevError::infrastructure(format!("read root Cargo.toml: {error}")))?;
    let mut in_package = false;
    let mut name = None;
    let mut version = None;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        if name.is_none() {
            name = quoted_assignment(line, "name")?;
        }
        if version.is_none() {
            version = quoted_assignment(line, "version")?;
        }
    }
    Ok((
        name.ok_or_else(|| DevError::corrupt("root package name is missing"))?,
        version.ok_or_else(|| DevError::corrupt("root package version is missing"))?,
    ))
}

fn quoted_assignment(line: &str, key: &str) -> Result<Option<String>, DevError> {
    let Some(rest) = line.strip_prefix(key) else {
        return Ok(None);
    };
    let rest = rest.trim_start();
    let Some(value) = rest.strip_prefix('=') else {
        return Ok(None);
    };
    let value = value.trim();
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return Err(DevError::corrupt(format!(
            "root Cargo.toml {key} must be a simple quoted string"
        )));
    }
    Ok(Some(value[1..value.len() - 1].to_owned()))
}

fn validate_strict_tag(tag: &str, package_version: &str) -> Result<(), DevError> {
    let expected = format!("v{package_version}");
    if tag != expected {
        return Err(DevError::corrupt(format!(
            "release tag '{tag}' does not equal package version tag '{expected}'"
        )));
    }
    let version = tag
        .strip_prefix('v')
        .ok_or_else(|| DevError::corrupt("release tag must start with 'v'"))?;
    let parts = version.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts.iter().any(|part| {
            part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
        })
    {
        return Err(DevError::corrupt(format!(
            "release tag '{tag}' is not strict vMAJOR.MINOR.PATCH"
        )));
    }
    Ok(())
}

fn toolchain_facts(repository: &Path) -> Result<ToolchainIdentity, DevError> {
    let toolchain = fs::read_to_string(repository.join("rust-toolchain.toml"))
        .map_err(|error| DevError::infrastructure(format!("read rust-toolchain.toml: {error}")))?;
    if !toolchain
        .lines()
        .any(|line| line.trim() == format!("channel = \"{TOOLCHAIN_CHANNEL}\""))
        || !toolchain
            .lines()
            .any(|line| line.contains("x86_64-unknown-linux-gnu"))
    {
        return Err(DevError::corrupt(
            "rust-toolchain.toml does not pin the selected toolchain and target",
        ));
    }
    let rustc = command_text("rustc", &["-Vv"], repository, 16 * 1024)?;
    let cargo = command_text("cargo", &["-V"], repository, 4096)?;
    if !rustc.starts_with(&format!("rustc {TOOLCHAIN_CHANNEL}"))
        || !cargo.starts_with(&format!("cargo {TOOLCHAIN_CHANNEL}"))
    {
        return Err(DevError::corrupt(format!(
            "active Rust toolchain disagrees with {TOOLCHAIN_CHANNEL}"
        )));
    }
    Ok(ToolchainIdentity {
        rustc,
        cargo,
        toolchain_channel: TOOLCHAIN_CHANNEL.to_owned(),
    })
}

fn inspect_elf(
    candidate: &Path,
    repository: &Path,
    readelf_version: String,
) -> Result<ElfIdentity, DevError> {
    let bytes = fs::read(candidate).map_err(|error| {
        DevError::infrastructure(format!("read release candidate ELF header: {error}"))
    })?;
    if bytes.len() < 64
        || bytes[0..4] != [0x7f, b'E', b'L', b'F']
        || bytes[4] != 2
        || bytes[5] != 1
        || u16::from_le_bytes([bytes[18], bytes[19]]) != 62
    {
        return Err(DevError::corrupt(
            "release candidate is not a little-endian ELF64 x86-64 executable",
        ));
    }
    let candidate_text = candidate
        .to_str()
        .ok_or_else(|| DevError::usage("candidate path must be portable UTF-8"))?;
    let output = command_text(
        "readelf",
        &[
            "--wide",
            "--file-header",
            "--program-headers",
            "--dynamic",
            "--version-info",
            candidate_text,
        ],
        repository,
        8 * 1024 * 1024,
    )?;
    let class = labeled_value(&output, "Class:")?;
    let machine = labeled_value(&output, "Machine:")?;
    if class != "ELF64" || machine != "Advanced Micro Devices X86-64" {
        return Err(DevError::corrupt(
            "readelf target identity does not match x86-64 ELF64",
        ));
    }
    let interpreter = output
        .lines()
        .find_map(|line| {
            line.split_once("[Requesting program interpreter: ")
                .and_then(|(_, rest)| rest.strip_suffix(']'))
                .map(str::to_owned)
        })
        .ok_or_else(|| DevError::corrupt("ELF interpreter is missing"))?;
    let required_shared_libraries = output
        .lines()
        .filter(|line| line.contains("(NEEDED)"))
        .map(|line| bracketed_value(line, "shared library"))
        .collect::<Result<BTreeSet<_>, _>>()?
        .into_iter()
        .collect::<Vec<_>>();
    if required_shared_libraries.is_empty() {
        return Err(DevError::corrupt(
            "GNU release candidate has no recorded shared libraries",
        ));
    }
    let highest_required_glibc_symbol_version = highest_glibc_version(&output)?;
    Ok(ElfIdentity {
        class,
        machine,
        interpreter,
        required_shared_libraries,
        highest_required_glibc_symbol_version,
        readelf_version,
    })
}

fn inspect_capabilities(
    candidate: &Path,
    repository: &Path,
) -> Result<CapabilitiesFacts, DevError> {
    let candidate = candidate
        .to_str()
        .ok_or_else(|| DevError::usage("candidate path must be portable UTF-8"))?;
    let output = command_text(candidate, &["capabilities"], repository, 1024 * 1024)?;
    let line = output
        .lines()
        .find(|line| line.starts_with("registry "))
        .ok_or_else(|| DevError::corrupt("candidate capabilities omitted registry record"))?;
    let fields = line
        .split_whitespace()
        .skip(1)
        .filter_map(|field| field.split_once('='))
        .collect::<BTreeMap<_, _>>();
    let cli_contract = fields
        .get("cli")
        .ok_or_else(|| DevError::corrupt("candidate capabilities omitted CLI contract"))?
        .parse::<u16>()
        .map_err(|_| DevError::corrupt("candidate CLI contract is malformed"))?;
    let registry_digest = fields
        .get("digest")
        .ok_or_else(|| DevError::corrupt("candidate capabilities omitted registry digest"))?
        .to_string();
    Ok(CapabilitiesFacts {
        cli_contract,
        registry_digest,
    })
}

fn generate_notices(
    repository: &Path,
    cargo_about: &Path,
    output: &Path,
    work: &Path,
    label: &str,
) -> Result<(), DevError> {
    archive::reject_existing(output, "third-party notice output")?;
    let stdout = work.join(format!("{label}.stdout"));
    let stderr = work.join(format!("{label}.stderr"));
    let observation = process::run(
        &ProcessSpec {
            command: vec![
                cargo_about.to_string_lossy().into_owned(),
                "generate".to_owned(),
                repository.join("about.hbs").to_string_lossy().into_owned(),
                "--config".to_owned(),
                repository.join("about.toml").to_string_lossy().into_owned(),
                "--manifest-path".to_owned(),
                repository.join("Cargo.toml").to_string_lossy().into_owned(),
                "--target".to_owned(),
                TARGET_TRIPLE.to_owned(),
                "--locked".to_owned(),
                "--offline".to_owned(),
                "--fail".to_owned(),
                "--output-file".to_owned(),
                output.to_string_lossy().into_owned(),
            ],
            cwd: repository.to_path_buf(),
            environment: process::environment(),
            timeout: Duration::from_secs(300),
            maximum_stdout_bytes: 1024 * 1024,
            maximum_stderr_bytes: 4 * 1024 * 1024,
            stdout_path: stdout,
            stderr_path: stderr,
            unavailable_exit_code: None,
        },
        repository,
    );
    if observation.status != ProcessStatus::Passed {
        return Err(DevError::infrastructure(format!(
            "cargo-about notice generation failed with {:?}: {}",
            observation.status,
            observation.reason.as_deref().unwrap_or("no reason")
        )));
    }
    archive::ensure_regular(output, "third-party notice output")?;
    Ok(())
}

fn normalized_notice_invocation() -> Vec<String> {
    vec![
        "cargo-about".to_owned(),
        "generate".to_owned(),
        "about.hbs".to_owned(),
        "--config".to_owned(),
        "about.toml".to_owned(),
        "--manifest-path".to_owned(),
        "Cargo.toml".to_owned(),
        "--target".to_owned(),
        TARGET_TRIPLE.to_owned(),
        "--locked".to_owned(),
        "--offline".to_owned(),
        "--fail".to_owned(),
        "--output-file".to_owned(),
        "$NOTICE".to_owned(),
    ]
}

fn audit_notice(path: &Path) -> Result<(), DevError> {
    let text = fs::read_to_string(path).map_err(|error| {
        DevError::infrastructure(format!("read generated third-party notice: {error}"))
    })?;
    for required in [
        "lkjscript third-party licenses",
        "argon2 0.5.3",
        "axum 0.8.9",
        "object_store 0.14.1",
        "postgres 0.19.14",
        "tokio 1.53.1",
    ] {
        if !text.contains(required) {
            return Err(DevError::corrupt(format!(
                "third-party notice omitted required production record '{required}'"
            )));
        }
    }
    Ok(())
}

fn run_candidate_lifecycle(
    repository: &Path,
    candidate: &Path,
    work: &Path,
) -> Result<ProcessObservation, DevError> {
    let mut environment = process::environment();
    environment.insert(
        "LKJSCRIPT_RELEASE_CANDIDATE".to_owned(),
        candidate.to_string_lossy().into_owned(),
    );
    let observation = process::run(
        &ProcessSpec {
            command: vec![
                "cargo".to_owned(),
                "test".to_owned(),
                "--locked".to_owned(),
                "--release".to_owned(),
                "--test".to_owned(),
                "public_cli".to_owned(),
                "copied_binary_completes_normalized_standard_dependent_command_lifecycle"
                    .to_owned(),
                "--".to_owned(),
                "--exact".to_owned(),
            ],
            cwd: repository.to_path_buf(),
            environment,
            timeout: Duration::from_secs(900),
            maximum_stdout_bytes: 16 * 1024 * 1024,
            maximum_stderr_bytes: 16 * 1024 * 1024,
            stdout_path: work.join("candidate-lifecycle.stdout"),
            stderr_path: work.join("candidate-lifecycle.stderr"),
            unavailable_exit_code: None,
        },
        repository,
    );
    if observation.status != ProcessStatus::Passed {
        return Err(DevError::infrastructure(format!(
            "exact-candidate copied-binary lifecycle failed with {:?}: {}",
            observation.status,
            observation.reason.as_deref().unwrap_or("no reason")
        )));
    }
    Ok(observation)
}

fn inspect_full_verification(
    path: &Path,
    commit_sha: &str,
) -> Result<FullVerificationFacts, DevError> {
    let bytes = fs::read(path).map_err(|error| {
        DevError::infrastructure(format!("read full verification receipt: {error}"))
    })?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err(DevError::corrupt(
            "full verification receipt exceeds 128 MiB",
        ));
    }
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| DevError::corrupt(format!("decode full verification receipt: {error}")))?;
    let object = value
        .as_object()
        .ok_or_else(|| DevError::corrupt("full verification receipt is not an object"))?;
    require_json_string(object, "status", "passed")?;
    require_json_string(object, "profile", "full")?;
    require_json_string(object, "git_head", commit_sha)?;
    require_json_bool(object, "input_stable", true)?;
    require_json_bool(object, "fresh_required", true)?;
    require_json_u64(object, "reused_passed_gates", 0)?;
    let selected = object
        .get("selected_gates")
        .and_then(Value::as_array)
        .ok_or_else(|| DevError::corrupt("full receipt selected_gates is missing"))?;
    let fresh = object
        .get("fresh_passed_gates")
        .and_then(Value::as_u64)
        .ok_or_else(|| DevError::corrupt("full receipt fresh_passed_gates is missing"))?;
    if fresh != selected.len() as u64 {
        return Err(DevError::corrupt(
            "full verification receipt contains non-fresh gates",
        ));
    }
    let gates = object
        .get("gates")
        .and_then(Value::as_array)
        .ok_or_else(|| DevError::corrupt("full receipt gates are missing"))?;
    let mut service_passed = false;
    for gate in gates {
        let gate = gate
            .as_object()
            .ok_or_else(|| DevError::corrupt("full receipt gate is not an object"))?;
        require_json_string(gate, "status", "passed")?;
        require_json_string(gate, "execution", "fresh")?;
        if gate.get("name").and_then(Value::as_str) == Some("service_acceptance") {
            service_passed = true;
        }
    }
    if !service_passed {
        return Err(DevError::corrupt(
            "full verification receipt lacks fresh passed service acceptance",
        ));
    }
    let (sha256, length) = archive::sha256_file(path)?;
    Ok(FullVerificationFacts {
        evidence: ExternalEvidence {
            path: path.to_string_lossy().into_owned(),
            byte_length: length,
            sha256,
        },
        selected_gates: selected.len(),
    })
}

fn require_json_string(
    object: &serde_json::Map<String, Value>,
    name: &str,
    expected: &str,
) -> Result<(), DevError> {
    if object.get(name).and_then(Value::as_str) != Some(expected) {
        return Err(DevError::corrupt(format!(
            "full verification receipt field '{name}' is not '{expected}'"
        )));
    }
    Ok(())
}

fn require_json_bool(
    object: &serde_json::Map<String, Value>,
    name: &str,
    expected: bool,
) -> Result<(), DevError> {
    if object.get(name).and_then(Value::as_bool) != Some(expected) {
        return Err(DevError::corrupt(format!(
            "full verification receipt field '{name}' is not {expected}"
        )));
    }
    Ok(())
}

fn require_json_u64(
    object: &serde_json::Map<String, Value>,
    name: &str,
    expected: u64,
) -> Result<(), DevError> {
    if object.get(name).and_then(Value::as_u64) != Some(expected) {
        return Err(DevError::corrupt(format!(
            "full verification receipt field '{name}' is not {expected}"
        )));
    }
    Ok(())
}

fn validate_manifest(manifest: &ReleaseManifest) -> Result<(), DevError> {
    if manifest.schema.identity != MANIFEST_SCHEMA
        || manifest.schema.version != MANIFEST_SCHEMA_VERSION
        || manifest.package.name != PACKAGE_NAME
        || manifest.target_triple != TARGET_TRIPLE
        || manifest.source.repository != REPOSITORY_IDENTITY
        || manifest.toolchain.toolchain_channel != TOOLCHAIN_CHANNEL
        || manifest.executable.archive_mode != 0o755
        || manifest.root_license.archive_mode != 0o644
        || manifest.third_party_notices.archive_mode != 0o644
        || manifest.executable.cli_contract != CLI_CONTRACT
        || manifest.packaging.tar_format != "posix-ustar"
        || manifest.packaging.gzip_level != 9
        || manifest.packaging.gzip_name_header
        || manifest.packaging.gzip_time_header
        || manifest.packaging.numeric_owner != 0
        || manifest.packaging.numeric_group != 0
        || manifest.packaging.members != archive::manifest_members()
        || manifest.packaging.tar_invocation
            != archive::normalized_tar_invocation(manifest.packaging.source_timestamp_unix_seconds)
        || manifest.packaging.gzip_invocation != archive::normalized_gzip_invocation()
        || manifest.packaging.source_timestamp_unix_seconds
            != manifest.source.commit_timestamp_unix_seconds
    {
        return Err(DevError::corrupt(
            "release manifest contains a noncanonical fixed contract field",
        ));
    }
    validate_strict_tag(
        &manifest.source.expected_release_tag,
        &manifest.package.version,
    )?;
    validate_git_sha(&manifest.source.tagged_commit_sha, "manifest commit SHA")?;
    validate_registry_digest(&manifest.executable.executable_registry_digest)?;
    match (
        manifest.publication_mode,
        manifest.source.annotated_tag_object_sha.as_deref(),
    ) {
        (PublicationMode::DryRun, None) => {}
        (PublicationMode::Release, Some(object)) => {
            validate_git_sha(object, "manifest annotated tag object SHA")?;
        }
        _ => {
            return Err(DevError::corrupt(
                "manifest tag-object state disagrees with publication mode",
            ));
        }
    }
    if manifest.executable.elf.class != "ELF64"
        || manifest.executable.elf.machine != "Advanced Micro Devices X86-64"
        || manifest.executable.elf.interpreter.is_empty()
        || manifest.executable.elf.required_shared_libraries.is_empty()
        || !manifest
            .executable
            .elf
            .highest_required_glibc_symbol_version
            .starts_with("GLIBC_")
        || manifest.third_party_notices.generator != "cargo-about"
        || manifest.third_party_notices.generator_version != CARGO_ABOUT_VERSION
        || manifest
            .third_party_notices
            .downloaded_archive_sha256
            .as_str()
            != CARGO_ABOUT_ARCHIVE_SHA256
        || manifest.third_party_notices.executable_sha256.as_str() != CARGO_ABOUT_EXECUTABLE_SHA256
        || manifest.third_party_notices.invocation != normalized_notice_invocation()
    {
        return Err(DevError::corrupt(
            "release manifest contains an invalid measured identity",
        ));
    }
    Ok(())
}

fn validate_receipt(
    receipt: &ReleaseReceipt,
    archive: &archive::VerifiedArchive,
) -> Result<(), DevError> {
    if receipt.schema.identity != RECEIPT_SCHEMA
        || receipt.schema.version != RECEIPT_SCHEMA_VERSION
        || receipt.publication_mode != archive.manifest.publication_mode
        || receipt.release_tag != archive.manifest.source.expected_release_tag
        || receipt.commit_sha != archive.manifest.source.tagged_commit_sha
        || receipt.manifest_sha256 != archive.manifest_sha256
        || receipt.archive.name != ARCHIVE_NAME
        || receipt.archive.byte_length != archive.archive_byte_length
        || receipt.archive.sha256 != archive.archive_sha256
        || receipt.checksum_file.name != CHECKSUM_NAME
        || receipt.candidate_lifecycle.status != ProcessStatus::Passed
        || receipt.completed_unix_nanoseconds < receipt.started_unix_nanoseconds
    {
        return Err(DevError::corrupt("release receipt binding mismatch"));
    }
    if receipt.classifications.len() != EXPECTED_CLASSIFICATIONS.len() {
        return Err(DevError::corrupt(
            "release receipt classification inventory mismatch",
        ));
    }
    for (observed, expected) in receipt.classifications.iter().zip(EXPECTED_CLASSIFICATIONS) {
        if observed.name != expected {
            return Err(DevError::corrupt(
                "release receipt classification order mismatch",
            ));
        }
        if observed.name != "full_verification"
            && observed.classification != EvidenceClassification::FreshPassed
        {
            return Err(DevError::corrupt(format!(
                "release receipt classification '{}' is not fresh passed",
                observed.name
            )));
        }
    }
    let full = receipt
        .classifications
        .iter()
        .find(|classification| classification.name == "full_verification")
        .ok_or_else(|| DevError::corrupt("full verification classification is missing"))?;
    match (
        full.classification,
        receipt.full_verification_receipt.as_ref(),
    ) {
        (EvidenceClassification::FreshPassed, Some(_))
        | (EvidenceClassification::NotProvided, None) => Ok(()),
        _ => Err(DevError::corrupt(
            "full verification classification disagrees with its evidence",
        )),
    }
}

fn classifications(
    full: &Option<FullVerificationFacts>,
    member_count: usize,
) -> Vec<VerificationClassification> {
    let fresh = |name: &str, detail: String| VerificationClassification {
        name: name.to_owned(),
        classification: EvidenceClassification::FreshPassed,
        detail,
    };
    vec![
        fresh(
            "source_identity",
            "clean exact Git source validated".to_owned(),
        ),
        fresh("toolchain", format!("Rust/Cargo {TOOLCHAIN_CHANNEL}")),
        fresh(
            "cargo_about",
            format!("cargo-about {CARGO_ABOUT_VERSION} archive and executable pinned"),
        ),
        fresh(
            "notice_generation",
            "two locked offline generations were byte-equal".to_owned(),
        ),
        fresh(
            "candidate_capabilities",
            format!("CLI contract {CLI_CONTRACT} and registry digest validated"),
        ),
        fresh(
            "candidate_lifecycle",
            "exact candidate completed copied-binary lifecycle".to_owned(),
        ),
        match full {
            Some(facts) => fresh(
                "full_verification",
                format!(
                    "{} fresh gates including service acceptance",
                    facts.selected_gates
                ),
            ),
            None => VerificationClassification {
                name: "full_verification".to_owned(),
                classification: EvidenceClassification::NotProvided,
                detail: "not provided to this preparation".to_owned(),
            },
        },
        fresh(
            "deterministic_packaging",
            "two same-input archive preparations were byte-equal".to_owned(),
        ),
        fresh(
            "archive_verification",
            format!("strictly verified {member_count} ordered ustar members"),
        ),
        fresh(
            "checksum_integrity",
            "exact one-line SHA256SUMS binding validated".to_owned(),
        ),
    ]
}

fn checksum_bytes(digest: &Sha256Digest) -> Vec<u8> {
    format!("{}  {ARCHIVE_NAME}\n", digest.as_str()).into_bytes()
}

fn verify_checksum_bytes(bytes: &[u8], expected: &Sha256Digest) -> Result<(), DevError> {
    if bytes != checksum_bytes(expected) {
        return Err(DevError::corrupt(
            "SHA256SUMS is not the exact canonical one-line archive checksum",
        ));
    }
    Ok(())
}

fn hosted_context() -> HostedContext {
    let server = env::var("GITHUB_SERVER_URL").ok();
    let repository = env::var("GITHUB_REPOSITORY").ok();
    let run_id = env::var("GITHUB_RUN_ID").ok();
    let run_url = match (&server, &repository, &run_id) {
        (Some(server), Some(repository), Some(run_id)) => {
            Some(format!("{server}/{repository}/actions/runs/{run_id}"))
        }
        _ => None,
    };
    HostedContext {
        github_actions: env::var("GITHUB_ACTIONS").ok(),
        repository,
        workflow: env::var("GITHUB_WORKFLOW").ok(),
        job: env::var("GITHUB_JOB").ok(),
        run_id,
        run_attempt: env::var("GITHUB_RUN_ATTEMPT").ok(),
        run_url,
        runner_os: env::var("RUNNER_OS").ok(),
        runner_architecture: env::var("RUNNER_ARCH").ok(),
        runner_image_os: env::var("ImageOS").ok(),
        runner_image_version: env::var("ImageVersion").ok(),
    }
}

fn publish_directory_no_replace(stage: &Path, output: &Path) -> Result<(), DevError> {
    let parent = output
        .parent()
        .ok_or_else(|| DevError::usage("output has no parent"))?;
    let stage_parent = stage
        .parent()
        .ok_or_else(|| DevError::infrastructure("stage has no parent"))?;
    if parent != stage_parent {
        return Err(DevError::infrastructure(
            "release output stage and destination do not share a parent",
        ));
    }
    let stage_name = stage
        .file_name()
        .ok_or_else(|| DevError::infrastructure("release output stage has no name"))?;
    let output_name = output
        .file_name()
        .ok_or_else(|| DevError::usage("release output has no file name"))?;
    let directory = File::open(parent).map_err(|error| {
        DevError::infrastructure(format!("open release output parent: {error}"))
    })?;
    rustix::fs::renameat_with(
        &directory,
        stage_name,
        &directory,
        output_name,
        rustix::fs::RenameFlags::NOREPLACE,
    )
    .map_err(|error| {
        DevError::infrastructure(format!(
            "publish release output '{}' without replacement: {error}",
            output.display()
        ))
    })
}

fn require_equal_files(left: &Path, right: &Path, label: &str) -> Result<(), DevError> {
    let (left_digest, left_bytes) = archive::sha256_file(left)?;
    let (right_digest, right_bytes) = archive::sha256_file(right)?;
    if left_bytes != right_bytes || left_digest != right_digest {
        return Err(DevError::corrupt(format!(
            "{label} produced different bytes"
        )));
    }
    Ok(())
}

fn command_version(program: &str, arguments: &[&str], cwd: &Path) -> Result<String, DevError> {
    let output = command_text(program, arguments, cwd, 64 * 1024)?;
    output
        .lines()
        .next()
        .map(str::to_owned)
        .ok_or_else(|| DevError::corrupt(format!("{program} version output is empty")))
}

fn command_text(
    program: &str,
    arguments: &[&str],
    cwd: &Path,
    maximum_bytes: usize,
) -> Result<String, DevError> {
    let output = Command::new(program)
        .args(arguments)
        .current_dir(cwd)
        .env_clear()
        .envs(process::environment())
        .output()
        .map_err(|error| DevError::infrastructure(format!("start '{program}': {error}")))?;
    let combined = output
        .stdout
        .len()
        .checked_add(output.stderr.len())
        .ok_or_else(|| DevError::infrastructure("child output length overflow"))?;
    if combined > maximum_bytes {
        return Err(DevError::infrastructure(format!(
            "'{program}' output exceeded {maximum_bytes} bytes"
        )));
    }
    if !output.status.success() {
        return Err(DevError::infrastructure(format!(
            "'{program}' failed with {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|_| DevError::corrupt(format!("'{program}' stdout is not UTF-8")))?;
    Ok(stdout.trim().to_owned())
}

fn run_git_success(repository: &Path, arguments: &[&str], failure: &str) -> Result<(), DevError> {
    let status = Command::new("git")
        .args(arguments)
        .current_dir(repository)
        .env_clear()
        .envs(process::environment())
        .status()
        .map_err(|error| DevError::infrastructure(format!("start git: {error}")))?;
    if !status.success() {
        return Err(DevError::corrupt(failure));
    }
    Ok(())
}

fn origin_matches_repository(origin: &str) -> bool {
    origin == "https://github.com/lkjsxc/lkjscript"
        || origin == "https://github.com/lkjsxc/lkjscript.git"
        || origin == "git@github.com:lkjsxc/lkjscript.git"
}

fn validate_git_sha(value: &str, label: &str) -> Result<(), DevError> {
    if value.len() != 40
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(DevError::corrupt(format!(
            "{label} is not a full lowercase Git object SHA"
        )));
    }
    Ok(())
}

fn validate_registry_digest(value: &str) -> Result<(), DevError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(DevError::corrupt(
            "executable registry digest is not lowercase SHA-256",
        ));
    }
    Ok(())
}

fn labeled_value(output: &str, label: &str) -> Result<String, DevError> {
    output
        .lines()
        .find_map(|line| line.trim().strip_prefix(label).map(str::trim))
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| DevError::corrupt(format!("readelf omitted '{label}'")))
}

fn bracketed_value(line: &str, label: &str) -> Result<String, DevError> {
    let start = line
        .find('[')
        .ok_or_else(|| DevError::corrupt(format!("readelf {label} is malformed")))?;
    let end = line[start + 1..]
        .find(']')
        .map(|offset| start + 1 + offset)
        .ok_or_else(|| DevError::corrupt(format!("readelf {label} is malformed")))?;
    Ok(line[start + 1..end].to_owned())
}

fn highest_glibc_version(output: &str) -> Result<String, DevError> {
    let mut versions = BTreeSet::new();
    for token in output.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '_' || character == '.')
    }) {
        let Some(version) = token.strip_prefix("GLIBC_") else {
            continue;
        };
        if version.is_empty()
            || version
                .split('.')
                .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
        {
            continue;
        }
        let parts = version
            .split('.')
            .map(|part| part.parse::<u64>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| DevError::corrupt("GLIBC version component overflow"))?;
        versions.insert((parts, token.to_owned()));
    }
    versions
        .into_iter()
        .next_back()
        .map(|(_, value)| value)
        .ok_or_else(|| DevError::corrupt("readelf output contains no numeric GLIBC requirement"))
}

fn unix_nanoseconds() -> Result<u128, DevError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|error| DevError::infrastructure(format!("system clock before epoch: {error}")))
}

fn duration_nanoseconds(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_strict_tag_accepts_only_matching_plain_semver() {
        assert!(validate_strict_tag("v0.1.0", "0.1.0").is_ok());
        for tag in ["0.1.0", "v01.1.0", "v0.1", "v0.1.0-rc.1", "v0.1.1"] {
            assert!(validate_strict_tag(tag, "0.1.0").is_err(), "accepted {tag}");
        }
    }

    #[test]
    fn release_checksum_is_exact_and_not_self_referential() {
        let digest = Sha256Digest::new("a".repeat(64)).expect("digest");
        let canonical = checksum_bytes(&digest);
        assert!(verify_checksum_bytes(&canonical, &digest).is_ok());
        assert!(!String::from_utf8_lossy(&canonical).contains(CHECKSUM_NAME));
        let mut extra = canonical.clone();
        extra.extend_from_slice(b"extra\n");
        assert!(verify_checksum_bytes(&extra, &digest).is_err());
    }

    #[test]
    fn release_glibc_version_uses_numeric_order() {
        let output = "GLIBC_2.9 GLIBC_2.34 GLIBC_2.3.4 GLIBC_ABI_DT_RELR";
        assert_eq!(
            highest_glibc_version(output).expect("version"),
            "GLIBC_2.34"
        );
    }

    #[test]
    fn release_registry_digest_requires_lowercase_sha256() {
        assert!(validate_registry_digest(&"0".repeat(64)).is_ok());
        assert!(validate_registry_digest(&"A".repeat(64)).is_err());
        assert!(validate_registry_digest("short").is_err());
    }
}
