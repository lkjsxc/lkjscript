//! Derived cache file failures neither block semantic acceptance nor replay history.
use super::*;
use std::os::unix::fs::{FileTypeExt, MetadataExt};

#[path = "incremental_cache_support.rs"]
mod helpers;

#[test]
fn compiler_cache_fifo_check_recovers_without_changing_authority() {
    let public = Native::template("command");
    public.cli(&["check"], true);
    let original = public.root.path().join("original.lkja");
    public.cli(&["build", "--output", path(&original)], true);
    let revision = public.revision();
    let before = content_inventory(&public.project);
    let current = helpers::fifo(&public);
    let checked = helpers::bounded(&public, "recover-check", &["check"]);
    assert_eq!(
        compact_field(compact_record(&checked, "compilation"), "cache"),
        "clean-recovery"
    );
    assert_eq!(
        compact_field(compact_record(&checked, "tests"), "failed"),
        "0"
    );
    assert!(std::fs::symlink_metadata(current).unwrap().is_file());
    assert_eq!(public.revision(), revision);
    assert_eq!(content_inventory(&public.project), before);
    let rebuilt = public.root.path().join("rebuilt.lkja");
    public.cli(&["build", "--output", path(&rebuilt)], true);
    assert_eq!(
        std::fs::read(original).unwrap(),
        std::fs::read(rebuilt).unwrap()
    );
    let exact = public.cli(&["check"], true);
    assert_eq!(
        compact_field(compact_record(&exact, "compilation"), "cache"),
        "exact-current"
    );
}

#[test]
fn compiler_cache_fifo_does_not_block_acceptance_or_immutable_replay() {
    let public = Native::template("command");
    public.cli(&["check"], true);
    let input = public.input(
        "once.lkjc",
        &format!(
            "request base={} idempotency=cache-file-admission-once\n{}",
            public.revision(),
            source(1, true),
        ),
    );
    let plan = public.plan(&input, true);
    assert_eq!(
        compact_field(compact_record(&plan, "validation"), "compiler-units"),
        "3"
    );
    let current = helpers::fifo(&public);
    let fifo = std::fs::symlink_metadata(&current).unwrap();
    let accepted = helpers::apply(&public, "apply-with-fifo", &input, &plan);
    assert_eq!(compact_field(&accepted[0], "status"), "accepted");
    let cache = compact_record(&accepted, "derived-cache");
    assert_eq!(compact_field(cache, "status"), "failed");
    assert_eq!(compact_field(cache, "diagnostic-class"), "corrupt");
    assert_eq!(
        compact_field(cache, "diagnostic-code"),
        "compilation_cache_regular_type"
    );
    let revision = public.revision();
    let before_replay = helpers::inventory(&public.project);
    let reviewed = public.plan(&input, true);
    let replay = helpers::apply(&public, "replay-with-fifo", &input, &reviewed);
    assert_eq!(compact_field(&replay[0], "status"), "already-accepted");
    assert_eq!(
        compact_field(compact_record(&replay, "derived-cache"), "status"),
        "not-attempted-replay"
    );
    for field in ["digest", "revision-record"] {
        assert_eq!(
            compact_field(compact_record(&replay, "receipt"), field),
            compact_field(compact_record(&accepted, "receipt"), field)
        );
    }
    let retained = std::fs::symlink_metadata(&current).unwrap();
    assert!(retained.file_type().is_fifo());
    assert_eq!((retained.dev(), retained.ino()), (fifo.dev(), fifo.ino()));
    assert_eq!(helpers::inventory(&public.project), before_replay);
    let checked = helpers::bounded(&public, "repair-after-replay", &["check"]);
    assert_eq!(
        compact_field(compact_record(&checked, "compilation"), "cache"),
        "clean-recovery"
    );
    assert_eq!(public.revision(), revision);
    let run = public.cli(&["run", "extra-main"], true);
    assert_eq!(
        compact_field(compact_record(&run, "execution"), "value"),
        "1"
    );
    assert_eq!(
        compact_field(compact_record(&run, "execution"), "differential"),
        "equal"
    );
    apply_exact(
        &public,
        "next.lkjc",
        &source(1, false).replace("extra", "next"),
        2,
    );
}
