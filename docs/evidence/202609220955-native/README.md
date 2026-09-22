# Native-authoring evidence and its boundaries

These are literal public inputs from the 202609220955 campaign. The original mandate and final
disposition belong to [the campaign](../../campaigns/202609220955.md). The executable used for the
observations below was an immutable **development** musl copy, `candidate-dev-4/lkjscript`, built
from the uncommitted continuation of `301fc05d3d490e5306566ab0d363cc43014a720e`. Later parser,
admission and guide changes require their own evidence; these observations are not stamped with
a later source commit.

`predecessor-flat.lkjc` was planned/applied with the unchanged official v0.1.39 copy. It creates a
generic record and function using the existing explicit parameter/type/field records and checks
successfully. This demonstrates existing expressiveness; it is not an unrun claim that the old
product could not express the program.

`library-create.lkjc` and `consumer-create.lkjc` created separate empty projects through the public
executable, with exact standard/library export and dependency staging. Library checks passed
40 tests (3 new, 37 standard); consumer checks passed 43. Literal portable bodies also live in
the maintained [library](../../../tests/fixtures/native-library.lkjc) and
[consumer](../../../tests/fixtures/native-consumer.lkjc) fixtures. The initial inputs were moved
outside the consumer environment before `change draft`; untouched module drafts planned without
semantic change or recreation. `library-evolve.lkjc` and `consumer-evolve.lkjc` are manual edits
of those product drafts. They add `include_excluded`, preserve existing function/parameter IDs,
and repair tests and exact dependency calls. Evolved checks passed 41 / 44 tests. Dependency-only
replacement failed with `kernel_type_call_arity`, preserving consumer revision
`rev_eb2430769aad27d732311b6a15a6be7e350c553b12e41881d93f0793ee61db24`.

After both authoring projects were outside the artifact-only environment, old/new bundles returned
`{"count":2,"sum":2}` / `{"count":3,"sum":102}`. The overflow command failed with
`normalized_integer_overflow` and joined cleanup. These are runtime-dependent artifacts.

`orbloam-inventory.lkjc` edits the accepted pinned Orbloam project at
`08594a09ca26568d820da1ae9a8a6f38fbed8d79`, without executing its Python generators. The official
v0.1.38 and candidate both passed the original 72 tests with production/reference equality. The
candidate's canonical `envelope` draft planned unchanged before editing. The native patch adds a
nominal definition, ordinary helpers and three graph tests, and changes only the existing envelope
body. The complete project then passed 75 tests. Fixed helper results are `(0,0)`, `(2,7)`, `(2,3)`;
the signed case is a pure helper input, not a gameplay grant.

The official runtime initialized an owned test store and registered a colony through normal HTTP.
The candidate then read that same store. Authenticated `GET /api/view` returned 200, preserved the
existing response fields and complete stored world, and added the summary in
[the independent observation](http-summary.json). Expected values were derived from that same
response's requesting-colony inventory using jq arithmetic. Unauthenticated requests returned
401. After joined shutdown/restart, the world, requesting colony and summary remained equal.
The experiment changed no real consumer pin, repository, deployment or operational save.

The original Docker TERM-stop exits were 137, so those joined restarts do not prove graceful
shutdown. A separate SIGINT stop probe with the same frozen candidate and owned store issued no
HTTP requests and exited 0 with no remaining tasks or cleanup failures. Both the unfavorable
original exits and the separate successful stopped receipt remain in `orbloam-http/`; the
[campaign addition](../../campaigns/202609220955.md#source-copy-diagnosis-and-http-shutdown-distinction--2026-09-22-utc)
keeps their claims distinct.

Both consumer environments used Debian
`sha256:70509c95d1857a3704c0a5d92ee2e0adac95f612a9386889d70760bfd7c1ebba` without Python, Node or
Rust/Cargo toolchains, with only the copied executable, native inputs and owned data mounted.
Early development attempts exposed ordinary setup failures: the GNU debug executable required
newer glibc; absolute artifact paths reject; data roots require public initialization at an absent
destination; deployment descriptors require the explicit streams contract. Their original outputs
are retained, not reclassified as passes.

An independent consumer used only the copied `candidate-dev-5` executable and generated guides,
without compiler sources or implementation hints. [Its original input](independent-create.lkjc)
created `Parcel`, a packing recommendation function, four graph tests and a command target.
After the original input was removed from the mounted environment,
[the untouched draft](independent-draft.lkjc) planned unchanged.
[The edit](independent-edit.lkjc) passed all four tests and changed the result from
`Use an insulated carton.` to `Use a padded insulated carton.` No product operation failed.
One host assertion wrongly counted affected dependents with identical before/after objects as
changed owners; the failed assertion and correction remain in the original report/transcripts.
The supplied development binary SHA-256 was
`e45b354856426ed8241a86fcacb402b323b1752c38359c76e6922fd9c5d341f7`.
Detailed evidence is retained in the owned `independent-consumer/` directory below. This is a
bounded independent usability exercise, not compiler self-hosting or a claim about final bytes.

The literal [wide declaration](wide-declaration.units.lkjc) was authored and drafted with a frozen
release-profile executable from `7d507e1ae9a6e00a7a9db9b24b6835956ee61fd5`. Independent canonical
inspection confirmed all 64 parameter names and positions; the 8,042-byte draft interns the three
compound types and plans unchanged. All product commands succeeded. The first host assertion
incorrectly expected four aliases by counting primitive I64; the corrected assertion reads the
original outputs without replaying product operations. That source's aggregate release-source
check failed a separate legacy runner-vocabulary expectation, so this is a bounded capacity
observation, not source or release acceptance. Originals are `wide-workflow.original.sh`,
`wide-workflow.original.status`, `wide-definition.out`, `wide-draft.lkjc`, `wide-noop.out` and
`wide-assertions.sh` in the owned directory below.

Detailed outputs, copied development/official executables and owned HTTP data remain under
`/home/coder/workspace/lkjscript-native-20260922`. Initial inputs and evolved authoring repositories
are under `/home/coder/workspace/lkjscript-native-evidence-20260922`, outside the consumer mounts.
These local resources have no observed configured expiry; they are not a hosted acceptance
artifact retention guarantee. Recovery keys and private registration responses stay local and
are not part of this tracked evidence.

Small original outputs are retained here byte-for-byte for durable review: the
[official Orbloam baseline](orbloam-official-baseline.out),
[edited Orbloam check](orbloam-inventory-check.out),
[old detached run](old-detached.out), [new detached run](new-detached.out), and
[separate SIGINT stopped receipt](orbloam-sigint-stop.out). The
[independent consumer's original report](independent-consumer-report.md) is also unchanged;
its relative transcript paths refer to the original external `independent-consumer/` directory,
not this evidence directory. None of these development originals certifies a later source SHA
or finalized release executable.

## Final local source acceptance

The [unaltered maintained summary](source-acceptance-summary.json) records fresh success for all
20 release-source gates at `1cdaf72888a1f46359a6d38956050747335f8e32`, stable inputs and zero reuse.
Its original receipt, input/DAG manifests, verifier, logs and retained outputs remain at
`.artifacts/lkjscript-dev/check/1790049279445190456-2331539-0`. Because that checker rotates after
eight managed runs, complete compared archives are additionally retained outside its rotation:
`.artifacts/native-20260922/accepted-source-originals.tar.gz` and
`.artifacts/native-20260922/failed-source-originals.tar.gz`. No date-based expiry was observed for
those archives or the external owned evidence. They do not substitute for hosted producer proof.
The campaign records mainline delivery; the separate hosted continuation below updates its earlier
pending final-candidate/publication observation without relabelling the development evidence.

## Hosted candidate and delivery continuation

Producer [35685667968/1](https://github.com/lkjsxc/lkjscript/actions/runs/35685667968) at
`1cdaf72888a1f46359a6d38956050747335f8e32` completed hosted source/finalized-candidate acceptance
and its candidate terminal on 2026-09-22. Its selection, publication and public-acquisition jobs
were intentionally skipped in that candidate-only invocation.

The [delivery continuation](../../campaigns/202609221813.md) selects the original accepted assets
unchanged as v0.1.40. Promotion [35711837606/1](https://github.com/lkjsxc/lkjscript/actions/runs/35711837606)
used controller `dd160c0e93fa15f895cbb9c3f65538b238a1f742` and completed selection, publication,
anonymous exact/latest acquisition and installed lifecycles, and its terminal. Its candidate job
was intentionally skipped, reusing the admitted original source/finalized-target/installation
acceptance. Immutable release `393611808` was published at 2026-09-22T09:49:23Z; both public routes
selected the accepted product. No product or controller repair or candidate rebuild was needed.

The [bounded final-public-binary observation](public-v0.1.40/README.md) used the anonymously
acquired installed exact executable, whose bytes match the admitted original terminal, outside
the compiler checkout without Python, Node or Rust/Cargo. Literal Parcel declaration units passed
all four independently specified cases and four graph tests. After the original input became
unavailable to that environment, a fresh canonical draft planned unchanged.

A meaning-altered request presenting the original review token rejected with
`commitment_mismatch`, preserving accepted HEAD and original behavior; the unchanged reviewed edit
then succeeded. All 13 observed named identities and the parameter contract remained stable.
Seven named review rows were affected, but only the function and matching test had changed stored
objects; five dependents had identical objects. Edited cases and graph tests passed. With the
authoring project unavailable, old/new bundles retained their independently expected distinguishing
results through public deployment; both exited with zero remaining tasks and cleanup failures.

The witness retains new harness failures for a missing query parent, an unsupported no-op plan
output request and a transcript guard. None establishes a product or controller defect. Owned
containers were removed after joined exits. Literal inputs, drafts, edits and concise observations
live at the linked owner. This is a bounded final-public-binary observation, not a new independent-agent
experiment, maintained adoption, native tool implementation or compiler self-hosting. These new
boundaries do not retest or relabel the original development consumers.
