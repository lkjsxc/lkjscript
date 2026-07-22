# Brainfuck Mandelbrot Benchmark

## Purpose

Reproduce correctness and end-to-end release-mode timing for the Brainfuck
interpreter implemented in
[`src/examples/brainfuck/`](../../../src/examples/brainfuck/).

## Status

**Current** harness and workload. Retained measurements and failed attempts are
recorded in the [experiment registry](../../../docs/vision/experiments.md).
This is **Brainfuck Mandelbrot interpreted by lkjscript**, not the native
lkjscript Mandelbrot workload under `src/examples/mandel/`.

## Upstream Input And Oracle

The harness downloads, but does not vendor, these files from
`pablojorge/brainfuck` commit
`153924714ae5e569ec39dcf0c0a5b5ae33600cc6`:

- `programs/mandelbrot.bf`: SHA-256
  `f0f048e90855450fb06f2bea21f914f0d24e6b6c15fd050c68176ff794c6229e`;
- `LICENSE.md`: SHA-256
  `68ffa8b51537b1fc1ca38b4ad6bb0c2c7230262d3309d1ef55a3f25de9360d2d`.

The source header attributes the Mandelbrot program to Erik Bosman. The
upstream repository uses the MIT License. The repository-authored
[`reference.c`](reference.c), SHA-256
`af6250f93ef18b35e35788958e6c1feed1a20155011e7208546940661dbedf1d`, is
compiled under `target/` and serves only as an independent byte-output oracle.
The harness requires its output to be exactly 6,240 bytes with SHA-256
`83a0aac65090b3b5e85c22337afac39d8ac17bfd88675f044b33bd55ca0c351b`;
it does not accept a changing local oracle result. Reference runtime is not
reported as a language-performance comparison.

## Commands

Run the direct and run-folded smoke suite:

```sh
python3 meta/benchmarks/brainfuck/benchmark.py --mode smoke
```

Run a full correctness check with the direct interpreter:

```sh
python3 meta/benchmarks/brainfuck/benchmark.py \
  --mode correctness --diagnostic-timeout 10 --timeout 1800
```

Run the repeated benchmark with the optional identical-run folding enabled:

```sh
python3 meta/benchmarks/brainfuck/benchmark.py \
  --fold-runs --warmups 1 --runs 3 \
  --diagnostic-timeout 10 --timeout 1800
```

The primary metric is **end-to-end process wall time (compile + initialize +
interpret + output)**. It includes lkjscript process startup, compilation of the
lkjscript interpreter, VM initialization, Brainfuck loading/preparation and
execution, output writes, and process shutdown. It is not pure interpreter-loop
time.

The default benchmark uses two warmups and seven measured runs. The explicit
one-warmup/three-run command above is for expensive runs. Every measured output
is checked for byte length and SHA-256. Compact JSON results are written below
`target/brainfuck-bench/results/`; downloaded input and the compiled oracle
remain untracked under `target/`. Full output artifacts are deleted after their
lengths and hashes are checked.

## Interpreter Modes

With only a program path, the example executes the direct filtered instruction
stream:

```sh
./target/release/lkjscript run \
  src/examples/brainfuck/main.lkjscript -- path/to/program.bf
```

The optional second argument `--fold-runs` folds only consecutive identical
`+`, `-`, `>`, and `<` commands into one internal instruction with a count.
Bracket destinations are rebuilt over that stream. No clear-loop,
multiplication-loop, source-translation, JIT, or VM-specific optimization is
used.
