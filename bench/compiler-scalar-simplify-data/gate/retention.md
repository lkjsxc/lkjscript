# Closed scalar gate evidence package

The independent full-gate summary remains directly readable in summary.json and summary.md, with unchanged source/producer pins.

The deterministic gate-captures.tar.gz is 249,265 bytes, SHA-256 21bf2d7999bfcb29a1cdebc452ed668f947bb0a26645bb4b1f6bc1fde3db13cf. It contains exactly 143 regular members: all20 outer cold-gate captures and all123 top-level gate logs, totaling898,185 payload bytes. All member paths, modes, byte counts and SHA-256 values match the copied and live inputs; uid/gid/mtime are normalized to zero.

Copies preserve original target-relative paths beneath captures/. captures-inventory.tsv maps every member to its original path/hash. No generated binaries, nested fixtures, caches, stores or private key contents are included; those scopes remain untouched.

Compression: 0.01 s GNU wall, 0.01 s user, 0 s system, 3604 KiB peak; load 1.64 2.04 2.43 to 1.64 2.04 2.43. Exact argv/cwd/status/output/time/load/UTC/ns records remain. This is packaging time, not compiler performance.

## Qualifications

- Every archive member path,size,mode,payload SHA and normalized uid/gid/mtime was verified against the copied and original live file.
- The independent audit used no compression; this later packaging step was separately released by root.
- The audit helper emitted a nonfatal ambiguous-print parser warning, returned0 and produced the complete valid summary/proof tables. Exact source and syntax/execution stderr are retained. Gate stderr is empty.
- A report-only finalization invocation omitted its directory argument and exited2 before writing retention files; its exact outputs/status are retained. No archive creation or compiler workload was repeated.
- The producer SHA in input-inventory.tsv identifies the inspected executable; this capture archive does not contain that binary.
- No compiler, native image, benchmark, deletion or process observer was executed for audit/retention.
