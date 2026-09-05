# Immutable source for offline package composition

Status: selected by campaign `202609050819-offline-packages`; implementation incomplete.

The public-authored baseline can export and stage a non-standard interface, but reviewed
`add.dependency` rejects it with `change_dependency_binding_unsupported`. Lifecycle preparation also
only links the embedded standard artifact. Removing either restriction alone cannot supply or
validate dependency implementations.

Use the existing logical package revision and physical transport owners for a single code-complete
container. Reconstruct current immutable canonical graphs, validate private meaning and projected
interfaces, atomically publish closure readiness under `PACKAGE-TRANSPORTS`, and compile immutable
package views with the existing compiler. Root accepted dependencies select meaning; staging never
selects semantic HEAD. Standard remains the maintained supplier and uses common admission.

This deliberately transports private bodies without promising confidentiality and rejects imported
object code as authority. It avoids writable shadow repositories and a second package authoring
model. Exact dependency conflicts reject across the whole closure, and installed transitive source
does not broaden direct public visibility.

Local disconfirmation also fixes cache recovery factoring: a corrupt immutable compiler-unit
object occupies the same content key as a clean recompilation, so ordinary persistence cannot
replace it. The bounded unit fault test now proves source-validated read-only compilation gives
identical artifacts without rewriting that pack. The inherited copied extraction test caught an
overbroad fallback: corrupt pointers over sound immutable objects must still repair persistently
and become exact-current on the next command. Both cases retain unchanged semantic HEAD and
canonical source. Raw cases are retained under `.artifacts/campaign/202609050819/`; the source
admission preflight records standard at 231,436 bytes, lkjournal at 1,090,191 bytes, and the diamond
at 255,589 bytes, all within the fixed independent dimensions.

Reference-path inspection found residual compiled function signatures, nominal layouts, root target
selection, and artifact blob reads after canonical dependency bodies were supplied. That mechanism
could mask a resolver fault. Replace these reads with a disposable index of the independent canonical
inventory and canonical source blobs, retaining point reads for evaluated owners. Repository-backed
reference observations now account for the initial inventory as well as execution reads; admitted
package views reuse their immutable inventory. Safe faults remove compiled callable/layout/type/
target tables and omit a compiled test: canonical execution must survive the former and complete
test-inventory comparison must reject the latter. Production compilation and live-effect grant
binding remain unchanged; live adapters are never replayed for differential proof.

The first full profile exposed a verification-ordering race: the release workspace build selected
candidate `a5efcb80f1343ba93c2e5c3d5d480f57066f78a6b2552299c49daaf17ce17d9a`,
then the concurrent release CLI test selected feature-unified candidate
`452ed604efe0136eef12f920a11c6fe3b0e60a9c6a6c718abfed39de0bdc60c0` at the same
path. The offline workflow and cleanup passed on its private copy, but the exact-source-candidate
receipt check correctly rejected the changed path. Bind the final executable output to the release
CLI lifecycle gate and order every copied application oracle after it. Keep every inherited gate,
exact fingerprint input, and candidate-equality check; this is dependency ordering, not evidence reuse.

Reconsider only with maintained workload evidence that this model cannot meet its stated bounded
public workflow, independent closure/reference proof, or dependency-closed migration. Registry
resolution, signatures, operational migration, and publication remain separate decisions.
