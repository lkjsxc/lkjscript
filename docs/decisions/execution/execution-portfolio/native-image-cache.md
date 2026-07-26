# Persistent Verified Native Image Cache

[Execution portfolio](decision.md)

## Status

<!-- LKJ-STATUS id=persistent-native-image-cache status=accepted-selection -->

**Accepted Implementation Selection.** This is the only selected persistent
cache slice. It becomes Current only after the correctness and measurement
gates below pass. No cache endpoint is Current merely because this contract is
accepted.

## Scope

The cache stores only canonical serialized `InstallableImage` values. Source,
HIR, verified SSA, bytecode, optimization authority, executable mappings, and
host resources are never cached. Every invocation still verifies the package
and lock, reloads and validates source, builds typed HIR, constructs and
verifies SSA, and validates bytecode before any lookup.

An optimizing invocation reruns optimization reconstruction, certificate
checking, and ordinary SSA verification before lookup. A hit skips only native
machine lowering and encoding. Every hit still decodes under bounds, validates
image integrity and exact identity, copies into a fresh RW mapping, applies
symbolic relocations, and seals the mapping RX through the existing installer.

The initial cache is local and unsigned. It trusts the package owner's local
same-user artifact tree; hashes detect accidental corruption, not a hostile
same-user replacement. Remote caches, signatures, shared-user stores, direct
executable mapping, compatibility decoders, and cache aliases are rejected for
this slice.

## Exact Key

`lkjscript.native-image-cache` is a content-addressed contract. A key is one
canonical length-framed record containing, in this order:

1. the cache, language, source, typed-HIR, verified-SSA, bytecode,
   resource-category, resource-profile, package-manifest, package-lock,
   module-interface, runtime-call, and native-layout contract digests;
2. the entry module's exact UTF-8 package-relative path, source digest, and
   module digest;
3. the root package digest and canonical lock-file digest;
4. the complete freshly verified SSA digest;
5. the complete resource-profile identity, including implementation maxima,
   base ceilings, and an optional lower-only host ceiling digest;
6. tier, reachable-group root, and optimizing-policy digest;
7. backend provider `lkjscript-native-linux-x86_64`, native-layout producer
   digest, and complete backend limits;
8. target `linux`, architecture `x86_64`, System V ABI, little endian,
   64-bit pointers, and the empty required-CPU-feature set.

Every field is framed with an eight-byte big-endian length. The filename is the
full lowercase SHA-256 of the domain-separated key bytes. Prefixes never
authorize artifacts. Baseline and optimizing entries cannot collide.

## Fresh SSA Identity

Verified SSA identity is a SHA-256 over one canonical, domain-separated,
length-framed traversal of every `Program` field: sources, nominal metadata,
traits and implementations, function identities and signatures, places,
effects, blocks, parameters, instructions, origins, frame states, control
edges, and main. Enum discriminants are explicit stable bytes. Unknown or
unencoded fields are a contract defect; debug text and process-local addresses
are forbidden inputs.

The optimizing key hashes the independently verified optimized program and the
complete optimization-limit policy. No persisted certificate grants proof
authority.

## Image Encoding

The canonical file is:

```text
magic[8] = LKJNIC01
cache-contract[32]
key-digest[32]
payload-length:u64be
payload[payload-length]
file-digest[32]
```

`file-digest` is SHA-256 over every preceding byte. The payload has one fixed
ordered encoding of image contracts, work units, code, entries, relocations,
runtime-call slots, frames and homes, safepoints and exact roots, required root
maps, heap runtime sites and descriptors, source maps, trap maps, and outcome
maps. Counts are `u32be`; byte/string lengths are `u64be`; booleans are only
zero or one; options are only zero or one; discriminants are closed `u8`
values. Function identities encode image-local dense indexes and receive one
fresh process-local namespace during decode.

Decoding rejects wrong magic or contract, key mismatch, bad hash, truncation,
trailing bytes, overflow, unknown discriminants, noncanonical values, count or
length mismatch, invalid UTF-8, non-dense identities, and any image integrity
failure. No bytes become installable until complete decode and integrity
validation succeed.

## Bounds And Publication

The cache root is exactly
`<verified-package-root>/target/lkjscript/native-cache/`. Canonical containment
is checked after directory creation. Object names are full key digests. Readers
reject symlinks and non-regular objects. The same-user local trust assumption
does not claim hostile concurrent filesystem containment.

Default limits are 16 MiB per encoded object, 64 objects, 256 MiB total object
bytes, 100,000 decoded records, and one bounded directory scan. An over-limit
cache remains a semantic miss and publication is skipped; no semantic input or
runtime resource limit is weakened. Eviction is not part of the initial slice.

Publication creates one unpredictable create-new staging file, writes fully,
flushes and syncs it, rereads and validates the complete staged artifact,
renames within the cache directory, and syncs the directory. Failure removes
the staging file and never changes a valid final object. Concurrent same-key
publishers accept only an already-present byte-identical valid winner.
Reproducible staging outputs are removed.

A missing, stale-key, corrupt, or over-limit object has no authority. It records
a miss and follows the ordinary native lowering path. Forced tiers never enter
the VM; auto execution retains its existing VM policy.

## Metrics And Adoption

Metrics record lookups, hits, misses by reason, corruptions, bytes read and
written, publications, skipped-full publications, lookup/decode/publication
time, and per-object cache status. A hit reports zero native lowering/encoding
time, nonzero installation time, and, for optimizing entries, the ordinary
proof-check work and time.

Adoption compares disabled cold runs, enabled cold misses, and warm hits with
interleaved randomized samples on scalar, allocation, Brainfuck, editor, and
SQLite workloads. Retained evidence records exact commit, source/key hashes,
CPU, toolchain, commands, at least 30 measured samples, p50/p95, RSS, disk
bytes, proof work, native entries, W^X, and zero forced fallback.

The candidate becomes Current only if warm-hit p50 time-to-first-native improves
at least 20%, end-to-end p50 improves at least 10% on three representative
workloads, cold-miss p50 regresses at most 10%, p95 regresses at most 5%, RSS
regresses at most 5%, and break-even is within five executions. Otherwise the
integration is removed or retained only as historical rejected evidence.
