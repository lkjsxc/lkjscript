use super::*;
use std::cell::Cell;

const SOURCES: [(&str, &str); 3] = [
    (
        "library",
        include_str!("../../../examples/owned-fifo/library.lkjc"),
    ),
    (
        "steps",
        include_str!("../../../examples/owned-fifo/steps.lkjc"),
    ),
    (
        "trace",
        include_str!("../../../examples/owned-fifo/trace.lkjc"),
    ),
];
const ELEMENTS: &str = include_str!("../../../examples/owned-fifo/elements.lkjc");
const APPLICATION: &str = include_str!("../../../examples/owned-fifo/application.lkjc");
pub(super) const TARGETS: [&str; 8] = [
    "fifo-cells",
    "fifo-buffers",
    "fifo-packets",
    "fifo-forwarded",
    "fifo-empty-front",
    "fifo-cells-batch",
    "fifo-buffers-batch",
    "fifo-packets-batch",
];

pub(super) struct Fifo {
    pub supplier: Native,
    pub consumer: Native,
    pub generic: Export,
    pub identities: Vec<(PathBuf, blake3::Hash)>,
    pub serial: Cell<usize>,
}

fn native() -> Native {
    let mut public = Native::template("command");
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        public.root.disable_cleanup(true);
        println!(
            "retained native FIFO proof, including rejected inputs: {}",
            public.root.path().display()
        );
    }
    public
}

fn accept(public: &Native, label: &str, source: &str) {
    let input = public.input(
        &format!("{label}.lkjc"),
        &format!("request base={}\n{source}", public.revision()),
    );
    let planned = public.plan(&input, true);
    public.apply(&input, &planned, true);
}

fn checked(public: &Native, expected: &str) {
    let records = public.cli(&["check"], true);
    let tests = compact_record(&records, "tests");
    assert_eq!(compact_field(tests, "passed"), expected);
    assert_eq!(compact_field(tests, "failed"), "0");
    assert_eq!(compact_field(tests, "differential"), "equal");
}

impl Fifo {
    pub fn new() -> Self {
        let supplier = native();
        for (label, source) in SOURCES {
            accept(&supplier, label, source);
        }
        checked(&supplier, "117"); // 116 exact standard tests and the command starter.
        for module in ["fifo-support", "fifo-steps", "fifo-trace"] {
            let draft = unchanged(&supplier, module);
            assert!(draft.contains("(constraint owned)"));
            assert!(!draft.contains("OwnedI64Cell") && !draft.contains("ByteBuffer"));
        }
        // Publish generic admission before any concrete element implementation exists.
        let generic = export(&supplier);
        let consumer = native();
        stage(&consumer, &generic);
        let imports = format!(
            "declarations.begin\n(units (use fifo-support {} {}) (use fifo-trace {} {}))\ndeclarations.end\n",
            generic.package, generic.revision, generic.package, generic.revision,
        );
        accept(
            &consumer,
            "elements",
            &format!("{}{imports}{ELEMENTS}", dependency(&generic)),
        );
        accept(&consumer, "application", &format!("{imports}{APPLICATION}"));
        checked(&consumer, "118"); // One additional consumer-owned starter test.
        let draft = unchanged(&consumer, "fifo-app");
        assert!(draft.contains("(borrow-from ") && draft.contains("(borrow-call "));
        let artifact = consumer.root.path().join("fifo.lkja");
        consumer.cli(&["build", "--output", path(&artifact)], true);
        let mut paths = vec![
            supplier.executable.clone(),
            consumer.executable.clone(),
            artifact,
        ];
        for target in TARGETS {
            paths.push(deployment(&consumer, "fifo.lkja", target));
        }
        let identities: Vec<_> = paths
            .into_iter()
            .map(|file| {
                let digest = blake3::hash(&std::fs::read(&file).unwrap());
                (file, digest)
            })
            .collect();
        assert_eq!(identities[0].1, identities[1].1, "one exact copied product");
        Self {
            supplier,
            consumer,
            generic,
            identities,
            serial: Cell::new(0),
        }
    }

    pub fn detach(&self) {
        for public in [&self.supplier, &self.consumer] {
            std::fs::remove_dir_all(&public.project).unwrap();
            for entry in std::fs::read_dir(public.root.path()).unwrap() {
                let file = entry.unwrap().path();
                if file
                    .extension()
                    .is_some_and(|suffix| suffix == "lkjc" || suffix == "lkjp")
                {
                    assert!(file.is_file());
                    std::fs::remove_file(file).unwrap();
                }
            }
            assert!(!public.project.exists());
        }
        assert!(!self.generic.path.exists());
        self.verify_identity();
    }

    pub fn verify_identity(&self) {
        for (file, expected) in &self.identities {
            assert_eq!(&blake3::hash(&std::fs::read(file).unwrap()), expected);
        }
    }
}
