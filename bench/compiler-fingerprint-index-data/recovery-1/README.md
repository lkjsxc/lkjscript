# Fingerprint integration: three-root recovery preparation

This is a finite recovery map, **not a cleanup release**. Every original remains in its existing root.

| Owned completed root under target/ | Files | Logical bytes |
|---|---:|---:|
| compiler-fingerprint-index-preparation-1 | 9 | 39,730 |
| compiler-fingerprint-index-golden-1 | 1,816 | 45,498,446 |
| compiler-fingerprint-index-integration-1 | 49 | 1,382,925 |
| Total | **1,874** | **46,921,101** |

The inventory also records all **57 directories**. No symlink, special file or hardlinked regular leaf was accepted. files.tsv and directories.tsv retain the exact original paths, modes, owners and inode identities. Providers recover file bytes; original permission modes are recovered from this inventory. No historical nanosecond-metadata claim is made.

## Recovery providers

**1,872 originals were directly compared byte-for-byte to immutable Git blobs.** providers.json names the exact commit and path prefix for each row:

- Nine preparation files: commit1f74338b.
- Forty-six golden preparation/control records and the two source overlays: commit042bce95.
- Twenty-three raw execution records, the exact release record and forty-nine integration records: commit88a06df2b.
- The remaining1,742 copied source files: Git9326dbaa.

All raw text records already had committed exact providers, so **zero duplicate raw-record copies** were added. Existing large source correspondence proofs are referenced through those providers rather than copied again.

The golden executable was retained exactly once as artifacts/golden-image-99a179be.gz using gzip -n -9. It is **315,511 bytes**, SHA-256 **f7497c1346b56a6eb513137dd86f44a0fb27e8ebde525e99783bc4d564dc7f57**. One full streaming expansion compared every byte with the owned1,568,827-byte image and confirmed original SHA-256 **99a179be5fd5f6cd9c8e7d3e4466468ff4b519a406303e77e935c1d27dcade89**. No expanded duplicate was written. This new gzip still requires root's Git retention before any later cleanup.

## Explicit pending comparison

The owned baseline compiler is the only row marked **pending direct seed-expansion comparison**. Its current32,588,975 bytes hash to b01eabfa…; its provider is e5294c339d9aefbbafc2e951c997f4fefabcc9ad:seed/lkjscript.gz. The6,551,428-byte compressed Git blob hashes to c7912e43…. The expanded identity agrees with the committed golden producer pin.

This task did **not** expand that seed, rebuild b01, or compare its expanded bytes anew. The map therefore identifies the exact known recovery provider without upgrading it to a new direct-byte proof. Baseline A remains in its original location and must remain available until the comparison has its own producer and root separately releases cleanup.

## Execution evidence and scope

recover.pl passed Perl syntax and ran once under4GiB/900s/10s external limits. It completed with status0 in **0.41 seconds**, user0.34/system0.05, RSS48,112KiB. The single gzip completed with status0 in **0.10 seconds**, user0.09/system0.00, RSS1,672KiB. Load remained **4.83/3.78/2.90**. Exact argv, source code, timestamps, status, stdout/stderr and GNU time are retained under records/. These are retention timings, not compiler measurements.

There was no deletion, compiler/native workload, key read, canonical read, source generation, unrelated archive expansion, process/FD observer or Git commit. Gate-store, measurement-preparation and all other roots were outside this task.
