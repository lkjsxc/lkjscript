//! Explicit current-format source republication with a pinned accepted HEAD.

use super::*;
use std::io::Read;

pub(super) fn execute(
    options: &[String],
    project: Option<PathBuf>,
) -> Result<Vec<u8>, Diagnostic> {
    ensure_options(options, &["--kind", "--transport", "--input-file", "--expected-head"], &[])?;
    if required_option(options, "--kind")? != "transport" {
        return Err(usage_error("package current republish --kind requires transport"));
    }
    let root = project.ok_or_else(|| usage_error("package current republish requires explicit --project PATH"))?;
    let transport = required_option(options, "--transport")?
        .parse::<PackageTransportDigest>()
        .map_err(|error| direct_option_error("--transport", error))?;
    let path = required_option(options, "--input-file")?;
    let expected = required_option(options, "--expected-head")?;
    let source = read_complete(Path::new(&path))?;
    let receipt = GraphRepository::republish_current_transport(
        &root, &expected, transport, &source, &ExecutionControl::uncancelled(),
    )?;
    let mut output = compact_response_writer()?;
    append_compact_record(&mut output, "result", &[
        ("status", "success".to_owned()),
        ("command", "package.current.republish".to_owned()),
        ("history", "fresh-current-format".to_owned()),
    ])?;
    append_compact_record(&mut output, "authority", &[
        ("previous-revision", receipt.previous.revision.to_string()),
        ("current-revision", receipt.current.revision.to_string()),
        ("previous-head-sha256", receipt.previous_head_sha256),
        ("current-head-sha256", receipt.current_head_sha256),
        ("semantic-state", receipt.semantic_state.to_string()),
        ("source-sha256", receipt.source_sha256),
        ("identities", "preserved".to_owned()),
    ])?;
    append_compact_record(&mut output, "inventory", &[
        ("owners", receipt.owners.to_string()),
        ("types", receipt.types.to_string()),
        ("retirements", receipt.retirements.to_string()),
        ("blobs", receipt.blobs.to_string()),
        ("dependencies", receipt.dependencies.to_string()),
        ("store-work-saturated", receipt.store_work.saturated.to_string()),
    ])?;
    Ok(output.finish())
}

fn read_complete(path: &Path) -> Result<Vec<u8>, Diagnostic> {
    let fd = rustix::fs::open(
        path,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::NONBLOCK | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    ).map_err(|error| Diagnostic::new(
        DiagnosticClass::Infrastructure, "package_republish_input",
        format!("cannot open complete source '{}': {error}", path.display()),
    ))?;
    let mut file = std::fs::File::from(fd);
    if !file.metadata().map_err(|error| Diagnostic::new(
        DiagnosticClass::Infrastructure, "package_republish_input", error.to_string(),
    ))?.is_file() {
        return Err(Diagnostic::new(
            DiagnosticClass::Source, "package_republish_input",
            "complete source input must be a regular file",
        ));
    }
    let mut result = Vec::new();
    let mut chunk = [0_u8; 65_536];
    loop {
        let count = file.read(&mut chunk).map_err(|error| Diagnostic::new(
            DiagnosticClass::Infrastructure, "package_republish_input", error.to_string(),
        ))?;
        if count == 0 { return Ok(result); }
        result.try_reserve(count).map_err(|_| Diagnostic::new(
            DiagnosticClass::Resource, "package_republish_input_allocation",
            "cannot allocate complete source input storage",
        ))?;
        result.extend_from_slice(&chunk[..count]);
    }
}
