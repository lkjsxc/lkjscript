#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
test_directory=$(mktemp -d target/optimizer-test.XXXXXX)
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if (( status != 0 )); then
        printf 'optimizer test failed; recent compiler diagnostics:\n' >&2
        while IFS= read -r log; do
            printf '%s\n' "$log" >&2
            tail -8 "$log" >&2
        done < <(ls -t "$test_directory"/*.log 2>/dev/null | head -3)
    fi
    rm -rf "$test_directory"
    exit "$status"
}
trap cleanup EXIT HUP INT TERM
umask 077
store="$test_directory/store"
key="$test_directory/key"
source_directory="$test_directory/source"
mkdir "$store" "$source_directory"
head -c 32 /dev/urandom > "$key"
cat > "$source_directory/main.rs" <<'SOURCE'
mod helper;
pub fn probe() -> i64 { helper::add(40) + 1 }
SOURCE
printf 'pub fn add(value: i64) -> i64 { value + 1 }\n' > "$source_directory/helper.rs"
cat > "$test_directory/invalid.lkjir" <<'IR'
lkjscript-ir 1
type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
fn "probe" 0 3
local "result" 3 let
@1 return 0
endfn
IR
fail() { printf 'optimizer: %s\n' "$*" >&2; exit 1; }
field() {
    sed -n "s/.*\"$2\":\\([0-9][0-9]*\\).*/\\1/p" "$1" |
        awk '{ count += $1 } END { print count + 0 }'
}
build() {
    local label=$1
    shift
    "$compiler" build main.rs --source-dir "$source_directory" --no-library --entry probe \
        --stats -o "$test_directory/result" "$@" \
        > "$test_directory/$label.out" 2> "$test_directory/$label.log"
}
cached_build() {
    local label=$1
    shift
    build "$label" --store "$store" --store-key "$key" "$@"
}
without_images() { find "$store" -maxdepth 1 -name 'inputs-image-*' -type f -delete; }

# A rejected original cannot acquire authority through a valid replacement.
for pass in cold warm; do
    if "$compiler" build "$test_directory/invalid.lkjir" --entry probe --inline-limit 64 \
        --store "$store" --store-key "$key" --stats -o "$test_directory/invalid-image" \
        > "$test_directory/reject-$pass.out" 2> "$test_directory/reject-$pass.log"; then
        fail "invalid original was admitted"
    fi
    grep -q E_MOVE "$test_directory/reject-$pass.log" || fail "original rejection changed"
    (( $(field "$test_directory/reject-$pass.log" inline_attempts) == 0 )) ||
        fail "optimizer ran before original admission"
    test ! -e "$test_directory/invalid-image" || fail "rejected original produced an image"
done

# IR and authority are observations of the original. Interpretation stays its
# independent oracle even when native code is selected from a checked candidate.
for command in ir authority; do
    "$compiler" "$command" main.rs --source-dir "$source_directory" --no-library --entry probe --no-opt \
        > "$test_directory/$command-zero"
    "$compiler" "$command" main.rs --source-dir "$source_directory" --no-library --entry probe --inline-limit 64 \
        > "$test_directory/$command-inline"
    cmp "$test_directory/$command-zero" "$test_directory/$command-inline"
done
for engine in interpreter native; do
    "$compiler" run main.rs --source-dir "$source_directory" --no-library --entry probe \
        --engine "$engine" --inline-limit 64 \
        > "$test_directory/value-$engine" 2> "$test_directory/value-$engine.log"
    test "$(cat "$test_directory/value-$engine")" = 42 || fail "$engine value changed"
done
cmp "$test_directory/value-interpreter" "$test_directory/value-native"
build plain-zero --inline-limit 0
cp "$test_directory/result" "$test_directory/expected-zero"
build plain-inline --inline-limit 64
(( $(field "$test_directory/plain-inline.log" inline_accepted) > 0 )) ||
    fail "small candidate was not admitted"
(( $(field "$test_directory/plain-inline.log" inline_calls) > 0 )) ||
    fail "small candidate removed no calls"
(( $(field "$test_directory/plain-inline.log" checked_local_reused) > 0 )) ||
    fail "candidate admission reused no unchanged original proof"
cp "$test_directory/result" "$test_directory/expected-inline"
if cmp -s "$test_directory/expected-zero" "$test_directory/expected-inline"; then
    fail "admitted inlining did not change native bytes"
fi

# None, Some(0), and Some(64) have distinct complete-image and observation keys.
# --no-opt is the same exact option as --inline-limit 0.
for setting in default zero inline; do
    options=()
    case "$setting" in
        zero) options=(--inline-limit 0) ;;
        inline) options=(--inline-limit 64) ;;
    esac
    cached_build "$setting-cold" "${options[@]}"
    (( $(field "$test_directory/$setting-cold.log" build_reused) == 0 )) ||
        fail "$setting replayed a different inline setting"
    cp "$test_directory/result" "$test_directory/$setting-expected"
    cached_build "$setting-warm" "${options[@]}"
    (( $(field "$test_directory/$setting-warm.log" build_reused) > 0 )) ||
        fail "$setting complete image was not reused"
    cmp "$test_directory/$setting-expected" "$test_directory/result"
done
cmp "$test_directory/expected-zero" "$test_directory/zero-expected"
cmp "$test_directory/expected-inline" "$test_directory/inline-expected"
cached_build no-opt --no-opt
(( $(field "$test_directory/no-opt.log" build_reused) > 0 )) ||
    fail "--no-opt did not retain the explicit zero identity"
cmp "$test_directory/expected-zero" "$test_directory/result"

# Force assembly from independently cached fragments, both with and without
# sampled address metadata. The same output path binds the profile prefix.
without_images
cached_build inline-fragments --inline-limit 64
(( $(field "$test_directory/inline-fragments.log" emitted_reused) > 0 )) ||
    fail "optimized fragments were not reused"
cmp "$test_directory/expected-inline" "$test_directory/result"
build profile-reference --inline-limit 64 --profile
cp "$test_directory/result" "$test_directory/expected-profile"
cached_build profile-cold --inline-limit 64 --profile
cmp "$test_directory/expected-profile" "$test_directory/result"
without_images
cached_build profile-warm --inline-limit 64 --profile
(( $(field "$test_directory/profile-warm.log" emitted_reused) > 0 )) ||
    fail "profiled optimized fragments were not reused"
cmp "$test_directory/expected-profile" "$test_directory/result"

printf 'run\tprobe\tmain.rs\n' > "$test_directory/batch"
for setting in default zero inline; do
    options=()
    case "$setting" in
        zero) options=(--inline-limit 0) ;;
        inline) options=(--inline-limit 64) ;;
    esac
    for pass in cold warm; do
        "$compiler" batch "$test_directory/batch" --source-dir "$source_directory" --no-library \
            --engines native,interpreter --roundtrip --store "$store" --store-key "$key" --stats \
            "${options[@]}" > "$test_directory/batch-$setting-$pass.out" \
            2> "$test_directory/batch-$setting-$pass.log"
        grep -q '"checkStatus":0,"status":0,"stdoutHex":"34320a"' "$test_directory/batch-$setting-$pass.out" ||
            fail "$setting batch did not execute the original value"
    done
    (( $(field "$test_directory/batch-$setting-cold.log" test_reused) == 0 )) ||
        fail "$setting reused another setting's execution"
    (( $(field "$test_directory/batch-$setting-warm.log" test_reused) > 0 )) ||
        fail "$setting did not reuse its verified execution"
    cmp "$test_directory/batch-$setting-cold.out" "$test_directory/batch-$setting-warm.out"
done

# A copied bounds check reports the callee's source file and source line in the
# native trap record. Compare raw records, before the driver renders a path.
cat > "$source_directory/main.rs" <<'SOURCE'
mod helper;
pub fn probe() -> i64 { helper::fail(1) }
SOURCE
cat > "$source_directory/helper.rs" <<'SOURCE'
pub fn fail(index: usize) -> i64 {
    let values = [42i64];
    values[index]
}
SOURCE
for limit in 0 64; do
    build "trap-$limit" --inline-limit "$limit"
    if "$test_directory/result" > "$test_directory/trap-$limit.out" 2> "$test_directory/trap-$limit.raw"; then
        fail "bounds trap returned success"
    fi
    grep -q 'R0003 at line 3 file 1:' "$test_directory/trap-$limit.raw" ||
        fail "raw cross-file trap lost its source"
done
(( $(field "$test_directory/trap-64.log" inline_accepted) > 0 )) ||
    fail "cross-file trap candidate was not admitted"
cmp "$test_directory/trap-0.raw" "$test_directory/trap-64.raw"

# Lexical custom-message identity survives an earlier copied callee, including
# authenticated closed-test replay. Compare complete records across both policies.
cat > "$source_directory/main.rs" <<'SOURCE'
mod helper;
pub fn probe() -> i64 { helper::alpha() }
fn zeta() -> i64 { panic!("zeta"); 0 }
SOURCE
cat > "$source_directory/helper.rs" <<'SOURCE'
pub fn alpha() -> i64 {
    panic!("alpha");
    42
}
SOURCE
for limit in 0 64; do
    for pass in cold warm; do
        "$compiler" batch "$test_directory/batch" --source-dir "$source_directory" --no-library \
            --engines native,interpreter --roundtrip --inline-limit "$limit" \
            --store "$store" --store-key "$key" --stats \
            > "$test_directory/messages-$limit-$pass.out" 2> "$test_directory/messages-$limit-$pass.log"
        grep -q '"checkStatus":0,"status":1' "$test_directory/messages-$limit-$pass.out" ||
            fail "closed custom trap did not terminate as a language trap"
        grep -q 'R0016 alpha' "$test_directory/messages-$limit-$pass.out" ||
            fail "custom trap lost its exact lexical identity"
    done
    (( $(field "$test_directory/messages-$limit-warm.log" test_reused) > 0 )) ||
        fail "complete custom trap was not reused"
    cmp "$test_directory/messages-$limit-cold.out" "$test_directory/messages-$limit-warm.out"
done
cmp "$test_directory/messages-0-cold.out" "$test_directory/messages-64-cold.out"

# Interpreted World programs embed the selected, rechecked native self-image.
for engine in native interpreter; do
    "$compiler" run tests/fixtures/world/self_spawn.rs --engine "$engine" --inline-limit 64 --stats \
        > "$test_directory/self-$engine.out" 2> "$test_directory/self-$engine.log"
    test "$(cat "$test_directory/self-$engine.out")" = 42 || fail "$engine self-image changed"
    (( $(field "$test_directory/self-$engine.log" inline_accepted) > 0 )) ||
        fail "$engine self-image did not use an admitted candidate"
done
cmp "$test_directory/self-native.out" "$test_directory/self-interpreter.out"

# Seed warming never contributes to these accepted/executed rewrite counters.
"$compiler" mutate 1701 128 --jobs 1 --inline-limit 64 --store "$store" --store-key "$key" \
    -o "$test_directory/mutation" > "$test_directory/mutation.out" 2> "$test_directory/mutation.log"
grep -q '0 findings;' "$test_directory/mutation.out" || fail "optimized mutant disagreed with original model"
grep -Eq '[1-9][0-9]* accepted, [1-9][0-9]* executed' "$test_directory/mutation.out" ||
    fail "mutation smoke executed no admitted rewrites"
grep -Eq '[1-9][0-9]* local body proofs' "$test_directory/mutation.out" ||
    fail "mutation candidate admission reused no original proofs"
printf 'ok optimizer (original admission and oracle, exact options, proof reuse, native/profile identity, cross-file traps, self-image, executed mutations)\n'
