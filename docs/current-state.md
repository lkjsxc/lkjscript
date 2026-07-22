# Current State

## Purpose

Separate observed behavior in this checkout from remaining work.

## Evidence Boundary

Docker verification is the acceptance path for claimed completion.
Local `quiet verify`, source-corpus validation, lkjedit/HTTP smokes, the numeric
benchmark, and
`docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify`
all passed on this checkout. The explicit `--build` prevents stale-image
success.

## Current Implementation

- Standalone repo: `https://github.com/lkjsxc/lkjscript2026`
- Language name: **lkjscript2026**; canonical LKJML sources use **`.lkjml`**
- LKJML: one column-one marker/atom per line, no indentation or attributes,
  and quote-free `str/`, `name/`, and `import/` text blocks
- Layout: `src/std` (primitives), `src/lib/lkjedit` (validation app package),
  `src/examples` (runnable demonstrations)
- Imports: `std/...`, `lib/...`, `examples/...` (mapped under `src/`; no `../`);
  installed std/lib/examples fallback through `LKJSCRIPT2026_ROOT`
- Runtime Docker image bundles the source libraries and validation examples; a
  bind-mounted external-project smoke
  importing `std/term/puts.lkjml` passed
- Scratch OS layer: `lkjscript2026-sys`; terminal + TCP + FS + time policy in `.lkjml`
- Hardcoded limit constants (no user-facing JSON limits)
- Every source is syntax-checked; all nine executable roots are typechecked and
  compiled by `check-sources`
- Bytecode VM; precise mark-sweep, tail-frame reuse, and intentional thin host
- Language special `while`; bit ops for flag poking
- `lkjedit`: terminal/filesystem validation app; planned as a future standalone
  repository, not part of the runtime product boundary
- Examples: `hello`, `mandel`, `lkjedit`, `http`, `bench`
- Honest C comparison script: `meta/scripts/bench-compare.sh`; F64 output and
  benchmark signatures are type-correct
- Laws: [decisions/scratch-host.md](decisions/scratch-host.md),
  [operations/agent-handoff.md](operations/agent-handoff.md)

## Open Work

See [vision/performance-roadmap.md](vision/performance-roadmap.md). Immediate
product gaps are native self-contained installation, a published immutable
Docker image, process-safe VM outcomes/host services, and a Linux-first,
daemon-default singleton supervisor motivated by resource efficiency. Runtime
gaps include truthful sys `Result` errors,
handle namespace separation, adaptive GC, and a real JIT execution handoff.

## Sprint Board

| Area | Status |
| --- | --- |
| `lkjedit` display / new-file validation | done |
| LKJML grammar + `.lkjml` corpus cutover | done |
| Hardcoded limits | done |
| Minimal HTTP + bench vs C | done |
| Rust-like `src/std` + `src/lib` | done |
| Standalone GitHub repo | done |
| `lkjedit` idle/cmdline/while/flush validation | done |
| Scratch host law + drop rustix | done |
| Terminal policy in `.lkjml` | done |
| TCP sockets in `.lkjml` (`src/std/net`) | done |
| Filesystem open/path-exists in `.lkjml` | done |
| Time wait/now in `.lkjml` | done |
| Keep thin `write-str` / `flush` | intentional |
| Types + LKJML + opaque sys + precise GC | landed (JIT stub remains) |
| Ban `Any` + sized numerics + `forall` polymorphism | landed (`print` is Str-only) |
| GC cliff + tail-recursive frame growth | fixed and benchmarked |
| One-runtime multi-process supervisor | not implemented |
