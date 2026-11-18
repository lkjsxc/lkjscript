# Anonymous writable storage without World

`TemporaryFiles` is an affine, entry-only authority to create fresh anonymous
read/write `File` values. An entry receives it through `--grant NAME`, with no
payload. `storage.temporary_file()` returns `Result<File,i64>` using exactly the
existing anonymous-file runtime operation. The grant cannot name a path, open
inherited streams, acquire an executable, or spawn a child. No default directory,
file-count limit, byte budget, new File operation, or writable mapping is added.
`World.temporary_file()` retains its existing implementation and behavior.

The requested browser-shaped [entry](../docs/proofs/anonymous-wayland-20261118/main.lkj)
uses exactly the Unix socket File, TemporaryFiles, TcpPort, Readiness, Clock and
Stdout. Both engines present a frame through real Weston and use an address
received from that compositor connection to exchange bytes with the independent
TCP peer. Neither World nor a directory grant is present.

## Before the change

The exact published `ca749bd44e1cece139fe5f875adc57b27bcfadd8` seed was extracted
before production edits. Its native SHA-256 is
`3d2fc84dfc880cdd52aee6ca9a16dbef6025500e8453cd7ab75f45f336fda353`.
The [baseline source](anonymous-files-data/baseline/main.lkj) is refused with
`E_NAME unresolved name TemporaryFiles`; the separate existing-Stdout attempt
is refused with `E_UNSUPPORTED method temporary_file on Stdout`.
[Exact diagnostics, identities and replay](anonymous-files-data/baseline/).

The disproof search found a real World-less anonymous File creator:
`Image.buffer(bytes)`. It requires an authenticated child Image and seals the
result against writing, resizing and further sealing. The existing route was
therefore tested before adding a grant, rather than described as nonexistent.
Published ca749bd44 creates the buffer and refuses a write; actual Weston rejects
its `wl_shm.create_pool` with `failed mmap fd 16: Operation not permitted`.
The compositor requires a writable shared mapping of this File. Receiving a
File from a socket or child can transfer someone else's storage but is not an
anonymous-file factory. [Image control and complete raw evidence](anonymous-files-data/image-buffer/).

## Narrow checked contract

The existing `world_file` operation has two independent signatures:

| Receiver and operands | Authority |
| --- | --- |
| `(&mut World, u8 selector)` | Existing selectors: anonymous storage, inherited streams or executable. |
| `(&mut TemporaryFiles)` | Anonymous read/write storage only; existing selector zero is supplied internally. |

A selector on TemporaryFiles is invalid even when zero. A shared receiver,
different capability, absent receiver, wrong result or missing World selector
is refused. The checker establishes this contract; the independent interpreter
also enforces receiver and arity on deliberately unchecked programs before any
host call. Native lowering receives checked IR and calls the unchanged runtime
with selector zero. There is no new trusted operation or host primitive.

Grant provisioning and destruction do not perform I/O. Each creation exclusively
borrows the grant and yields an independent affine File that may outlive it.
The original File operations and ownership rules apply, including sealing,
seek/write and typed transfer. New text/binary IR and authenticated unit type
identities preserve the grant; integer arguments and aggregate constructors
cannot fabricate it. Copy, Clone, Data, Send and Sync remain unavailable.
The grant itself has no child transfer slot; its returned File is transferable.

## One program: frame and restricted TCP

The [combined proof](../docs/proofs/anonymous-wayland-20261118/README.md) creates
its frame File, drops the factory grant, fills and reads the File, then sends it
in the actual Wayland pool request. A small external test module supplies a
numeric address through standard `wl_output.geometry` metadata. The program
receives that value at run time and invokes `TcpPort.connector(address)` followed
by `connect_deadline`. The unchanged independent C peer sends `R`, receives `Q`,
and closes the connection. No host address is compiled into the entry source.

The entry waits for the actual frame callback before signaling stdout. The
observer captures Weston's screenshot and verifies all pixels before closing
its private compositor; socket EOF signals completion. Application signaling
uses no named files. Screenshot and evidence files belong to the observer.

Both [native](anonymous-files-data/weston-native/result.txt) and
[interpreter](anonymous-files-data/weston-interpreter/result.txt) runs match all
**76,800 pixels at 320 by 240** and complete the same TCP exchange:

```text
anonymous frame committed: runtime_address=127.0.0.1 tcp=RQ committed=1 frame_bytes=616704
anonymous frame complete: sent=27 received=39 committed=1
```

The [checked authority report](anonymous-files-data/weston-native/authority.txt)
lists exactly the six grants and contains no reachable named-file create, write,
rename or remove operation. `world_file` remains the IR spelling; its checked
one-operand signature is the anonymous-only operation, not a World grant.

Entry source SHA-256:
`ed8d6893519aacad88ce7edac5fff8c34af625401b40155488aa227cdefcfaa5`.
Both independent compositor screenshots have SHA-256
`d2980c8251ceec85bbec69840807d0b375b97cb67f0a9b64fb03293aacf71e4b`.
The native/interpreter evidence includes raw commands, protocol traces, client
output, pixel verification and source/producer identities.

## Validation and reproduction

The final native producer is
`f1d74833f8d17bd9c4939bd4c2a04065827c800bd9f9399a3775e6cb1c145788`;
the refreshed gzip seed is
`1015f721a91ce8bd216d97a88ba692686ba31a269a4a0d7d0c4883ba89459544`.
Two consecutive native generations reproduce byte for byte.

The fresh-store four-worker [full gate](anonymous-files-data/full-gate.log)
passes in **1015.265 seconds**: all **6466 case requests** run freshly, with zero
case reuse, and 128 IR mutants produce zero findings. The immediately following
[standalone real-program proof](anonymous-files-data/standalone-real-programs.log)
passes its upstream provenance controls, eight tests per engine and all 7,239
native corpus rows. The producer and seed identities remain unchanged after both.
[Machine-readable local validation](anonymous-files-data/validation.json).

Eighteen focused source/IR cases pass all 36 native/interpreter route checks,
with zero pending. Eight independent Rust refusal verdicts agree. The boundary
fixture executes on native, interpreter and Rust and covers independent files,
file lifetime beyond the grant, writing, seeking, resizing and sealing.
Missing, duplicate, unknown and payload-bearing grants reject before execution
or publication, including preservation of an existing output destination.
Wrong typed grants and raw scalar entry arguments reject in both execution APIs
before host acquisition. Unsupported child transfer publishes no child image
or admission. Cold and warm authenticated unit images run and match exactly.
All five old World file selectors remain unchanged on both engines.

[Focused logs](anonymous-files-data/focused/) and
[raw refusal/boundary controls](anonymous-files-data/boundary/) retain commands,
stdout, stderr and exit status. The initial candidate's text-IR dispatch omission
is retained as a failed observation; its repair is in the final producer.

```sh
bash bench/anonymous-files-data/baseline/reproduce.sh
bash bench/anonymous-files-data/image-buffer/reproduce.sh --output target/image-buffer-control
bash tools/temporary-files-test.sh target/lkjscript \
  --witness "$(rustup which rustc)" --evidence target/temporary-files-proof
bash docs/proofs/anonymous-wayland-20261118/run.sh \
  --compiler target/lkjscript --output target/anonymous-frame-native
bash docs/proofs/anonymous-wayland-20261118/run.sh \
  --compiler target/lkjscript --engine interpreter --output target/anonymous-frame-interpreter
LKJSCRIPT_GATE_JOBS=4 bash tools/check.sh
bash tools/real-programs-test.sh target/lkjscript
```

All twelve original-source observations have unchanged exit status, signal,
stdout and stderr identities against published parent `6f6236afe`: nom 17,
DOM 37 and Sozu 427 classified sites plus the retained 34 prior-unclassified
rows. [Exact fresh census comparison](anonymous-files-data/census/comparison.json).
No new original large root or nom own-test admission is inferred. The browser
repository and its compiler pin are left to the browser stream.
