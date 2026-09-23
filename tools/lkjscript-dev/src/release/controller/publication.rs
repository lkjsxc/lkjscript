use super::model::{Authority, LatestState, Published, REPOSITORY};
use super::{
    CandidateContent, Context, DevError, FileIdentity, Operations, Selection, array, json, number,
    remote_main, require_ancestor, string, validate_sha, verify_files, write_json,
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(super) fn authorize(
    operations: &mut impl Operations,
    selection: &Selection,
    context: &Context,
    authorization: &str,
) -> Result<Authority, DevError> {
    let content = &selection.content;
    super::super::verifier::validate_tag(&content.tag)?;
    let main = remote_main(operations)?;
    require_ancestor(operations, &content.source_commit, &main)?;
    require_ancestor(operations, &context.source, &main)?;
    let reference = operations.api(
        "GET",
        &format!("repos/{REPOSITORY}/git/ref/tags/{}", content.tag),
        None,
    )?;
    if reference.pointer("/object/type").and_then(Value::as_str) != Some("tag") {
        return Err(DevError::corrupt(
            "publication requires an annotated tag, not a lightweight ref",
        ));
    }
    let object = reference
        .pointer("/object/sha")
        .and_then(Value::as_str)
        .ok_or_else(|| DevError::corrupt("tag object identity absent"))?
        .to_owned();
    validate_sha(&object, 40)?;
    let tag = operations.api(
        "GET",
        &format!("repos/{REPOSITORY}/git/tags/{object}"),
        None,
    )?;
    if string(&tag, "sha")? != object
        || string(&tag, "tag")? != content.tag
        || tag.pointer("/object/type").and_then(Value::as_str) != Some("commit")
        || tag.pointer("/object/sha").and_then(Value::as_str)
            != Some(content.source_commit.as_str())
    {
        return Err(DevError::corrupt(
            "annotated tag name/object/source differs from the accepted candidate; a main descendant cannot substitute",
        ));
    }
    if authorization != object {
        return Err(DevError::corrupt(
            "scoped immutable-release authorization is missing or differs from the exact annotated tag object",
        ));
    }
    let notes = string(&tag, "message")?.to_owned();
    if notes.trim().is_empty() || notes.len() > 65536 {
        return Err(DevError::corrupt(
            "annotated release notes are empty or exceed bounds",
        ));
    }
    Ok(Authority {
        controller_source: context.source.clone(),
        product_source: content.source_commit.clone(),
        main_source: main,
        annotated_tag_object: object,
        tag: content.tag.clone(),
        producer_run_id: selection.producer.run_id,
        producer_run_attempt: selection.producer.run_attempt,
        release_notes: notes,
    })
}

fn ownership(authority: &Authority) -> String {
    format!(
        "<!-- lkjscript-candidate {}/{} source={} tag-object={} -->",
        authority.producer_run_id,
        authority.producer_run_attempt,
        authority.product_source,
        authority.annotated_tag_object
    )
}
fn notes(authority: &Authority) -> String {
    format!(
        "{}\n\n{}\n",
        authority.release_notes.trim_end(),
        ownership(authority)
    )
}

fn load_release(operations: &mut impl Operations, tag: &str) -> Result<Option<Value>, DevError> {
    let mut selected = None;
    // Bounded, complete pagination is deliberate: draft lookup must not mistake a
    // missing public tag endpoint for absent occupancy, and never deletes state.
    for page in 1..=10 {
        let page = operations.api(
            "GET",
            &format!("repos/{REPOSITORY}/releases?per_page=100&page={page}"),
            None,
        )?;
        let releases = page
            .as_array()
            .ok_or_else(|| DevError::corrupt("release list is not an array"))?;
        if releases.len() > 100 {
            return Err(DevError::corrupt("release listing exceeds requested bound"));
        }
        for release in releases {
            if string(release, "tag_name")? == tag && selected.replace(release.clone()).is_some() {
                return Err(DevError::corrupt(
                    "multiple releases occupy the intended tag",
                ));
            }
        }
        if releases.len() < 100 {
            return Ok(selected);
        }
    }
    Err(DevError::unavailable(
        "release occupancy exceeds bounded discovery; no write performed",
    ))
}

fn verify_remote_assets(
    release: &Value,
    content: &CandidateContent,
    complete: bool,
) -> Result<(), DevError> {
    let assets = array(release, "assets")?;
    let mut names = BTreeSet::new();
    if (complete && assets.len() != content.assets.len()) || assets.len() > content.assets.len() {
        return Err(DevError::corrupt(
            "remote release inventory is incomplete or contains extra assets",
        ));
    }
    for asset in assets {
        let name = string(asset, "name")?;
        if !names.insert(name) {
            return Err(DevError::corrupt(
                "remote release has duplicate asset names",
            ));
        }
        let expected = content
            .assets
            .iter()
            .find(|identity| identity.name == name)
            .ok_or_else(|| DevError::corrupt("remote release has foreign assets"))?;
        verify_remote_asset(asset, expected)?;
    }
    Ok(())
}
fn verify_remote_asset(asset: &Value, expected: &FileIdentity) -> Result<(), DevError> {
    if string(asset, "name")? != expected.name
        || number(asset, "size")? != expected.byte_length
        || string(asset, "digest")? != format!("sha256:{}", expected.sha256)
        || string(asset, "state")? != "uploaded"
        || number(asset, "id")? == 0
    {
        return Err(DevError::corrupt(format!(
            "remote immutable asset conflicts: {}",
            expected.name
        )));
    }
    Ok(())
}
fn admit_remote(
    release: &Value,
    selection: &Selection,
    authority: &Authority,
) -> Result<bool, DevError> {
    if string(release, "tag_name")? != authority.tag
        || string(release, "name")? != authority.tag
        || release.get("prerelease").and_then(Value::as_bool) != Some(false)
        || number(release, "id")? == 0
    {
        return Err(DevError::corrupt(
            "occupied release has a conflicting identity",
        ));
    }
    let draft = release
        .get("draft")
        .and_then(Value::as_bool)
        .ok_or_else(|| DevError::corrupt("release draft state unavailable"))?;
    if draft {
        // GitHub's target_commitish is tag-creation metadata and may name the
        // default branch even for this existing annotated tag. The actual tag
        // object/source is independently admitted; exact notes bind draft ownership.
        if string(release, "body")? != notes(authority)
            || release.pointer("/author/login").and_then(Value::as_str)
                != Some("github-actions[bot]")
        {
            return Err(DevError::corrupt(
                "partial draft is foreign or lacks exact producer/source/ownership binding",
            ));
        }
    } else if release.get("immutable").and_then(Value::as_bool) != Some(true) {
        return Err(DevError::corrupt(
            "existing published release is not immutable",
        ));
    }
    verify_remote_assets(release, &selection.content, !draft)?;
    Ok(draft)
}

pub(super) fn inspect_occupancy(
    operations: &mut impl Operations,
    selection: &Selection,
    authority: &Authority,
) -> Result<(), DevError> {
    if let Some(release) = load_release(operations, &authority.tag)? {
        admit_remote(&release, selection, authority)?;
    }
    Ok(())
}

pub(super) fn publish(
    operations: &mut impl Operations,
    selection: &Selection,
    root: &Path,
    authority: &Authority,
) -> Result<Published, DevError> {
    verify_files(&root.join("assets"), &selection.content.assets)?;
    let context = Context {
        source: authority.controller_source.clone(),
        run_id: selection.consumer_run_id,
        run_attempt: selection.consumer_run_attempt,
    };
    let mut release = match load_release(operations, &authority.tag)? {
        Some(release) => {
            admit_remote(&release, selection, authority)?;
            release
        }
        None => {
            authorize(
                operations,
                selection,
                &context,
                &authority.annotated_tag_object,
            )?;
            // The authorized annotated tag already exists. Do not send the unused
            // tag-creation target: an old workflow tree there would unnecessarily
            // require Workflows write permission from the publication token.
            let request = json!({
                "tag_name": authority.tag,
                "name": authority.tag,
                "body": notes(authority),
                "draft": true,
                "prerelease": false,
            });
            operations.api(
                "POST",
                &format!("repos/{REPOSITORY}/releases"),
                Some(&request),
            )?
        }
    };
    let draft = admit_remote(&release, selection, authority)?;
    if draft {
        for expected in &selection.content.assets {
            if array(&release, "assets")?
                .iter()
                .any(|asset| asset.get("name").and_then(Value::as_str) == Some(&expected.name))
            {
                continue;
            }
            // Recheck local immutable bytes and actual occupied remote bytes before each
            // missing upload; no overwrite, delete, retag, or broad test operation exists.
            verify_files(&root.join("assets"), &selection.content.assets)?;
            authorize(
                operations,
                selection,
                &context,
                &authority.annotated_tag_object,
            )?;
            let uploaded = operations.upload(
                number(&release, "id")?,
                &expected.name,
                &root.join("assets").join(&expected.name),
            )?;
            verify_remote_asset(&uploaded, expected)?;
            release=load_release(operations,&authority.tag)?.ok_or_else(||DevError::unavailable("created draft is not yet visible; resume this boundary with the original candidate"))?;
            if !admit_remote(&release, selection, authority)? {
                return Err(DevError::corrupt(
                    "draft was published concurrently before the selected publication boundary",
                ));
            }
        }
        verify_remote_assets(&release, &selection.content, true)?;
        // Refresh tag authority immediately before irreversible publication.
        let fresh = authorize(
            operations,
            selection,
            &context,
            &authority.annotated_tag_object,
        )?;
        if fresh.annotated_tag_object != authority.annotated_tag_object
            || fresh.product_source != authority.product_source
        {
            return Err(DevError::corrupt(
                "publication authority changed during transfer",
            ));
        }
        release = operations.api(
            "PATCH",
            &format!("repos/{REPOSITORY}/releases/{}", number(&release, "id")?),
            Some(&json!({"draft":false,"prerelease":false,"make_latest":"true"})),
        )?;
    }
    if admit_remote(&release, selection, authority)? {
        return Err(DevError::unavailable(
            "publication still reports draft; retry this boundary without rebuilding",
        ));
    }
    published_state(operations, &release, &selection.content)
}

fn published_state(
    operations: &mut impl Operations,
    release: &Value,
    content: &CandidateContent,
) -> Result<Published, DevError> {
    let latest = operations.api("GET", &format!("repos/{REPOSITORY}/releases/latest"), None)?;
    let latest_tag = string(&latest, "tag_name")?.to_owned();
    if latest.get("draft").and_then(Value::as_bool) != Some(false)
        || latest.get("immutable").and_then(Value::as_bool) != Some(true)
        || latest.get("prerelease").and_then(Value::as_bool) != Some(false)
    {
        return Err(DevError::corrupt(
            "latest metadata is not an immutable ordinary release",
        ));
    }
    let selected = latest_tag == content.tag;
    if selected {
        if number(&latest, "id")? != number(release, "id")? {
            return Err(DevError::corrupt(
                "latest claims selected tag with a different release identity",
            ));
        }
        verify_remote_assets(&latest, content, true)?;
    }
    super::super::verifier::validate_tag(&latest_tag)?;
    let reference = operations.api(
        "GET",
        &format!("repos/{REPOSITORY}/git/ref/tags/{latest_tag}"),
        None,
    )?;
    if reference.pointer("/object/type").and_then(Value::as_str) != Some("tag") {
        return Err(DevError::corrupt("latest source tag is not annotated"));
    }
    let object = reference
        .pointer("/object/sha")
        .and_then(Value::as_str)
        .ok_or_else(|| DevError::corrupt("latest tag identity missing"))?;
    validate_sha(object, 40)?;
    let tag = operations.api(
        "GET",
        &format!("repos/{REPOSITORY}/git/tags/{object}"),
        None,
    )?;
    if tag.pointer("/object/type").and_then(Value::as_str) != Some("commit")
        || string(&tag, "tag")? != latest_tag
    {
        return Err(DevError::corrupt(
            "latest annotated tag cannot identify its product source",
        ));
    }
    let latest_source_commit = tag
        .pointer("/object/sha")
        .and_then(Value::as_str)
        .ok_or_else(|| DevError::corrupt("latest source absent"))?
        .to_owned();
    validate_sha(&latest_source_commit, 40)?;
    if selected && latest_source_commit != content.source_commit {
        return Err(DevError::corrupt(
            "latest source differs from selected candidate",
        ));
    }
    Ok(Published {
        release_id: number(release, "id")?,
        release_url: string(release, "html_url")?.to_owned(),
        immutable: true,
        latest: if selected {
            LatestState::Selected
        } else {
            LatestState::Superseded
        },
        latest_tag,
        latest_release_id: number(&latest, "id")?,
        latest_source_commit,
    })
}

fn admit_public_release(release: &Value, tag: &str) -> Result<u64, DevError> {
    let id = number(release, "id")?;
    if id == 0
        || release.get("draft").and_then(Value::as_bool) != Some(false)
        || release.get("immutable").and_then(Value::as_bool) != Some(true)
        || release.get("prerelease").and_then(Value::as_bool) != Some(false)
        || string(release, "tag_name")? != tag
    {
        return Err(DevError::corrupt(
            "public exact release is not the selected immutable release",
        ));
    }
    Ok(id)
}

pub(super) fn public_download(
    operations: &mut impl Operations,
    selection: &Selection,
    output: &Path,
    latest_required: bool,
) -> Result<Published, DevError> {
    let tag = &selection.content.tag;
    // Read immutable exact identity independently from the mutable latest alias.
    let reference = operations.api(
        "GET",
        &format!("repos/{REPOSITORY}/git/ref/tags/{tag}"),
        None,
    )?;
    if reference.pointer("/object/type").and_then(Value::as_str) != Some("tag") {
        return Err(DevError::corrupt("public exact tag is not annotated"));
    }
    let tag_sha = reference
        .pointer("/object/sha")
        .and_then(Value::as_str)
        .ok_or_else(|| DevError::corrupt("public tag object absent"))?;
    validate_sha(tag_sha, 40)?;
    let object = operations.api(
        "GET",
        &format!("repos/{REPOSITORY}/git/tags/{tag_sha}"),
        None,
    )?;
    if object.pointer("/object/type").and_then(Value::as_str) != Some("commit")
        || object.pointer("/object/sha").and_then(Value::as_str)
            != Some(selection.content.source_commit.as_str())
        || string(&object, "tag")? != tag
    {
        return Err(DevError::corrupt(
            "public exact release selects a different source",
        ));
    }
    let alias = operations.api(
        "GET",
        &format!("repos/{REPOSITORY}/releases/tags/{tag}"),
        None,
    )?;
    let release_id = admit_public_release(&alias, tag)?;
    // The tag lookup can omit assets that are present at the same release ID.
    // Admit every advertised entry, then require the complete immutable inventory
    // at that exact ID; contradictory alias metadata is never a fallback trigger.
    verify_remote_assets(&alias, &selection.content, false)?;
    let release = operations.api(
        "GET",
        &format!("repos/{REPOSITORY}/releases/{release_id}"),
        None,
    )?;
    if admit_public_release(&release, tag)? != release_id {
        return Err(DevError::corrupt(
            "public exact release identity changed after tag lookup",
        ));
    }
    verify_remote_assets(&release, &selection.content, true)?;
    let state = published_state(operations, &release, &selection.content)?;
    if state.latest == LatestState::Superseded && latest_required {
        return Err(DevError::corrupt(format!(
            "latest selected {}; this invocation requires selected candidate {tag}",
            state.latest_tag
        )));
    }
    operations.release_attestation(tag, &output.join("release-attestation.json"))?;
    for route in ["exact", "latest"] {
        if route == "latest" && state.latest == LatestState::Superseded {
            continue;
        }
        let directory = output.join(route);
        fs::create_dir(&directory)?;
        for asset in &selection.content.assets {
            let url = if route == "exact" {
                format!(
                    "https://github.com/{REPOSITORY}/releases/download/{tag}/{}",
                    asset.name
                )
            } else {
                format!(
                    "https://github.com/{REPOSITORY}/releases/latest/download/{}",
                    asset.name
                )
            };
            operations.download(&url, &directory.join(&asset.name), false)?;
        }
        // Exact bytes are authenticated before any downstream read-only job executes them.
        verify_files(&directory, &selection.content.assets)?;
        for asset in &selection.content.assets {
            operations.attestation(
                &directory.join(&asset.name),
                tag,
                &output.join(format!("{route}-{}-attestation.json", asset.name)),
            )?;
        }
    }
    let final_state = published_state(operations, &release, &selection.content)?;
    if final_state != state {
        return Err(DevError::unavailable(
            "latest moved during acquisition; preserve the exact result and explicitly repeat the mutable alias boundary",
        ));
    }
    write_json(&output.join("public-identity.json"), &state)?;
    Ok(state)
}
