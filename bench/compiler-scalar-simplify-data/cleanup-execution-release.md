# Scalar cohort closure execution release

Root reviewed guard 486fcd8db56a2ccd1d36117df5621cb97fa5656320cfe15511ce509632f4bf27, launcher 5b41fed67f2a9c6b84ff01dabda0adc0d4f4e9e5d98c5b62efdd28426ea311d4 and snapshot 529395e9b74464531137d27bc05b72df3d368bbe0c77edfbea97f68149efe132. The separate bounded peer review (compiler-scalar-cleanup-metadata-review-283) found no concrete scope or false-accept blocker. It compared the inventory, policy and corrected proof with the committed files; it did not execute the guard or inspect original-file contents.

After this release is committed, run this exact launcher once with --remove. Its complete preflight must verify all 72 recovery files against accepted SHA-256 pins and exact Git blobs at 0b0b27803d27b932a5b91cde2d766bd3c8bb82b9, then all 337 live originals/207,027,139 bytes and 22 directories before any unlink. It excludes every path except the two literal cohort roots in the retained plan. The canonical target/lkjscript must still be the pinned 32,584,542-byte 6cd52308 executable.

The launcher retains complete diagnostics, time/load and per-file progress under one fresh exclusive output directory with its declared 4 GiB/900-second/ten-second-grace envelope. No check-only warmup is released. Stop on any failure, preserve partial progress and do not retry automatically. Removal is cooperative and non-atomic; no hostile-race or rollback claim is made. A successful result requires both explicit ENOENT observations and canonical preservation, followed by a separate root result audit.

This release does not delete the scalar worktree, caches, source/proposal/audit preparation directories, global key, unrelated operation credentials, services or other projects. The discarded optimization remains discarded.

A root JavaScript parse error occurred while composing the first release call, before any gateway tool was submitted. It performed no copy, commit, guard or deletion; this is the first submitted release operation.
