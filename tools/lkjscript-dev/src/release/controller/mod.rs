//! Release control runs from the explicitly selected, integrated workflow source.
//! Candidate assets and the transferred behavioral verifier are data to this module.
mod model;
mod operations;
mod publication;
#[cfg(test)]
mod tests;

use crate::error::DevError;
use model::{
    ACCEPTANCE_JOB, Artifact, Context, Producer, REPOSITORY, SELECTION_FORMAT, Selection, WORKFLOW,
};
pub(super) use model::{CandidateContent, FileIdentity};
use operations::{HostedOperations, MAX_ARTIFACT, MAX_JSON, Operations};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub(crate) fn command(mut arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let verb = crate::next_utf8(&mut arguments, "controller operation")?.ok_or_else(|| {
        DevError::usage(
            "controller requires candidate-context, select, authority, publish, or public-download",
        )
    })?;
    let options = options(arguments)?;
    let output = absolute_option(&options, "output")?;
    super::require_absolute_extraction_output(&output)?;
    fs::create_dir(&output)?;
    fs::set_permissions(&output, fs::Permissions::from_mode(0o700))?;
    write_json(
        &output.join("state.json"),
        &json!({"status":"incomplete", "operation":verb}),
    )?;
    let mut cancelled = false;
    let result = (|| {
        let context = hosted_context()?;
        let mut operations = HostedOperations::new(output.join("operations"))?;
        let cancellation = super::transferred::Cancellation::new()?;
        operations.control = cancellation.control.clone();
        let result = execute(&verb, &options, &output, &context, &mut operations);
        cancelled = cancellation.control.cancelled();
        cancellation.finish()?;
        if cancelled {
            return Err(DevError::unavailable(
                "release boundary cancelled after joining owned processes; successful independent writes may remain",
            ));
        }
        result
    })();
    let state = match &result {
        Ok(value) => value.clone(),
        Err(error) => {
            json!({"status":if cancelled {"cancelled"} else if error.kind()=="unavailable" {"unavailable"} else {"rejected"}, "operation":verb,"reason":error.message(),"next_action":"Retain this failed boundary. Restore trusted inputs and explicitly select the same producer run/attempt; expired acceptance needs a new candidate or separately authorized trusted recovery."})
        }
    };
    // State is private diagnostic output; all accepted content remains create-new and immutable.
    crate::evidence::publish_json(&output.join("state.json"), &state)?;
    println!("{}", serde_json::to_string(&state)?);
    result.map(|_| 0)
}

fn execute(
    verb: &str,
    options: &BTreeMap<String, String>,
    output: &Path,
    context: &Context,
    operations: &mut impl Operations,
) -> Result<Value, DevError> {
    let main = trusted_controller(operations, context)?;
    if verb == "candidate-context" {
        if options.len() != 1 {
            return Err(DevError::usage("candidate-context accepts only --output"));
        }
        return Ok(
            json!({"status":"candidate_source_admitted","source_commit":context.source,"main_commit":main,"controller_source":context.source}),
        );
    }
    if verb == "select" {
        if options.len() != 3 {
            return Err(DevError::usage(
                "select requires --producer-run --producer-attempt --output",
            ));
        }
        let run = positive(required(options, "producer-run")?)?;
        let attempt = positive(required(options, "producer-attempt")?)?;
        let producer = authenticate_producer(operations, run, attempt, &main)?;
        download_handoffs(operations, &producer, output)?;
        let content = admit_content(output, &producer)?;
        let selection = Selection {
            format: SELECTION_FORMAT.to_owned(),
            controller_source: context.source.clone(),
            consumer_run_id: context.run_id,
            consumer_run_attempt: context.run_attempt,
            producer,
            content,
        };
        write_json(&output.join("selection.json"), &selection)?;
        return Ok(
            json!({"status":"candidate_accepted","selection":output.join("selection.json"),"producer":selection.producer,"content":selection.content,"controller_source":context.source,"product_builds":0,"heavy_application_invocations":0}),
        );
    }
    let selection_path = absolute_option(options, "selection")?;
    let selection: Selection = read_json(&selection_path)?;
    let root = selection_path
        .parent()
        .ok_or_else(|| DevError::usage("selection has no parent"))?;
    validate_selection(operations, &selection, root, &main)?;
    match verb {
        "authority" => {
            let authorization =
                std::env::var("LKJSCRIPT_IMMUTABLE_RELEASE_TAG_OBJECT_SHA").unwrap_or_default();
            let authority =
                publication::authorize(operations, &selection, context, &authorization)?;
            publication::inspect_occupancy(operations, &selection, &authority)?;
            Ok(
                json!({"status":"promotion_authorized","authority":authority,"product_builds":0,"heavy_application_invocations":0}),
            )
        }
        "publish" => {
            let authorization =
                std::env::var("LKJSCRIPT_IMMUTABLE_RELEASE_TAG_OBJECT_SHA").unwrap_or_default();
            let authority =
                publication::authorize(operations, &selection, context, &authorization)?;
            write_json(&output.join("authorization.json"), &authority)?;
            let published = publication::publish(operations, &selection, root, &authority)?;
            Ok(
                json!({"status":"immutable_assets_published","producer":selection.producer,"authority":authority,"publication":published,"public_verification":"unrun","product_builds":0,"heavy_application_invocations":0}),
            )
        }
        "public-download" => {
            let latest_required = options
                .get("latest")
                .map(String::as_str)
                .unwrap_or("required");
            if !matches!(latest_required, "required" | "allow-superseded") {
                return Err(DevError::usage(
                    "--latest requires required or allow-superseded",
                ));
            }
            let public = publication::public_download(
                operations,
                &selection,
                output,
                latest_required == "required",
            )?;
            Ok(
                json!({"status":"public_assets_admitted","producer":selection.producer,"publication":public,"public_verification":"smoke_required","product_builds":0,"heavy_application_invocations":0}),
            )
        }
        _ => Err(DevError::usage(format!(
            "unknown controller operation {verb}"
        ))),
    }
}

fn hosted_context() -> Result<Context, DevError> {
    let value = |name: &str| {
        std::env::var(name)
            .map_err(|_| DevError::usage(format!("trusted controller requires {name}")))
    };
    if value("GITHUB_ACTIONS")? != "true"
        || value("GITHUB_REPOSITORY")? != REPOSITORY
        || value("GITHUB_EVENT_NAME")? != "workflow_dispatch"
        || value("GITHUB_REF")? != "refs/heads/main"
        || value("GITHUB_WORKFLOW_REF")? != format!("{REPOSITORY}/{WORKFLOW}@refs/heads/main")
    {
        return Err(DevError::corrupt(
            "release controller must come from the maintained mainline workflow_dispatch",
        ));
    }
    let source = value("GITHUB_WORKFLOW_SHA")?;
    validate_sha(&source, 40)?;
    if value("GITHUB_SHA")? != source {
        return Err(DevError::corrupt(
            "controller checkout/event/workflow source mismatch",
        ));
    }
    Ok(Context {
        source,
        run_id: positive(&value("GITHUB_RUN_ID")?)?,
        run_attempt: positive(&value("GITHUB_RUN_ATTEMPT")?)?,
    })
}

fn trusted_controller(
    operations: &mut impl Operations,
    context: &Context,
) -> Result<String, DevError> {
    validate_sha(&context.source, 40)?;
    let run = operations.api(
        "GET",
        &format!(
            "repos/{REPOSITORY}/actions/runs/{}/attempts/{}",
            context.run_id, context.run_attempt
        ),
        None,
    )?;
    validate_run(
        &run,
        context.run_id,
        context.run_attempt,
        &context.source,
        false,
    )?;
    let workflow = operations.api(
        "GET",
        &format!("repos/{REPOSITORY}/actions/workflows/release.yml"),
        None,
    )?;
    if string(&workflow, "path")? != WORKFLOW
        || number(&workflow, "id")? != number(&run, "workflow_id")?
    {
        return Err(DevError::corrupt("controller workflow identity mismatch"));
    }
    let main = remote_main(operations)?;
    require_ancestor(operations, &context.source, &main)?;
    Ok(main)
}

fn remote_main(operations: &mut impl Operations) -> Result<String, DevError> {
    let main = operations.api(
        "GET",
        &format!("repos/{REPOSITORY}/git/ref/heads/main"),
        None,
    )?;
    if main.pointer("/object/type").and_then(Value::as_str) != Some("commit") {
        return Err(DevError::corrupt("remote main is not a commit"));
    }
    let sha = main
        .pointer("/object/sha")
        .and_then(Value::as_str)
        .ok_or_else(|| DevError::corrupt("remote main lacks a source"))?;
    validate_sha(sha, 40)?;
    Ok(sha.to_owned())
}

fn require_ancestor(
    operations: &mut impl Operations,
    source: &str,
    main: &str,
) -> Result<(), DevError> {
    validate_sha(source, 40)?;
    validate_sha(main, 40)?;
    let comparison = operations.api(
        "GET",
        &format!("repos/{REPOSITORY}/compare/{source}...{main}"),
        None,
    )?;
    let status = string(&comparison, "status")?;
    if !matches!(status, "identical" | "ahead")
        || comparison
            .pointer("/merge_base_commit/sha")
            .and_then(Value::as_str)
            != Some(source)
        || number(&comparison, "behind_by")? != 0
    {
        return Err(DevError::corrupt(
            "selected source is not reachable from freshly resolved remote main",
        ));
    }
    Ok(())
}

fn validate_run(
    run: &Value,
    id: u64,
    attempt: u64,
    source: &str,
    completed: bool,
) -> Result<(), DevError> {
    if number(run, "id")? != id
        || number(run, "run_attempt")? != attempt
        || string(run, "head_sha")? != source
        || string(run, "event")? != "workflow_dispatch"
        || string(run, "head_branch")? != "main"
        || string(run, "path")? != WORKFLOW
        || run.pointer("/repository/full_name").and_then(Value::as_str) != Some(REPOSITORY)
        || run
            .pointer("/head_repository/full_name")
            .and_then(Value::as_str)
            != Some(REPOSITORY)
        || (completed
            && (string(run, "status")? != "completed" || string(run, "conclusion")? != "success"))
    {
        return Err(DevError::corrupt(
            "producer/controller run, attempt, repository, event, workflow, source or terminal status is not trusted",
        ));
    }
    validate_sha(source, 40)
}

fn authenticate_producer(
    operations: &mut impl Operations,
    run_id: u64,
    run_attempt: u64,
    main: &str,
) -> Result<Producer, DevError> {
    let run = operations.api(
        "GET",
        &format!("repos/{REPOSITORY}/actions/runs/{run_id}/attempts/{run_attempt}"),
        None,
    )?;
    let source = string(&run, "head_sha")?.to_owned();
    validate_run(&run, run_id, run_attempt, &source, true)?;
    require_ancestor(operations, &source, main)?;
    let workflow = operations.api(
        "GET",
        &format!("repos/{REPOSITORY}/actions/workflows/release.yml"),
        None,
    )?;
    let workflow_id = number(&workflow, "id")?;
    if workflow_id != number(&run, "workflow_id")? || string(&workflow, "path")? != WORKFLOW {
        return Err(DevError::corrupt("producer uses a different workflow"));
    }
    let jobs = operations.api(
        "GET",
        &format!(
            "repos/{REPOSITORY}/actions/runs/{run_id}/attempts/{run_attempt}/jobs?per_page=100"
        ),
        None,
    )?;
    let jobs_array = array(&jobs, "jobs")?;
    if number(&jobs, "total_count")? != jobs_array.len() as u64 {
        return Err(DevError::corrupt(
            "producer jobs exceed bounded complete inventory",
        ));
    }
    let matching: Vec<_> = jobs_array
        .iter()
        .filter(|job| job.get("name").and_then(Value::as_str) == Some(ACCEPTANCE_JOB))
        .collect();
    if matching.len() != 1 {
        return Err(DevError::corrupt(
            "producer lacks exactly one required acceptance job",
        ));
    }
    let job = matching[0];
    if number(job, "run_id")? != run_id
        || string(job, "head_sha")? != source
        || string(job, "status")? != "completed"
        || string(job, "conclusion")? != "success"
    {
        return Err(DevError::corrupt(
            "producer required acceptance job was not successful",
        ));
    }
    let required_steps = [
        "Admit the final candidate and original evidence",
        "Upload immutable candidate assets",
        "Upload original candidate verifier",
        "Upload essential acceptance handoff",
    ];
    for name in required_steps {
        let matching: Vec<_> = array(job, "steps")?
            .iter()
            .filter(|step| step.get("name").and_then(Value::as_str) == Some(name))
            .collect();
        if matching.len() != 1
            || string(matching[0], "conclusion")? != "success"
            || string(matching[0], "status")? != "completed"
        {
            return Err(DevError::corrupt(format!(
                "mandatory producer step missing, skipped or failed: {name}"
            )));
        }
    }
    let listing = operations.api(
        "GET",
        &format!("repos/{REPOSITORY}/actions/runs/{run_id}/artifacts?per_page=100"),
        None,
    )?;
    let listed = array(&listing, "artifacts")?;
    if number(&listing, "total_count")? != listed.len() as u64 {
        return Err(DevError::unavailable(
            "artifact inventory exceeds bounded listing; no latest-name fallback is permitted",
        ));
    }
    let mut artifacts = Vec::new();
    for role in ["assets", "verifier", "acceptance"] {
        let name = format!("candidate-{role}-{run_id}-{run_attempt}");
        let matching: Vec<_> = listed
            .iter()
            .filter(|value| value.get("name").and_then(Value::as_str) == Some(&name))
            .collect();
        if matching.len() != 1 {
            return Err(DevError::unavailable(format!(
                "exact producer artifact {name} is missing or ambiguous; retain original producer identity and create new candidate if trust cannot be recovered"
            )));
        }
        let id = number(matching[0], "id")?;
        let value = operations.api(
            "GET",
            &format!("repos/{REPOSITORY}/actions/artifacts/{id}"),
            None,
        )?;
        if value.get("expired").and_then(Value::as_bool) != Some(false) {
            return Err(DevError::unavailable(format!(
                "producer artifact {id} expired; requested retention was 14 days, not indefinite"
            )));
        }
        if number(&value, "id")? != id
            || string(&value, "name")? != name
            || value.pointer("/workflow_run/id").and_then(Value::as_u64) != Some(run_id)
            || value
                .pointer("/workflow_run/head_sha")
                .and_then(Value::as_str)
                != Some(source.as_str())
            || matching[0].get("digest") != value.get("digest")
        {
            return Err(DevError::corrupt(
                "artifact service identity does not match the selected producer",
            ));
        }
        let digest = string(&value, "digest")?.to_owned();
        validate_service_digest(&digest)?;
        let bytes = number(&value, "size_in_bytes")?;
        if bytes == 0 || bytes > MAX_ARTIFACT {
            return Err(DevError::corrupt("artifact size exceeds bounded transport"));
        }
        artifacts.push(Artifact {
            id,
            name,
            role: role.to_owned(),
            digest,
            byte_length: bytes,
            expires_at: string(&value, "expires_at")?.to_owned(),
        });
    }
    Ok(Producer {
        repository: REPOSITORY.to_owned(),
        run_id,
        run_attempt,
        source_commit: source,
        workflow_id,
        workflow_path: WORKFLOW.to_owned(),
        acceptance_job_id: number(job, "id")?,
        artifacts,
    })
}

fn download_handoffs(
    operations: &mut impl Operations,
    producer: &Producer,
    root: &Path,
) -> Result<(), DevError> {
    extract_handoffs(operations, producer, root, root, true)
}

fn extract_handoffs(
    operations: &mut impl Operations,
    producer: &Producer,
    zip_root: &Path,
    root: &Path,
    download: bool,
) -> Result<(), DevError> {
    for artifact in &producer.artifacts {
        let zip = zip_root.join(format!("{}.zip", artifact.role));
        if download {
            operations.download(
                &format!("repos/{REPOSITORY}/actions/artifacts/{}/zip", artifact.id),
                &zip,
                true,
            )?;
        }
        bounded_regular(&zip, MAX_ARTIFACT)?;
        let (digest, bytes) = super::archive::sha256_file(&zip)?;
        if artifact.digest != format!("sha256:{}", digest.as_str()) || artifact.byte_length != bytes
        {
            return Err(DevError::corrupt(
                "artifact ZIP bytes differ from authenticated service digest/size",
            ));
        }
        let expected: Vec<&str> = match artifact.role.as_str() {
            "assets" => vec![super::archive::ARCHIVE_NAME, "SHA256SUMS", "install.sh"],
            "verifier" => vec!["lkjscript-dev", "verifier-identity.json"],
            "acceptance" => vec!["release-receipt.json"],
            _ => return Err(DevError::corrupt("unrecognized handoff role")),
        };
        let list = root.join(format!("{}-members", artifact.role));
        operations.zip_member(&zip, None, &list)?;
        let listing = String::from_utf8(crate::process::read_bounded(&list, MAX_JSON)?)
            .map_err(|_| DevError::corrupt("ZIP inventory is not UTF-8"))?;
        let names: Vec<_> = listing.lines().collect();
        if names.len() != expected.len()
            || names.iter().copied().collect::<BTreeSet<_>>()
                != expected.iter().copied().collect::<BTreeSet<_>>()
        {
            return Err(DevError::corrupt(
                "handoff ZIP has missing, extra, duplicate, or nested members",
            ));
        }
        let destination = root.join(&artifact.role);
        fs::create_dir(&destination)?;
        // Extract each fixed member through stdout into a create-new regular file.
        // ZIP path/mode/link metadata is never allowed to mutate the filesystem.
        for name in expected {
            operations.zip_member(&zip, Some(name), &destination.join(name))?;
        }
    }
    Ok(())
}

fn admit_content(root: &Path, producer: &Producer) -> Result<CandidateContent, DevError> {
    let verifier = root.join("verifier/lkjscript-dev");
    bounded_regular(&verifier, 384 * 1024 * 1024)?;
    let (digest, bytes) = super::archive::sha256_file(&verifier)?;
    let content = super::candidate::controller_content(
        &root.join("acceptance/release-receipt.json"),
        &producer.source_commit,
        digest.as_str(),
        producer.run_id,
        producer.run_attempt,
    )?;
    if content.verifier.name != "lkjscript-dev"
        || content.verifier.byte_length != bytes
        || content.verifier.sha256 != digest.as_str()
        || content.source_commit != producer.source_commit
    {
        return Err(DevError::corrupt(
            "terminal content/verifier is not bound to the authenticated producer",
        ));
    }
    verify_files(&root.join("assets"), &content.assets)?;
    verify_inventory(
        &root.join("verifier"),
        &["lkjscript-dev", "verifier-identity.json"],
    )?;
    verify_inventory(&root.join("acceptance"), &["release-receipt.json"])?;
    fs::set_permissions(&verifier, fs::Permissions::from_mode(0o755))?;
    super::verifier::validate_handoff(
        &verifier,
        &root.join("verifier/verifier-identity.json"),
        &content.tag,
        &content.source_commit,
        &content.verifier.sha256,
        content.verifier.byte_length,
    )?;
    Ok(content)
}

fn validate_selection(
    operations: &mut impl Operations,
    selection: &Selection,
    root: &Path,
    main: &str,
) -> Result<(), DevError> {
    if selection.format != SELECTION_FORMAT {
        return Err(DevError::corrupt("unsupported candidate selection"));
    }
    let producer = authenticate_producer(
        operations,
        selection.producer.run_id,
        selection.producer.run_attempt,
        main,
    )?;
    if producer != selection.producer {
        return Err(DevError::corrupt(
            "selection no longer matches authenticated producer originals",
        ));
    }
    let original = tempfile::Builder::new()
        .prefix(".read-original-handoffs-")
        .tempdir_in(root)?;
    let result = (|| {
        extract_handoffs(operations, &producer, root, original.path(), false)?;
        let authentic = admit_content(original.path(), &producer)?;
        if authentic != selection.content || admit_content(root, &producer)? != authentic {
            return Err(DevError::corrupt(
                "selection content differs from the authenticated original handoff",
            ));
        }
        for member in [
            "acceptance/release-receipt.json",
            "verifier/verifier-identity.json",
        ] {
            if super::archive::sha256_file(&root.join(member))?
                != super::archive::sha256_file(&original.path().join(member))?
            {
                return Err(DevError::corrupt(
                    "terminal or verifier identity differs from authenticated original ZIP member",
                ));
            }
        }
        Ok(())
    })();
    original.close()?;
    result
}

pub(super) fn verify_files(root: &Path, files: &[FileIdentity]) -> Result<(), DevError> {
    let expected = [super::archive::ARCHIVE_NAME, "SHA256SUMS", "install.sh"];
    if files.len() != 3
        || files
            .iter()
            .map(|file| file.name.as_str())
            .collect::<BTreeSet<_>>()
            != expected.into_iter().collect::<BTreeSet<_>>()
    {
        return Err(DevError::corrupt(
            "candidate has an unexpected public asset inventory",
        ));
    }
    verify_inventory(root, &expected)?;
    for file in files {
        validate_sha(&file.sha256, 64)?;
        let bound = match file.name.as_str() {
            "install.sh" => super::bootstrap::MAXIMUM_BYTES,
            "SHA256SUMS" => 4096,
            _ => lkjscript::release_container::MAXIMUM_COMPRESSED_BYTES,
        };
        bounded_regular(&root.join(&file.name), bound)?;
        let (sha, bytes) = super::archive::sha256_file(&root.join(&file.name))?;
        if sha.as_str() != file.sha256 || bytes != file.byte_length {
            return Err(DevError::corrupt(format!(
                "accepted asset changed: {}",
                file.name
            )));
        }
    }
    Ok(())
}

fn verify_inventory(root: &Path, expected: &[&str]) -> Result<(), DevError> {
    super::archive::ensure_directory(root, "controller handoff")?;
    if !root.is_absolute() || root.canonicalize()? != root {
        return Err(DevError::corrupt(
            "handoff directory must be canonical, absolute, and free of symlink parents",
        ));
    }
    let mut found = BTreeSet::new();
    for entry in fs::read_dir(root)?.take(expected.len() + 1) {
        let entry = entry?;
        super::archive::ensure_regular(&entry.path(), "controller handoff member")?;
        found.insert(
            entry
                .file_name()
                .into_string()
                .map_err(|_| DevError::corrupt("handoff name is not UTF-8"))?,
        );
    }
    if found != expected.iter().map(|name| (*name).to_owned()).collect() {
        return Err(DevError::corrupt("controller handoff inventory mismatch"));
    }
    Ok(())
}

fn bounded_regular(path: &Path, maximum: u64) -> Result<(), DevError> {
    let metadata = super::archive::ensure_regular(path, "bounded release handoff member")?;
    if metadata.len() == 0 || metadata.len() > maximum {
        return Err(DevError::corrupt(
            "release handoff member is empty or exceeds its byte bound",
        ));
    }
    Ok(())
}

fn options(
    arguments: impl Iterator<Item = OsString>,
) -> Result<BTreeMap<String, String>, DevError> {
    let mut result = BTreeMap::new();
    let mut arguments = arguments;
    while let Some(name) = crate::next_utf8(&mut arguments, "controller option")? {
        let name = name
            .strip_prefix("--")
            .ok_or_else(|| DevError::usage("controller options require --name value"))?;
        if !matches!(
            name,
            "output" | "producer-run" | "producer-attempt" | "selection" | "latest"
        ) {
            return Err(DevError::usage(format!("unknown controller option {name}")));
        }
        let value = crate::next_utf8(&mut arguments, "controller option value")?
            .ok_or_else(|| DevError::usage(format!("missing {name}")))?;
        if result.insert(name.to_owned(), value).is_some() {
            return Err(DevError::usage(format!("duplicate {name}")));
        }
    }
    Ok(result)
}
fn required<'a>(options: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, DevError> {
    options
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| DevError::usage(format!("--{key} is required")))
}
fn absolute_option(options: &BTreeMap<String, String>, key: &str) -> Result<PathBuf, DevError> {
    let value = PathBuf::from(required(options, key)?);
    if !value.is_absolute() {
        return Err(DevError::usage(format!(
            "--{key} requires an absolute path"
        )));
    }
    Ok(value)
}
fn positive(value: &str) -> Result<u64, DevError> {
    if value.is_empty()
        || value.starts_with('0')
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(DevError::usage(
            "run and attempt must be exact positive decimal identities",
        ));
    }
    value
        .parse()
        .map_err(|_| DevError::usage("numeric identity overflow"))
}
fn validate_sha(value: &str, length: usize) -> Result<(), DevError> {
    if value.len() != length
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(DevError::corrupt("invalid immutable digest identity"));
    }
    Ok(())
}
fn validate_service_digest(value: &str) -> Result<(), DevError> {
    validate_sha(
        value
            .strip_prefix("sha256:")
            .ok_or_else(|| DevError::corrupt("service artifact digest is unavailable"))?,
        64,
    )
}
fn validate_name(value: &str) -> Result<(), DevError> {
    if value.is_empty()
        || value.len() > 128
        || value.starts_with('.')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(DevError::corrupt("unsafe asset/member name"));
    }
    Ok(())
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, DevError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| DevError::corrupt(format!("required service string {key} missing")))
}
fn number(value: &Value, key: &str) -> Result<u64, DevError> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| DevError::corrupt(format!("required service integer {key} missing")))
}
fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, DevError> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| DevError::corrupt(format!("required service array {key} missing")))
}
fn write_json(path: &Path, value: &impl Serialize) -> Result<(), DevError> {
    super::archive::write_new(path, &super::archive::canonical_json(value)?, 0o600)
}
fn read_json<T: DeserializeOwned + Serialize>(path: &Path) -> Result<T, DevError> {
    super::archive::ensure_regular(path, "controller JSON")?;
    let bytes = crate::process::read_bounded(path, MAX_JSON)?;
    let value: T = serde_json::from_slice(&bytes)?;
    if super::archive::canonical_json(&value)? != bytes {
        return Err(DevError::corrupt("controller JSON is noncanonical"));
    }
    Ok(value)
}
fn copy_new(source: &Path, destination: &Path) -> Result<(), DevError> {
    super::archive::copy_new(source, destination, 0o600)
}
