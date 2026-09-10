# Explicit parametric nominal data

Status: accepted for the Graph 13 source cutover. Public distribution is a separate decision.

Records and variants own ordered stable type parameters with explicit `none` or `capture-safe`
constraints. Positive-arity references and constructors supply all arguments. Named types remain
zero-arity. These choices let agents inspect and change a single complete semantic candidate without
inference, implicit defaults or a parallel monomorphized authority. Constraints on phantom parameters
remain interface meaning. Renaming does not change identity.

The base TypeObject 10 encoding is preserved byte for byte. An application uses `LKJTAP01`, version 1,
an explicit kind tag, declaration and ordered argument identities in its own digest domain. Graph 13,
interface 9, compiled unit 8, bytecode 5 and Artifact 17 carry the affected meaning. Existing canonical
transport framing and operational formats are unchanged. Typed-data layout tag 9 includes complete
arguments before substituted members; monomorphic tag 5 and existing golden bytes remain exact.
Changing a phantom argument changes the application layout even when payload bytes coincide.

Production and canonical reference preparation independently derive finite instantiated closures.
Both include concrete signatures, tests, constants, ports, constructors, calls and substituted members,
with checked work, depth and metadata limits. Runtime layout identity includes exact application and
preparation provenance. Raw ingress checks actual values before callbacks; internal checked values
retain their admitted proof without repeated descendant scans.

| Property | Every argument and substituted stored member | Function type | Secret |
|---|---|---|---|
| Ordinary | Checked, including phantom/empty/inactive positions | Transient leaf | Admitted |
| Capture-safe | Checked; bare parameters require an explicit bound | Leaf; actual origin/environment checked separately | Rejected |
| Equality | Checked | Rejected | Rejected |
| JSON / typed data | Checked under their codec rules | Rejected | Rejected |
| Session state | Checked under the concrete repeated-state relation | Rejected | Rejected |

Resources and streams never become ordinary through applications. Capture permission does not imply
durability or session retention. Existing Option/Result codec exclusions remain. JSON has no phantom
identity authentication because its input carries only the existing field/case representation.

Generic definition cycles reject across records, variants, function signatures and monomorphic
wrappers, including growing substitutions. Concrete application substitution must not introduce a
cycle. Finite nesting is supported; wholly monomorphic recursion retains its prior semantics. This
boundary can be revisited only with a precise finite/expanding recursion contract and independently
bounded derivation proof, without weakening provenance or property checks.

The maintained standard adds graph-owned pair construction, projection and ordered mapping. A new
independent public batch/edit/snapshot workload demonstrates package, closure, HTTP data and session
composition; it is new witness adoption. lkjournal receives the encoding and exact dependency cutover,
with its existing owner identities, application behavior and operational data formats preserved.
The temporary predecessor materializer is removed after migration; normal production has one reader
for the current meaning and rejects incompatible predecessors.

Reversal requires an affected-owner and maintained-consumer cutover with explicit format rejection
and recovery. A future public release requires a new authorization; the nominal campaign authorizes
source integration and local target proof only.
