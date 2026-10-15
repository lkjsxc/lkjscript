use super::*;
use serde_json::json;

struct Fixture {
    _directory: tempfile::TempDir,
    repository: PathBuf,
    target: PathBuf,
    out_dir: PathBuf,
    stdout: PathBuf,
    gate: Gate,
    records: Vec<Value>,
}

impl Fixture {
    fn new() -> Result<Self, DevError> {
        let directory = tempfile::tempdir()?;
        let repository = fs::canonicalize(directory.path())?;
        let target = repository.join("target");
        let out_dir = target.join("debug/build/lkjscript-2222222222222222/out");
        let builder = target.join("debug/build/lkjscript-1111111111111111/build-script-build");
        fs::create_dir_all(builder.parent().expect("builder parent"))?;
        fs::create_dir_all(&out_dir)?;
        fs::create_dir_all(repository.join("src"))?;
        fs::create_dir_all(target.join("debug/deps"))?;
        fs::write(
            repository.join("Cargo.toml"),
            "[package]\nname=\"lkjscript\"\n",
        )?;
        fs::write(repository.join("build.rs"), "fn main() {}\n")?;
        fs::write(repository.join("src/lib.rs"), "pub fn value() {}\n")?;
        fs::write(&builder, "observed generator executable")?;
        fs::write(
            out_dir.join("canonical_schema.rs"),
            lkjscript::platform::canonical_schema::generated_source(),
        )?;
        fs::write(
            out_dir.parent().expect("output parent").join("output"),
            "cargo:rerun-if-changed=build.rs\n",
        )?;
        let library = target.join("debug/deps/liblkjscript-fixture.rlib");
        fs::write(&library, "cold compiled product")?;
        let package = format!("path+file://{}#lkjscript@0.0.0", repository.display());
        let artifact = |kind: &str, name: &str, entry: &str, file: &Path| {
            json!({
                "reason":"compiler-artifact", "package_id":package,
                "manifest_path":repository.join("Cargo.toml"),
                "target":{"kind":[kind],"crate_types":[if kind == "lib" {"lib"} else {"bin"}],
                    "name":name,"src_path":repository.join(entry),"edition":"2024",
                    "doc":false,"doctest":false,"test":false},
                "profile":{"opt_level":"0","debuginfo":0,"debug_assertions":true,"overflow_checks":true,"test":false},
                "features":[],"filenames":[file],"executable":null,"fresh":false
            })
        };
        let records = vec![
            artifact("custom-build", "build-script-build", "build.rs", &builder),
            json!({"reason":"build-script-executed","package_id":package,
                "linked_libs":[],"linked_paths":[],"cfgs":[],"env":[],"out_dir":out_dir}),
            artifact("lib", "lkjscript", "src/lib.rs", &library),
            json!({"reason":"build-finished","success":true}),
        ];
        let stdout = repository.join("cargo.jsonl");
        Ok(Self {
            _directory: directory,
            repository,
            target,
            out_dir,
            stdout,
            gate: Gate::new(
                "schema",
                vec![
                    "cargo".into(),
                    "build".into(),
                    "--locked".into(),
                    "--message-format=json".into(),
                ],
            ),
            records,
        })
    }

    fn capture(&self) -> Result<Vec<Unit>, DevError> {
        let mut bytes = Vec::new();
        for record in &self.records {
            serde_json::to_writer(&mut bytes, record)?;
            bytes.push(b'\n');
        }
        fs::write(&self.stdout, bytes)?;
        let stdout = evidence::proof(&self.stdout, path_string(&self.stdout)?)?;
        read_units(
            &self.repository,
            &self.target,
            &["Cargo.toml".into()],
            &self.gate,
            &self.stdout,
            &stdout,
        )
    }
}

#[test]
fn schema_generation_retains_observed_executable_and_exact_derived_input() -> Result<(), DevError> {
    for fresh in [false, true] {
        let mut fixture = Fixture::new()?;
        fixture.records[0]["fresh"] = fresh.into();
        let mut units = fixture.capture()?;
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].fresh, fresh);
        assert!(schema_generator(&units[0]));
        assert!(!schema_generator(&units[1]));
        assert!(
            units
                .iter()
                .all(|unit| unit.build_script.as_ref() == Some(&fixture.records[1]))
        );
        assert!(units.iter().all(|unit| {
            unit.filenames
                .iter()
                .any(|file| file.path.ends_with("/out/canonical_schema.rs"))
        }));
        let run = fixture.repository.join("retained");
        fs::create_dir(&run)?;
        retain_units(&fixture.repository, &run, &fixture.gate, &mut units)?;
        let stdout = evidence::proof(&fixture.stdout, path_string(&fixture.stdout)?)?;
        verify_records(
            &fixture.repository,
            &fixture.target,
            &["Cargo.toml".into()],
            &fixture.gate,
            &fixture.stdout,
            &stdout,
            &units,
        )?;
        let generated = units[1]
            .filenames
            .iter()
            .position(|proof| proof.path.ends_with("/out/canonical_schema.rs"))
            .expect("derived input");
        let retained = fixture.repository.join(&units[1].retained[generated].path);
        fs::write(retained, "tampered retained schema")?;
        assert!(
            verify_records(
                &fixture.repository,
                &fixture.target,
                &["Cargo.toml".into()],
                &fixture.gate,
                &fixture.stdout,
                &stdout,
                &units
            )
            .is_err()
        );
    }
    Ok(())
}

#[test]
fn schema_generation_rejects_wrong_bytes_at_first_capture_and_incomplete_or_foreign_records()
-> Result<(), DevError> {
    for fault in [
        "wrong-bytes",
        "missing-generated",
        "extra-output",
        "missing-stdout",
        "missing-builder",
        "missing-execution",
        "duplicate-execution",
        "foreign-package",
        "foreign-source",
        "foreign-out-dir",
        "cfg-output",
        "env-output",
        "link-output",
    ] {
        let mut fixture = Fixture::new()?;
        match fault {
            "wrong-bytes" => fs::write(
                fixture.out_dir.join("canonical_schema.rs"),
                "plausible but unauthenticated schema",
            )?,
            "missing-generated" => fs::remove_file(fixture.out_dir.join("canonical_schema.rs"))?,
            "extra-output" => fs::write(
                fixture.out_dir.join("unadmitted.rs"),
                "unexpected compiler input",
            )?,
            "missing-stdout" => {
                fs::remove_file(fixture.out_dir.parent().expect("parent").join("output"))?
            }
            "missing-builder" => {
                fixture.records.remove(0);
            }
            "missing-execution" => {
                fixture.records.remove(1);
            }
            "duplicate-execution" => fixture.records.insert(1, fixture.records[1].clone()),
            "foreign-package" => fixture.records[1]["package_id"] = "foreign-package".into(),
            "foreign-source" => {
                fs::write(fixture.repository.join("unrelated.rs"), "fn main() {}")?;
                fixture.records[0]["target"]["src_path"] = fixture
                    .repository
                    .join("unrelated.rs")
                    .display()
                    .to_string()
                    .into();
            }
            "foreign-out-dir" => {
                fixture.records[1]["out_dir"] =
                    "/foreign/build/lkjscript-2222222222222222/out".into()
            }
            "cfg-output" => fixture.records[1]["cfgs"] = json!(["unsourced_configuration"]),
            "env-output" => fixture.records[1]["env"] = json!([["UNSOURCED", "value"]]),
            "link-output" => fixture.records[1]["linked_libs"] = json!(["foreign"]),
            _ => unreachable!(),
        }
        assert!(fixture.capture().is_err(), "{fault}");
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn schema_generation_refuses_symlinked_generated_bytes() -> Result<(), DevError> {
    let fixture = Fixture::new()?;
    let generated = fixture.out_dir.join("canonical_schema.rs");
    fs::rename(&generated, fixture.repository.join("foreign.rs"))?;
    std::os::unix::fs::symlink(fixture.repository.join("foreign.rs"), generated)?;
    assert!(fixture.capture().is_err());
    Ok(())
}
