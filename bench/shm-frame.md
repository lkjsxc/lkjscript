# Shared-frame backing without writable mappings

The existing owned-file interface can prepare a 1920 × 1080 XRGB8888 frame without a writable mapping. This measurement covers the backing-file copy only. It does not measure rasterization, Wayland dispatch, compositor work, scanout or frame latency.

## Reproduce

```sh
target/lkjscript build tools/shm-frame-bench.lkj -o target/shm-frame-bench
target/shm-frame-bench
```

The specimen creates one anonymous read/write file, truncates it to 8,294,400 bytes, then repeats `seek(0, 0)` and `write_all` 32 times. Monotonic samples exclude logging and the initial pixel-vector construction. The first write includes the backing file's initial page allocation; later writes reuse it. The specimen verifies the final length and 16 bytes at the beginning, middle and end.

## Observed native result

Measured on the shared development host, 2026-11-18 UTC, using the published seed from `6d6f0feee2e3b03449fb7bbb524c900459c109a4`.

| Item | Observation |
| --- | ---: |
| Frame bytes | 8,294,400 |
| Samples | 32 |
| File creation and truncate | 0.012374 ms |
| First write / maximum | 1.633603 ms |
| Minimum | 0.254138 ms |
| Median | 0.274581 ms |
| Mean | 0.351611 ms |
| Size and three byte samples | Passed |

Producer SHA-256: `59c7df4cdd4b1355cd8ef2b85d71044733298145f1879010d82b9de749a0836a`.
Specimen SHA-256: `1b3b0eb461939acd3c1ca52596747f70f0b47ce32f1e191e69ea0c772222c682`.
The [raw 32 samples](connect-deadline-data/shm-frame-native.txt) retain the native observations.

These observations do not establish a need for writable shared mappings. The browser must still wait for compositor release before rewriting a backing buffer; ownership and release tracking remain protocol responsibilities. The capability work therefore keeps the existing seek/write route.
