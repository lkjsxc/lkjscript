use super::model::{CHECK_CONTRACT_VERSION, DagManifest, DagNode, Gate};
use crate::error::DevError;
use crate::evidence::VerificationDigest;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Clone, Debug)]
pub(crate) struct GateRegistry {
    gates: Vec<Gate>,
    indices: BTreeMap<String, usize>,
}

impl GateRegistry {
    pub(crate) fn new(gates: Vec<Gate>) -> Result<Self, DevError> {
        let mut indices = BTreeMap::new();
        for (index, gate) in gates.iter().enumerate() {
            if !valid_gate_name(&gate.name) {
                return Err(DevError::infrastructure(format!(
                    "invalid gate identity '{}'",
                    gate.name
                )));
            }
            if indices.insert(gate.name.clone(), index).is_some() {
                return Err(DevError::infrastructure(format!(
                    "duplicate gate identity '{}'",
                    gate.name
                )));
            }
            if gate.command.is_empty() {
                return Err(DevError::infrastructure(format!(
                    "gate '{}' has an empty command",
                    gate.name
                )));
            }
            if gate.unavailable_exit_code == Some(0) {
                return Err(DevError::infrastructure(format!(
                    "gate '{}' cannot classify successful exit code zero as unavailable",
                    gate.name
                )));
            }
        }
        let registry = Self { gates, indices };
        registry.validate_dependencies()?;
        registry.validate_equivalent_commands()?;
        registry.validate_cycles()?;
        Ok(registry)
    }

    pub(crate) fn gate(&self, name: &str) -> Result<&Gate, DevError> {
        self.indices
            .get(name)
            .and_then(|index| self.gates.get(*index))
            .ok_or_else(|| DevError::infrastructure(format!("unknown gate '{name}'")))
    }

    pub(crate) fn closure(&self, requested: &[String]) -> Result<Vec<String>, DevError> {
        let mut selected = BTreeSet::new();
        for name in requested {
            self.add_dependencies(name, &mut selected)?;
        }
        Ok(self
            .gates
            .iter()
            .filter(|gate| selected.contains(&gate.name))
            .map(|gate| gate.name.clone())
            .collect())
    }

    pub(crate) fn manifest(
        &self,
        requested: &[String],
        selected: &[String],
        maximum_workers: usize,
    ) -> Result<DagManifest, DevError> {
        let mut nodes = Vec::with_capacity(selected.len());
        for name in selected {
            let gate = self.gate(name)?;
            nodes.push(DagNode {
                name: name.clone(),
                dependencies: gate.dependencies.clone(),
                command: gate.identity_command().to_vec(),
                cacheable: gate.cacheable,
                unavailable_exit_code: gate.unavailable_exit_code,
            });
        }
        Ok(DagManifest {
            contract_version: CHECK_CONTRACT_VERSION,
            requested: requested.to_vec(),
            selected_closure: selected.to_vec(),
            maximum_workers,
            nodes,
        })
    }

    pub(crate) fn profile_digest(
        &self,
        profile: &str,
        requested: &[String],
    ) -> Result<VerificationDigest, DevError> {
        #[derive(Serialize)]
        struct ProfileIdentity<'a> {
            contract_version: u32,
            profile: &'a str,
            requested: &'a [String],
            gates: Vec<GateIdentity<'a>>,
        }
        #[derive(Serialize)]
        struct GateIdentity<'a> {
            name: &'a str,
            command: &'a [String],
            dependencies: &'a [String],
            timeout_nanoseconds: u128,
            maximum_stdout_bytes: u64,
            maximum_stderr_bytes: u64,
            cacheable: bool,
            unavailable_exit_code: Option<i32>,
            required_outputs: Vec<String>,
        }
        let gates = self
            .gates
            .iter()
            .map(|gate| GateIdentity {
                name: &gate.name,
                command: gate.identity_command(),
                dependencies: &gate.dependencies,
                timeout_nanoseconds: gate.timeout.as_nanos(),
                maximum_stdout_bytes: gate.maximum_stdout_bytes,
                maximum_stderr_bytes: gate.maximum_stderr_bytes,
                cacheable: gate.cacheable,
                unavailable_exit_code: gate.unavailable_exit_code,
                required_outputs: gate
                    .required_outputs
                    .iter()
                    .map(|path| {
                        path.file_name()
                            .and_then(|value| value.to_str())
                            .unwrap_or("nonportable-output")
                            .to_owned()
                    })
                    .collect(),
            })
            .collect();
        let bytes = serde_json::to_vec(&ProfileIdentity {
            contract_version: CHECK_CONTRACT_VERSION,
            profile,
            requested,
            gates,
        })
        .map_err(|error| DevError::infrastructure(format!("encode profile identity: {error}")))?;
        Ok(VerificationDigest::of(&bytes))
    }

    fn add_dependencies(
        &self,
        name: &str,
        selected: &mut BTreeSet<String>,
    ) -> Result<(), DevError> {
        if selected.contains(name) {
            return Ok(());
        }
        let gate = self.gate(name)?;
        for dependency in &gate.dependencies {
            self.add_dependencies(dependency, selected)?;
        }
        selected.insert(name.to_owned());
        Ok(())
    }

    fn validate_dependencies(&self) -> Result<(), DevError> {
        for gate in &self.gates {
            for dependency in &gate.dependencies {
                if !self.indices.contains_key(dependency) {
                    return Err(DevError::infrastructure(format!(
                        "gate '{}' has unknown dependency '{dependency}'",
                        gate.name
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate_equivalent_commands(&self) -> Result<(), DevError> {
        #[derive(Serialize)]
        struct CommandIdentity<'a> {
            command: &'a [String],
            timeout_nanoseconds: u128,
            maximum_stdout_bytes: u64,
            maximum_stderr_bytes: u64,
            outputs: Vec<String>,
        }
        let mut owners: BTreeMap<Vec<u8>, String> = BTreeMap::new();
        for gate in &self.gates {
            let bytes = serde_json::to_vec(&CommandIdentity {
                command: gate.identity_command(),
                timeout_nanoseconds: gate.timeout.as_nanos(),
                maximum_stdout_bytes: gate.maximum_stdout_bytes,
                maximum_stderr_bytes: gate.maximum_stderr_bytes,
                outputs: gate
                    .required_outputs
                    .iter()
                    .map(|path| path.to_string_lossy().into_owned())
                    .collect(),
            })
            .map_err(|error| DevError::infrastructure(format!("encode gate identity: {error}")))?;
            if let Some(previous) = owners.insert(bytes, gate.name.clone()) {
                return Err(DevError::infrastructure(format!(
                    "equivalent gates '{previous}' and '{}' must be one DAG node",
                    gate.name
                )));
            }
        }
        Ok(())
    }

    fn validate_cycles(&self) -> Result<(), DevError> {
        let mut states = vec![VisitState::Unvisited; self.gates.len()];
        for index in 0..self.gates.len() {
            self.visit(index, &mut states)?;
        }
        Ok(())
    }

    fn visit(&self, index: usize, states: &mut [VisitState]) -> Result<(), DevError> {
        match states.get(index) {
            Some(VisitState::Visited) => return Ok(()),
            Some(VisitState::Visiting) => {
                let name = self
                    .gates
                    .get(index)
                    .map(|gate| gate.name.as_str())
                    .unwrap_or("unknown");
                return Err(DevError::infrastructure(format!(
                    "verification DAG contains a cycle at '{name}'"
                )));
            }
            Some(VisitState::Unvisited) => {}
            None => return Err(DevError::infrastructure("invalid DAG node index")),
        }
        states[index] = VisitState::Visiting;
        let gate = self
            .gates
            .get(index)
            .ok_or_else(|| DevError::infrastructure("invalid DAG gate index"))?;
        for dependency in &gate.dependencies {
            let dependency_index = self.indices.get(dependency).copied().ok_or_else(|| {
                DevError::infrastructure(format!("unknown DAG dependency '{dependency}'"))
            })?;
            self.visit(dependency_index, states)?;
        }
        states[index] = VisitState::Visited;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

pub(crate) fn base_registry(
    repository: &Path,
    run_directory: &Path,
    self_test_executable: &Path,
) -> Result<GateRegistry, DevError> {
    let outputs = run_directory.join("outputs");
    std::fs::create_dir_all(&outputs).map_err(|error| {
        DevError::infrastructure(format!("create check output directory: {error}"))
    })?;
    let binary = repository.join("target/release/lkjscript");
    let standard = outputs.join("standard.lkja");
    let builtin_artifact = outputs.join("builtin-standard.lkja");
    let builtin_transport = outputs.join("builtin-standard.lkjp");
    let application = outputs.join("lkjournal.lkja");
    let mut gates = vec![
        cargo_gate("fmt", &["fmt", "--all", "--", "--check"], &[]),
        cargo_gate(
            "clippy",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--locked",
                "--",
                "-D",
                "warnings",
            ],
            &["fmt"],
        ),
        cargo_gate("library_tests", &["test", "--lib", "--locked"], &["fmt"]),
        cargo_gate(
            "general_service_tests",
            &["test", "--test", "general_service", "--locked"],
            &["fmt"],
        ),
        cargo_gate(
            "public_cli_tests",
            &["test", "--test", "public_cli", "--locked"],
            &["library_tests", "general_service_tests"],
        ),
        cargo_gate(
            "workspace_tests",
            &[
                "test",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--locked",
            ],
            &["clippy", "release_build"],
        ),
    ];
    let mut release = cargo_gate(
        "release_build",
        &["build", "--workspace", "--release", "--locked"],
        &["fmt"],
    );
    release.required_outputs.push(binary.clone());
    gates.push(release);
    gates.push(cargo_gate(
        "release_command_lifecycle",
        &[
            "test",
            "--locked",
            "--release",
            "--test",
            "public_cli",
            "copied_binary_completes_normalized_standard_dependent_command_lifecycle",
            "--",
            "--exact",
        ],
        &["release_build"],
    ));
    let mut distributed_http = gate(
        "distributed_http_application",
        vec![
            path_string(self_test_executable),
            "distributed-http".to_owned(),
            "--binary".to_owned(),
            path_string(&binary),
            "--machine".to_owned(),
        ],
        &["release_build"],
    );
    distributed_http.identity_command = Some(vec![
        "$HARNESS".to_owned(),
        "distributed-http".to_owned(),
        "--binary".to_owned(),
        path_string(&binary),
        "--machine".to_owned(),
    ]);
    distributed_http.cacheable = false;
    gates.push(distributed_http);
    let mut outbound_http = gate(
        "outbound_http_application",
        vec![
            path_string(self_test_executable),
            "outbound-http".to_owned(),
            "--binary".to_owned(),
            path_string(&binary),
            "--machine".to_owned(),
        ],
        &["release_build"],
    );
    outbound_http.identity_command = Some(vec![
        "$HARNESS".to_owned(),
        "outbound-http".to_owned(),
        "--binary".to_owned(),
        path_string(&binary),
        "--machine".to_owned(),
    ]);
    outbound_http.cacheable = false;
    gates.push(outbound_http);
    let mut offline_packages = gate(
        "offline_packages",
        vec![
            path_string(self_test_executable),
            "offline-packages".to_owned(),
            "--binary".to_owned(),
            path_string(&binary),
            "--machine".to_owned(),
        ],
        &["release_build"],
    );
    offline_packages.identity_command = Some(vec![
        "$HARNESS".to_owned(),
        "offline-packages".to_owned(),
        "--binary".to_owned(),
        path_string(&binary),
        "--machine".to_owned(),
    ]);
    offline_packages.cacheable = false;
    gates.push(offline_packages);
    let mut stateful_http = gate(
        "stateful_http_application",
        vec![
            path_string(self_test_executable),
            "stateful-http".to_owned(),
            "--binary".to_owned(),
            path_string(&binary),
            "--machine".to_owned(),
        ],
        &["release_build"],
    );
    stateful_http.identity_command = Some(vec![
        "$HARNESS".to_owned(),
        "stateful-http".to_owned(),
        "--binary".to_owned(),
        path_string(&binary),
        "--machine".to_owned(),
    ]);
    stateful_http.cacheable = false;
    stateful_http.unavailable_exit_code = Some(2);
    gates.push(stateful_http);

    let mut self_test = Gate::new(
        "checker_self_test",
        vec![
            self_test_executable.to_string_lossy().into_owned(),
            "check".to_owned(),
            "self-test".to_owned(),
            "--machine".to_owned(),
        ],
    );
    self_test.identity_command = Some(vec![
        "$HARNESS".to_owned(),
        "check".to_owned(),
        "self-test".to_owned(),
        "--machine".to_owned(),
    ]);
    gates.push(self_test);

    let mut rust_only = Gate::new(
        "rust_only_tooling",
        vec![
            self_test_executable.to_string_lossy().into_owned(),
            "policy".to_owned(),
            "no-python".to_owned(),
            "--machine".to_owned(),
        ],
    );
    rust_only.identity_command = Some(vec![
        "$HARNESS".to_owned(),
        "policy".to_owned(),
        "no-python".to_owned(),
        "--machine".to_owned(),
    ]);
    gates.push(rust_only);

    gates.push(gate(
        "generated_public_guides",
        vec![
            path_string(&binary),
            "capabilities".to_owned(),
            "--verify-generated".to_owned(),
            path_string(&repository.join("docs/generated")),
        ],
        &["release_command_lifecycle"],
    ));
    let mut product_surface = gate(
        "product_surface_audit",
        vec![
            path_string(self_test_executable),
            "policy".to_owned(),
            "product-surface".to_owned(),
            "--binary".to_owned(),
            path_string(&binary),
            "--machine".to_owned(),
        ],
        &["generated_public_guides"],
    );
    product_surface.identity_command = Some(vec![
        "$HARNESS".to_owned(),
        "policy".to_owned(),
        "product-surface".to_owned(),
        "--binary".to_owned(),
        path_string(&binary),
        "--machine".to_owned(),
    ]);
    gates.push(product_surface);
    gates.push(project_gate(
        "standard_package_test",
        &binary,
        "packages/standard",
        &["check"],
    ));
    gates.push(project_gate(
        "application_package_test",
        &binary,
        "applications/lkjournal",
        &["check"],
    ));
    gates.push(output_gate(
        "standard_artifact_build",
        vec![
            path_string(&binary),
            "--project".to_owned(),
            "packages/standard".to_owned(),
            "build".to_owned(),
            "--output".to_owned(),
            path_string(&standard),
        ],
        vec![
            path_string(&binary),
            "--project".to_owned(),
            "packages/standard".to_owned(),
            "build".to_owned(),
            "--output".to_owned(),
            "$RUN/outputs/standard.lkja".to_owned(),
        ],
        &standard,
    ));
    gates.push(compare_gate(
        "standard_artifact_compare",
        &standard,
        &repository.join("packages/standard/generated/standard.lkja"),
        "standard_artifact_build",
    ));
    gates.push(output_gate(
        "builtin_package_export",
        vec![
            path_string(&binary),
            "package".to_owned(),
            "builtin".to_owned(),
            "export".to_owned(),
            "--kind".to_owned(),
            "artifact".to_owned(),
            "--output".to_owned(),
            path_string(&builtin_artifact),
        ],
        vec![
            path_string(&binary),
            "package".to_owned(),
            "builtin".to_owned(),
            "export".to_owned(),
            "--kind".to_owned(),
            "artifact".to_owned(),
            "--output".to_owned(),
            "$RUN/outputs/builtin-standard.lkja".to_owned(),
        ],
        &builtin_artifact,
    ));
    gates.push(compare_gate(
        "builtin_package_compare",
        &builtin_artifact,
        &repository.join("packages/standard/generated/standard.lkja"),
        "builtin_package_export",
    ));
    gates.push(output_gate(
        "builtin_transport_export",
        vec![
            path_string(&binary),
            "package".to_owned(),
            "builtin".to_owned(),
            "export".to_owned(),
            "--kind".to_owned(),
            "transport".to_owned(),
            "--output".to_owned(),
            path_string(&builtin_transport),
        ],
        vec![
            path_string(&binary),
            "package".to_owned(),
            "builtin".to_owned(),
            "export".to_owned(),
            "--kind".to_owned(),
            "transport".to_owned(),
            "--output".to_owned(),
            "$RUN/outputs/builtin-standard.lkjp".to_owned(),
        ],
        &builtin_transport,
    ));
    gates.push(compare_gate(
        "builtin_transport_compare",
        &builtin_transport,
        &repository.join("packages/standard/generated/standard.lkjp"),
        "builtin_transport_export",
    ));
    gates.push(output_gate(
        "application_artifact_build",
        vec![
            path_string(&binary),
            "--project".to_owned(),
            "applications/lkjournal".to_owned(),
            "build".to_owned(),
            "--output".to_owned(),
            path_string(&application),
        ],
        vec![
            path_string(&binary),
            "--project".to_owned(),
            "applications/lkjournal".to_owned(),
            "build".to_owned(),
            "--output".to_owned(),
            "$RUN/outputs/lkjournal.lkja".to_owned(),
        ],
        &application,
    ));
    gates.push(compare_gate(
        "application_artifact_compare",
        &application,
        &repository.join("applications/lkjournal/generated/lkjournal.lkja"),
        "application_artifact_build",
    ));

    let mut service = gate(
        "service_acceptance",
        vec![
            path_string(self_test_executable),
            "service".to_owned(),
            "--binary".to_owned(),
            path_string(&binary),
            "--machine".to_owned(),
        ],
        &["release_command_lifecycle"],
    );
    service.identity_command = Some(vec![
        "$HARNESS".to_owned(),
        "service".to_owned(),
        "--binary".to_owned(),
        path_string(&binary),
        "--machine".to_owned(),
    ]);
    service.cacheable = false;
    service.unavailable_exit_code = Some(2);
    gates.push(service);
    gates.push(gate(
        "diff_check",
        vec![
            "git".to_owned(),
            "diff".to_owned(),
            "--check".to_owned(),
            "HEAD".to_owned(),
            "--".to_owned(),
        ],
        &[],
    ));
    GateRegistry::new(gates)
}

pub(crate) fn profile(name: &str) -> Option<Vec<String>> {
    let values: &[&str] = match name {
        "focused" => &[
            "fmt",
            "rust_only_tooling",
            "library_tests",
            "general_service_tests",
            "public_cli_tests",
            "diff_check",
        ],
        "product" => &[
            "fmt",
            "rust_only_tooling",
            "release_build",
            "release_command_lifecycle",
            "distributed_http_application",
            "outbound_http_application",
            "offline_packages",
            "generated_public_guides",
            "product_surface_audit",
            "standard_package_test",
            "application_package_test",
            "standard_artifact_build",
            "standard_artifact_compare",
            "builtin_package_export",
            "builtin_package_compare",
            "builtin_transport_export",
            "builtin_transport_compare",
            "application_artifact_build",
            "application_artifact_compare",
            "diff_check",
        ],
        "service" => &[
            "fmt",
            "rust_only_tooling",
            "release_build",
            "release_command_lifecycle",
            "distributed_http_application",
            "outbound_http_application",
            "stateful_http_application",
            "service_acceptance",
            "diff_check",
        ],
        "full" => &[
            "fmt",
            "checker_self_test",
            "rust_only_tooling",
            "clippy",
            "workspace_tests",
            "release_build",
            "release_command_lifecycle",
            "distributed_http_application",
            "outbound_http_application",
            "offline_packages",
            "stateful_http_application",
            "generated_public_guides",
            "product_surface_audit",
            "standard_package_test",
            "application_package_test",
            "standard_artifact_build",
            "standard_artifact_compare",
            "builtin_package_export",
            "builtin_package_compare",
            "builtin_transport_export",
            "builtin_transport_compare",
            "application_artifact_build",
            "application_artifact_compare",
            "service_acceptance",
            "diff_check",
        ],
        _ => return None,
    };
    Some(values.iter().map(|value| (*value).to_owned()).collect())
}

fn cargo_gate(name: &str, arguments: &[&str], dependencies: &[&str]) -> Gate {
    let mut command = vec!["cargo".to_owned()];
    command.extend(arguments.iter().map(|value| (*value).to_owned()));
    gate(name, command, dependencies)
}

fn project_gate(name: &str, binary: &Path, project: &str, arguments: &[&str]) -> Gate {
    let mut command = vec![
        path_string(binary),
        "--project".to_owned(),
        project.to_owned(),
    ];
    command.extend(arguments.iter().map(|value| (*value).to_owned()));
    gate(name, command, &["release_command_lifecycle"])
}

fn output_gate(
    name: &str,
    command: Vec<String>,
    identity_command: Vec<String>,
    output: &Path,
) -> Gate {
    let mut gate = gate(name, command, &["release_command_lifecycle"]);
    gate.identity_command = Some(identity_command);
    gate.cacheable = false;
    gate.required_outputs.push(output.to_path_buf());
    gate
}

fn compare_gate(name: &str, actual: &Path, expected: &Path, dependency: &str) -> Gate {
    let mut gate = gate(
        name,
        vec!["cmp".to_owned(), path_string(actual), path_string(expected)],
        &[dependency],
    );
    gate.identity_command = Some(vec![
        "cmp".to_owned(),
        format!(
            "$RUN/outputs/{}",
            actual
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("output")
        ),
        path_string(expected),
    ]);
    gate.cacheable = false;
    gate
}

fn gate(name: &str, command: Vec<String>, dependencies: &[&str]) -> Gate {
    let mut gate = Gate::new(name, command);
    gate.dependencies = dependencies
        .iter()
        .map(|dependency| (*dependency).to_owned())
        .collect();
    gate
}

fn valid_gate_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    matches!(bytes.next(), Some(b'a'..=b'z'))
        && bytes.all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == b'_')
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_rejects_missing_dependencies_cycles_and_equivalent_nodes() {
        let mut missing = Gate::new("missing", vec!["true".to_owned()]);
        missing.dependencies.push("absent".to_owned());
        assert!(GateRegistry::new(vec![missing]).is_err());

        let mut cycle = Gate::new("cycle", vec!["true".to_owned()]);
        cycle.dependencies.push("cycle".to_owned());
        assert!(GateRegistry::new(vec![cycle]).is_err());

        let first = Gate::new("first", vec!["true".to_owned()]);
        let second = Gate::new("second", vec!["true".to_owned()]);
        assert!(GateRegistry::new(vec![first, second]).is_err());

        let mut impossible_unavailable = Gate::new("impossible", vec!["true".to_owned()]);
        impossible_unavailable.unavailable_exit_code = Some(0);
        assert!(GateRegistry::new(vec![impossible_unavailable]).is_err());
    }

    #[test]
    fn closure_preserves_registry_order_and_deduplicates_shared_dependencies() {
        let shared = Gate::new("shared", vec!["shared".to_owned()]);
        let mut left = Gate::new("left", vec!["left".to_owned()]);
        left.dependencies.push("shared".to_owned());
        let mut right = Gate::new("right", vec!["right".to_owned()]);
        right.dependencies.push("shared".to_owned());
        let registry = GateRegistry::new(vec![shared, left, right]).expect("valid registry");
        assert_eq!(
            registry
                .closure(&["left".to_owned(), "right".to_owned()])
                .expect("dependency closure"),
            vec!["shared", "left", "right"]
        );
    }

    #[test]
    fn maintained_profiles_resolve_and_have_run_stable_identities() {
        let temporary = tempfile::tempdir().expect("temporary registry repository");
        let first_run = temporary.path().join("first");
        let second_run = temporary.path().join("second");
        std::fs::create_dir(&first_run).expect("create first registry run");
        std::fs::create_dir(&second_run).expect("create second registry run");
        let first = base_registry(temporary.path(), &first_run, Path::new("/bin/true"))
            .expect("first maintained registry");
        let second = base_registry(temporary.path(), &second_run, Path::new("/bin/true"))
            .expect("second maintained registry");
        assert_eq!(first.gates.len(), 28);
        for profile_name in ["focused", "product", "service", "full"] {
            let requested = profile(profile_name).expect("maintained profile");
            assert!(requested.iter().any(|name| name == "rust_only_tooling"));
            if profile_name != "focused" {
                assert!(
                    requested
                        .iter()
                        .any(|name| name == "release_command_lifecycle")
                );
                assert!(
                    requested
                        .iter()
                        .any(|name| name == "distributed_http_application")
                );
                assert!(
                    requested
                        .iter()
                        .any(|name| name == "outbound_http_application")
                );
            }
            assert_eq!(
                requested
                    .iter()
                    .any(|name| name == "stateful_http_application"),
                matches!(profile_name, "service" | "full")
            );
            assert_eq!(
                requested.iter().any(|name| name == "product_surface_audit"),
                matches!(profile_name, "product" | "full")
            );
            assert_eq!(
                requested.iter().any(|name| name == "offline_packages"),
                matches!(profile_name, "product" | "full")
            );
            let offline = first
                .gate("offline_packages")
                .expect("offline package oracle");
            assert!(!offline.cacheable);
            assert_eq!(offline.dependencies, ["release_build"]);
            assert!(
                !first
                    .closure(&requested)
                    .expect("profile closure")
                    .is_empty()
            );
            assert_eq!(
                first
                    .profile_digest(profile_name, &requested)
                    .expect("first profile digest"),
                second
                    .profile_digest(profile_name, &requested)
                    .expect("second profile digest")
            );
        }
    }
}
