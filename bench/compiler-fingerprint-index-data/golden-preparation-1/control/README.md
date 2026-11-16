# Baseline fingerprint golden preparation

Status: **prepared only**. No compiler build or native dump has run.

The owned source is copied directly from Git 9326dbaab1bf6832b9f35dc156e12d0d62eccd9e, restricted to tracked src/library/tests. It contains 1744 files / 9390696 bytes after exactly two overlays: src/main.lkj from golden-main.txt and new src/fingerprint_fixture.lkj from incremental-fingerprint-module.txt. The original 1,743-file Git inventory and the complete overlaid inventory are retained. Non-overlay bytes and modes were checked against every Git blob; the baseline incremental module remains 760485a5a12d97ce320ba9113797370df584c154084ee4a1f9689da28c7d1127.

The owned producer is the exact canonical b01 executable (32588975 bytes). It was compared before/copy/after during preparation. Runtime selection is explicit library/runtime.lkj from the pinned source tree (eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14). The source grant is the owned source root only; producer/output/control directories are siblings. No key or store is copied, created, selected or read.

## Pending execution

Root must separately create release.json from release.template.json with the exact control-manifest digest and release status. The reviewed command is in plan.json. capture.pl then performs one no-store inline0 stats build and one native dump, sequentially, each under 4 GiB AS, 900 seconds and a 10-second TERM grace. It refuses an existing execution directory or native output. A failed command or validation terminates this attempt; it does not retry. The script is not run during preparation.

Each command records full argv/wrapper argv/cwd, source and producer hashes before/after, stdout/stderr, raw wait/exit status, GNU time, UTC, integer nanoseconds and three load averages. Optional no-store-only missing stats (such as store_writes) remain NA. The wrapper interval includes process/resource launch, output capture and terminal timestamp acquisition; source/hash/control setup and later validation are outside it. This is a correctness capture, not a performance cohort.

The native dump contains the frozen seventeen cases, including exactly six None results. Validation preserves every Some shape, complete body vector and empty body slot. No baseline values are generated during preparation; expected() still deliberately panics if called, and dump() does not call it. Neither candidate fingerprint code nor an added test case supplies the expected output.

## Preparation capture qualification

The free-space preflight reported 350,903,280 KiB available. Source/producer copying completed in 0.26 s GNU elapsed, 0.17 user / 0.07 system, RSS 81,788 KiB, load 1.82/1.52/1.22 unchanged. No compiler or native code was executed.

A read-only inspection named nonexistent src/driver/stats.lkj; its rg diagnostic is retained in records/preparation/source-inspection-diagnostic.json. The actual stats implementation was then read in src/driver.lkj. This is not a compiler failure. Preparation argv.json uses a cwd-relative logical script path; argv.actual.json and prepare-capture-command.txt retain the exact absolute script and resource-wrapper command actually submitted. No original capture was overwritten.
