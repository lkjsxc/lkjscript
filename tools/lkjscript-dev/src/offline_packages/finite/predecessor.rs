//! Exact public-authored predecessor fixtures, embedded for outside-checkout/installed proof.
use super::*;
const FILES: &[(&str, &[u8])] = &[
    (
        "expanding/HEAD",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/expanding/HEAD"
        )),
    ),
    (
        "expanding/LOCK",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/expanding/LOCK"
        )),
    ),
    (
        "expanding/catalog/current.lkjc",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/expanding/catalog/current.lkjc"
        )),
    ),
    (
        "expanding/catalog/segments/segment_7d56c5a5c52c7ca0dd8a9835277296a030a0faa2d2900337b4421188a18ec13c.lkjs",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/expanding/catalog/segments/segment_7d56c5a5c52c7ca0dd8a9835277296a030a0faa2d2900337b4421188a18ec13c.lkjs"
        )),
    ),
    (
        "expanding/packs/pack_a74c7e986e91d28a6a1712e50984e9d610953716ef8f7ec9998ba2e56daa3810.lkjp",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/expanding/packs/pack_a74c7e986e91d28a6a1712e50984e9d610953716ef8f7ec9998ba2e56daa3810.lkjp"
        )),
    ),
    (
        "expanding/packs/pack_dd19a2fcb98930d1b3c173bb40faf2af26b351215fe9d964352d756fa7297ef3.lkjp",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/expanding/packs/pack_dd19a2fcb98930d1b3c173bb40faf2af26b351215fe9d964352d756fa7297ef3.lkjp"
        )),
    ),
    (
        "valid/HEAD",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/valid/HEAD"
        )),
    ),
    (
        "valid/LOCK",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/valid/LOCK"
        )),
    ),
    (
        "valid/catalog/current.lkjc",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/valid/catalog/current.lkjc"
        )),
    ),
    (
        "valid/catalog/segments/segment_0297b9f03c28c216eab950642efe745ed1b2ac1502f1a4b7d581482366b4eb76.lkjs",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/valid/catalog/segments/segment_0297b9f03c28c216eab950642efe745ed1b2ac1502f1a4b7d581482366b4eb76.lkjs"
        )),
    ),
    (
        "valid/catalog/segments/segment_502d2eacba0768043207c82d39489357627485032ef9b4850f54450b73ec427f.lkjs",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/valid/catalog/segments/segment_502d2eacba0768043207c82d39489357627485032ef9b4850f54450b73ec427f.lkjs"
        )),
    ),
    (
        "valid/derived/compiler/CURRENT",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/valid/derived/compiler/CURRENT"
        )),
    ),
    (
        "valid/derived/compiler/LOCK",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/valid/derived/compiler/LOCK"
        )),
    ),
    (
        "valid/packs/pack_340c72b693a2ea8286bb7b74ef2fd98398fdcc45fd681aea2af8ad6267b770c5.lkjp",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/valid/packs/pack_340c72b693a2ea8286bb7b74ef2fd98398fdcc45fd681aea2af8ad6267b770c5.lkjp"
        )),
    ),
    (
        "valid/packs/pack_956018dd5567bee64f45e0a82dd4785541f7b2922d0b6c4489e8a746b9485681.lkjp",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/valid/packs/pack_956018dd5567bee64f45e0a82dd4785541f7b2922d0b6c4489e8a746b9485681.lkjp"
        )),
    ),
    (
        "valid/packs/pack_cb120017b22bc6b1717e24e5868325f7bf993f88b10effd930b00351497ffbd4.lkjp",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/valid/packs/pack_cb120017b22bc6b1717e24e5868325f7bf993f88b10effd930b00351497ffbd4.lkjp"
        )),
    ),
];
const VALID: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/finite-callable-predecessor/valid.request"
));
const EXPANDING: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/finite-callable-predecessor/expanding.request"
));

fn cli(
    context: &mut Context,
    project: &Path,
    arguments: &[&str],
    success: bool,
    installed: bool,
    indexes: &mut Vec<usize>,
) -> Result<Vec<CompactRecord>, DevError> {
    let executable = if installed {
        context.receipt.pinned_runtime_path.clone()
    } else {
        context.binary.display().to_string()
    };
    if installed {
        indexes.push(context.receipt.commands.len());
    }
    context.cli_using(
        &context.root.clone(),
        Some(project),
        arguments,
        success,
        executable,
    )
}

fn change(
    context: &mut Context,
    project: &Path,
    request: &str,
    installed: bool,
    indexes: &mut Vec<usize>,
) -> Result<Vec<CompactRecord>, DevError> {
    let path = context.evidence.join(format!(
        "finite-upgrade-{}.lkjc",
        context.receipt.commands.len()
    ));
    fs::write(&path, request)?;
    let path = path.display().to_string();
    let plan = cli(
        context,
        project,
        &["change", "plan", "--input-file", &path],
        true,
        installed,
        indexes,
    )?;
    cli(
        context,
        project,
        &[
            "change",
            "apply",
            "--input-file",
            &path,
            "--plan",
            &field(&plan, "plan", "token")?,
        ],
        true,
        installed,
        indexes,
    )
}

pub(super) fn workflow(context: &mut Context) -> Result<(), DevError> {
    let mut installed_commands = Vec::new();
    let mut result_commands = Vec::new();
    for installed in [false, true] {
        for name in ["valid", "expanding"] {
            let project = context
                .root
                .join(format!("finite-upgrade-{installed}-{name}"));
            fs::create_dir(&project)?;
            // Empty operational directories have no tracked bytes in the authentic fixture.
            fs::create_dir(project.join("PACKAGE-TRANSPORTS"))?;
            for (path, bytes) in FILES {
                if let Some(relative) = path.strip_prefix(&format!("{name}/")) {
                    let target = project.join(relative);
                    fs::create_dir_all(
                        target
                            .parent()
                            .ok_or_else(|| DevError::corrupt("fixture parent"))?,
                    )?;
                    fs::write(target, bytes)?;
                }
            }
            let before = crate::authority::observe_graph_authority(&project)?;
            let catalog = digest_file(&project.join("catalog/current.lkjc"), 1024 * 1024)?;
            let status = cli(
                context,
                &project,
                &["status"],
                true,
                installed,
                &mut installed_commands,
            )?;
            require(
                field(&status, "current-validation", "status")?
                    == if name == "valid" { "valid" } else { "invalid" },
                "historical current validity misclassified",
            )?;
            let original_result = field(&status, "revision", "id")?;
            let function = if name == "valid" {
                "decl_52e9a47ae193d7810f96bee421356087"
            } else {
                "decl_f3752a4a0730f23fce789f7844053c9e"
            };
            cli(
                context,
                &project,
                &[
                    "inspect",
                    "owner",
                    "pure_function",
                    function,
                    "--detail",
                    "definition",
                    "--limit",
                    "50",
                ],
                true,
                installed,
                &mut installed_commands,
            )?;
            require(
                crate::authority::observe_graph_authority(&project)? == before
                    && digest_file(&project.join("catalog/current.lkjc"), 1024 * 1024)? == catalog,
                "historical read or validation rewrote authority/catalog",
            )?;
            if name == "valid" {
                cli(
                    context,
                    &project,
                    &["check"],
                    true,
                    installed,
                    &mut installed_commands,
                )?;
                let retry = change(context, &project, VALID, installed, &mut installed_commands)?;
                require(
                    field(&retry, "result", "status")? == "already-accepted"
                        && field(&retry, "revision", "result")? == original_result
                        && field(&retry, "receipt", "digest")?
                            == field(&status, "receipt", "digest")?
                        && field(&retry, "receipt", "revision-record")?
                            == field(&status, "revision", "record")?,
                    "historical valid retry changed result identity",
                )?;
            } else {
                let rejected = cli(
                    context,
                    &project,
                    &["check"],
                    false,
                    installed,
                    &mut installed_commands,
                )?;
                require(
                    field(&rejected, "diagnostic", "code")? == "kernel_callable_expansion",
                    "historical expansion admitted for check",
                )?;
                let request = format!(
                    "request base={original_result} idempotency=finite-repair\nexpression.i64 as=$body value=7\nreplace.body function=probe/growing body=$body\n"
                );
                let repair = change(
                    context,
                    &project,
                    &request,
                    installed,
                    &mut installed_commands,
                )?;
                require(
                    field(&repair, "result", "status")? == "accepted",
                    "current binary repair did not publish",
                )?;
                result_commands.push(context.receipt.commands.len() - 1);
                cli(
                    context,
                    &project,
                    &["check"],
                    true,
                    installed,
                    &mut installed_commands,
                )?;
                let next = format!(
                    "request base={} idempotency=finite-later\nexpression.i64 as=$body value=8\nreplace.body function=probe/growing body=$body\n",
                    field(&repair, "revision", "result")?
                );
                change(context, &project, &next, installed, &mut installed_commands)?;
                let after = crate::authority::observe_graph_authority(&project)?;
                let retry = change(
                    context,
                    &project,
                    &request,
                    installed,
                    &mut installed_commands,
                )?;
                require(
                    field(&retry, "result", "status")? == "already-accepted"
                        && field(&retry, "revision", "result")?
                            == field(&repair, "revision", "result")?
                        && field(&retry, "receipt", "digest")?
                            == field(&repair, "receipt", "digest")?
                        && field(&retry, "receipt", "revision-record")?
                            == field(&repair, "receipt", "revision-record")?,
                    "repair retry lost original result",
                )?;
                let path = context
                    .evidence
                    .join(format!("finite-old-request-{installed}.lkjc"));
                fs::write(&path, EXPANDING)?;
                let incompatible = cli(
                    context,
                    &project,
                    &[
                        "change",
                        "plan",
                        "--input-file",
                        &path.display().to_string(),
                    ],
                    false,
                    installed,
                    &mut installed_commands,
                )?;
                require(
                    field(&incompatible, "diagnostic", "code")?
                        == "change_historical_request_incompatible"
                        && field(&incompatible, "diagnostic", "message")?
                            .contains(&original_result),
                    "historical retry lost targeted incompatibility/result information",
                )?;
                require(
                    crate::authority::observe_graph_authority(&project)? == after,
                    "retry or rejection republished historical meaning",
                )?;
            }
            fs::remove_dir_all(&project)?;
        }
    }
    context.receipt.observations.insert(
        "finite_installed_upgrade_commands".into(),
        serde_json::to_string(&installed_commands)?,
    );
    context.receipt.observations.insert(
        "finite_upgrade_repairs".into(),
        serde_json::to_string(&result_commands)?,
    );
    Ok(())
}

pub(super) fn validate(receipt: &Receipt, root: &Path) -> Result<Vec<usize>, DevError> {
    let indexes: Vec<usize> = serde_json::from_str(
        receipt
            .observations
            .get("finite_installed_upgrade_commands")
            .ok_or_else(|| DevError::corrupt("installed upgrade commands absent"))?,
    )?;
    let repairs: Vec<usize> = serde_json::from_str(
        receipt
            .observations
            .get("finite_upgrade_repairs")
            .ok_or_else(|| DevError::corrupt("current repair observations absent"))?,
    )?;
    require(
        indexes.len() >= 15 && indexes.windows(2).all(|w| w[0] < w[1]) && repairs.len() == 2,
        "upgrade command inventory incomplete",
    )?;
    for index in &indexes {
        let command = receipt
            .commands
            .get(*index)
            .ok_or_else(|| DevError::corrupt("upgrade command index missing"))?;
        require(
            command.command.first() == Some(&receipt.pinned_runtime_path)
                && command.command.get(1).is_some_and(|arg| arg == "--project")
                && command
                    .command
                    .get(2)
                    .is_some_and(|arg| Path::new(arg).starts_with(&receipt.isolated_root)),
            "upgrade did not use installed absolute runtime and owned project",
        )?;
    }
    for (installed, index) in repairs.iter().enumerate() {
        let command = receipt
            .commands
            .get(*index)
            .ok_or_else(|| DevError::corrupt("upgrade repair index absent"))?;
        require(
            indexes.contains(index) == (installed == 1),
            "copied and installed repair proofs conflated",
        )?;
        let stdout = process::read_bounded(
            &root.join(&command.observation.stdout.path),
            MAXIMUM_OUTPUT_BYTES,
        )?;
        let records = parse_records("upgrade", &stdout)
            .map_err(|e| DevError::corrupt(format!("upgrade output: {e:?}")))?;
        require(
            field(&records, "result", "status")? == "accepted"
                && field(&records, "revision", "base")?
                    == "rev_f3e49f1725234acd064a6938b764166d3696b13da570d6581302a9ba5a4e998d",
            "repair did not start from authentic invalid historical result",
        )?;
        let read = |index: usize| -> Result<Vec<CompactRecord>, DevError> {
            let command = receipt
                .commands
                .get(index)
                .ok_or_else(|| DevError::corrupt("upgrade history command absent"))?;
            let bytes = process::read_bounded(
                &root.join(&command.observation.stdout.path),
                MAXIMUM_OUTPUT_BYTES,
            )?;
            parse_records("upgrade-history", &bytes)
                .map_err(|e| DevError::corrupt(format!("upgrade history: {e:?}")))
        };
        let valid_status = read(
            index
                .checked_sub(9)
                .ok_or_else(|| DevError::corrupt("upgrade history index"))?,
        )?;
        let valid_retry = read(index - 5)?;
        let repair_retry = read(index + 5)?;
        for (retry, original, revision_field, record_operation, record_field) in [
            (&valid_retry, &valid_status, "id", "revision", "record"),
            (
                &repair_retry,
                &records,
                "result",
                "receipt",
                "revision-record",
            ),
        ] {
            require(
                field(retry, "result", "status")? == "already-accepted"
                    && field(retry, "revision", "result")?
                        == field(original, "revision", revision_field)?
                    && field(retry, "receipt", "digest")? == field(original, "receipt", "digest")?
                    && field(retry, "receipt", "revision-record")?
                        == field(original, record_operation, record_field)?,
                "upgrade retry did not retain the original accepted publication evidence",
            )?;
        }
    }
    Ok(indexes)
}
