# Authenticated child images

This contract belongs to the compiler stream. Programs owns affine transfer,
process lifetime, deadlines and the supervisor API. A path, a readonly file,
a hash supplied by an untrusted producer, or an authority report is not admission.

## Compiler interface

`build-child SOURCE --entry NAME --image-key KEY -o IMAGE` checks source and
emits a transfer-only Linux x86-64 ELF plus `IMAGE.admission`.
`image-authority IMAGE --admission RECORD --image-key KEY` authenticates the
record against the exact executing compiler, creates and seals an owned memfd,
checks exact ELF length/SHA-256 on that immutable object, and prints the canonical
checked entry manifest. The default record is `IMAGE.admission`. Unsupported
profiles reject; `--entry NAME` on the query additionally requires that identity.

KEY is exactly 32 private bytes outside every source/library/program grant.
A key reader has compiler-replacement authority. No key enters either child
or parent program. Parents embed an admitted image identity; runtime verifies
the same immutable descriptor that it executes. Changing the compiler requires
re-admission; a stale producer does not silently inherit current checker policy.

The admission payload uses length-delimited little-endian fields, and binds:
target, transfer ABI digest, selected checked entry, scalar result kind,
parameter-ordered slot names and stable kinds, exact ELF byte length and digest.
The envelope uses the authenticated store root framing with a separate
`child-image` domain. Canonical encoding is checked on decode; trailing data,
wrong target/ABI/producer/key, reordered/duplicate slots and image changes reject.

## Transfer ABI version 1

Descriptor 3 is the existing private scalar/trap result channel. A zero-slot
entry accepts no packet: it validates descriptor 3, closes descriptors 0–2 and
4 through the kernel maximum, and has no Arguments, input or stream authority.
This form uses the existing `Process.spawn_file` interface directly. The ABI
digest explicitly includes the zero-slot exception.

For an entry with parameters, descriptor 4 is a private packet memfd sealed with F_SEAL_SEAL, F_SEAL_SHRINK, F_SEAL_GROW
and F_SEAL_WRITE. Descriptor 8+i is the descriptor belonging to parameter i;
descriptor numbers never come from packet bytes. The image descriptor is a
separate sealed memfd, retained from verification through execveat(AT_EMPTY_PATH)
and marked close-on-exec. A writable alias cannot change a sealed memfd.

The packet is little-endian:

| Offset | Size | Meaning |
| --- | ---: | --- |
| 0 | 8 | ASCII `LKJCHD1\0` |
| 8 | 32 | SHA-256 of canonical entry manifest |
| 40 | 8 | Parameter count |
| 48 | 24 per parameter | Kind code, payload offset, payload byte length |
| after slot table | variable | Canonical consecutive payloads |

A descriptor or token slot has offset and length zero. Value payloads follow
slot order, without gaps, aliases or trailing bytes. Startup checks packet seals,
length, magic, manifest, slot count/kinds and complete framing before entering
user code. It imports descriptors and values, closes every undeclared descriptor
(including ambient standard streams), and does not open paths, bind listeners,
read environment variables, or reacquire World.

Supported descriptor kinds are Directory, ReadDirectory, File, ReadFile,
Listener, Readiness, NullDevice, Stdout, Stderr and Stdin. Standard-stream
descriptors are installed at their fixed runtime descriptors only after all
slots validate. Descriptor kinds are checked with the relevant kernel operation. Readiness uses
a fresh private eventfd and requires EPOLL_CTL_MOD to return ENOENT; the probe
closes immediately without consuming an event or changing a registration. A
regular-file EPERM result cannot establish epoll identity. The checked parent's affine contract remains the proof of transferred ownership.
No packet-supplied pointer becomes a program address.

Clock, Entropy and ExitStatus are explicit token slots. Connector contains the
canonical sixteen-byte IPv4 sockaddr, and Environment contains an option byte
followed, when present, by non-NUL value bytes and exactly one trailing NUL.
Absent is the one byte `0`; present-empty is the two bytes `1,0`.
Arguments explicitly admits the supplied argv; no Arguments slot means argv is
not observable. Stop, Process, ForeignCommand, Child and World currently have
no transfer profile and reject rather than manufacture new authority.

Invalid startup exits with status 126 before the entry, without touching even
the private result descriptor. Signal and language-stack setup are included in
this silent pre-admission phase. Runtime diagnostics become available only after
all imports validate, streams install and undeclared descriptors close. Packet
mapping stays readonly for the
process lifetime. Programs must clear ambient environment, close unpassed
descriptors, reject missing/extra/type-mismatched transfer values before exec,
and enforce parent-death and deadline behavior. Those lifecycle obligations
are independent of compiler image authentication.

## Proof ownership

Compiler admission authenticates checked entry types and the trusted emitter's
exact transfer-only bytes. Runtime sealing fixes byte identity before hashing.
Only the sealed descriptor is retained for execution; reopening a pathname,
read-only alias, or a second copy after verification is forbidden. Descriptor
3/4/8+i positions cannot alias each other or the image. The compiler does not
claim that a caller with arbitrary OS authority is a checked affine parent.

## Stable slot codes

| Code | Kind | Representation |
| ---: | --- | --- |
| 1 | Arguments | Explicit token admitting supplied argv |
| 2 | Environment | Packet option and value |
| 3 | ExitStatus | Explicit token |
| 4 | Stdout | Descriptor, writable |
| 5 | Stdin | Descriptor, readable |
| 6 | Stderr | Descriptor, writable |
| 7 | Clock | Explicit token |
| 8 | Directory | Directory descriptor |
| 9 | ReadDirectory | Directory descriptor, checked readonly operations |
| 10 | File | Operational file descriptor |
| 11 | ReadFile | Readable regular-file descriptor |
| 12 | NullDevice | Character device with Linux null identity |
| 13 | Listener | Listening IPv4 stream socket |
| 14 | Connector | Canonical IPv4 endpoint in packet |
| 15 | Readiness | epoll descriptor |
| 16 | Entropy | Explicit token |

O_PATH is accepted only for directory kinds. Descriptor 3 cannot be a sealed
packet/image alias. Files and directories with the same underlying kernel object
still require the checked parent's ownership proof; untrusted packet bytes cannot
supply that proof.

## Immutable compiler object and witnesses

`admitted_image::SealedImage` has private fields. Its constructor accepts an
authenticated `child_image::Admitted`, copies the exact input length once into
an owned temporary memfd, marks it executable, and transfers a descriptor alias
to the independent syscall broker. The broker applies all four seals and reads
them back with F_GET_SEALS before the compiler hashes the sealed File. SCM_RIGHTS
retains the same kernel open-file description. The broker is joined and its alias
closed on success and refusal; it never copies a verified executable.

The constructor returns the canonical manifest and a shared executable borrow
only after the length and SHA-256 match. That File must reach execveat through
duplication or descriptor transfer; no pathname reopening or writable recopy is
permitted. The general `File.seal()` operation restricts an existing owned File
and does not manufacture image admission or grant authority.

`tools/child-image-test.sh` covers authenticated envelope mutations, exact
producer/entry/image matching, source and include attempts to read the private
key, File sealing in both engines, and the real constructor's retained File
through `Process.spawn_file`. That zero-slot child returns a scalar after
writable aliases fail and its old pathname is replaced; ambient input/output
sentinels remain untouched.

`--native-witness` additionally uses an independent optional C/Linux launcher
to attack nonzero packet framing and descriptor kinds, sealed aliases, pathname
substitution, ambient descriptor cleanup, and denied signal/mapping setup.
C is never a production host. Affine transfer and process lifecycle remain the
Programs stream's pending contract in `external_child_authority.lkj`.
