use super::model::{Authority, LatestState};
use super::*;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

const SOURCE: &str = "1111111111111111111111111111111111111111";
const CONTROLLER: &str = "2222222222222222222222222222222222222222";
const TAG_OBJECT: &str = "3333333333333333333333333333333333333333";

#[derive(Clone, Copy, Debug)]
enum TagChange {
    Missing,
    Lightweight,
    WrongSource,
    DifferentAnnotation,
}

/// This adapter observes actual controller operations and refuses execution. There is
/// deliberately no orchestrator-supplied build/heavy-owner counter used as its oracle.
struct FakeApi {
    release: Option<Value>,
    requests: Vec<(String, String)>,
    release_create_requests: Vec<Value>,
    deny_historical_workflow_target: bool,
    uploads: Vec<String>,
    fail_upload: Option<String>,
    fail_public: bool,
    corrupt_latest: bool,
    corrupt_latest_metadata: bool,
    latest: String,
    tag_present: bool,
    tag_object: String,
    tag_source: String,
    tag_type: String,
    tag_change_after_create: Option<TagChange>,
    tag_change_after_upload: Option<(usize, TagChange)>,
    relation: String,
    artifacts: Vec<Value>,
    run: Value,
    job: Value,
    public_bytes: BTreeMap<String, Vec<u8>>,
}
impl FakeApi {
    fn new() -> Self {
        Self {release:None,requests:Vec::new(),release_create_requests:Vec::new(),deny_historical_workflow_target:false,uploads:Vec::new(),fail_upload:None,fail_public:false,corrupt_latest:false,corrupt_latest_metadata:false,latest:"v9.8.7".to_owned(),tag_present:true,tag_object:TAG_OBJECT.to_owned(),tag_source:SOURCE.to_owned(),tag_type:"tag".to_owned(),tag_change_after_create:None,tag_change_after_upload:None,relation:"ahead".to_owned(),artifacts:["assets","verifier","acceptance"].iter().enumerate().map(|(i,role)|json!({"id":i+10,"name":format!("candidate-{role}-17-2"),"expired":false,"digest":format!("sha256:{}","a".repeat(64)),"size_in_bytes":3,"expires_at":"2026-10-01T00:00:00Z","workflow_run":{"id":17,"head_sha":SOURCE}})).collect(),run:json!({"id":17,"run_attempt":2,"head_sha":SOURCE,"event":"workflow_dispatch","head_branch":"main","path":WORKFLOW,"repository":{"full_name":REPOSITORY},"head_repository":{"full_name":REPOSITORY},"status":"completed","conclusion":"success","workflow_id":41}),job:json!({"id":51,"run_id":17,"head_sha":SOURCE,"name":ACCEPTANCE_JOB,"status":"completed","conclusion":"success","steps":[{"name":"Admit the final candidate and original evidence","status":"completed","conclusion":"success"},{"name":"Upload immutable candidate assets","status":"completed","conclusion":"success"},{"name":"Upload original candidate verifier","status":"completed","conclusion":"success"},{"name":"Upload essential acceptance handoff","status":"completed","conclusion":"success"}]}),public_bytes:BTreeMap::new()}
    }

    fn change_tag(&mut self, change: TagChange) {
        match change {
            TagChange::Missing => self.tag_present = false,
            TagChange::Lightweight => self.tag_type = "commit".to_owned(),
            TagChange::WrongSource => self.tag_source = CONTROLLER.to_owned(),
            TagChange::DifferentAnnotation => self.tag_object = "5".repeat(40),
        }
    }
}
impl Operations for FakeApi {
    fn api(&mut self, method: &str, path: &str, body: Option<&Value>) -> Result<Value, DevError> {
        self.requests.push((method.to_owned(), path.to_owned()));
        if path.ends_with("/git/ref/heads/main") {
            return Ok(json!({"object":{"type":"commit","sha":CONTROLLER}}));
        }
        if let Some(comparison) = path.split("/compare/").nth(1) {
            let source = comparison.split("...").next().expect("source");
            return Ok(
                json!({"status":self.relation,"behind_by":if self.relation=="diverged"{1}else{0},"merge_base_commit":{"sha":source}}),
            );
        }
        if path.ends_with("/git/ref/tags/v9.8.8") {
            return Ok(
                json!({"object":{"type":"tag","sha":"4444444444444444444444444444444444444444"}}),
            );
        }
        if path.ends_with("/git/tags/4444444444444444444444444444444444444444") {
            return Ok(
                json!({"sha":"4444444444444444444444444444444444444444","tag":"v9.8.8","object":{"type":"commit","sha":CONTROLLER},"message":"Later public release"}),
            );
        }
        if path.ends_with("/git/ref/tags/v9.8.7") {
            if !self.tag_present {
                return Err(DevError::unavailable("fixture annotated tag is absent"));
            }
            return Ok(json!({"object":{"type":self.tag_type,"sha":self.tag_object}}));
        }
        if path.ends_with(&format!("/git/tags/{}", self.tag_object)) {
            return Ok(
                json!({"sha":self.tag_object,"tag":"v9.8.7","object":{"type":"commit","sha":self.tag_source},"message":"Useful capability"}),
            );
        }
        if path.ends_with("/actions/workflows/release.yml") {
            return Ok(json!({"id":41,"path":WORKFLOW}));
        }
        if path.ends_with("/attempts/2") {
            return Ok(self.run.clone());
        }
        if path.ends_with("/attempts/2/jobs?per_page=100") {
            return Ok(json!({"total_count":1,"jobs":[self.job.clone()]}));
        }
        if path.ends_with("/artifacts?per_page=100") {
            return Ok(json!({"total_count":self.artifacts.len(),"artifacts":self.artifacts}));
        }
        if let Some(id) = path.split("/actions/artifacts/").nth(1) {
            let id: u64 = id.parse().expect("artifact id");
            return self
                .artifacts
                .iter()
                .find(|a| a["id"] == id)
                .cloned()
                .ok_or_else(|| DevError::unavailable("fixture missing artifact"));
        }
        if path.contains("/releases?per_page=") {
            return Ok(Value::Array(self.release.clone().into_iter().collect()));
        }
        if path.ends_with("/releases/latest") {
            let mut latest = self.release.clone().expect("fixture release");
            latest["tag_name"] = json!(self.latest);
            if self.latest != "v9.8.7" {
                latest["id"] = json!(92);
            }
            if self.corrupt_latest_metadata {
                latest["assets"][0]["digest"] = json!(format!("sha256:{}", "0".repeat(64)));
            }
            return Ok(latest);
        }
        if path.ends_with("/releases/tags/v9.8.7") {
            return self
                .release
                .clone()
                .ok_or_else(|| DevError::unavailable("no fixture release"));
        }
        if method == "POST" && path.ends_with("/releases") {
            assert!(self.release.is_none());
            let mut release = body.expect("create body").clone();
            self.release_create_requests.push(release.clone());
            // GitHub's create-release contract resolves an omitted target to the
            // default branch. An explicitly selected historical workflow tree
            // needs Workflows write even though an existing tag makes this field
            // irrelevant to the release's source. GITHUB_TOKEN lacks that grant.
            let target = release
                .get("target_commitish")
                .and_then(Value::as_str)
                .unwrap_or("main");
            if self.deny_historical_workflow_target && target == SOURCE {
                return Err(DevError::unavailable(
                    "create-release fixture: Resource not accessible by integration (HTTP 403); historical target changes workflows",
                ));
            }
            if release.get("target_commitish").is_none() {
                release["target_commitish"] = json!("main");
            }
            release["id"] = json!(91);
            release["assets"] = json!([]);
            release["author"] = json!({"login":"github-actions[bot]"});
            release["html_url"] = json!("https://github.com/lkjsxc/lkjscript/releases/tag/v9.8.7");
            self.release = Some(release.clone());
            if let Some(change) = self.tag_change_after_create.take() {
                self.change_tag(change);
            }
            return Ok(release);
        }
        if method == "PATCH" && path.ends_with("/releases/91") {
            let release = self.release.as_mut().expect("fixture release");
            assert_eq!(
                body.expect("publish body"),
                &json!({"draft":false,"prerelease":false,"make_latest":"true"})
            );
            release["draft"] = json!(false);
            release["immutable"] = json!(true);
            return Ok(release.clone());
        }
        Err(DevError::corrupt(format!(
            "independent API fixture refused unexpected {method} {path}"
        )))
    }
    fn download(&mut self, path: &str, output: &Path, authenticated: bool) -> Result<(), DevError> {
        assert!(
            !authenticated,
            "this public fixture does not accept arbitrary artifact transport"
        );
        if self.fail_public {
            self.fail_public = false;
            return Err(DevError::unavailable("injected public acquisition failure"));
        }
        let name = path.rsplit('/').next().expect("asset URL");
        let bytes = self
            .public_bytes
            .get(name)
            .ok_or_else(|| DevError::corrupt("unconfigured download"))?;
        let mut received = serve_owned_asset(bytes)?;
        if self.corrupt_latest && path.contains("/latest/") {
            received.push(b'!');
        }
        super::super::archive::write_new(output, &received, 0o600)
    }
    fn upload(&mut self, id: u64, name: &str, path: &Path) -> Result<Value, DevError> {
        assert_eq!(id, 91);
        if self.fail_upload.as_deref() == Some(name) {
            self.fail_upload = None;
            return Err(DevError::unavailable("injected publication API failure"));
        }
        self.uploads.push(name.to_owned());
        let (sha, size) = super::super::archive::sha256_file(path)?;
        let asset = json!({"id":100+self.uploads.len(),"name":name,"size":size,"digest":format!("sha256:{}",sha.as_str()),"state":"uploaded"});
        self.release.as_mut().expect("created release")["assets"]
            .as_array_mut()
            .expect("assets")
            .push(asset.clone());
        if let Some((ordinal, change)) = self.tag_change_after_upload
            && self.uploads.len() == ordinal
        {
            self.tag_change_after_upload = None;
            self.change_tag(change);
        }
        Ok(asset)
    }
    fn zip_member(&mut self, _: &Path, _: Option<&str>, _: &Path) -> Result<(), DevError> {
        Err(DevError::corrupt("fixture refuses unconfigured extraction"))
    }
    fn attestation(&mut self, path: &Path, tag: &str, output: &Path) -> Result<(), DevError> {
        assert_eq!(tag, "v9.8.7");
        assert!(path.is_file());
        write_json(output, &json!({"fixture":"authenticated subject and tag"}))
    }
    fn release_attestation(&mut self, tag: &str, output: &Path) -> Result<(), DevError> {
        assert_eq!(tag, "v9.8.7");
        write_json(output, &json!({"fixture":"authenticated release"}))
    }
}

fn serve_owned_asset(bytes: &[u8]) -> Result<Vec<u8>, DevError> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let address = listener.local_addr()?;
    listener.set_nonblocking(true)?;
    let body = bytes.to_vec();
    let server = std::thread::spawn(move || -> Result<(), std::io::Error> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_read_timeout(Some(std::time::Duration::from_secs(3)))?;
                    let mut request = [0u8; 4096];
                    let count = stream.read(&mut request)?;
                    if !request[..count].starts_with(b"GET /frozen HTTP/1.1\r\n") {
                        return Err(std::io::Error::other("unexpected fixture request"));
                    }
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )?;
                    stream.write_all(&body)?;
                    return Ok(());
                }
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && std::time::Instant::now() < deadline =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(1))
                }
                Err(error) => return Err(error),
            }
        }
    });
    let received = (|| -> Result<Vec<u8>, DevError> {
        let mut stream = TcpStream::connect_timeout(&address, std::time::Duration::from_secs(3))?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(3)))?;
        stream.write_all(
            b"GET /frozen HTTP/1.1\r\nHost: fixture.invalid\r\nConnection: close\r\n\r\n",
        )?;
        let mut response = Vec::new();
        stream.take(MAX_ARTIFACT + 1).read_to_end(&mut response)?;
        let boundary = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .ok_or_else(|| DevError::corrupt("fixture HTTP header missing"))?;
        if !response.starts_with(b"HTTP/1.1 200 OK\r\n") {
            return Err(DevError::unavailable("fixture public acquisition failed"));
        }
        Ok(response[boundary + 4..].to_vec())
    })();
    server
        .join()
        .map_err(|_| DevError::infrastructure("owned HTTP fixture thread panicked"))??;
    received
}
fn content(root: &Path) -> CandidateContent {
    fs::create_dir(root.join("assets")).expect("assets root");
    let mut assets = Vec::new();
    for (name, bytes) in [
        (
            super::super::archive::ARCHIVE_NAME,
            b"unexecutable archive\n".as_slice(),
        ),
        ("SHA256SUMS", b"fixed checksum\n"),
        ("install.sh", b"#!/bin/sh\ntouch forbidden-canary\n"),
    ] {
        let path = root.join("assets").join(name);
        fs::write(&path, bytes).expect("asset fixture");
        let (sha, byte_length) = super::super::archive::sha256_file(&path).expect("asset identity");
        assets.push(FileIdentity {
            name: name.to_owned(),
            sha256: sha.as_str().to_owned(),
            byte_length,
        });
    }
    CandidateContent {
        source_commit: SOURCE.to_owned(),
        tag: "v9.8.7".to_owned(),
        verifier: FileIdentity {
            name: "lkjscript-dev".to_owned(),
            sha256: "f".repeat(64),
            byte_length: 42,
        },
        assets,
        acceptance_contract: "fixture".to_owned(),
        target_policy_sha256: "b".repeat(64),
    }
}
fn selection(root: &Path) -> Selection {
    Selection {
        format: SELECTION_FORMAT.to_owned(),
        controller_source: CONTROLLER.to_owned(),
        consumer_run_id: 18,
        consumer_run_attempt: 1,
        producer: Producer {
            repository: REPOSITORY.to_owned(),
            run_id: 17,
            run_attempt: 2,
            source_commit: SOURCE.to_owned(),
            workflow_id: 41,
            workflow_path: WORKFLOW.to_owned(),
            acceptance_job_id: 51,
            artifacts: Vec::new(),
        },
        content: content(root),
    }
}
fn context() -> Context {
    Context {
        source: CONTROLLER.to_owned(),
        run_id: 18,
        run_attempt: 1,
    }
}
fn authority(api: &mut FakeApi, selection: &Selection) -> Authority {
    publication::authorize(api, selection, &context(), TAG_OBJECT).expect("authorized fixture")
}

#[test]
fn controller_authenticates_exact_producer_and_rejects_self_asserted_success() {
    let original = FakeApi::new();
    type Mutation = Box<dyn Fn(&mut FakeApi)>;
    let cases: Vec<Mutation> = vec![
        Box::new(|a| a.run["repository"]["full_name"] = json!("foreign/lkjscript")),
        Box::new(|a| a.run["head_repository"]["full_name"] = json!("fork/lkjscript")),
        Box::new(|a| a.run["event"] = json!("pull_request")),
        Box::new(|a| a.run["path"] = json!(".github/workflows/foreign.yml")),
        Box::new(|a| a.run["id"] = json!(99)),
        Box::new(|a| a.run["run_attempt"] = json!(1)),
        Box::new(|a| a.run["workflow_id"] = json!(99)),
        Box::new(|a| a.run["conclusion"] = json!("failure")),
        Box::new(|a| a.job["conclusion"] = json!("skipped")),
        Box::new(|a| a.job["conclusion"] = json!("cancelled")),
        Box::new(|a| a.job["steps"][0]["conclusion"] = json!("skipped")),
        Box::new(|a| a.artifacts[0]["expired"] = json!(true)),
        Box::new(|a| a.artifacts[0]["workflow_run"]["head_sha"] = json!(CONTROLLER)),
        Box::new(|a| a.artifacts.clear()),
    ];
    let mut positive = original;
    let accepted =
        authenticate_producer(&mut positive, 17, 2, CONTROLLER).expect("genuine producer");
    assert_eq!(accepted.run_attempt, 2);
    assert_eq!(accepted.artifacts.len(), 3);
    for mutate in cases {
        let mut api = FakeApi::new();
        mutate(&mut api);
        assert!(authenticate_producer(&mut api, 17, 2, CONTROLLER).is_err());
        assert!(api.requests.iter().all(|(method, _)| method == "GET"));
    }
}

#[test]
fn publication_authority_is_independent_of_candidate_bytes_and_controller_revision() {
    let temporary = tempfile::tempdir().expect("owned fixtures");
    let selection = selection(temporary.path());
    let before = selection.content.clone();
    let mut api = FakeApi::new();
    let accepted = authority(&mut api, &selection);
    assert_eq!(accepted.product_source, SOURCE);
    assert_eq!(accepted.controller_source, CONTROLLER);
    for authorization in ["", SOURCE, "0000000000000000000000000000000000000000"] {
        assert!(publication::authorize(&mut api, &selection, &context(), authorization).is_err());
    }
    api.tag_type = "commit".to_owned();
    assert!(publication::authorize(&mut api, &selection, &context(), TAG_OBJECT).is_err());
    api.tag_type = "tag".to_owned();
    api.tag_source = CONTROLLER.to_owned();
    assert!(publication::authorize(&mut api, &selection, &context(), TAG_OBJECT).is_err());
    api.tag_source = SOURCE.to_owned();
    api.relation = "diverged".to_owned();
    assert!(publication::authorize(&mut api, &selection, &context(), TAG_OBJECT).is_err());
    assert_eq!(selection.content, before);
    assert!(api.requests.iter().all(|(method, _)| method == "GET"));
}

#[test]
fn existing_annotated_tag_publication_needs_no_historical_workflow_target() {
    let temporary = tempfile::tempdir().expect("owned existing-tag fixture");
    let selection = selection(temporary.path());
    let mut api = FakeApi::new();
    api.deny_historical_workflow_target = true;
    let authority = authority(&mut api, &selection);

    let published = publication::publish(&mut api, &selection, temporary.path(), &authority)
        .expect("publish exact existing tag without requesting workflow modification");

    assert!(published.immutable);
    assert_eq!(published.latest, LatestState::Selected);
    assert_eq!(published.latest_source_commit, SOURCE);
    assert_eq!(authority.product_source, SOURCE);
    assert_eq!(authority.controller_source, CONTROLLER);
    assert_eq!(authority.annotated_tag_object, TAG_OBJECT);
    assert_eq!(api.tag_source, SOURCE);
    assert_eq!(api.tag_object, TAG_OBJECT);
    assert_eq!(api.release_create_requests.len(), 1);
    let request = &api.release_create_requests[0];
    assert_eq!(request["tag_name"], "v9.8.7");
    assert!(request.get("target_commitish").is_none());
    assert_eq!(request["draft"], true);
    let release = api.release.as_ref().expect("published release");
    assert_eq!(release["target_commitish"], "main");
    assert_eq!(release["tag_name"], "v9.8.7");
    assert_eq!(api.uploads.len(), 3);
    verify_files(&temporary.path().join("assets"), &selection.content.assets)
        .expect("publication retains exact accepted bytes");
}

#[test]
fn publication_rechecks_annotated_tag_before_create_upload_and_publication() {
    for boundary in ["before-create", "after-create", "after-final-upload"] {
        for change in [
            TagChange::Missing,
            TagChange::Lightweight,
            TagChange::WrongSource,
            TagChange::DifferentAnnotation,
        ] {
            let temporary = tempfile::tempdir().expect("owned tag-race fixture");
            let selection = selection(temporary.path());
            let mut api = FakeApi::new();
            let authority = authority(&mut api, &selection);
            match boundary {
                "before-create" => api.change_tag(change),
                "after-create" => api.tag_change_after_create = Some(change),
                "after-final-upload" => api.tag_change_after_upload = Some((3, change)),
                _ => unreachable!(),
            }
            let error = publication::publish(&mut api, &selection, temporary.path(), &authority)
                .expect_err("changed tag cannot authorize publication");
            let expected_reason = match change {
                TagChange::Missing => "fixture annotated tag is absent",
                TagChange::Lightweight => "requires an annotated tag",
                TagChange::WrongSource => "annotated tag name/object/source differs",
                TagChange::DifferentAnnotation => "scoped immutable-release authorization",
            };
            assert!(
                error.message().contains(expected_reason),
                "{boundary}/{change:?}: {error}"
            );
            assert!(api.requests.iter().all(|(method, _)| method != "PATCH"));
            if boundary == "before-create" {
                assert!(api.requests.iter().all(|(method, _)| method == "GET"));
                assert!(api.release.is_none());
                assert!(api.uploads.is_empty());
            } else {
                let draft = api.release.as_ref().expect("preserved owned draft");
                assert_eq!(draft["draft"], true);
                assert_eq!(draft["target_commitish"], "main");
                assert_eq!(api.release_create_requests.len(), 1);
                assert_eq!(
                    api.uploads.len(),
                    if boundary == "after-create" { 0 } else { 3 }
                );
            }
            verify_files(&temporary.path().join("assets"), &selection.content.assets)
                .expect("tag races do not modify accepted assets");
        }
    }
}

#[test]
fn default_target_draft_still_requires_exact_producer_source_object_and_author() {
    for changed in ["producer", "source", "tag-object", "notes", "author"] {
        let temporary = tempfile::tempdir().expect("owned foreign-draft fixture");
        let selection = selection(temporary.path());
        let mut api = FakeApi::new();
        let authority = authority(&mut api, &selection);
        api.fail_upload = Some(super::super::archive::ARCHIVE_NAME.to_owned());
        let error = publication::publish(&mut api, &selection, temporary.path(), &authority)
            .expect_err("retain draft before first asset upload");
        assert!(error.message().contains("injected publication API failure"));
        let release = api.release.as_mut().expect("owned draft");
        assert_eq!(release["target_commitish"], "main");
        let body = release["body"].as_str().expect("exact ownership notes");
        let replacement = match changed {
            "producer" => Some(body.replace("17/2", "17/1")),
            "source" => Some(body.replace(SOURCE, CONTROLLER)),
            "tag-object" => Some(body.replace(TAG_OBJECT, &"5".repeat(40))),
            "notes" => Some(format!("Foreign annotation\n{body}")),
            "author" => None,
            _ => unreachable!(),
        };
        if let Some(body) = replacement {
            release["body"] = json!(body);
        } else {
            release["author"]["login"] = json!("foreign-operator");
        }
        let before = api.release.clone();
        let requests = api.requests.len();
        let error = publication::publish(&mut api, &selection, temporary.path(), &authority)
            .expect_err("foreign draft cannot be resumed");
        assert!(
            error.message().contains("partial draft is foreign"),
            "{changed}: {error}"
        );
        assert_eq!(api.release, before);
        assert!(
            api.requests[requests..]
                .iter()
                .all(|(method, _)| method == "GET")
        );
        assert!(api.uploads.is_empty());
    }
}

#[test]
fn publication_failure_resumes_only_missing_uploads_and_never_executes_handoff_canary() {
    let temporary = tempfile::tempdir().expect("owned fixture");
    let selection = selection(temporary.path());
    let mut api = FakeApi::new();
    let authority = authority(&mut api, &selection);
    api.fail_upload = Some("SHA256SUMS".to_owned());
    assert!(publication::publish(&mut api, &selection, temporary.path(), &authority).is_err());
    assert_eq!(api.uploads, [super::super::archive::ARCHIVE_NAME]);
    let published = publication::publish(&mut api, &selection, temporary.path(), &authority)
        .expect("resume exact draft");
    assert!(published.immutable);
    assert_eq!(api.uploads.len(), 3);
    let writes = api
        .requests
        .iter()
        .filter(|(method, _)| method != "GET")
        .count();
    publication::publish(&mut api, &selection, temporary.path(), &authority)
        .expect("immutable idempotent resume");
    assert_eq!(
        api.requests
            .iter()
            .filter(|(method, _)| method != "GET")
            .count(),
        writes
    );
    assert_eq!(api.uploads.len(), 3);
    assert!(!temporary.path().join("forbidden-canary").exists());
    verify_files(&temporary.path().join("assets"), &selection.content.assets)
        .expect("all promoted assets unchanged");
}

#[test]
fn conflicting_draft_or_published_asset_rejects_without_mutation() {
    for published in [false, true] {
        let temporary = tempfile::tempdir().expect("owned fixture");
        let selection = selection(temporary.path());
        let mut api = FakeApi::new();
        let authority = authority(&mut api, &selection);
        publication::publish(&mut api, &selection, temporary.path(), &authority)
            .expect("prepare fixture release");
        let release = api.release.as_mut().expect("fixture release");
        release["draft"] = json!(!published);
        release["assets"][0]["digest"] = json!(format!("sha256:{}", "0".repeat(64)));
        let before = api.release.clone();
        let writes = api
            .requests
            .iter()
            .filter(|(method, _)| method != "GET")
            .count();
        assert!(publication::publish(&mut api, &selection, temporary.path(), &authority).is_err());
        assert_eq!(api.release, before);
        assert_eq!(
            api.requests
                .iter()
                .filter(|(method, _)| method != "GET")
                .count(),
            writes
        );
    }
}

#[test]
fn public_failure_retries_only_public_boundary_and_latest_never_certifies_other_bytes() {
    let temporary = tempfile::tempdir().expect("owned fixture");
    let selection = selection(temporary.path());
    let mut api = FakeApi::new();
    let authority = authority(&mut api, &selection);
    publication::publish(&mut api, &selection, temporary.path(), &authority)
        .expect("published fixture");
    for asset in &selection.content.assets {
        api.public_bytes.insert(
            asset.name.clone(),
            fs::read(temporary.path().join("assets").join(&asset.name)).expect("frozen asset"),
        );
    }
    let failed = temporary.path().join("public-failed");
    fs::create_dir(&failed).expect("failed boundary");
    api.fail_public = true;
    assert!(publication::public_download(&mut api, &selection, &failed, true).is_err());
    let resumed = temporary.path().join("public-resumed");
    fs::create_dir(&resumed).expect("resume boundary");
    let published = publication::public_download(&mut api, &selection, &resumed, true)
        .expect("resume public only");
    assert_eq!(published.latest, LatestState::Selected);
    assert_eq!(api.uploads.len(), 3);
    for route in ["exact", "latest"] {
        verify_files(&resumed.join(route), &selection.content.assets)
            .expect("exact accepted asset bytes");
    }
    api.latest = "v9.8.8".to_owned();
    let superseded = temporary.path().join("superseded");
    fs::create_dir(&superseded).expect("superseded boundary");
    assert!(publication::public_download(&mut api, &selection, &superseded, true).is_err());
    let observed = publication::public_download(&mut api, &selection, &superseded, false)
        .expect("explicit exact-only recheck");
    assert_eq!(observed.latest, LatestState::Superseded);
    assert!(!superseded.join("latest").exists());
}

#[test]
fn changed_local_asset_rejects_before_first_remote_write() {
    let temporary = tempfile::tempdir().expect("owned fixture");
    let selection = selection(temporary.path());
    let mut api = FakeApi::new();
    let authority = authority(&mut api, &selection);
    fs::write(
        temporary.path().join("assets/install.sh"),
        b"one-byte-change",
    )
    .expect("tamper");
    assert!(publication::publish(&mut api, &selection, temporary.path(), &authority).is_err());
    assert!(api.requests.iter().all(|(method, _)| method == "GET"));
    assert!(api.uploads.is_empty());
}

#[test]
fn controller_process_adapter_refuses_every_product_or_verifier_command() {
    // The production subprocess adapter's allowlist is an independent barrier;
    // the API fixture cannot quietly add an execution fallback during retries.
    let temporary = tempfile::tempdir().expect("owned fixture");
    let mut operations =
        HostedOperations::new(temporary.path().to_path_buf()).expect("host adapter");
    for program in [
        "cargo",
        "lkjscript",
        "lkjscript-dev",
        "install.sh",
        "sh",
        "bash",
    ] {
        assert!(operations.reject_forbidden_fixture(program).is_err());
    }
}

#[test]
fn compare_errors_are_not_false_ancestry_and_known_git_relations_remain_distinct() {
    struct Comparison {
        results: VecDeque<Result<Value, DevError>>,
    }
    impl Operations for Comparison {
        fn api(&mut self, _: &str, _: &str, _: Option<&Value>) -> Result<Value, DevError> {
            self.results.pop_front().expect("single comparison")
        }
        fn download(&mut self, _: &str, _: &Path, _: bool) -> Result<(), DevError> {
            panic!("no download")
        }
        fn upload(&mut self, _: u64, _: &str, _: &Path) -> Result<Value, DevError> {
            panic!("no write")
        }
        fn zip_member(&mut self, _: &Path, _: Option<&str>, _: &Path) -> Result<(), DevError> {
            panic!("no extraction")
        }
        fn attestation(&mut self, _: &Path, _: &str, _: &Path) -> Result<(), DevError> {
            panic!("no attestation")
        }
        fn release_attestation(&mut self, _: &str, _: &Path) -> Result<(), DevError> {
            panic!("no release attestation")
        }
    }
    for (relation, behind, admitted) in [
        ("identical", 0, true),
        ("ahead", 0, true),
        ("behind", 1, false),
        ("diverged", 1, false),
    ] {
        let mut api = Comparison {
            results: VecDeque::from([Ok(
                json!({"status":relation,"behind_by":behind,"merge_base_commit":{"sha":SOURCE}}),
            )]),
        };
        assert_eq!(
            require_ancestor(&mut api, SOURCE, CONTROLLER).is_ok(),
            admitted
        );
    }
    let mut failed = Comparison {
        results: VecDeque::from([Err(DevError::unavailable("API failure"))]),
    };
    assert_eq!(
        require_ancestor(&mut failed, SOURCE, CONTROLLER)
            .expect_err("transport failure")
            .kind(),
        "unavailable"
    );
}

fn portable_terminal(content: &CandidateContent) -> Value {
    let observation = json!({"status":"passed","exit_code":0,"signal":null,"reason":null,"elapsed_nanoseconds":1,"cpu_nanoseconds":null,"peak_rss_kib":null,"stdout_limit_bytes":16777216,"stderr_limit_bytes":16777216,"stdout_limit_exhausted":false,"stderr_limit_exhausted":false,"stdout":{"path":"retained.log","kind":"file","mode":420,"bytes":0,"digest":"blake3:af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262","link_target":null},"stderr":{"path":"retained.log","kind":"file","mode":420,"bytes":0,"digest":"blake3:af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262","link_target":null}});
    let proof = json!({"name":"receipt.json","sha256":"2".repeat(64),"byte_length":8});
    json!({"schema":{"identity":"lkjscript-candidate-terminal","version":1},"status":"candidate_accepted","phase":"complete","source_commit":SOURCE,"controller_source_commit":SOURCE,"tag":content.tag,"acceptance_contract":"lkjscript-final-candidate-acceptance-1","workload":"release-source+six-target-owners+two-pinned-userlands+installed-recovery-1","target_triple":"x86_64-unknown-linux-musl","target_policy_sha256":super::super::target::policy_sha256().expect("policy"),"producer":{"github_actions":"true","repository":REPOSITORY,"workflow":"Release","job":"candidate","run_id":"17","run_attempt":"2","run_url":"https://github.com/lkjsxc/lkjscript/actions/runs/17","runner_os":"Linux","runner_architecture":"X64","runner_image_os":"ubuntu24","runner_image_version":"fixture-image"},"verifier":content.verifier,"assets":content.assets,"manifest_sha256":"3".repeat(64),"executable":{"name":"lkjscript","sha256":"4".repeat(64),"byte_length":8},"source_gates":20,"target_owners":6,"userlands":2,"proofs":[{"name":"release-source","receipt":proof},{"name":"final-target","receipt":proof},{"name":"installation","receipt":proof}],"stages":[{"name":"target-admission","process":observation},{"name":"installation","process":observation},{"name":"installation-reader","process":observation}],"started_unix_nanoseconds":1,"completed_unix_nanoseconds":2,"elapsed_nanoseconds":1,"cleanup_complete":true,"failure":null})
}

// Independent uncompressed ZIP fixture writer: production uses the platform unzip
// reader and service digest, never this encoder. Exact names and bytes are explicit.
fn zip_fixture(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut local = Vec::new();
    let mut central = Vec::new();
    let put16 = |out: &mut Vec<u8>, value: u16| out.extend(value.to_le_bytes());
    let put32 = |out: &mut Vec<u8>, value: u32| out.extend(value.to_le_bytes());
    for (name, bytes) in files {
        let offset = local.len() as u32;
        let mut crc = 0xffff_ffffu32;
        for byte in bytes {
            crc ^= *byte as u32;
            for _ in 0..8 {
                crc = (crc >> 1) ^ if crc & 1 == 1 { 0xedb8_8320 } else { 0 };
            }
        }
        crc = !crc;
        put32(&mut local, 0x04034b50);
        put16(&mut local, 20);
        put16(&mut local, 0);
        put16(&mut local, 0);
        put16(&mut local, 0);
        put16(&mut local, 0);
        put32(&mut local, crc);
        put32(&mut local, bytes.len() as u32);
        put32(&mut local, bytes.len() as u32);
        put16(&mut local, name.len() as u16);
        put16(&mut local, 0);
        local.extend(name.as_bytes());
        local.extend(bytes);
        put32(&mut central, 0x02014b50);
        put16(&mut central, 20);
        put16(&mut central, 20);
        put16(&mut central, 0);
        put16(&mut central, 0);
        put16(&mut central, 0);
        put16(&mut central, 0);
        put32(&mut central, crc);
        put32(&mut central, bytes.len() as u32);
        put32(&mut central, bytes.len() as u32);
        put16(&mut central, name.len() as u16);
        for _ in 0..4 {
            put16(&mut central, 0);
        }
        put32(&mut central, 0);
        put32(&mut central, offset);
        central.extend(name.as_bytes());
    }
    let offset = local.len() as u32;
    let size = central.len() as u32;
    local.extend(central);
    put32(&mut local, 0x06054b50);
    put16(&mut local, 0);
    put16(&mut local, 0);
    put16(&mut local, files.len() as u16);
    put16(&mut local, files.len() as u16);
    put32(&mut local, size);
    put32(&mut local, offset);
    put16(&mut local, 0);
    local
}

struct HandoffApi {
    service: FakeApi,
    zips: BTreeMap<String, Vec<u8>>,
    zip_reader: HostedOperations,
}
impl Operations for HandoffApi {
    fn api(&mut self, method: &str, path: &str, body: Option<&Value>) -> Result<Value, DevError> {
        if path.ends_with("/actions/runs/18/attempts/1") {
            let mut value = self.service.run.clone();
            value["id"] = json!(18);
            value["run_attempt"] = json!(1);
            value["head_sha"] = json!(CONTROLLER);
            value["status"] = json!("in_progress");
            value["conclusion"] = Value::Null;
            return Ok(value);
        }
        self.service.api(method, path, body)
    }
    fn download(&mut self, path: &str, output: &Path, authenticated: bool) -> Result<(), DevError> {
        if authenticated {
            let bytes = self
                .zips
                .get(path)
                .ok_or_else(|| DevError::unavailable("missing exact ZIP fixture"))?;
            return super::super::archive::write_new(output, bytes, 0o600);
        }
        self.service.download(path, output, false)
    }
    fn upload(&mut self, id: u64, name: &str, path: &Path) -> Result<Value, DevError> {
        self.service.upload(id, name, path)
    }
    fn zip_member(
        &mut self,
        path: &Path,
        name: Option<&str>,
        output: &Path,
    ) -> Result<(), DevError> {
        self.zip_reader.zip_member(path, name, output)
    }
    fn attestation(&mut self, path: &Path, tag: &str, output: &Path) -> Result<(), DevError> {
        self.service.attestation(path, tag, output)
    }
    fn release_attestation(&mut self, tag: &str, output: &Path) -> Result<(), DevError> {
        self.service.release_attestation(tag, output)
    }
}

#[test]
fn actual_selection_dispatch_uses_authenticated_zip_bytes_and_portable_reader_before_authority() {
    let temporary = tempfile::tempdir().expect("owned dispatch fixture");
    let root = temporary.path();
    let mut content = content(root);
    let canary = root.join("candidate-verifier-canary");
    fs::write(
        &canary,
        b"#!/bin/sh\ntouch forbidden-verifier-execution\nexit 73\n",
    )
    .expect("canary");
    fs::set_permissions(&canary, fs::Permissions::from_mode(0o755)).expect("canary mode");
    let verifier = root.join("verifier-original");
    super::super::verifier::command(
        [
            "prepare".to_owned(),
            "--executable".to_owned(),
            canary.to_string_lossy().into_owned(),
            "--output".to_owned(),
            verifier.to_string_lossy().into_owned(),
            "--tag".to_owned(),
            content.tag.clone(),
            "--commit".to_owned(),
            SOURCE.to_owned(),
        ]
        .into_iter()
        .map(OsString::from),
    )
    .expect("canonical verifier identity fixture");
    let (sha, byte_length) = super::super::archive::sha256_file(&canary).expect("canary identity");
    content.verifier = FileIdentity {
        name: "lkjscript-dev".to_owned(),
        sha256: sha.as_str().to_owned(),
        byte_length,
    };
    let terminal = portable_terminal(&content);
    let mut api = HandoffApi {
        service: FakeApi::new(),
        zips: BTreeMap::new(),
        zip_reader: HostedOperations::new(root.join("zip-reader"))
            .expect("real bounded ZIP reader"),
    };
    for (index, role) in ["assets", "verifier", "acceptance"].into_iter().enumerate() {
        let files = match role {
            "assets" => content
                .assets
                .iter()
                .map(|asset| {
                    (
                        asset.name.clone(),
                        fs::read(root.join("assets").join(&asset.name)).expect("asset"),
                    )
                })
                .collect(),
            "verifier" => ["lkjscript-dev", "verifier-identity.json"]
                .into_iter()
                .map(|name| {
                    (
                        name.to_owned(),
                        fs::read(verifier.join(name)).expect("verifier"),
                    )
                })
                .collect(),
            _ => vec![(
                "release-receipt.json".to_owned(),
                super::super::candidate::canonical_terminal_fixture(terminal.clone())
                    .expect("canonical terminal fixture"),
            )],
        };
        let zip = zip_fixture(&files);
        let file = root.join(format!("{role}.zip"));
        fs::write(&file, &zip).expect("ZIP fixture");
        let (sha, bytes) = super::super::archive::sha256_file(&file).expect("service identity");
        api.service.artifacts[index]["digest"] = json!(format!("sha256:{}", sha.as_str()));
        api.service.artifacts[index]["size_in_bytes"] = json!(bytes);
        api.zips.insert(
            format!("repos/{REPOSITORY}/actions/artifacts/{}/zip", index + 10),
            zip,
        );
    }
    let selected = root.join("selected");
    fs::create_dir(&selected).expect("selection root");
    let options = BTreeMap::from([
        ("producer-run".to_owned(), "17".to_owned()),
        ("producer-attempt".to_owned(), "2".to_owned()),
        ("output".to_owned(), selected.to_string_lossy().into_owned()),
    ]);
    let result = execute("select", &options, &selected, &context(), &mut api)
        .expect("actual controller selection dispatcher");
    assert_eq!(result["status"], "candidate_accepted");
    let selection: Selection =
        read_json(&selected.join("selection.json")).expect("selected producer");
    assert_eq!(selection.producer.run_attempt, 2);
    assert_eq!(selection.content.verifier.sha256, content.verifier.sha256);
    let original =
        fs::read(selected.join("acceptance/release-receipt.json")).expect("original terminal");
    let mut tampered: Value = serde_json::from_slice(&original).expect("terminal");
    tampered["target_owners"] = json!(5);
    fs::write(
        selected.join("acceptance/release-receipt.json"),
        super::super::candidate::canonical_terminal_fixture(tampered)
            .expect("tampered canonical receipt"),
    )
    .expect("tamper");
    assert!(validate_selection(&mut api, &selection, &selected, CONTROLLER).is_err());
    fs::write(selected.join("acceptance/release-receipt.json"), &original)
        .expect("restore original");
    validate_selection(&mut api, &selection, &selected, CONTROLLER)
        .expect("restored original recovers");
    let original_installer = fs::read(selected.join("assets/install.sh")).expect("original asset");
    let mut forged = selection.clone();
    fs::write(
        selected.join("assets/install.sh"),
        b"plausible different bootstrap",
    )
    .expect("alter asset");
    let (sha, byte_length) =
        super::super::archive::sha256_file(&selected.join("assets/install.sh"))
            .expect("rehashed forgery");
    forged.content.assets[2].sha256 = sha.as_str().to_owned();
    forged.content.assets[2].byte_length = byte_length;
    let mut forged_terminal: Value = serde_json::from_slice(&original).expect("terminal");
    forged_terminal["assets"][2] =
        serde_json::to_value(&forged.content.assets[2]).expect("plausible identity");
    fs::write(
        selected.join("acceptance/release-receipt.json"),
        super::super::candidate::canonical_terminal_fixture(forged_terminal)
            .expect("plausible rehashed terminal"),
    )
    .expect("forge local terminal");
    assert!(
        validate_selection(&mut api, &forged, &selected, CONTROLLER).is_err(),
        "rehashing local claims cannot replace service-authenticated ZIP originals"
    );
    fs::write(selected.join("assets/install.sh"), original_installer).expect("restore asset");
    fs::write(selected.join("acceptance/release-receipt.json"), &original)
        .expect("restore terminal");
    let authority = publication::authorize(&mut api, &selection, &context(), TAG_OBJECT)
        .expect("independent scoped authority");
    publication::publish(&mut api, &selection, &selected, &authority)
        .expect("actual privileged operation consumes only data");
    assert!(!root.join("forbidden-verifier-execution").exists());
    assert!(!selected.join("forbidden-canary").exists());
    assert!(
        api.zip_reader
            .reject_forbidden_fixture(canary.to_str().expect("canary path"))
            .is_err()
    );
}

#[test]
fn exact_and_latest_wrong_source_metadata_and_bytes_are_independent_rejections() {
    for fault in [
        "exact-source",
        "exact-bytes",
        "latest-metadata",
        "latest-bytes",
    ] {
        let temporary = tempfile::tempdir().expect("owned public identity fixture");
        let selection = selection(temporary.path());
        let mut api = FakeApi::new();
        let authority = authority(&mut api, &selection);
        publication::publish(&mut api, &selection, temporary.path(), &authority)
            .expect("frozen release");
        for asset in &selection.content.assets {
            api.public_bytes.insert(
                asset.name.clone(),
                fs::read(temporary.path().join("assets").join(&asset.name)).expect("frozen bytes"),
            );
        }
        match fault {
            "exact-source" => api.tag_source = CONTROLLER.to_owned(),
            "exact-bytes" => {
                api.public_bytes
                    .get_mut("install.sh")
                    .expect("bootstrap")
                    .push(b'!');
            }
            "latest-metadata" => api.corrupt_latest_metadata = true,
            "latest-bytes" => api.corrupt_latest = true,
            _ => unreachable!(),
        }
        let output = temporary.path().join("public");
        fs::create_dir(&output).expect("public evidence");
        assert!(
            publication::public_download(&mut api, &selection, &output, true).is_err(),
            "{fault}"
        );
        assert!(!temporary.path().join("forbidden-canary").exists());
        assert_eq!(api.uploads.len(), 3);
    }
}

fn workflow_script(step: &str) -> String {
    let workflow = include_str!("../../../../../.github/workflows/release.yml");
    let block = workflow
        .split_once(&format!("      - name: {step}\n"))
        .expect("actual workflow step")
        .1
        .split_once("        run: |\n")
        .expect("actual Bash owner")
        .1;
    let mut script = String::new();
    for line in block.lines() {
        if line.is_empty() {
            script.push('\n');
        } else if let Some(line) = line.strip_prefix("          ") {
            script.push_str(line);
            script.push('\n');
        } else {
            break;
        }
    }
    assert!(script.starts_with("set -euo pipefail\n"));
    script
}

fn workflow_process(
    script: &str,
    root: &Path,
    environment: &[(&str, &str)],
) -> std::process::Output {
    let mut command = std::process::Command::new("/bin/bash");
    command
        .args(["--noprofile", "--norc", "-c", script])
        .current_dir(root)
        .env_clear()
        .envs(crate::process::environment())
        .env("RUNNER_TEMP", root)
        .env("GITHUB_ACTIONS", "true")
        .env("GITHUB_REPOSITORY", REPOSITORY)
        .env("GITHUB_WORKFLOW", "Release")
        .env("GITHUB_JOB", "public")
        .env("GITHUB_RUN_ID", "18")
        .env("GITHUB_RUN_ATTEMPT", "1");
    for (name, value) in environment {
        command.env(name, value);
    }
    command
        .output()
        .expect("execute actual trusted workflow boundary in owned fixture")
}

#[test]
fn actual_workflow_authority_requires_a_completed_consistent_decision() {
    let script = workflow_script("Independently admit current publication authority and occupancy");
    let authorized = r#"{"status":"promotion_authorized","authority":{}}"#;
    let rejected =
        r#"{"status":"rejected","operation":"authority","reason":"annotated tag source differs"}"#;
    let unavailable =
        r#"{"status":"unavailable","operation":"authority","reason":"API unavailable"}"#;
    let cancelled = r#"{"status":"cancelled","operation":"authority","reason":"cancelled after joined cleanup"}"#;
    let mut mismatches = Vec::new();
    for (label, exit, state, consume_passes, output) in [
        ("authorized", 0, Some(authorized), true, "authorized"),
        ("rejected", 1, Some(rejected), true, "rejected"),
        ("unavailable", 1, Some(unavailable), false, "unavailable"),
        ("cancelled", 1, Some(cancelled), false, "cancelled"),
        ("missing", 1, None, false, ""),
        ("malformed", 1, Some("{"), false, ""),
        (
            "unknown",
            1,
            Some(r#"{"status":"unexpected","operation":"authority","reason":"unknown"}"#),
            false,
            "",
        ),
        (
            "incomplete",
            1,
            Some(r#"{"status":"incomplete","operation":"authority"}"#),
            false,
            "",
        ),
        (
            "wrong-operation",
            1,
            Some(r#"{"status":"rejected","operation":"select","reason":"wrong producer"}"#),
            false,
            "",
        ),
        (
            "missing-reason",
            1,
            Some(r#"{"status":"rejected","operation":"authority"}"#),
            false,
            "",
        ),
        (
            "empty-reason",
            1,
            Some(r#"{"status":"rejected","operation":"authority","reason":""}"#),
            false,
            "",
        ),
        (
            "malformed-reason",
            1,
            Some(r#"{"status":"rejected","operation":"authority","reason":[]}"#),
            false,
            "",
        ),
        (
            "missing-status",
            1,
            Some(r#"{"operation":"authority","reason":"missing status"}"#),
            false,
            "",
        ),
        (
            "malformed-status",
            1,
            Some(r#"{"status":["rejected"],"operation":"authority","reason":"bad status"}"#),
            false,
            "",
        ),
        (
            "newline-status",
            1,
            Some(r#"{"status":"rejected\n","operation":"authority","reason":"bad status"}"#),
            false,
            "",
        ),
        (
            "nul-status",
            1,
            Some(r#"{"status":"rejected\u0000","operation":"authority","reason":"bad status"}"#),
            false,
            "",
        ),
        (
            "multiple-results",
            1,
            Some(&format!("{rejected}\n{unavailable}")),
            false,
            "",
        ),
        ("successful-rejection", 0, Some(rejected), false, ""),
        ("successful-unavailable", 0, Some(unavailable), false, ""),
        ("successful-cancelled", 0, Some(cancelled), false, ""),
        ("successful-missing", 0, None, false, ""),
        ("successful-malformed", 0, Some("{"), false, ""),
        (
            "successful-multiple",
            0,
            Some(&format!("{authorized}\n{rejected}")),
            false,
            "",
        ),
        ("failed-authorization", 1, Some(authorized), false, ""),
    ] {
        for operation in ["consume", "promote", "resume-publication"] {
            let temporary = tempfile::tempdir().expect("owned authority fixture");
            let root = temporary.path();
            fs::create_dir(root.join("controller-tool")).expect("fixture controller directory");
            let controller = root.join("controller-tool/lkjscript-release-controller");
            fs::write(
                &controller,
                br##"#!/bin/sh
set -eu
test "$#" = 5
test "$1" = authority
test "$2" = --selection && test "$3" = "$RUNNER_TEMP/selection/selection.json"
test "$4" = --output && test "$5" = "$RUNNER_TEMP/authority"
printf '%s\n' "$*" >> "$RUNNER_TEMP/invocations"
mkdir "$5"
if test -f "$RUNNER_TEMP/fixture-state.json"; then
  cp "$RUNNER_TEMP/fixture-state.json" "$5/state.json"
fi
exit "$FIXTURE_EXIT"
"##,
            )
            .expect("independent authority process adapter");
            fs::set_permissions(&controller, fs::Permissions::from_mode(0o755))
                .expect("adapter mode");
            if let Some(state) = state {
                fs::write(root.join("fixture-state.json"), state)
                    .expect("controller state fixture");
            }
            let output_path = root.join("github-output");
            fs::write(&output_path, "").expect("step output fixture");
            let result = workflow_process(
                &script,
                root,
                &[
                    ("OPERATION", operation),
                    ("FIXTURE_EXIT", &exit.to_string()),
                    (
                        "GITHUB_OUTPUT",
                        output_path.to_str().expect("fixture output path"),
                    ),
                ],
            );
            let expected = consume_passes && (operation == "consume" || exit == 0);
            let actual_output = fs::read_to_string(&output_path).expect("actual step output");
            let expected_output = if output.is_empty() {
                String::new()
            } else {
                format!("status={output}\n")
            };
            if result.status.success() != expected || actual_output != expected_output {
                mismatches.push(format!("{operation}/{label}: success={}, output={actual_output:?}, expected success={expected}, output={expected_output:?}", result.status.success()));
            }
            assert_eq!(
                fs::read_to_string(root.join("invocations"))
                    .expect("independent invocation trace")
                    .lines()
                    .count(),
                1
            );
            if let Some(state) = state {
                assert_eq!(
                    fs::read_to_string(root.join("authority/state.json"))
                        .expect("retained diagnostic")
                        .as_str(),
                    state
                );
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn actual_workflow_terminal_owner_cannot_pass_missing_skipped_failed_or_cancelled_required_jobs() {
    let script = workflow_script("Require every stage selected by this invocation");
    let mut mismatches = Vec::new();
    for (operation, candidate, controller, publication, public, authority, passes) in [
        (
            "candidate",
            "success",
            "skipped",
            "skipped",
            "skipped",
            "",
            true,
        ),
        (
            "candidate",
            "skipped",
            "skipped",
            "skipped",
            "skipped",
            "",
            false,
        ),
        (
            "candidate",
            "failure",
            "skipped",
            "skipped",
            "skipped",
            "",
            false,
        ),
        (
            "candidate",
            "cancelled",
            "skipped",
            "skipped",
            "skipped",
            "",
            false,
        ),
        (
            "consume", "skipped", "success", "skipped", "skipped", "rejected", true,
        ),
        (
            "consume",
            "skipped",
            "success",
            "skipped",
            "skipped",
            "authorized",
            true,
        ),
        (
            "consume",
            "skipped",
            "success",
            "skipped",
            "skipped",
            "unavailable",
            false,
        ),
        (
            "consume",
            "skipped",
            "success",
            "skipped",
            "skipped",
            "cancelled",
            false,
        ),
        (
            "consume",
            "skipped",
            "success",
            "skipped",
            "skipped",
            "incomplete",
            false,
        ),
        (
            "consume", "skipped", "success", "skipped", "skipped", "", false,
        ),
        (
            "consume", "skipped", "success", "skipped", "skipped", "{}", false,
        ),
        (
            "consume", "skipped", "success", "skipped", "skipped", "unknown", false,
        ),
        (
            "consume",
            "skipped",
            "success",
            "skipped",
            "skipped",
            "promotion_authorized",
            false,
        ),
        (
            "consume", "skipped", "", "skipped", "skipped", "rejected", false,
        ),
        (
            "consume", "skipped", "skipped", "skipped", "skipped", "rejected", false,
        ),
        (
            "consume",
            "skipped",
            "cancelled",
            "skipped",
            "skipped",
            "rejected",
            false,
        ),
        (
            "consume", "skipped", "failure", "skipped", "skipped", "rejected", false,
        ),
        (
            "promote",
            "skipped",
            "success",
            "success",
            "success",
            "authorized",
            true,
        ),
        (
            "promote",
            "skipped",
            "success",
            "success",
            "skipped",
            "authorized",
            false,
        ),
        (
            "promote",
            "skipped",
            "success",
            "skipped",
            "success",
            "authorized",
            false,
        ),
        (
            "promote",
            "skipped",
            "success",
            "success",
            "failure",
            "authorized",
            false,
        ),
        (
            "promote",
            "skipped",
            "success",
            "success",
            "cancelled",
            "authorized",
            false,
        ),
        (
            "promote", "skipped", "success", "success", "success", "rejected", false,
        ),
        (
            "resume-publication",
            "skipped",
            "success",
            "success",
            "success",
            "authorized",
            true,
        ),
        (
            "resume-publication",
            "skipped",
            "success",
            "failure",
            "skipped",
            "authorized",
            false,
        ),
        (
            "resume-public",
            "skipped",
            "success",
            "skipped",
            "success",
            "",
            true,
        ),
        (
            "resume-public",
            "skipped",
            "success",
            "skipped",
            "unavailable",
            "",
            false,
        ),
        (
            "resume-public",
            "skipped",
            "skipped",
            "skipped",
            "success",
            "",
            false,
        ),
    ] {
        let temporary = tempfile::tempdir().expect("owned terminal fixture");
        let result = workflow_process(
            &script,
            temporary.path(),
            &[
                ("OPERATION", operation),
                ("CANDIDATE", candidate),
                ("CONTROLLER", controller),
                ("PUBLISH", publication),
                ("PUBLIC", public),
                ("AUTHORITY", authority),
                ("LATEST_STATE", "superseded"),
                ("LATEST_TAG", "v9.8.8"),
                ("LATEST_SOURCE", CONTROLLER),
            ],
        );
        let terminal: Value = serde_json::from_slice(
            &fs::read(temporary.path().join("release-terminal.json"))
                .expect("failed and successful terminal retained"),
        )
        .expect("actual terminal JSON");
        assert_eq!(terminal["public_verification"], public);
        assert_eq!(terminal["promotion_authority"], authority);
        assert_eq!(terminal["controller"], controller);
        assert_eq!(terminal["latest"], "superseded");
        let expected_status = if !passes {
            "incomplete"
        } else {
            match operation {
                "candidate" => "candidate_accepted",
                "consume" => "candidate_consumed_read_only",
                "promote" | "resume-publication" => "immutable_published_and_public_verified",
                "resume-public" => "public_recheck_passed",
                _ => unreachable!(),
            }
        };
        if result.status.success() != passes || terminal["status"] != expected_status {
            mismatches.push(format!("{operation}/{candidate}/{controller}/{publication}/{public}/{authority}: success={}, status={}, expected success={passes}, status={expected_status}", result.status.success(), terminal["status"]));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn actual_public_smoke_script_resumes_failed_smoke_without_build_or_broad_owner_invocations() {
    for boundary in ["transfer", "public-pair", "public-exact"] {
        let transfer = boundary == "transfer";
        let exact_only = boundary == "public-exact";
        let script = workflow_script(if transfer {
            "Admit transfer and run the small installed lifecycle"
        } else {
            "Verify the actual public installed lifecycle"
        });
        let temporary = tempfile::tempdir().expect("owned smoke boundary fixture");
        let root = temporary.path();
        fs::create_dir_all(root.join("selection/verifier")).expect("fixture verifier handoff");
        fs::create_dir(root.join("selection/assets")).expect("fixture original assets");
        fs::write(
            root.join("selection/assets/unchanged"),
            b"original asset bytes",
        )
        .expect("fixture candidate bytes");
        fs::create_dir_all(root.join("public-acquisition/exact")).expect("exact acquisition");
        if !exact_only {
            fs::create_dir(root.join("public-acquisition/latest")).expect("latest acquisition");
        }
        let verifier = root.join("selection/verifier/lkjscript-dev");
        fs::write(&verifier,br##"#!/bin/sh
set -eu
fixture_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
test "$#" -gt 3
test "$1" = release && test "$2" = transferred
case "$3" in pair-run|exact-run) ;; *) exit 93 ;; esac
for argument in "$@"; do
  case "$argument" in build|admit|installation-run|distributed-http|outbound-http|offline-packages|pure-tail|stateful-http) exit 94 ;; esac
done
printf '%s\n' "$*" >> "$fixture_root/invocations"
if test -f "$fixture_root/fail-next-smoke"; then
  rm "$fixture_root/fail-next-smoke"
  exit 42
fi
printf '{"status":"passed","fixture":"boundary process adapter only"}\n'
"##).expect("independent process adapter");
        fs::set_permissions(&verifier, fs::Permissions::from_mode(0o755)).expect("adapter mode");
        let (sha, bytes) =
            super::super::archive::sha256_file(&verifier).expect("adapter expected identity");
        write_json(&root.join("selection/selection.json"),&json!({"content":{"tag":"v9.8.7","source_commit":SOURCE,"verifier":{"sha256":sha.as_str(),"byte_length":bytes}},"producer":{"run_id":17,"run_attempt":2}})).expect("bound original producer fixture");
        fs::write(
            root.join("selection/verifier/verifier-identity.json"),
            b"fixture identity",
        )
        .expect("identity");
        fs::write(root.join("fail-next-smoke"), b"one real process failure").expect("inject once");
        let guards = root.join("forbidden-tools");
        fs::create_dir(&guards).expect("independent tool guards");
        for program in [
            "cargo",
            "rustc",
            "lkjscript",
            "distributed-http",
            "outbound-http",
            "offline-packages",
            "pure-tail",
            "stateful-http",
            "gh",
            "curl",
        ] {
            let guard = guards.join(program);
            fs::write(&guard, b"#!/bin/sh\nprintf '%s\\n' \"$0\" >> \"$(dirname -- \"$0\")/forbidden.log\"\nexit 94\n").expect("forbidden tool adapter");
            fs::set_permissions(guard, fs::Permissions::from_mode(0o755)).expect("guard mode");
        }
        let path = format!(
            "{}:{}",
            guards.display(),
            std::env::var("PATH").expect("fixture PATH")
        );
        let before = fs::read(root.join("selection/selection.json")).expect("original selection");
        let failure = workflow_process(&script, root, &[("PATH", &path)]);
        assert_eq!(
            failure.status.code(),
            Some(42),
            "{}",
            String::from_utf8_lossy(&failure.stderr)
        );
        let terminal_script = workflow_script("Require every stage selected by this invocation");
        let terminal_environment = |controller| {
            [
                ("OPERATION", "consume"),
                ("CANDIDATE", "skipped"),
                ("CONTROLLER", controller),
                ("PUBLISH", "skipped"),
                ("PUBLIC", "skipped"),
                ("AUTHORITY", "rejected"),
                ("LATEST_STATE", ""),
                ("LATEST_TAG", ""),
                ("LATEST_SOURCE", ""),
            ]
        };
        if transfer {
            let terminal =
                workflow_process(&terminal_script, root, &terminal_environment("failure"));
            assert!(
                !terminal.status.success(),
                "failed transfer cannot be consumed"
            );
            let state: Value = serde_json::from_slice(
                &fs::read(root.join("release-terminal.json")).expect("failed consume terminal"),
            )
            .expect("terminal JSON");
            assert_eq!(state["status"], "incomplete");
            // A fresh hosted invocation has a new runner directory; preserve the selected
            // producer while removing only this fixture's previous simulated acquisition.
            fs::remove_dir_all(root.join("transfer-latest")).expect("owned failed transfer copy");
        }
        let recovery = workflow_process(&script, root, &[("PATH", &path), ("GITHUB_RUN_ID", "19")]);
        assert!(
            recovery.status.success(),
            "{}",
            String::from_utf8_lossy(&recovery.stderr)
        );
        if transfer {
            let terminal =
                workflow_process(&terminal_script, root, &terminal_environment("success"));
            assert!(
                terminal.status.success(),
                "recovered transfer can be consumed"
            );
            let state: Value = serde_json::from_slice(
                &fs::read(root.join("release-terminal.json")).expect("recovered consume terminal"),
            )
            .expect("terminal JSON");
            assert_eq!(state["status"], "candidate_consumed_read_only");
            assert_eq!(state["promotion_authority"], "rejected");
            assert_eq!(
                fs::read(root.join("transfer-latest/unchanged")).expect("transferred bytes"),
                b"original asset bytes"
            );
        }
        assert_eq!(
            fs::read(root.join("selection/assets/unchanged")).expect("original bytes"),
            b"original asset bytes"
        );
        assert_eq!(
            fs::read(root.join("selection/selection.json")).expect("retained selection"),
            before
        );
        assert!(!guards.join("forbidden.log").exists());
        let invocations =
            fs::read_to_string(root.join("invocations")).expect("independent process trace");
        let lines: Vec<_> = invocations.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], lines[1]);
        assert!(lines[0].starts_with(if exact_only {
            "release transferred exact-run "
        } else {
            "release transferred pair-run "
        }));
        assert!(lines[0].contains(SOURCE));
        assert!(lines[0].contains(if transfer {
            "--acquisition simulated"
        } else {
            "--acquisition anonymous"
        }));
        assert_eq!(lines[0].contains("--latest-assets"), !exact_only);
    }
}
