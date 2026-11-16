# Corrected preparation after the archive-reader failure

The original prospective protocol 249af04e480480490f059b89af48b7469730c8e98c49c5627d21514ee11b9f80
terminated when setup1 failed. Its full result, controls, archive and all twelve
unexecuted comparison rows remain unchanged in original-cohort-closed.json
and setup-failure/. No native command or measured build occurred.

The first failure exposes a preparation-reader bug: Archive::Tar 2.40 leaves
two Git per-file PAX paths unresolved. Root approves one corrected preparation,
which repeats input recovery using the reviewed raw-header/PAX association
decoder. This is an explicit technical amendment to the preparation boundary;
the old execution release does not authorize the new manifest or root.
The same source bytes, producers, shape, seed, desired 100,000 lines, limits
and proposed twelve-build order are retained. There are no observed samples
to discard or select. This amendment permits no automatic further retry.

Root and the independent reviewer inspected the exact correction
62a3bf6c9318556f3e3919e39f382e60c1ccab4c7c7b325bdd4e80cfe679181c.
The separately authorized read-only validation passed once: 1,742 regular
members, 42 directories, 1,784 raw/decoded associations and exactly two
immediately associated PAX paths, with all Git content/mode/set checks.
It took 0.28 s, load 1.16/1.50/2.23 to 1.22/1.51/2.23, and performed no
extraction or native execution. The proposed run root remains absent.

This commit releases only the fixed five-phase setup at
target/compiler-reference-capture-measurement-2, using controls manifest
fe8321b36a88864b33dd3c8bda9a71ab954f9dae14926d3ba9e45fbb564af847:

    bash target/compiler-reference-capture-setup-preparation-2/setup.sh --execute-after-root-release fe8321b36a88864b33dd3c8bda9a71ab954f9dae14926d3ba9e45fbb564af847

The unchanged bounds are one compute worker, 4 GiB address space, 900 s
watchdog, 10 s grace and 3 GiB free before each phase. Stop at the first
failure and retain every remaining phase as not-run. The two native commands
are the fixed generator construction and its single 100000/20261116 run.
Complete input archives and actual generator recovery must be verified and
committed before a distinct comparison release. No A/B build, larger rung,
profile or observer is authorized by this setup release.

Implementation and seed are already pushed through 1148e56a. CI43925403668
currently has a successful Rust witness and an in-progress native gate;
that pending state is not treated as a passing candidate CI result.
