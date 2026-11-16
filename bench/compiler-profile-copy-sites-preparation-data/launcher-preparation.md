# Launcher preparation only

The launcher implements the approved plan with four sequential compiler
build/execution phases, two official profile reads, one read-only accounting
helper invocation and content-addressed gzip preservation. No workload has run.

Root must install the launcher, plan and helper as committed bench files and
commit a completed release JSON before separately releasing this invocation:

    bash bench/compiler-profile-copy-sites.sh run bench/EXACT_RELEASE.json

The example JSON is intentionally non-executable: unresolved repository,
producer, tool and acceptance pins refuse before any compiler command. It has
no moving-producer default. All acceptance records must be retained bench files
with exact Git and working-byte identities; the launcher verifies their pins
and explicit passed status, not their substantive contents. Root's independent
review remains the authority for acceptance and stated coverage limits.

The source/library working tree must equal the gated producer source commit.
The current HEAD may additionally contain seed, accepted evidence and protocol
commits. The producer seed archive and bootstrap recipe are checked against
their declared Git bytes; the root's fixed-point/seed acceptance establishes
their relation to the exact producer binary. The launcher does not run a seed
or bootstrap as a preflight.

The evidence and fixture are distinct fresh paths, atomically claimed with
ordinary PID/start ownership markers. There is no process/FD inspection,
tracing, race or kill campaign. Commands run in the foreground under the fixed
watchdog and address-space limit. The completed wrapper and capture status
release the next phase. Partial evidence is kept and consumes the attempt;
there is no resume, retry, cleanup mode or automatic cleanup.

The accounting helper CLI is:

    perl HELPER --profile SAMPLE --companion IR --summary SUMMARY --detail DETAIL --output ABSENT_DIRECTORY

Its eight required outputs are accounting.json, kinds.tsv, functions.tsv,
function-map.tsv, widths.tsv, roles.tsv, width-roles.tsv and manifest.sha256.
The launcher preserves the successful official commands' raw stdout/status/
time/load records separately; helper stdout is not an execution attestation.

The archived source, immutable producer/helper copies, both emitted producer
images, both oracle outputs, raw profile and selected IR remain for independent
root acceptance. Raw profile/IR hashes are checked again after analysis; their
gzip copies bind both original and compressed bytes. A later guarded cleanup
is a separate root decision.

Only Bash syntax and extracted embedded-Perl compilation are authorized for
this preparation. In particular, do not invoke run, a producer, the accounting
helper against data, or a substitute dry-run workload to test this launcher.
