use super::*;
use std::cell::Cell;

const LIBRARY: &str =
    include_str!("../../../examples/map-entry-projection/consumer-fold/library.lkjc");
const CONSUMER: &str =
    include_str!("../../../examples/map-entry-projection/consumer-fold/consumer.lkjc");
const HISTORY: &str =
    include_str!("../../../examples/map-entry-projection/consumer-fold/history.lkjc");

pub(super) struct Folds {
    supplier: Native,
    pub consumer: Native,
    pub package: String,
    pub package_revision: String,
    proposals: Vec<PathBuf>,
    artifact: PathBuf,
    identities: Vec<(PathBuf, blake3::Hash)>,
    sequence: Cell<usize>,
}

impl Folds {
    pub fn new() -> Self {
        Self::from_request(CONSUMER, &["imported", "trap-probe"], "128")
    }

    pub fn history() -> Self {
        Self::from_request(HISTORY, &["history"], "124")
    }

    fn from_request(request: &str, targets: &[&str], expected_tests: &str) -> Self {
        let supplier = Native::template("command");
        let library = supplier.input(
            "library.lkjc",
            &LIBRARY.replacen("base=BASE", &format!("base={}", supplier.revision()), 1),
        );
        supplier.apply(&library, &supplier.plan(&library, true), true);
        let checked = supplier.cli(&["check"], true);
        assert_eq!(
            compact_field(compact_record(&checked, "tests"), "passed"),
            "121"
        );
        let transport = supplier.root.path().join("library.lkjp");
        let exported = supplier.cli(
            &[
                "package",
                "current",
                "export",
                "--kind",
                "transport",
                "--output",
                path(&transport),
            ],
            true,
        );
        let package = compact_record(&exported, "package");
        let consumer = Native::template("command");
        consumer.cli(
            &[
                "package",
                "dependency",
                "stage",
                "--transport",
                compact_field(package, "transport"),
                "--input-file",
                path(&transport),
            ],
            true,
        );
        let source = request
            .replacen("base=BASE", &format!("base={}", consumer.revision()), 1)
            .replace("FOLD_PACKAGE", compact_field(package, "id"))
            .replace("FOLD_SEMANTIC", compact_field(package, "revision"))
            .replace("FOLD_REVISION", compact_field(package, "package-revision"));
        let input = consumer.input("consumer.lkjc", &source);
        consumer.apply(&input, &consumer.plan(&input, true), true);
        let checked = consumer.cli(&["check"], true);
        assert_eq!(
            compact_field(compact_record(&checked, "tests"), "passed"),
            expected_tests
        );
        assert_eq!(
            compact_field(compact_record(&checked, "tests"), "differential"),
            "equal"
        );
        let artifact = consumer.root.path().join("folds.lkja");
        consumer.cli(&["build", "--output", path(&artifact)], true);
        let mut descriptor: Value = serde_json::from_slice(
            &std::fs::read(consumer.project.join("command.deployment.json")).unwrap(),
        )
        .unwrap();
        descriptor["artifact"] = json!("folds.lkja");
        let mut paths = vec![
            supplier.executable.clone(),
            consumer.executable.clone(),
            artifact.clone(),
        ];
        for target in targets {
            descriptor["target"] = json!(target);
            paths.push(consumer.input(
                &format!("{target}.deployment.json"),
                &descriptor.to_string(),
            ));
        }
        let identities = paths
            .into_iter()
            .map(|p| {
                let digest = blake3::hash(&std::fs::read(&p).unwrap());
                (p, digest)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            identities[0].1, identities[1].1,
            "one exact selected product"
        );
        Self {
            package: compact_field(package, "id").to_owned(),
            package_revision: compact_field(package, "package-revision").to_owned(),
            supplier,
            consumer,
            proposals: vec![library, input, transport],
            artifact,
            identities,
            sequence: Cell::new(0),
        }
    }

    pub fn detach(&self) {
        std::fs::remove_dir_all(&self.supplier.project).unwrap();
        std::fs::remove_dir_all(&self.consumer.project).unwrap();
        for path in &self.proposals {
            std::fs::remove_file(path).unwrap();
        }
        assert!(self.artifact.is_file());
        assert!(!self.supplier.project.exists() && !self.consumer.project.exists());
    }

    pub fn run(
        &self,
        target: &str,
        input: &Value,
        detached: bool,
        success: bool,
    ) -> (Vec<CompactRecord>, Option<Value>) {
        let index = self.sequence.get();
        self.sequence.set(index + 1);
        let arguments = self
            .consumer
            .input(&format!("args-{index}.json"), &input.to_string());
        let result = self
            .consumer
            .root
            .path()
            .join(format!("result-{index}.json"));
        let descriptor = self
            .consumer
            .root
            .path()
            .join(format!("{target}.deployment.json"));
        let mut command = if detached {
            vec!["run", "--deployment", path(&descriptor)]
        } else {
            vec!["run", target]
        };
        command.extend([
            "--arguments-file",
            path(&arguments),
            "--result-file",
            path(&result),
        ]);
        let records = self.consumer.cli(&command, success);
        assert_eq!(
            std::fs::read(&arguments).unwrap(),
            input.to_string().as_bytes()
        );
        let value = if success {
            if detached {
                cases::joined(compact_record(&records, "execution"));
            }
            Some(serde_json::from_slice(&std::fs::read(result).unwrap()).unwrap())
        } else {
            assert!(!result.exists(), "failure cannot expose a partial result");
            None
        };
        (records, value)
    }

    pub fn verify_identity(&self) {
        for (path, expected) in &self.identities {
            assert_eq!(&blake3::hash(&std::fs::read(path).unwrap()), expected);
        }
    }
}
