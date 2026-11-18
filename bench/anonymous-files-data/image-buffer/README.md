# Existing Image.buffer route: published-producer negative control

Published source `ca749bd44` has a World-less anonymous File route:
`Image.buffer(bytes)`. It seals the returned object before the caller receives
it. The retained source verifies that a write is refused, then transfers the
File to real Weston using the existing `send_message` and `wl_shm.create_pool`
path. Weston rejects the pool: `failed mmap fd 16: Operation not permitted`.
This does not provide anonymous **read/write** frame storage.

The executing native SHA-256 is
`3d2fc84dfc880cdd52aee6ca9a16dbef6025500e8453cd7ab75f45f336fda353`.
`observed/provenance.txt` retains the exact source commit and compressed-seed
hash. `main.lkj` and `child.lkj` are the complete source additions. The protocol
helper is the unchanged Unix proof, with a checked hash in `reproduce.sh`.

`observed/` contains the raw client, authority, and actual compositor protocol
output. No private key or executable is retained. The helper child is admitted
only to acquire an Image; it is never spawned by the frame program. The
conservative authority report consequently includes `image_buffer` and cleanup,
without reachable child spawning.

From the repository root:

```sh
bash bench/anonymous-files-data/image-buffer/reproduce.sh \
  --deps /home/coder/workspace/lkjbrowser/.deps/root \
  --output target/image-buffer-control
```

The script extracts and checks the published executable, creates an independent
private key with the existing store-key helper outside the source root, and
uses the published compiler's embedded library/runtime. It starts and reaps its
own private Weston and removes the key, executable, sockets, and source scratch.
A successful control script exits zero after observing client exit one and the
compositor's precise mmap refusal.
