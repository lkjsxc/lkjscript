# Persistent immutable lists and graph-owned mapping

Status: accepted; complete implementation acceptance is recorded in
[`202609091322-persistent-lists.json`](../evidence/202609091322-persistent-lists.json).

Repeated immutable append previously allocated and copied every preceding value occurrence.
A graph fold producing N outputs therefore copied N(N-1)/2 preceding occurrences. Existing bind,
rank-one function values, and tail calls could express mapping, but the default collection budget
made large output-producing composition unusable.

Use one neutral persistent index trie with fanout 32 and a tail of at most 32 immutable element
handles. Append copies only the tail or changed branch spine, irrespective of alias ownership.
Length is constant time; traversal is linear; indexing now follows a logarithmic path. This adds
node/refcount metadata and can increase small-list storage or indexing cost. The deterministic
reservation schedule charges all 32 slots in every new node, including unused capacity, plus
node and new element metadata bytes before allocation. Logical length remains separately bounded.
No cumulative refund or reference-count-dependent discount is admitted.

Standard `list-map<Input,Output>` is ordinary accepted meaning over fold-left and append. Bind
retains only its mapper in a private generic step. No map opcode, builder language, application
special case, new collection kind, generic capture constraint, or encoding migration is introduced.
The maintained application rebinds its exact standard dependency without changing application owners.

Public authoring exposed a preparation omission: a generic fold instantiated with `List<Output>`
could require a composite substituted type absent from the stored type inventory. Finite disposable
closures now derive those canonical identities independently from bytecode and accepted owners,
under existing validation-work and depth bounds. Graph 11, TypeObject 10, compiler-unit 6,
bytecode 4, Artifact 16, and unchanged typed-data bytes remain current.

VM/reference agreement cannot establish the shared carrier's correctness. A separate flat-Vec
history oracle and independent node/handle charge schedule provide the storage claim, with wrong
tail and restored-prefix-copy fault sensitivity. Public configured mapping and a removed producer's
private-helper factory prove new composition usefulness. They are new witnesses, not evidence of
pre-existing application adoption. Transactional HTTP exercises mapped wire and stored sequences
with one commit, trap/cancellation rollback, and healthy recovery.

Reconsider fanout or packing if measured metadata or indexed-read costs materially obstruct useful
programs and another immutable bounded layout retains the same alias, accounting, admission,
encoding, and independent-proof obligations. Timing disappointment alone does not justify restoring
whole-prefix rebuilding. Broader collections and explicit generic capture constraints remain separate
possible decisions, requiring their own concrete outcome and authorization.
