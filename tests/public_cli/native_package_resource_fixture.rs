use super::*;

pub(super) struct Packages {
    pub(super) library: Native,
    pub(super) consumer: Native,
    pub(super) dependency: String,
    pub(super) imports: String,
}

impl Packages {
    pub(super) fn stage(source: &str) -> Self {
        let library = Native::template("command");
        let input = library.input(
            "library.lkjc",
            &format!("request base={}\n{source}", library.revision()),
        );
        let applied = library.apply(&input, &library.plan(&input, true), true);
        library.cli(&["check"], true);
        assert_draft(&library, &identity(&applied, "$library"));
        let transport = library.root.path().join("library.lkjp");
        let export = library.cli(
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
        let exported = compact_record(&export, "package");
        let consumer = Native::template("command");
        consumer.cli(
            &[
                "package",
                "dependency",
                "stage",
                "--transport",
                compact_field(exported, "transport"),
                "--input-file",
                path(&transport),
            ],
            true,
        );
        Self {
            dependency: format!(
                "add.dependency package={} semantic-revision={} package-revision={}\n",
                compact_field(exported, "id"),
                compact_field(exported, "revision"),
                compact_field(exported, "package-revision")
            ),
            imports: format!(
                "(use lib {} {})",
                compact_field(exported, "id"),
                compact_field(exported, "package-revision")
            ),
            library,
            consumer,
        }
    }

    pub(super) fn request(&self, source: &str) -> String {
        format!(
            "request base={}\n{}declarations.begin\n(units {})\ndeclarations.end\n{source}",
            self.consumer.revision(),
            self.dependency,
            self.imports
        )
    }

    pub(super) fn apply(&self, source: &str) {
        let input = self.consumer.input("consumer.lkjc", &self.request(source));
        let applied = self
            .consumer
            .apply(&input, &self.consumer.plan(&input, true), true);
        self.consumer.cli(&["check"], true);
        assert_draft(&self.consumer, &identity(&applied, "$consumer"));
    }

    pub(super) fn detach(&self) -> PathBuf {
        let artifact = self.consumer.root.path().join("generic.lkja");
        self.consumer
            .cli(&["build", "--output", path(&artifact)], true);
        let data = self.consumer.root.path().join("queue");
        compact_success_at(
            &self.consumer.executable,
            self.consumer.root.path(),
            &["data", "initialize", "--root", path(&data)],
        );
        std::fs::remove_dir_all(&self.library.project).unwrap();
        std::fs::remove_dir_all(&self.consumer.project).unwrap();
        data
    }
}

pub(super) fn assert_draft(public: &Native, owner: &str) {
    let draft = public.root.path().join("draft.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--owner",
            owner,
            "--output",
            path(&draft),
        ],
        true,
    );
    let text = std::fs::read_to_string(&draft).unwrap();
    assert!(text.contains("(use borrow)"));
    assert!(text.contains("(use consume)"));
    assert_eq!(
        compact_field(
            compact_record(&public.plan(&draft, true), "result"),
            "outcome"
        ),
        "unchanged"
    );
}
