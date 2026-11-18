# Anonymous frame and run-time TCP destination

One browser-shaped entry uses exactly six grants: `File` for its direct Unix
compositor connection, `TemporaryFiles` for anonymous read/write backing,
`TcpPort` for one outbound port, `Readiness`, `Clock`, and `Stdout`. There is no
`World`, `Directory`, `ReadDirectory`, `Arguments`, or `Environment` parameter.
The program creates its own anonymous File, drops the factory grant, fills the
File, and transfers it through `send_message` in `wl_shm.create_pool`.

The same execution reads a numeric IPv4 address from a real compositor event,
uses `tcp.connector(address).connect_deadline(deadline)`, and exchanges `R`/`Q`
with the unchanged independent C peer from `tools/connect-deadline-peer.c`.
It then commits one 320 by 240 frame. The harness compares all 76,800 pixels in
Weston's own screenshot with the source-generated gradient.

## Exact entry authority

| Parameter | Grant | Purpose |
| --- | --- | --- |
| `socket: File` | `socket=unix:env:LKJSCRIPT_UNIX_FRAME_SOCKET` | Connect directly to the harness's private Weston socket. |
| `storage: TemporaryFiles` | `storage` | Create the anonymous writable frame backing. |
| `tcp: TcpPort` | `tcp=PORT` | Connect to the peer's nonzero port at the address received at run time. |
| `ready: Readiness` | `ready` | Wait for Wayland events. |
| `clock: Clock` | `clock` | Construct absolute monotonic deadlines. |
| `output: Stdout` | `output` | Signal the completed frame and TCP exchange. |

The authority assertion requires all six parameters and the existing `world_file`,
`file_send_message`, `tcp_port_connector`, and `connector_connect_deadline`
operations. It rejects World or directory grants and reachable named-file
creation, writing, renaming, removal, executable acquisition, or image spawning.
`world_file` is the retained IR spelling for anonymous-file creation; its name
does not mean this entry receives World.

## Run-time address input and synchronization

The restricted entry has no argument or environment-reading grant. A tiny
**external test harness module** sets the private compositor's head model to the
numeric address chosen by the harness. The source client binds standard
`wl_output` and reads that model from `wl_output.geometry`. This metadata is only
a transport for test input. The client connects directly to Weston: no Unix
proxy, custom wire prefix, byte bridge, or descriptor bridge is involved.
The independent peer still binds its ordinary loopback address and selects its
port at run time. Neither its port nor its numeric address is compiled into the
lkjscript entry source.

`weston-input.c` targets the already available Weston 13 ABI. Its opaque
iteration declaration was checked against Ubuntu's matching
`libweston-13-dev_13.0.0-4build3_amd64.deb` (SHA-256
`ded615d95e57458d48c65d85c4baf92b0c76f4d7859d47101b1ed46948da7d3f`).
The monitor-string signature was checked against the official
[Weston 13.0.0 backend header](https://gitlab.freedesktop.org/wayland/weston/-/blob/13.0.0/libweston/backend.h)
(SHA-256 `16c32bbf137c7cb0912afd6241141c1d9983784a8ea993a7f103763dbadf023f`).
The harness compiles this small module into its private temporary directory and
loads it only into its own compositor. It installs nothing.

The entry waits for `wl_surface.frame` completion before writing its ready line
to stdout. The harness captures and verifies the frame, then closes its own
compositor. Unix socket EOF is the capture-complete signal to the entry. There
are no ready or capture-complete files. Screenshot and log files belong only to
the external observer; the lkjscript program cannot name them.

## Reproduce

Run from the compiler checkout, using existing dependencies:

```sh
bash docs/proofs/anonymous-wayland-20261118/run.sh \
  --compiler target/lkjscript \
  --deps /home/coder/workspace/lkjbrowser/.deps/root \
  --output bench/anonymous-files-data/weston-native

bash docs/proofs/anonymous-wayland-20261118/run.sh \
  --compiler target/lkjscript \
  --deps /home/coder/workspace/lkjbrowser/.deps/root \
  --engine interpreter \
  --output bench/anonymous-files-data/weston-interpreter
```

The script requires C, Node, and existing Weston 13, its headless backend, kiosk
shell, and screenshooter. The PNG verifier is the independent built-in Node
zlib implementation from the previous Unix proof. The script owns and reaps all
client, peer, and compositor processes. This optional proof is separate from the
portable default gate.

Each output directory retains the entry authority, compiler/source/module/peer
hashes, exact build and run commands, actual protocol log, client output,
compositor screenshot, and exact pixel-verification JSON. The protocol log
shows the runtime model, owned-File pool creation, frame callback, and surface
commit. The screenshot is captured from Weston rather than copied from backing
storage.

`wayland.lkj` retains the earlier source protocol client and changes only what
this proof needs: `wl_output` model reception, a monotonic-time accessor, frame
callback observation, and close observation. The original client ancestry and
notices remain in the [Unix proof](../unix-wayland-20261118/README.md).
The browser repository and compiler pin are not modified by this proof.

## Rejected existing route

Before adding a grant, the published `ca749bd44` producer was used to try
`Image.buffer` as backing. It creates an anonymous **sealed** File. A writable
alias was refused, and the actual Weston server rejected `wl_shm.create_pool`
with `failed mmap fd ...: Operation not permitted`. This is evidence about
that concrete route, not a claim that no anonymous File exists anywhere.

The source, published producer identity, complete raw output and reproduction
script are retained in
[`bench/anonymous-files-data/image-buffer`](../../../bench/anonymous-files-data/image-buffer).
The script regenerates its private admission key outside the source grant and
removes the key and temporary child executable when it finishes:

```sh
bash bench/anonymous-files-data/image-buffer/reproduce.sh \
  --output target/image-buffer-control
```

Without `--compiler`, the script extracts the exact published `ca749bd44` seed
and checks its executable SHA-256. It does not build against current sources.
