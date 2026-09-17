use super::model::REPOSITORY;
use crate::error::DevError;
use crate::process::{self, ProcessSpec, ProcessStatus};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub(super) const MAX_JSON: u64 = 8 * 1024 * 1024;
pub(super) const MAX_ARTIFACT: u64 = 1024 * 1024 * 1024;

/// The same bounded operations interface is used by hosted operation and failure fixtures.
/// It has no build, application, candidate, or transferred-verifier execution operation.
pub(super) trait Operations {
    fn api(&mut self, method: &str, path: &str, body: Option<&Value>) -> Result<Value, DevError>;
    fn download(&mut self, path: &str, output: &Path, authenticated: bool) -> Result<(), DevError>;
    fn upload(&mut self, release_id: u64, name: &str, path: &Path) -> Result<Value, DevError>;
    fn zip_member(&mut self, zip: &Path, name: Option<&str>, output: &Path)
    -> Result<(), DevError>;
    fn attestation(&mut self, path: &Path, tag: &str, output: &Path) -> Result<(), DevError>;
    fn release_attestation(&mut self, tag: &str, output: &Path) -> Result<(), DevError>;
}

pub(super) struct HostedOperations {
    root: PathBuf,
    sequence: u64,
    pub(super) control: process::ProcessControl,
}

impl HostedOperations {
    pub(super) fn new(root: PathBuf) -> Result<Self, DevError> {
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            sequence: 0,
            control: process::ProcessControl::default(),
        })
    }

    #[cfg(test)]
    pub(super) fn reject_forbidden_fixture(&mut self, program: &str) -> Result<PathBuf, DevError> {
        self.run(vec![program.to_owned()], false, MAX_JSON)
    }

    fn run(
        &mut self,
        command: Vec<String>,
        authenticated: bool,
        maximum: u64,
    ) -> Result<PathBuf, DevError> {
        let program = command.first().map(String::as_str);
        if !matches!(program, Some("gh" | "curl" | "unzip")) {
            return Err(DevError::corrupt(
                "controller attempted a forbidden process",
            ));
        }
        if authenticated && program != Some("gh") {
            return Err(DevError::corrupt(
                "only the trusted GitHub CLI receives authentication",
            ));
        }
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or_else(|| DevError::infrastructure("controller process accounting overflow"))?;
        let stdout = self.root.join(format!("{}-stdout", self.sequence));
        let stderr = self.root.join(format!("{}-stderr", self.sequence));
        let mut environment = process::environment();
        // Programs from public assets never enter this process allowlist. Anonymous curl
        // has neither GitHub credentials nor user configuration/automatic netrc support.
        if authenticated {
            let token = std::env::var("GH_TOKEN").map_err(|_| {
                DevError::unavailable("GH_TOKEN is required for authenticated release control")
            })?;
            environment.insert("GH_TOKEN".to_owned(), token);
        }
        environment.insert("GH_PROMPT_DISABLED".to_owned(), "1".to_owned());
        environment.insert("GH_HOST".to_owned(), "github.com".to_owned());
        let spec = ProcessSpec {
            command,
            cwd: self.root.clone(),
            environment,
            timeout: Duration::from_secs(240),
            maximum_stdout_bytes: maximum,
            maximum_stderr_bytes: 1024 * 1024,
            stdout_path: stdout.clone(),
            stderr_path: stderr,
            unavailable_exit_code: None,
        };
        let observed = process::run_supervised(&spec, &self.root, Some(&self.control));
        if observed.status != ProcessStatus::Passed
            || observed.exit_code != Some(0)
            || observed.signal.is_some()
            || observed.reason.is_some()
            || observed.stdout_limit_exhausted
            || observed.stderr_limit_exhausted
        {
            return Err(DevError::unavailable(format!(
                "controller process failed or did not finish joined cleanup: {:?}; diagnostic {}",
                observed.status, self.sequence
            )));
        }
        Ok(stdout)
    }

    fn json(&mut self, arguments: Vec<String>) -> Result<Value, DevError> {
        let file = self.run(arguments, true, MAX_JSON)?;
        Ok(serde_json::from_slice(&process::read_bounded(
            &file, MAX_JSON,
        )?)?)
    }
}

impl Operations for HostedOperations {
    fn api(&mut self, method: &str, path: &str, body: Option<&Value>) -> Result<Value, DevError> {
        if !matches!(method, "GET" | "POST" | "PATCH")
            || !path.starts_with(&format!("repos/{REPOSITORY}/"))
        {
            return Err(DevError::corrupt(
                "controller API request escaped the maintained repository or methods",
            ));
        }
        let mut command = vec![
            "gh".to_owned(),
            "api".to_owned(),
            "--method".to_owned(),
            method.to_owned(),
            path.to_owned(),
            "-H".to_owned(),
            "X-GitHub-Api-Version: 2022-11-28".to_owned(),
        ];
        if let Some(body) = body {
            let input = self.root.join(format!("{}-input.json", self.sequence + 1));
            super::write_json(&input, body)?;
            command.extend(["--input".to_owned(), input.to_string_lossy().into_owned()]);
        }
        self.json(command)
    }

    fn download(&mut self, path: &str, output: &Path, authenticated: bool) -> Result<(), DevError> {
        let command = if authenticated {
            if !path.starts_with(&format!("repos/{REPOSITORY}/actions/artifacts/"))
                || !path.ends_with("/zip")
            {
                return Err(DevError::corrupt(
                    "authenticated download is not an artifact endpoint",
                ));
            }
            vec!["gh".to_owned(), "api".to_owned(), path.to_owned()]
        } else {
            if !path.starts_with(&format!("https://github.com/{REPOSITORY}/releases/")) {
                return Err(DevError::corrupt(
                    "anonymous download escaped the maintained release URL",
                ));
            }
            vec![
                "curl".to_owned(),
                "--disable".to_owned(),
                "--fail".to_owned(),
                "--location".to_owned(),
                "--silent".to_owned(),
                "--show-error".to_owned(),
                "--proto".to_owned(),
                "=https".to_owned(),
                "--max-time".to_owned(),
                "180".to_owned(),
                path.to_owned(),
            ]
        };
        let data = self.run(command, authenticated, MAX_ARTIFACT)?;
        super::copy_new(&data, output)?;
        Ok(())
    }

    fn upload(&mut self, release_id: u64, name: &str, path: &Path) -> Result<Value, DevError> {
        super::validate_name(name)?;
        self.json(vec!["gh".to_owned(), "api".to_owned(), "--method".to_owned(), "POST".to_owned(), format!("https://uploads.github.com/repos/{REPOSITORY}/releases/{release_id}/assets?name={name}"), "-H".to_owned(), "Content-Type: application/octet-stream".to_owned(), "--input".to_owned(), path.to_string_lossy().into_owned()])
    }

    fn zip_member(
        &mut self,
        zip: &Path,
        name: Option<&str>,
        output: &Path,
    ) -> Result<(), DevError> {
        let command = match name {
            Some(name) => {
                super::validate_name(name)?;
                vec![
                    "unzip".to_owned(),
                    "-p".to_owned(),
                    zip.to_string_lossy().into_owned(),
                    name.to_owned(),
                ]
            }
            None => vec![
                "unzip".to_owned(),
                "-Z1".to_owned(),
                zip.to_string_lossy().into_owned(),
            ],
        };
        let data = self.run(
            command,
            false,
            if name.is_some() {
                MAX_ARTIFACT
            } else {
                MAX_JSON
            },
        )?;
        super::copy_new(&data, output)
    }

    fn attestation(&mut self, path: &Path, tag: &str, output: &Path) -> Result<(), DevError> {
        let result = self.run(
            vec![
                "gh".to_owned(),
                "release".to_owned(),
                "verify-asset".to_owned(),
                tag.to_owned(),
                path.to_string_lossy().into_owned(),
                "--repo".to_owned(),
                REPOSITORY.to_owned(),
                "--format".to_owned(),
                "json".to_owned(),
            ],
            true,
            MAX_JSON,
        )?;
        super::copy_new(&result, output)
    }

    fn release_attestation(&mut self, tag: &str, output: &Path) -> Result<(), DevError> {
        let result = self.run(
            vec![
                "gh".to_owned(),
                "release".to_owned(),
                "verify".to_owned(),
                tag.to_owned(),
                "--repo".to_owned(),
                REPOSITORY.to_owned(),
                "--format".to_owned(),
                "json".to_owned(),
            ],
            true,
            MAX_JSON,
        )?;
        super::copy_new(&result, output)
    }
}
