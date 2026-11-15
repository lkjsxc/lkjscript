#!/usr/bin/env bash
# One native million-node specimen and small independent-interpreter cases.
# Optional --witness compiles exactly the same arena/graph source with Rust.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=target/lkjscript
nodes=1000000
mutations=2000
seed=6840227782638526189
witness=no
while (($#)); do
    case "$1" in
        --compiler) compiler=$2; shift 2 ;;
        --nodes) nodes=$2; shift 2 ;;
        --mutations) mutations=$2; shift 2 ;;
        --seed) seed=$2; shift 2 ;;
        --witness) witness=yes; shift ;;
        *) printf 'usage: %s [--compiler PATH] [--nodes N] [--mutations N] [--seed N] [--witness]\n' "$0" >&2; exit 2 ;;
    esac
done
for value in "$nodes" "$mutations" "$seed"; do
    [[ "$value" =~ ^[0-9]+$ ]] || { printf 'nonnegative integer required\n' >&2; exit 2; }
done
if [[ "$nodes" =~ ^0*[01]$ || "$seed" =~ ^0+$ ]]; then
    printf 'nodes > 1 and seed > 0 required\n' >&2; exit 2
fi
work=$(mktemp -d "$PWD/target/programs-graph.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT
    if ((status)); then
        for log in "$work/build.err" "$work/native.err" "$work/interpreter.err"; do
            [[ ! -f "$log" ]] || cat "$log" >&2
        done
    fi
    rm -rf -- "$work"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
before=$(cat /proc/loadavg)
started=$(date +%s%N)
"$compiler" build examples/object_graph --library-dir library --no-store -o "$work/graph" 2>"$work/build.err"
"$compiler" test examples/object_graph/main.lkj --library-dir library --no-store --jobs 1
# Module-private exhaustion/affine cases use the exact shipped library bytes.
cp library/arena.lkj "$work/arena.lkj"
cat >"$work/arena_main.lkj" <<'SOURCE'
pub use std::Option;
mod arena;
SOURCE
"$compiler" test "$work/arena_main.lkj" --library-dir library --no-store --jobs 1
"$compiler" test tests/diagnostics/graph_receiver_argument.lkj --library-dir library --no-store --jobs 1
"$compiler" test tests/diagnostics/graph_handle_kind.lkj --library-dir library --no-store --jobs 1
"$compiler" test tests/diagnostics/graph_immutable_arguments.lkj --library-dir library --no-store --jobs 1
"$compiler" test tests/diagnostics/graph_uninitialized_inference.lkj --library-dir library --no-store --jobs 1
"$compiler" test tests/diagnostics/graph_output_result.lkj --library-dir library --no-store --jobs 1
"$compiler" authority examples/object_graph --library-dir library >"$work/authority"
for kind in Arguments Stdout; do grep -Fq "$kind" "$work/authority"; done
for small_seed in 1 17 6840227782638526189; do
    "$work/graph" 127 63 "$small_seed" >"$work/native" 2>"$work/native.err"
    "$compiler" run examples/object_graph --library-dir library --no-store --engine interpreter -- 127 63 "$small_seed" >"$work/interpreter" 2>"$work/interpreter.err"
    cmp "$work/native" "$work/interpreter"
    test ! -s "$work/native.err"
    test ! -s "$work/interpreter.err"
done
printf 'graph focused: %s ms, load %s -> %s\n' "$((($(date +%s%N)-started)/1000000))" "$before" "$(cat /proc/loadavg)"
measure() {
    local label=$1 executable=$2
    local load_before started_ns
    load_before=$(cat /proc/loadavg)
    started_ns=$(date +%s%N)
    if [[ -x /usr/bin/time ]]; then
    /usr/bin/time -f '%e %U %S %M' -o "$work/$label.time" "$executable" "$nodes" "$mutations" "$seed" >"$work/$label.output" 2>"$work/$label.err"
    else
        "$executable" "$nodes" "$mutations" "$seed" >"$work/$label.output" 2>"$work/$label.err"
        printf 'not collected (GNU time unavailable)' >"$work/$label.time"
    fi
    test ! -s "$work/$label.err"
    cat "$work/$label.output"
    printf 'graph %s: %s ms; wall/user/system/RSS-KiB %s; load %s -> %s\n' "$label" "$((($(date +%s%N)-started_ns)/1000000))" "$(cat "$work/$label.time")" "$load_before" "$(cat /proc/loadavg)"
}
measure native "$work/graph"
if [[ "$witness" = yes ]]; then
    cp library/arena.lkj "$work/arena.rs"
    cp examples/object_graph/graph.lkj "$work/graph.rs"
    cat >"$work/witness.rs" <<'RUST'
#![allow(dead_code)]
pub use std::option::Option;
mod arena;
mod graph;
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let count = args[0].parse::<usize>().unwrap();
    let mutations = args[1].parse::<usize>().unwrap();
    let seed = args[2].parse::<u64>().unwrap();
    let report = graph::exercise(count, mutations, seed);
    println!("graph initial={} removed={} added={} live={} stale={} slots={} checksum={}",
        report.initial, report.removed, report.added, report.live,
        report.stale, report.slots, report.checksum);
}
RUST
    rustc --edition=2024 --crate-name graph_witness -C opt-level=0 -C overflow-checks=yes "$work/witness.rs" -o "$work/witness-opt0" 2>"$work/build.err"
    rustc --edition=2024 --crate-name graph_witness -C opt-level=3 -C overflow-checks=yes "$work/witness.rs" -o "$work/witness-opt3" 2>"$work/build.err"
    measure rust-opt0 "$work/witness-opt0"
    measure rust-opt3 "$work/witness-opt3"
    cmp "$work/native.output" "$work/rust-opt0.output"
    cmp "$work/native.output" "$work/rust-opt3.output"
fi
printf 'ok graph owner handles, generation retirement, affine removal, seeded mutation, observers and native/interpreter agreement\n'
