#!/usr/bin/env bash
# Concrete export graphs only: no package identity or future-template ceiling.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-${LKJSCRIPT_BINARY:-target/lkjscript}}
[[ $# -le 1 ]] || { printf 'usage: %s [COMPILER]\n' "$0" >&2; exit 2; }
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler" ;; esac
mkdir -p target
work=$(mktemp -d "$PWD/target/programs-unit-policy.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT
    if ((status)); then
        for log in "$work"/*.err "$work"/controls.out; do
            [[ ! -f "$log" ]] || { printf '%s\n' "$log" >&2; cat "$log" >&2; }
        done
    fi
    rm -rf -- "$work"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
cat > "$work/compute.lkj" <<'LKJ'
fn entry() -> i64 {
    let mut values = vec![1i64, 2i64, 3i64];
    values.push(4i64);
    let text = "hello".to_string();
    (values.len() + text.as_str().len()) as i64
}
LKJ
cat > "$work/callback_pure.lkj" <<'LKJ'
fn invoke<F: FnMut(i64) -> i64>(mut callback: F, value: i64) -> i64 { callback(value) }
fn entry() -> i64 { let offset = 2i64; invoke(|value: i64| -> i64 { value + offset }, 40i64) }
LKJ
cat > "$work/file.lkj" <<'LKJ'
fn entry(mut directory: ReadDirectory) { let _file = directory.open("source").unwrap(); }
LKJ
cat > "$work/socket.lkj" <<'LKJ'
fn entry(mut connector: Connector) { let _socket = connector.connect().unwrap(); }
LKJ
cat > "$work/callback.lkj" <<'LKJ'
fn invoke<F: FnMut()>(mut callback: F) { callback(); }
fn entry(mut out: Stdout) { invoke(|| { out.write(b"callback"); }); }
LKJ
cat > "$work/custom_drop.lkj" <<'LKJ'
struct Marker { out: Stdout }
impl Drop for Marker { fn drop(&mut self) { self.out.write(b"drop"); } }
fn entry(value: Marker) { }
LKJ
cat > "$work/structural_drop.lkj" <<'LKJ'
struct Holder { file: File }
fn entry(value: Holder) { }
LKJ
cat > "$work/literal.lkj" <<'LKJ'
fn entry() -> i64 { 42i64 }
LKJ
cat > "$work/literal_changed.lkj" <<'LKJ'
fn entry() -> i64 { 43i64 }
LKJ
for name in compute callback_pure file socket callback custom_drop structural_drop literal literal_changed; do
    "$compiler" ir "${work#"$PWD/"}/$name.lkj" --source-dir . --library-dir library --no-store \
        > "$work/$name.lkjir" 2> "$work/$name.err"
done
"$compiler" runtime-ir > "$work/runtime.lkjir" 2> "$work/runtime.err"
"$compiler" build src/programs_unit_policy_main.lkj --source-dir . --library-dir library \
    --runtime-source library/runtime.lkj --no-store -o "$work/policy" \
    > "$work/build.out" 2> "$work/build.err"
producer=$(sha256sum "$compiler")
producer=${producer%% *}
"$work/policy" "${work#"$PWD/"}" "$producer" > "$work/controls.out" 2> "$work/controls.err"
printf 'ok concrete computation 27 controls\n' > "$work/expected.out"
cmp "$work/expected.out" "$work/controls.out"
cat "$work/controls.out"
