# Independent scalar cohort audit notes

The accepted immutable cohort is main target/compiler-scalar-simplify.20261116-1, released at c887b7c5aca7bb80d70ca26d7dfdf1a44213a163. Its outer capture is target/compiler-scalar-trial-capture.20261116-1. No measured workload, compiler, emitted native program or frozen harness was executed by this audit.

The accepted reader is audit-v4.pl, SHA-256 f15dede569ec718986fb818a3ab9937fbf9b01b7a54d669ddf573744f722ec9c. Its successful execution is captured under attempt-4; gateway requests compiler-scalar-cohort-audit-execute-v4-268 and compiler-scalar-cohort-audit-poll-v4-269 report exit 0 and complete output. acceptance.json, source-comparisons.json, input-inventory.json and git-read-commands.json contain its proof and exact read population.

Earlier audit failures remain in place:
- audit.pl and root-level captures: list-context regex returned two captures rather than a scalar predicate, causing the assertion diagnostic "20".
- audit-v2.pl and attempt-2: syntax-only failure because JavaScript replacement reduced the intended double-dollar Perl prototype to one scalar argument.
- audit-v3.pl and attempt-3: phase reads passed, then Perl interpreted sort walk(...) as a comparator call without arguments.
- audit-v4.pl only corrects those assertion/sort source forms and adds their disclosure. It does not alter measured files, assumptions, source population, or the decision threshold.

The read-only source comparison includes the uncompressed Git archive, complete extracted src/library/seed population, the exact frozen seed expanded in memory, current producer bytes and current candidate/main src/library. It does not create compressed files, reconstruct a repository, inspect keys/processes, or imply that historical CI validates the candidate.

Scale_and_profile independently joined the same text captures in its single bounded request compiler-scalar-cohort-text-read-20261116-232, reporting no discrepancy. Its calculation agrees: A sum 45.54 s, B sum 45.22 s; 452200000000 > 409860000000, so the 10% rule fails. That review was text-only; the full-byte comparisons are in this audit.

The successful audit used 2.34 s GNU wall, 2.21 s user and 0.10 s system time, with peak 128584 KiB. Its exact load is retained in attempt-4/load-before and load-after. Those audit costs are not benchmark observations.

All helper versions, captures, JSON and report are frozen by manifest.sha256. There is no cleanup permission in this record.
