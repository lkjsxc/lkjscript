#!/usr/bin/env bash
# Focused accounting controls. All named files belong to one fresh test tree.
set -euo pipefail
cd "$(dirname "$0")/.."
(($# == 1)) || { echo 'usage: budget-test.sh COMPILER' >&2; exit 2; }
compiler=$(realpath "$1")
fixture=$(mktemp -d target/compiler-budget-test.XXXXXX)
fixture=$(realpath "$fixture")
finish() {
    local status=$?
    trap - EXIT HUP INT TERM
    if [[ -n ${LKJSCRIPT_BUDGET_EVIDENCE:-} ]]; then
        mkdir -p "$LKJSCRIPT_BUDGET_EVIDENCE"
        for record in "$fixture"/*.stdout "$fixture"/*.stderr "$fixture"/*.time "$fixture"/*.sha256; do
            [[ -f $record ]] && cp "$record" "$LKJSCRIPT_BUDGET_EVIDENCE/"
        done
    fi
    if ((status != 0)); then
        for record in "$fixture"/*.stderr; do
            [[ -f $record ]] && { printf '%s\n' "$record" >&2; tail -12 "$record" >&2; }
        done
    fi
    rm -rf -- "$fixture"
    exit "$status"
}
trap finish EXIT
trap 'exit 130' HUP INT TERM
umask 022
mkdir "$fixture/source" "$fixture/owned"
bash tools/store-fixture-source.sh "$fixture/source"
cp src/ir_text.lkj "$fixture/source/"
cat > "$fixture/source/unit_budget_main.lkj" <<'ANONYMOUS_SOURCE'
//! Anonymous-file controls for the authenticated Budget implementation.
mod diagnostic;
mod ir;
mod ir_text;
mod ir_binary;
mod reuse;
pub use std::sha256;
mod native { pub mod records; }
mod store;
fn main(mut world: World) {
    let count = store::budget_contracts(&mut world);
    world.stdout().write(format!("ok budget records {count} controls\n").as_bytes());
}
ANONYMOUS_SOURCE
cat > "$fixture/source/unit_budget_host_main.lkj" <<'HOST_SOURCE'
//! Named host controls are confined to one freshly created child of PARENT.
mod diagnostic;
mod ir;
mod ir_text;
mod ir_binary;
mod reuse;
pub use std::sha256;
mod native { pub mod records; }
mod store;
fn main(mut world: World) {
    let args = world.args();
    if args.len() != 1usize {
        world.stderr().write(b"usage: unit-budget-host-contracts PARENT\n");
        world.set_exit_status(2u8); return;
    }
    let parent = String::from_utf8(args[0usize].clone()).unwrap();
    match store::Store::budget_host_contracts(&mut world, &parent) {
        Ok(message) => world.stdout().write(message.as_bytes()),
        Err(error) => {
            world.stderr().write(format!("{error}\n").as_bytes());
            world.set_exit_status(1u8);
        }
    }
}
HOST_SOURCE
sha256sum "$compiler" > "$fixture/compiler.sha256"
for entry in unit_budget_main unit_budget_host_main; do
    /usr/bin/time -f 'elapsed_seconds=%e\nmaximum_resident_kib=%M\nexit_status=%x' \
        -o "$fixture/$entry.build.time" "$compiler" build "$entry.lkj" \
        --source-dir "$fixture/source" --library-dir "$PWD/library" --no-store \
        -o "$fixture/$entry" > "$fixture/$entry.build.stdout" 2> "$fixture/$entry.build.stderr"
    sha256sum "$fixture/$entry" > "$fixture/$entry.sha256"
done
/usr/bin/time -f 'elapsed_seconds=%e\nmaximum_resident_kib=%M\nexit_status=%x' \
    -o "$fixture/anonymous.time" "$fixture/unit_budget_main" \
    > "$fixture/anonymous.stdout" 2> "$fixture/anonymous.stderr"
/usr/bin/time -f 'elapsed_seconds=%e\nmaximum_resident_kib=%M\nexit_status=%x' \
    -o "$fixture/host.time" "$fixture/unit_budget_host_main" "$fixture/owned" \
    > "$fixture/host.stdout" 2> "$fixture/host.stderr"
# Index replacement preserves the ordinary publisher's mode under this umask.
owned=$(sed -n 's/^ok budget host 11 controls; owned fixture //p' "$fixture/host.stdout")
case "$owned" in "$fixture"/owned/.unit-budget-controls-*) ;; *) echo 'unexpected fixture owner' >&2; exit 1 ;; esac
ordinary=$(find "$owned/profiling" -maxdepth 1 -type f -name 'capacity-*' -print -quit)
[[ -n $ordinary ]]
[[ $(stat -c %a "$owned/profiling/.budget-index") == $(stat -c %a "$ordinary") ]]
cat "$fixture/anonymous.stdout" "$fixture/host.stdout"
