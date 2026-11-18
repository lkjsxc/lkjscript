# Direct owned Unix File compositor proof

This optional proof presents one 320 by 240 frame through the compiler's typed
Unix stream and owned descriptor operations. It uses an explicitly granted File,
Directory, Readiness, Clock and Stdout. The frame backing is a read/write
Directory.create_new File. The native program fills it with ordinary write calls,
then consumes that owner in wl_shm.create_pool. There is no writable mapping API
and no byte/descriptor bridge.

`wayland.lkj` is a retained copy of the protocol client from lkjbrowser commit
`2d37da55ffd5b881b2d441b049c72642f8ef4b52`. The original source SHA256 is
`026490c4ca6a0d81ed60922cb54d697dd93329c0561ee07e790311c175e09f2a`.
Its transport is adapted locally: construction takes owned capabilities, the
private bridge header is removed, the backing File is moved into send_message,
and receive_message returns owners that this frame-only proof drops. Registry,
xdg-shell, shm and surface logic remain source code. The browser checkout and
its compiler pin are unchanged.

## Reproduce using an already available compositor

From the compiler checkout:

```sh
bash docs/proofs/unix-wayland-20261118/run.sh \
  --compiler target/lkjscript \
  --deps /home/coder/workspace/lkjbrowser/.deps/root
```

The script requires existing Weston, weston-screenshooter, the headless backend
and kiosk shell. It performs no installation. A private temporary runtime root
holds all sockets and frame files. Every compositor and client process is owned
and reaped. The script retains evidence in this directory's `evidence/` subtree:

- `frame.png` comes from Weston's own screenshooter, after the native client's
  surface commit. It is not copied from the client's backing file.
- `verification.json` is an independent exact comparison of all 76,800 compositor
  pixels against the source-generated opaque gradient. The verifier uses only
  Node's built-in PNG byte processing and zlib decompression.
- `provenance.txt` records the compiler source commit/diff, compiler bytes,
  source hashes, source-origin hash, built image hash and frame-storage hash.
- The authority report, launch command, compositor version and client/compositor
  logs retain the actual capability and presentation context.

The proof uses a real compositor and needs no public network. It is optional;
the portable default gate is `tools/unix-stream-test.sh`.

## Portable owned-descriptor evidence

The default gate uses the independent C peer in `tools/unix-stream-witness.c`.
It verifies raw wl_display.get_registry, wl_registry global/bind,
wl_keyboard.keymap receive with an owned File, and wl_shm.create_pool send with
an owned File. Its subsequent transport checks cover:

- Positive short sends transfer rights once; the remainder carries no rights.
  The program explicitly transfers a socket duplicate so the witness can set a
  small send buffer, making the short send independent of host defaults.
- Timeout, empty-buffer, wrong kernel kind and broken-peer failures consume
  supplied owners. TCP and Unix datagrams cannot silently discard rights.
- A limit of one rejects a peer's two descriptors even when CMSG_SPACE padding
  fits both. Larger ancillary truncation and zero limits are repeated 144 times,
  with a live descriptor census of the actual native process/interpreter broker
  under a low descriptor limit. Error buffers remain unchanged.
- Received descriptors have CLOEXEC and are promoted away from the closed
  standard-stream sentinel. Normal File read/drop and Readiness remain usable.
- A child-owned syscall tracer pauses the actual runtime immediately after
  successful O_PATH resolution of a Directory's socket. The witness replaces
  its pathname with a symlink to an outside listener. The original pinned
  listener receives the bytes; the outside listener receives no connection.
- Literal/environment entry grants, missing/malformed authority, source move
  refusals, direct-IR signature mutations and scalar descriptor forgery are
  checked. A separately admitted unit image exercises runtime feature bit 18.

Both native and independent interpreter routes execute the same protocol
obligations. The external peer observes actual bytes, descriptor identities,
flags and lifetime; it does not use the compiler's emitter or interpreter.
