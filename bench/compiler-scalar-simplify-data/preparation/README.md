# Scalar validation retention package

This package preserves the released closed evidence from source commit
5b5ce7759c035ae0437e3512e18efc440aa50d2e above base
e0b21e66659dc1830a5bebddf0c1f30fd2d8fa94. It creates no new compiler,
native-test, benchmark or gate observation.

## Contents and verification

- closed-records.tar.gz: all 386 approved proposal/capture/manifest records,
  4,331,830 uncompressed member bytes. This preserves the successful core
  15-operation, explicit tests three-operation and mutation 12-operation
  publications, refusals, original proposal versions, ordinary source checks,
  preliminary live check, bootstrap and focused captures.
- desired-overlay.tar.gz: exactly the three pinned desired src/... files,
  160,341 member bytes. These retain the blank-line differences relative to
  committed actual source; the other 1,741 desired files recover from Git.
- artifacts/: gzip -n -9 copies of the actual f85624e3 compiler, preliminary
  fff89090 native test image and exact 556b0293 tests-input tar.
- copied-proposals/: the retention proposal's six files and pending release
  proposal's eight files copied without changing names or bytes. Pending
  producer/release fields remain pending.
- gateway/: the two named saved auditor results with exact available
  stdout/stderr and metadata. No unavailable submitted command/source was
  invented. The failed auditor was a Perl compile error, not a compiler refusal.
- capture/: actual argv/cwd, limits, UTC/ns times, load, GNU time, exit and
  stdout/stderr for this retention execution.
- verification.json, verified-*.sha256 and package-inventory.json: archive,
  binary, copy and package correspondence.

Both archives use an explicit NUL file list, ustar, fixed mtime/uid/gid and
gzip -n -9. Verification decoded every member, requiring exact population,
regular-file type, original mode, length and SHA-256; it also checked tar
header checksum/padding/end framing and gzip CRC. The three standalone
gzips were decompressed and compared byte-for-byte against their source
files, with exact original lengths and hashes. All 388 original closed-scope
files, the actual producer, the three overlay sources and the 14 copied
proposal inputs were rechecked unchanged after packaging.

| Artifact | Compressed bytes | Compressed SHA-256 |
| --- | ---: | --- |
| closed-records.tar.gz | 1,575,767 | 7eb694401cdd83c3b77c963405adabcd45bae390ef6e3ff8b959ea540a79ee6c |
| desired-overlay.tar.gz | 35,168 | 0fcb239c7d06c6566b1f64699d7a2b8d7e80e38129b0aaf03fc4666f96c5583d |
| artifacts/f85624e3cc694f6a3a3e10c33f822789ae6084e2d5a5b41c3b1fc8fcc9bd2d64.compiler.gz | 6,582,463 | ec4d686eb3a296c97757c479ce308a7c6388f3da8d30dca0079af9cb5684e7c2 |
| artifacts/fff89090526d3c9f0ed6081f57bd54121e51773912b833bff62db31e531a1b85.live.gz | 3,884,857 | 7d93a0051d1cfb8002ea3603b98ea894132b3d57da937247211b5a729433e0ea |
| artifacts/556b02937776b789adc51e04127ba4ff919400c8be047509970f28418f47e156.tests-input.tar.gz | 690,741 | debf8876e6932f06ef381e4268747834eaa147ed98275955579533941219179b |

The corresponding original binary/archive lengths are 32,761,734,
21,251,056 and 5,376,000 bytes. The content-addressed filenames identify
their exact original SHA-256 digests.

## Execution

Gateway launch: compiler-scalar-package-run-1331.
Session: pipe-bakmnngmgifpekenlfdnibhoanagngfd.
Completion retrieval: compiler-scalar-package-wait-1333.

One sequential compression worker; external 4 GiB address-space ceiling,
900-second timeout and 10-second TERM grace. Exit 0; GNU wall 3.98 s,
user 3.98 s, system 0.04 s, peak RSS 10,948 KiB.
Load 1.72/2.13/2.48 before, 1.98/2.18/2.50 after.
These are packaging observations, not compiler performance measurements.
The launcher and parser syntax checks passed before packaging.
The functions wrapper's earlier source-string interpolation failure is
preserved in preparation-note.txt; it did not reach the gateway or alter files.

## Boundaries

Root owns the separate source Git bundle and closed-gate audit/log package.
Neither is recreated or silently incorporated here. The original 13 scopes,
actual executable, desired tree, and both proposal directories remain present.
No deletion or main-worktree edit occurred. No gate/cold-gate/cold-store/
bootstrap-store contents, keys, credentials or durable edit metadata were read
or archived. Exact key paths appearing in original argv records are metadata;
no key contents are included.

The live image remains the preliminary old-emitter validation. Focused success
remains separate from the full gate. Compressed duplicate proposal/overlay source
receives no additional operation-adoption credit. This package does not authorize
a benchmark, cleanup, publication or replacement of the main executable/seed.
