//! Contributor-only PostgreSQL 16.15 differential-oracle process boundary.

use crate::error::DevError;
use crate::process::{self, ProcessObservation, ProcessSpec, ProcessStatus};
use postgres::{Client, NoTls};
use serde::Deserialize;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

pub(crate) const POSTGRES_VERSION: &str = "16.15";
pub(crate) const POSTGRES_IMAGE: &str =
    "postgres@sha256:485935f94cc7165afa896978809c37b592dc07f0a37d2c8f645f12412d0212c8";
const POSTGRES_IMAGE_CONFIG: &str =
    "sha256:80f4c7a5e91618546dce5b4fe60cf03b14c0f9efa7e40157278d122772ced8d2";
const COMMAND_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const READY_TIMEOUT: Duration = Duration::from_secs(60);
const MAXIMUM_LOG_BYTES: u64 = 16 * 1024 * 1024;
static INSTANCE_ORDINAL: AtomicU64 = AtomicU64::new(0);

#[derive(Deserialize)]
struct LocalImageInspection {
    #[serde(rename = "RepoDigests")]
    repository_digests: Vec<String>,
    #[serde(rename = "Id")]
    identity: String,
    #[serde(rename = "Os")]
    operating_system: String,
    #[serde(rename = "Architecture")]
    architecture: String,
}

#[derive(Deserialize)]
struct ManifestInspection {
    #[serde(rename = "Descriptor")]
    descriptor: ManifestDescriptor,
    #[serde(rename = "OCIManifest")]
    manifest: OciManifest,
}

#[derive(Deserialize)]
struct ManifestDescriptor {
    digest: String,
    platform: ManifestPlatform,
}

#[derive(Deserialize)]
struct ManifestPlatform {
    architecture: String,
    os: String,
}

#[derive(Deserialize)]
struct OciManifest {
    config: OciConfig,
}

#[derive(Deserialize)]
struct OciConfig {
    digest: String,
}

pub(crate) struct PostgresInstance {
    name: String,
    port: u16,
    repository: PathBuf,
    evidence: PathBuf,
    command_ordinal: u64,
    stopped: bool,
}

impl PostgresInstance {
    pub(crate) fn start(
        repository: &Path,
        evidence: &Path,
        observations: &mut Vec<ProcessObservation>,
    ) -> Result<Self, DevError> {
        let ordinal = INSTANCE_ORDINAL.fetch_add(1, Ordering::Relaxed);
        let name = format!("lkjscript-data-oracle-{}-{ordinal}", std::process::id());
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|error| {
            DevError::infrastructure(format!("reserve PostgreSQL oracle port: {error}"))
        })?;
        let port = listener
            .local_addr()
            .map_err(|error| {
                DevError::infrastructure(format!("inspect PostgreSQL oracle port: {error}"))
            })?
            .port();
        drop(listener);

        let mut instance = Self {
            name,
            port,
            repository: repository.to_path_buf(),
            evidence: evidence.to_path_buf(),
            command_ordinal: 0,
            stopped: false,
        };
        observations.push(instance.docker(
            "pull",
            &["pull", "--platform", "linux/amd64", POSTGRES_IMAGE],
        )?);
        let inspect = instance.docker_output(
            "inspect-image",
            &["image", "inspect", POSTGRES_IMAGE],
            observations,
        )?;
        let local: Vec<LocalImageInspection> = serde_json::from_str(&inspect).map_err(|error| {
            DevError::corrupt(format!("decode Docker image inspection: {error}"))
        })?;
        let [local] = local.as_slice() else {
            return Err(DevError::corrupt(
                "Docker image inspection did not return one exact image",
            ));
        };
        let manifest_digest = POSTGRES_IMAGE
            .rsplit_once('@')
            .map(|(_, digest)| digest)
            .ok_or_else(|| DevError::corrupt("PostgreSQL oracle image omitted its digest"))?;
        let repository_digest_matches = local.repository_digests.iter().any(|digest| {
            digest
                .rsplit_once('@')
                .is_some_and(|(_, found)| found == manifest_digest)
        });
        let local_identity_matches =
            local.identity == manifest_digest || local.identity == POSTGRES_IMAGE_CONFIG;
        if !repository_digest_matches
            || !local_identity_matches
            || local.operating_system != "linux"
            || local.architecture != "amd64"
        {
            return Err(DevError::corrupt(format!(
                "local PostgreSQL oracle image does not bind manifest '{manifest_digest}', config '{POSTGRES_IMAGE_CONFIG}', and linux/amd64"
            )));
        }
        let manifest_output = instance.docker_output(
            "inspect-manifest",
            &["manifest", "inspect", "--verbose", POSTGRES_IMAGE],
            observations,
        )?;
        let manifest: ManifestInspection =
            serde_json::from_str(&manifest_output).map_err(|error| {
                DevError::corrupt(format!("decode Docker manifest inspection: {error}"))
            })?;
        if manifest.descriptor.digest != manifest_digest
            || manifest.descriptor.platform.os != "linux"
            || manifest.descriptor.platform.architecture != "amd64"
            || manifest.manifest.config.digest != POSTGRES_IMAGE_CONFIG
        {
            return Err(DevError::corrupt(
                "PostgreSQL oracle manifest, config, or linux/amd64 platform identity diverged",
            ));
        }
        let published = format!("127.0.0.1:{}:5432", instance.port);
        let name = instance.name.clone();
        observations.push(instance.docker(
            "start",
            &[
                "run",
                "--detach",
                "--name",
                &name,
                "--platform",
                "linux/amd64",
                "--publish",
                &published,
                "--env",
                "POSTGRES_HOST_AUTH_METHOD=trust",
                "--env",
                "POSTGRES_DB=oracle",
                "--tmpfs",
                "/var/lib/postgresql/data:rw,nosuid,nodev,noexec,size=268435456",
                POSTGRES_IMAGE,
                "-c",
                "fsync=on",
                "-c",
                "synchronous_commit=on",
                "-c",
                "full_page_writes=on",
                "-c",
                "jit=off",
                "-c",
                "max_connections=16",
            ],
        )?);
        instance.wait_ready()?;
        let mut client = instance.connect()?;
        let version: String = client
            .query_one("SHOW server_version", &[])
            .map_err(|error| DevError::infrastructure(format!("read PostgreSQL version: {error}")))?
            .get(0);
        let version_number: String = client
            .query_one("SHOW server_version_num", &[])
            .map_err(|error| {
                DevError::infrastructure(format!("read PostgreSQL numeric version: {error}"))
            })?
            .get(0);
        if version_number != "160015"
            || !(version == POSTGRES_VERSION
                || version
                    .strip_prefix(POSTGRES_VERSION)
                    .is_some_and(|suffix| suffix.starts_with(' ') || suffix.starts_with('(')))
        {
            return Err(DevError::corrupt(format!(
                "PostgreSQL oracle version is '{version}' ({version_number}), expected exact numeric version 160015"
            )));
        }
        Ok(instance)
    }

    pub(crate) const fn image() -> &'static str {
        POSTGRES_IMAGE
    }

    pub(crate) const fn image_config() -> &'static str {
        POSTGRES_IMAGE_CONFIG
    }

    pub(crate) const fn port(&self) -> u16 {
        self.port
    }

    pub(crate) fn connect(&self) -> Result<Client, DevError> {
        Client::connect(
            &format!(
                "host=127.0.0.1 port={} user=postgres dbname=oracle connect_timeout=2",
                self.port
            ),
            NoTls,
        )
        .map_err(|error| DevError::infrastructure(format!("connect PostgreSQL oracle: {error}")))
    }

    pub(crate) fn sampled_resident_kib(
        &mut self,
        observations: &mut Vec<ProcessObservation>,
    ) -> Result<u64, DevError> {
        let name = self.name.clone();
        let output = self.docker_output(
            "resident",
            &["stats", "--no-stream", "--format", "{{.MemUsage}}", &name],
            observations,
        )?;
        parse_memory_kib(output.split('/').next().unwrap_or_default().trim())
    }

    pub(crate) fn stop(
        &mut self,
        observations: &mut Vec<ProcessObservation>,
    ) -> Result<(), DevError> {
        if self.stopped {
            return Ok(());
        }
        let name = self.name.clone();
        observations.push(self.docker("stop", &["stop", "--time", "5", &name])?);
        observations.push(self.docker("remove", &["rm", &name])?);
        self.stopped = true;
        Ok(())
    }

    fn wait_ready(&self) -> Result<(), DevError> {
        let started = Instant::now();
        loop {
            match self.connect() {
                Ok(_) => return Ok(()),
                Err(error) if started.elapsed() < READY_TIMEOUT => {
                    let _ = error;
                    thread::sleep(Duration::from_millis(100));
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn docker(&mut self, label: &str, arguments: &[&str]) -> Result<ProcessObservation, DevError> {
        let observation = self.observe(label, arguments);
        if observation.status != ProcessStatus::Passed {
            return Err(DevError::infrastructure(format!(
                "Docker PostgreSQL oracle command '{label}' failed ({})",
                observation.reason.as_deref().unwrap_or("child_failed")
            )));
        }
        Ok(observation)
    }

    fn docker_output(
        &mut self,
        label: &str,
        arguments: &[&str],
        observations: &mut Vec<ProcessObservation>,
    ) -> Result<String, DevError> {
        let observation = self.docker(label, arguments)?;
        let bytes = process::read_bounded(
            &self.repository.join(&observation.stdout.path),
            MAXIMUM_LOG_BYTES,
        )?;
        observations.push(observation);
        String::from_utf8(bytes)
            .map_err(|_| DevError::corrupt("Docker PostgreSQL oracle output is not UTF-8"))
    }

    fn observe(&mut self, label: &str, arguments: &[&str]) -> ProcessObservation {
        let ordinal = self.command_ordinal;
        self.command_ordinal = self.command_ordinal.saturating_add(1);
        let stdout = self
            .evidence
            .join(format!("postgres-{ordinal:03}-{label}.stdout.log"));
        let stderr = self
            .evidence
            .join(format!("postgres-{ordinal:03}-{label}.stderr.log"));
        let mut environment = process::environment();
        environment.insert("LANG".to_owned(), "C".to_owned());
        let mut command = vec!["docker".to_owned()];
        command.extend(arguments.iter().map(|value| (*value).to_owned()));
        process::run(
            &ProcessSpec {
                command,
                cwd: self.repository.clone(),
                environment,
                timeout: COMMAND_TIMEOUT,
                maximum_stdout_bytes: MAXIMUM_LOG_BYTES,
                maximum_stderr_bytes: MAXIMUM_LOG_BYTES,
                stdout_path: stdout,
                stderr_path: stderr,
                unavailable_exit_code: None,
            },
            &self.repository,
        )
    }
}

impl Drop for PostgresInstance {
    fn drop(&mut self) {
        if !self.stopped {
            let _ = Command::new("docker")
                .args(["rm", "--force", &self.name])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

fn parse_memory_kib(value: &str) -> Result<u64, DevError> {
    let split = value
        .find(|character: char| !character.is_ascii_digit() && character != '.')
        .ok_or_else(|| DevError::corrupt("Docker memory sample omitted a unit"))?;
    let number = value[..split]
        .parse::<f64>()
        .map_err(|error| DevError::corrupt(format!("parse Docker memory sample: {error}")))?;
    let unit = value[split..].trim();
    let multiplier = match unit {
        "B" => 1.0 / 1024.0,
        "KiB" | "kB" => 1.0,
        "MiB" => 1024.0,
        "GiB" => 1024.0 * 1024.0,
        _ => {
            return Err(DevError::corrupt(format!(
                "Docker memory sample has unsupported unit '{unit}'"
            )));
        }
    };
    let kib = number * multiplier;
    if !kib.is_finite() || kib < 0.0 || kib > u64::MAX as f64 {
        return Err(DevError::corrupt("Docker memory sample is out of range"));
    }
    Ok(kib.ceil() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_image_and_memory_units_are_closed() {
        assert!(POSTGRES_IMAGE.starts_with("postgres@sha256:"));
        assert_eq!(POSTGRES_IMAGE.len(), "postgres@sha256:".len() + 64);
        assert_eq!(parse_memory_kib("1024B").expect("bytes"), 1);
        assert_eq!(parse_memory_kib("1.5MiB").expect("mebibytes"), 1536);
        assert!(parse_memory_kib("1MB").is_err());
    }
}
