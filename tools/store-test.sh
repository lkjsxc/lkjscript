#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
test_directory=$(mktemp -d target/store-test.XXXXXX)
children=()
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    for child in "${children[@]}"; do kill "$child" 2>/dev/null || :; done
    for child in "${children[@]}"; do wait "$child" 2>/dev/null || :; done
    if (( status != 0 )); then
        printf 'store test failed; recent compiler diagnostics:\n' >&2
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
source_file="$source_directory/main.lkj"
mkdir "$store" "$source_directory"
head -c 32 /dev/urandom > "$key"
cp "$key" "$test_directory/key-copy"
cat > "$source_file" <<'SOURCE'
mod dependency;
fn add(value: i64) -> i64 { value + 2 }
pub fn probe() -> i64 { add(dependency::answer()) }
SOURCE
printf 'pub fn answer() -> i64 { 40 }\n' > "$source_directory/dependency.lkj"
cat > "$test_directory/invalid.lkjir" <<'IR'
lkjscript-ir 1
type "()" unit
type "i64" int 64 signed
fn "probe" 0 1
local "result" 1 let
@1 return 0
endfn
IR
fail() { printf 'store: %s\n' "$*" >&2; exit 1; }
field() {
    sed -n "s/.*\"$2\":\\([0-9][0-9]*\\).*/\\1/p" "$1" |
        awk '{ count += $1 } END { print count + 0 }'
}
cached_build() {
    local label=$1
    shift
    "$compiler" build main.lkj --source-dir "$source_directory" --store "$store" --store-key "$key" --stats \
        -o "$test_directory/result" "$@" > "$test_directory/$label.out" 2> "$test_directory/$label.log"
}
without_image() {
    if [[ -d "$store" ]]; then find "$store" -maxdepth 1 -name 'inputs-image-*' -type f -delete; fi
}
same_build() {
    local label=$1
    without_image
    cached_build "$label"
    cmp "$test_directory/expected" "$test_directory/result"
}
restore() { rm -rf "$store"; cp -a "$test_directory/snapshot" "$store"; }
rejected() {
    (( $(field "$test_directory/$1.log" store_rejected) > 0 )) ||
        fail "$1 did not reject an altered entry"
}
entries() { find "$store" -maxdepth 1 -type f ! -name '.*' -print; }

# All emitted bytes are independent of an absent, cold or warm store.
"$compiler" build main.lkj --source-dir "$source_directory" -o "$test_directory/expected"
same_build cold
(( $(field "$test_directory/cold.log" store_writes) > 0 )) || fail "cold store wrote no entries"
same_build warm
(( $(field "$test_directory/warm.log" store_hits) > 0 )) || fail "warm store reused no entries"
(( $(field "$test_directory/warm.log" checked_reused) > 0 )) || fail "warm bodies were not reused"
(( $(field "$test_directory/warm.log" emitted_reused) > 0 )) || fail "warm native functions were not reused"
cached_build image-warm
cmp "$test_directory/expected" "$test_directory/result"
(( $(field "$test_directory/image-warm.log" build_reused) > 0 )) || fail "warm complete image was not reused"
cp -a "$store" "$test_directory/snapshot"
mapfile -t original_entries < <(find "$store" -maxdepth 1 -type f \( -name 'check-*' -o -name 'native-*' \) -print)
(( ${#original_entries[@]} >= 2 )) || fail "need two independently authenticated entries"

restore
while IFS= read -r entry; do printf 'forged image\n' > "$entry"; done < <(entries)
cached_build image-forged
cmp "$test_directory/expected" "$test_directory/result"
rejected image-forged
(( $(field "$test_directory/image-forged.log" build_reused) == 0 )) || fail "forged image was reused"

for attack in truncated corrupted forged stale-root truncated-payload hostile-length; do
    restore
    while IFS= read -r entry; do
        case "$attack" in
            truncated) truncate -s 1 "$entry" ;;
            corrupted) printf '\377untrusted suffix\000' >> "$entry" ;;
            forged) printf 'checked\n42\nvalid\n' > "$entry" ;;
            # These preserve the signed root while changing the promised bytes.
            stale-root)
                attack_byte=$(od -An -tu1 -N1 -j90 "$entry")
                [[ -n "$attack_byte" ]] || fail "retained-root fixture has no payload"
                printf -v attack_escape '\\%03o' "$((attack_byte ^ 1))"
                printf '%b' "$attack_escape" | dd of="$entry" bs=1 seek=90 conv=notrunc status=none
                ;;
            truncated-payload) truncate -s 90 "$entry" ;;
            # Unsigned lengths are rejected before they can drive allocation.
            hostile-length)
                printf '\377\377\377\377\377\377\377\377' | dd of="$entry" bs=1 seek=18 conv=notrunc status=none
                ;;
        esac
    done < <(entries)
    same_build "$attack"
    rejected "$attack"
done

restore
cp "${original_entries[0]}" "$test_directory/swap"
cp "${original_entries[1]}" "${original_entries[0]}"
cp "$test_directory/swap" "${original_entries[1]}"
same_build swapped
rejected swapped

# A hostile store writer cannot overwrite a separately granted signing key.
restore
rm "${original_entries[0]}"
ln -s "$(realpath "$key")" "${original_entries[0]}"
same_build symlink
cmp "$key" "$test_directory/key-copy"

# Old results cannot survive a changed source dependency, even at the same path.
restore
printf 'pub fn answer() -> i64 { 41 }\n' > "$source_directory/dependency.lkj"
"$compiler" build main.lkj --source-dir "$source_directory" -o "$test_directory/changed-expected"
cached_build replayed
(( $(field "$test_directory/replayed.log" front_reused) > 0 )) || fail "body edit did not reuse unaffected front-end definitions"
(( $(field "$test_directory/replayed.log" front_definitions) == 1 )) || fail "body edit elaborated more than its changed definition"
cmp "$test_directory/changed-expected" "$test_directory/result"
"$compiler" run main.lkj --source-dir "$source_directory" --store "$store" --store-key "$key" > "$test_directory/changed-value"
[[ $(cat "$test_directory/changed-value") == 43 ]] || fail "changed dependency replayed old native code"
printf 'pub fn answer() -> i64 { 42 }\n' > "$source_directory/dependency.lkj"
"$compiler" build main.lkj --source-dir "$source_directory" --no-store -o "$test_directory/changed-next-expected"
cached_build replayed-next
(( $(field "$test_directory/replayed-next.log" front_reused) > 0 )) || fail "successive body edit did not reuse the advanced checkpoint"
(( $(field "$test_directory/replayed-next.log" front_definitions) == 1 )) || fail "successive body edit elaborated more than one definition"
cmp "$test_directory/changed-next-expected" "$test_directory/result"
"$compiler" run main.lkj --source-dir "$source_directory" --store "$store" --store-key "$key" > "$test_directory/changed-next-value"
[[ $(cat "$test_directory/changed-next-value") == 44 ]] || fail "successive dependency edit replayed an earlier body"
printf 'pub fn answer() -> i64 { 40 }\n' > "$source_directory/dependency.lkj"

# Signature, constant-control and source-span changes cannot replay an old front end.
cp "$source_file" "$test_directory/front-original.lkj"
for variant in generic control length; do
    cp "$test_directory/front-original.lkj" "$source_file"
    printf 'pub fn answer() -> i64 { 40 }\n' > "$source_directory/dependency.lkj"
    case "$variant" in
        generic)
            cat > "$source_file" <<'SOURCE'
mod dependency;
fn add<T>(value: T) -> T { value }
pub fn probe() -> i64 { add::<i64>(dependency::answer()) + 2 }
SOURCE
            ;;
        control)
            cat > "$source_file" <<'SOURCE'
mod dependency;
const EXTRA: i64 = 3;
fn add(value: i64) -> i64 { value + EXTRA }
pub fn probe() -> i64 { add(dependency::answer()) }
SOURCE
            ;;
        length)
            printf 'pub fn answer() -> i64 { 400 }\n' > "$source_directory/dependency.lkj"
            ;;
    esac
    "$compiler" build main.lkj --source-dir "$source_directory" --no-store -o "$test_directory/front-expected"
    cached_build "front-$variant"
    cmp "$test_directory/front-expected" "$test_directory/result"
    (( $(field "$test_directory/front-$variant.log" front_reused) == 0 )) ||
        fail "$variant context change reused an inapplicable front end"
done
cp "$test_directory/front-original.lkj" "$source_file"
printf 'pub fn answer() -> i64 { 40 }\n' > "$source_directory/dependency.lkj"

# Includes can read a file which is also parsed as a module. The included
# literal is a peer dependency, including equal-span and successive body edits.
include_source="$source_directory/parsed-includes"
mkdir "$include_source"
for kind in str bytes; do
    include_store="$test_directory/include-$kind-store"
    mkdir "$include_store"
    if [[ "$kind" == str ]]; then
        expression='include_str!("dependency.lkj").as_bytes()[26usize]'
    else
        expression='include_bytes!("dependency.lkj")[26usize]'
    fi
    cat > "$include_source/main.lkj" <<SOURCE
mod dependency;
pub fn main(mut world: World) {
    let byte = $expression;
    world.stdout().write(&[byte]);
}
SOURCE
    for value in 40 41 42; do
        printf 'pub fn answer() -> i64 { %s }\n' "$value" > "$include_source/dependency.lkj"
        "$compiler" build main.lkj --source-dir "$include_source" --no-library --no-store \
            -o "$test_directory/include-expected"
        "$compiler" build main.lkj --source-dir "$include_source" --no-library \
            --store "$include_store" --store-key "$key" --stats -o "$test_directory/include-result" \
            > "$test_directory/include-$kind-$value.out" 2> "$test_directory/include-$kind-$value.log"
        cmp "$test_directory/include-expected" "$test_directory/include-result"
        [[ $("$test_directory/include-result") == "$((value - 40))" ]] ||
            fail "$kind parsed-source include retained old bytes after body $value"
        (( $(field "$test_directory/include-$kind-$value.log" front_reused) == 0 )) ||
            fail "$kind changed include payload resumed stale peer IR"
        "$compiler" build main.lkj --source-dir "$include_source" --no-library \
            --store "$include_store" --store-key "$key" --stats -o "$test_directory/include-result" \
            > "$test_directory/include-$kind-$value-warm.out" 2> "$test_directory/include-$kind-$value-warm.log"
        cmp "$test_directory/include-expected" "$test_directory/include-result"
        (( $(field "$test_directory/include-$kind-$value-warm.log" build_reused) > 0 )) ||
            fail "$kind unchanged include image was not reused"
    done
done

# A previously absent alternate module is also an input dependency.
restore
mkdir "$source_directory/dependency"
printf 'pub fn answer() -> i64 { 9 }\n' > "$source_directory/dependency/mod.lkj"
if "$compiler" check main.lkj --source-dir "$source_directory" > "$test_directory/ambiguous.out" 2> "$test_directory/ambiguous.expected"; then
    fail "alternate module fixture was not ambiguous"
fi
if cached_build absent-alternative; then fail "cached source plan ignored a newly present module"; fi
sed '/^LKJSCRIPT_STATS /d; /^LKJSCRIPT_STORE_IO /d' "$test_directory/absent-alternative.log" > "$test_directory/ambiguous.actual"
cmp "$test_directory/ambiguous.expected" "$test_directory/ambiguous.actual"
rm "$source_directory/dependency/mod.lkj"
rmdir "$source_directory/dependency"

# A cached build never expands the source Directory grant. This sibling is
# harmless fixture data; the signing key is never included in source.
printf 'outside source grant\\n' > "$test_directory/outside"
cat > "$source_directory/escape.lkj" <<'SOURCE'
pub fn probe() -> i64 { include_bytes!("../outside")[0] as i64 }
SOURCE
if "$compiler" check escape.lkj --source-dir "$source_directory" --store "$store" --store-key "$key" \
    > "$test_directory/escape.out" 2> "$test_directory/escape.log"; then
    fail "source include escaped its Directory"
fi

# Options also identify code. Use the same output path because profile output
# embeds its sample-file prefix.
restore
"$compiler" build main.lkj --source-dir "$source_directory" --profile -o "$test_directory/profile"
cp "$test_directory/profile" "$test_directory/profile-expected"
"$compiler" build main.lkj --source-dir "$source_directory" --profile --store "$store" --store-key "$key" \
    -o "$test_directory/profile"
cmp "$test_directory/profile-expected" "$test_directory/profile"

# Invalid direct IR remains rejected with valid, corrupted and warm entries.
if "$compiler" check "$test_directory/invalid.lkjir" > "$test_directory/reject.out" 2> "$test_directory/reject.expected"; then
    fail "invalid fixture was accepted without a store"
fi
for attempt in cold warm corrupted; do
    if [[ $attempt == corrupted ]]; then
        while IFS= read -r entry; do printf 'forged verdict\n' > "$entry"; done < <(entries)
    fi
    if "$compiler" check "$test_directory/invalid.lkjir" --store "$store" --store-key "$key" \
        > "$test_directory/reject.out" 2> "$test_directory/reject.actual"; then
        fail "$attempt store admitted invalid IR"
    fi
    cmp "$test_directory/reject.expected" "$test_directory/reject.actual"
done

restore
head -c 32 /dev/urandom > "$key"
same_build wrong-key
rejected wrong-key
cp "$test_directory/key-copy" "$key"
restore
rm "$key"
same_build missing-key
(( $(field "$test_directory/missing-key.log" store_hits) == 0 )) || fail "cache used without key"
(( $(field "$test_directory/missing-key.log" store_writes) == 0 )) || fail "cache written without key"
head -c 31 "$test_directory/key-copy" > "$key"
same_build short-key
(( $(field "$test_directory/short-key.log" store_hits) == 0 )) || fail "cache used with short key"
cp "$test_directory/key-copy" "$key"

# Appending ignored ELF bytes makes a different producer, although the old
# executable's behavior is otherwise unchanged.
restore
cp "$compiler" "$test_directory/another-compiler"
printf 'different compiler identity\n' >> "$test_directory/another-compiler"
"$test_directory/another-compiler" build main.lkj --source-dir "$source_directory" --store "$store" --store-key "$key" --stats \
    -o "$test_directory/result" > "$test_directory/producer.out" 2> "$test_directory/producer.log"
cmp "$test_directory/expected" "$test_directory/result"
(( $(field "$test_directory/producer.log" store_hits) == 0 )) || fail "another producer reused entries"
cached_build producer-a-repeat
cmp "$test_directory/expected" "$test_directory/result"
(( $(field "$test_directory/producer-a-repeat.log" build_reused) > 0 )) ||
    fail "another producer displaced the original producer's image"
"$test_directory/another-compiler" build main.lkj --source-dir "$source_directory" --store "$store" --store-key "$key" --stats \
    -o "$test_directory/result" > "$test_directory/producer-b-repeat.out" 2> "$test_directory/producer-b-repeat.log"
cmp "$test_directory/expected" "$test_directory/result"
(( $(field "$test_directory/producer-b-repeat.log" build_reused) > 0 )) ||
    fail "producer entries do not coexist"


# Refused writes are merely misses. Ignore SIGXFSZ so the kernel returns EFBIG
# instead of killing the process; this command publishes no ordinary output.
rm -rf "$store"
mkdir "$store"
( trap '' XFSZ; ulimit -f 0; "$compiler" check main.lkj --source-dir "$source_directory" --store "$store" --store-key "$key" )
same_build after-refusal

rm -rf "$store"
printf 'not a directory\n' > "$store"
same_build unavailable
rm "$store"
mkdir "$store"

# Both writers publish exactly the same independently checked bytes.
"$compiler" build main.lkj --source-dir "$source_directory" --store "$store" --store-key "$key" -o "$test_directory/race-a" \
    > "$test_directory/race-a.out" 2> "$test_directory/race-a.log" &
children+=("$!")
"$compiler" build main.lkj --source-dir "$source_directory" --store "$store" --store-key "$key" -o "$test_directory/race-b" \
    > "$test_directory/race-b.out" 2> "$test_directory/race-b.log" &
children+=("$!")
for child in "${children[@]}"; do wait "$child"; done
children=()
cmp "$test_directory/expected" "$test_directory/race-a"
cmp "$test_directory/expected" "$test_directory/race-b"
same_build after-race
(( $(field "$test_directory/after-race.log" store_hits) > 0 )) || fail "concurrent publication left no usable entries"

# Test observations are reusable only with their complete pure input closure.
mkdir "$source_directory/cases"
cat > "$source_directory/cases/pure.lkj" <<'SOURCE'
// expect: value 42
pub fn probe() -> i64 { 42 }
SOURCE
for pass in cold warm; do
    "$compiler" test "$source_directory/cases" --jobs 2 --store "$store" --store-key "$key" --stats \
        > "$test_directory/test-$pass.out" 2> "$test_directory/test-$pass.log"
    sed 's/ ([0-9][0-9]*[.][0-9][0-9]* s)$//' "$test_directory/test-$pass.out" > "$test_directory/test-$pass.summary"
done
cmp "$test_directory/test-cold.summary" "$test_directory/test-warm.summary"
(( $(field "$test_directory/test-warm.log" test_reused) > 0 )) || fail "warm pure test was not reused"
sed -i 's/42/43/g' "$source_directory/cases/pure.lkj"
"$compiler" test "$source_directory/cases" --jobs 2 --store "$store" --store-key "$key" \
    > "$test_directory/test-changed.out" 2> "$test_directory/test-changed.log"
sed -i 's/expect: value 43/expect: value 44/' "$source_directory/cases/pure.lkj"
if "$compiler" test "$source_directory/cases" --jobs 2 --store "$store" --store-key "$key" \
    > "$test_directory/test-expectation.out" 2> "$test_directory/test-expectation.log"; then
    fail "changed test expectation replayed an old pass"
fi

# Fixed private stdout/stderr captures are closed observations, including writes
# reached through calls and custom cleanup. Both checked graphs must prove this.
mkdir "$source_directory/captured"
cat > "$source_directory/captured/output.lkj" <<'SOURCE'
// expect: output 6162
// stderr: 6364
struct Tail { errors: Stderr }
impl Drop for Tail { fn drop(&mut self) { self.errors.write(b"d"); } }
fn speak(world: &mut World) {
    world.stdout().write(b"a");
    world.stderr().write(b"c");
}
pub fn main(mut world: World) {
    let tail = Tail { errors: world.stderr() };
    speak(&mut world);
    world.stdout().write(b"b");
}
SOURCE
cat > "$source_directory/captured/trap.lkj" <<'SOURCE'
// expect: trap R0005
// stdout: 78
// stderr: 79
pub fn main(mut world: World) {
    world.stdout().write(b"x");
    world.stderr().write(b"y");
    assert!(false);
}
SOURCE
for limit in 0 64; do
    for pass in cold warm; do
        "$compiler" test "$source_directory/captured" --jobs 2 --inline-limit "$limit" \
            --store "$store" --store-key "$key" --stats \
            > "$test_directory/captured-$limit-$pass.out" 2> "$test_directory/captured-$limit-$pass.log"
        sed 's/ ([0-9][0-9]*[.][0-9][0-9]* s)$//' "$test_directory/captured-$limit-$pass.out" \
            > "$test_directory/captured-$limit-$pass.summary"
    done
    cmp "$test_directory/captured-$limit-cold.summary" "$test_directory/captured-$limit-warm.summary"
    (( $(field "$test_directory/captured-$limit-warm.log" test_reused) == 2 )) ||
        fail "fixed stdout/stderr success and trap were not reused at inline $limit"
done
sed -i 's/write(b"b")/write(b"e")/' "$source_directory/captured/output.lkj"
if "$compiler" test "$source_directory/captured/output.lkj" --jobs 1 --inline-limit 0 \
    --store "$store" --store-key "$key" --stats \
    > "$test_directory/captured-changed.out" 2> "$test_directory/captured-changed.log"; then
    fail "changed output body replayed an old passing capture"
fi
(( $(field "$test_directory/captured-changed.log" test_reused) == 0 )) ||
    fail "changed output body was reused"
sed -i 's/expect: output 6162/expect: output 6165/' "$source_directory/captured/output.lkj"
"$compiler" test "$source_directory/captured/output.lkj" --jobs 1 --inline-limit 0 \
    --store "$store" --store-key "$key" --stats \
    > "$test_directory/captured-corrected.out" 2> "$test_directory/captured-corrected.log"
sed -i 's/expect: output 6165/expect: output 6166/' "$source_directory/captured/output.lkj"
if "$compiler" test "$source_directory/captured/output.lkj" --jobs 1 --inline-limit 0 \
    --store "$store" --store-key "$key" --stats \
    > "$test_directory/captured-expectation.out" 2> "$test_directory/captured-expectation.log"; then
    fail "changed output expectation replayed an old pass"
fi
(( $(field "$test_directory/captured-expectation.log" test_reused) == 0 )) ||
    fail "changed output expectation was reused"

# A fixed observed value cannot make external authority deterministic. Effects
# in a callee, an owned destructor, and either checked branch all prevent replay.
mkdir "$source_directory/external"
cat > "$source_directory/external/clock.lkj" <<'SOURCE'
// expect: output
pub fn main(mut world: World) { let _ = world.clock().nanoseconds().unwrap(); }
SOURCE
cat > "$source_directory/external/args.lkj" <<'SOURCE'
// expect: output
pub fn main(mut world: World) { let _ = world.args(); }
SOURCE
cat > "$source_directory/external/entropy.lkj" <<'SOURCE'
// expect: output
pub fn main(mut world: World) {
    let mut bytes = [0u8; 1];
    world.entropy().fill(&mut bytes).unwrap();
}
SOURCE
cat > "$source_directory/external/callee.lkj" <<'SOURCE'
// expect: output
fn hidden(world: &mut World) { let _ = world.args(); }
pub fn main(mut world: World) { hidden(&mut world); }
SOURCE
cat > "$source_directory/external/destructor.lkj" <<'SOURCE'
// expect: output
struct Hidden { world: World }
impl Drop for Hidden { fn drop(&mut self) { let _ = self.world.args(); } }
pub fn main(world: World) { let hidden = Hidden { world }; }
SOURCE
cat > "$source_directory/external/branch.lkj" <<'SOURCE'
// expect: output 42
pub fn main(mut world: World) {
    if false { let _ = world.args(); }
    world.stdout().write(b"B");
}
SOURCE
for limit in 0 64; do
    for pass in cold warm; do
        "$compiler" test "$source_directory/external" --jobs 2 --inline-limit "$limit" \
            --store "$store" --store-key "$key" --stats \
            > "$test_directory/external-$limit-$pass.out" 2> "$test_directory/external-$limit-$pass.log"
        (( $(field "$test_directory/external-$limit-$pass.log" test_reused) == 0 )) ||
            fail "external effect was reused at inline $limit ($pass)"
        (( $(field "$test_directory/external-$limit-$pass.log" test_rerun) == 6 )) ||
            fail "external effect case did not execute at inline $limit ($pass)"
    done
done

# Closed language traps are deterministic observations on both execution routes.
# Resource/host failures remain observations to rerun, even with trap-looking paths.
mkdir "$source_directory/traps"
cp tests/conformance/trap/integer_shift_width.lkj "$source_directory/traps/overflow.lkj"
cat > "$source_directory/traps/division.lkj" <<'SOURCE'
// expect: trap R0002
pub fn probe() -> i64 {
    let divisor = 0i64;
    7i64 / divisor
}
SOURCE
cat > "$source_directory/traps/bounds.lkj" <<'SOURCE'
// expect: trap R0003
pub fn probe() -> i64 {
    let values = vec![1i64];
    values[1]
}
SOURCE
cat > "$source_directory/traps/assertion.lkj" <<'SOURCE'
// expect: trap R0005
pub fn probe() { assert!(false); }
SOURCE
cat > "$source_directory/traps/explicit.lkj" <<'SOURCE'
// expect: trap R0004
pub fn probe() { panic!("a closed semantic trap\nR0006 is message text"); }
SOURCE
for pass in cold warm; do
    "$compiler" test "$source_directory/traps" --jobs 2 --store "$store" --store-key "$key" --stats \
        > "$test_directory/traps-$pass.out" 2> "$test_directory/traps-$pass.log"
    sed 's/ ([0-9][0-9]*[.][0-9][0-9]* s)$//' "$test_directory/traps-$pass.out" > "$test_directory/traps-$pass.summary"
done
cmp "$test_directory/traps-cold.summary" "$test_directory/traps-warm.summary"
(( $(field "$test_directory/traps-warm.log" test_reused) == 5 )) || fail "closed semantic traps were not reused"

cat > "$source_directory/R0003-native-resource.lkj" <<'SOURCE'
pub fn probe() -> i64 { 42 }
SOURCE
printf 'run\tprobe\t%s/R0003-native-resource.lkj\n' "$source_directory" > "$test_directory/resource-batch"
for pass in first repeated; do
    (ulimit -n 10; exec "$compiler" batch "$test_directory/resource-batch" --engines native \
        --store "$store" --store-key "$key" --stats) \
        > "$test_directory/resource-$pass.out" 2> "$test_directory/resource-$pass.log"
    grep -Eq '"checkStatus":0,"status":[1-9][0-9]*.*E_(IO|HOST)' "$test_directory/resource-$pass.out" ||
        fail "descriptor refusal did not reach native execution setup"
    (( $(field "$test_directory/resource-$pass.log" test_reused) == 0 )) ||
        fail "transient resource failure was reused"
    if [[ "$pass" == repeated ]]; then
        (( $(field "$test_directory/resource-$pass.log" checked_reused) > 0 )) ||
            fail "descriptor refusal disabled the store"
    fi
done
"$compiler" batch "$test_directory/resource-batch" --engines native \
    --store "$store" --store-key "$key" --stats \
    > "$test_directory/resource-recovered.out" 2> "$test_directory/resource-recovered.log"
grep -q '"checkStatus":0,"status":0,"stdoutHex":"34320a"' "$test_directory/resource-recovered.out" ||
    fail "restored descriptor limit replayed a resource failure"
(( $(field "$test_directory/resource-recovered.log" test_reused) == 0 )) ||
    fail "resource failure became a reusable observation"

# An explicit raw runtime can inspect kernel allocation outcomes. Its code and
# checked bodies remain reusable, but its executions are not closed observations.
for pass in cold warm; do
    "$compiler" batch "$test_directory/resource-batch" --engines native,interpreter --roundtrip \
        --runtime-source library/runtime.lkj --store "$store" --store-key "$key" --stats \
        > "$test_directory/runtime-$pass.out" 2> "$test_directory/runtime-$pass.log"
    grep -q '"checkStatus":0,"status":0,"stdoutHex":"34320a"' "$test_directory/runtime-$pass.out" ||
        fail "explicit runtime probe did not complete"
    (( $(field "$test_directory/runtime-$pass.log" test_reused) == 0 )) ||
        fail "explicit runtime execution was reused"
done
(( $(field "$test_directory/runtime-warm.log" checked_reused) > 0 )) ||
    fail "explicit runtime disabled independent body reuse"

# Whole-image identity includes exact runtime-source bytes, even a harmless edit.
cp library/runtime.lkj "$test_directory/runtime.lkj"
cached_build runtime-image-cold --runtime-source "$test_directory/runtime.lkj"
cp "$test_directory/result" "$test_directory/runtime-image-expected"
cached_build runtime-image-warm --runtime-source "$test_directory/runtime.lkj"
(( $(field "$test_directory/runtime-image-warm.log" build_reused) > 0 )) ||
    fail "explicit runtime image was not reused"
printf '\n// changed runtime input closure\n' >> "$test_directory/runtime.lkj"
cached_build runtime-image-changed --runtime-source "$test_directory/runtime.lkj"
cmp "$test_directory/runtime-image-expected" "$test_directory/result"
(( $(field "$test_directory/runtime-image-changed.log" build_reused) == 0 )) ||
    fail "runtime-source edit replayed an old image"

# Directory effects must run even if source and expected output are unchanged.
cat > "$source_directory/effect.lkj" <<SOURCE
// expect: output
pub fn main(mut world: World) {
    let mut directory = world.directory("$test_directory").unwrap();
    let mut file = directory.create("effect").unwrap();
    file.write_all(b"executed").unwrap();
}
SOURCE
for pass in cold warm; do
    rm -f "$test_directory/effect"
    "$compiler" test "$source_directory/effect.lkj" --jobs 1 --store "$store" --store-key "$key" --stats \
        > "$test_directory/effect-$pass.out" 2> "$test_directory/effect-$pass.log"
    [[ $(cat "$test_directory/effect") == executed ]] || fail "$pass effectful test was skipped"
    (( $(field "$test_directory/effect-$pass.log" test_reused) == 0 )) ||
        fail "$pass Directory effect was reused"
done

# Function fragments remain relocatable across mixed hits, changing earlier
# control flow, literal interning and demanded owning cleanup.
relocation_store="$test_directory/relocation-store"
mkdir "$relocation_store"
cat > "$source_directory/relocation.lkj" <<'SOURCE'
struct Owner { value: i64 }
impl Drop for Owner {
    fn drop(&mut self) { assert!(self.value == 7); }
}
fn earlier(x: i64) -> i64 {
    let shared = "reused literal";
    if x > 2 { shared.len() as i64 } else { x }
}
fn later(x: i64) -> i64 {
    let shared = "reused literal";
    let fresh = "new literal";
    if shared.len() > 3 { x + fresh.len() as i64 } else { 0 }
}
fn cleanup() -> i64 {
    let mut values = Vec::new();
    values.push(Box::new(Owner { value: 7 }));
    values.len() as i64
}
pub fn probe() -> i64 { earlier(4) + later(30) + cleanup() }
SOURCE
relocation_build() {
    local label=$1
    rm -f "$relocation_store"/inputs-image-*
    "$compiler" build relocation.lkj --source-dir "$source_directory" --store "$relocation_store" --store-key "$key" --stats \
        -o "$test_directory/relocation-result" > "$test_directory/relocation-$label.out" 2> "$test_directory/relocation-$label.log"
    cmp "$test_directory/relocation-expected" "$test_directory/relocation-result"
}
"$compiler" build relocation.lkj --source-dir "$source_directory" --no-store -o "$test_directory/relocation-expected"
relocation_build cold
relocation_build warm
warm_emitted=$(field "$test_directory/relocation-warm.log" emitted_reused)
(( warm_emitted > 1 )) || fail "relocation fixture did not reuse multiple functions"
# A pack authenticates several fragments together. Changing one equal-span
# body leaves its peers reusable while requiring one new native fragment.
sed -i 's/if shared.len() > 3/if shared.len() > 4/' "$source_directory/relocation.lkj"
"$compiler" build relocation.lkj --source-dir "$source_directory" --no-store -o "$test_directory/relocation-expected"
relocation_build mixed
mixed_emitted=$(field "$test_directory/relocation-mixed.log" emitted_reused)
(( mixed_emitted > 0 && mixed_emitted < warm_emitted )) || fail "mixed emission did not combine hits and misses"
# Preserve line numbers so unchanged bodies retain their exact diagnostic input.
sed -i 's/if x > 2 { shared.len() as i64 } else { x }/if x > 2 { if x > 3 { let extra = "earlier extra literal"; shared.len() as i64 + extra.len() as i64 - extra.len() as i64 } else { x } } else { x }/' "$source_directory/relocation.lkj"
"$compiler" build relocation.lkj --source-dir "$source_directory" --no-store -o "$test_directory/relocation-expected"
relocation_build changed
changed_emitted=$(field "$test_directory/relocation-changed.log" emitted_reused)
(( changed_emitted > 0 && changed_emitted < warm_emitted )) || fail "body change did not retain unchanged function fragments"
"$compiler" run relocation.lkj --source-dir "$source_directory" --engine native --store "$relocation_store" --store-key "$key" \
    > "$test_directory/relocation-native"
"$compiler" run relocation.lkj --source-dir "$source_directory" --engine interpreter --no-store \
    > "$test_directory/relocation-interpreted"
cmp "$test_directory/relocation-native" "$test_directory/relocation-interpreted"
[[ $(cat "$test_directory/relocation-native") == 56 ]] || fail "relocated fragments changed observable behavior"

# The same store implementation uses a tiny bound here; exercise real signed
# entries and oldest-generation eviction without hashing hundreds of MiB.
bounded="$test_directory/bounded"
mkdir "$bounded"
mkdir "$test_directory/capacity-source"
cp tools/store-capacity.lkj "$test_directory/capacity-source/main.lkj"
cp src/diagnostic.lkj src/ir.lkj src/ir_text.lkj src/ir_binary.lkj src/reuse.lkj src/store.lkj library/sha256.lkj "$test_directory/capacity-source/"
"$compiler" build main.lkj --source-dir "$test_directory/capacity-source" --no-store -o "$test_directory/capacity-a"
"$test_directory/capacity-a" "$bounded" "$key" 4096 0 3 > "$test_directory/capacity-a.log"
mapfile -t generation_a < <(find "$bounded" -maxdepth 1 -type f -name 'capacity-*')
(( ${#generation_a[@]} == 3 )) || fail "capacity fixture did not retain its first generation"
cp "$test_directory/capacity-a" "$test_directory/capacity-b"
printf 'another bounded-store generation\n' >> "$test_directory/capacity-b"
"$test_directory/capacity-b" "$bounded" "$key" 4096 100 3 > "$test_directory/capacity-b.log"
for entry in "${generation_a[@]}"; do
    [[ ! -e "$entry" ]] || fail "capacity evicted the new generation before the old one"
done
"$test_directory/capacity-b" "$bounded" "$key" 4096 200 32 > "$test_directory/capacity-many.log"
grep -Eq 'evicted=[1-9][0-9]*' "$test_directory/capacity-many.log" || fail "capacity fixture did not cross its bound"
bounded_bytes=$(find "$bounded" -maxdepth 1 -type f -printf '%s\n' | awk '{ sum += $1 } END { print sum+0 }')
(( bounded_bytes <= 4096 )) || fail "authenticated bytes plus index exceed the store bound"
printf 'forged size index\n' > "$bounded/.budget-index"
"$test_directory/capacity-b" "$bounded" "$key" 4096 300 1 > "$test_directory/capacity-forged.log"
bounded_bytes=$(find "$bounded" -maxdepth 1 -type f -printf '%s\n' | awk '{ sum += $1 } END { print sum+0 }')
(( bounded_bytes <= 4096 )) || fail "forged policy metadata escaped bounded publication"

# Losing a process before its index checkpoint leaves only disposable misses.
stale_entry=$(find "$bounded" -maxdepth 1 -type f -name 'capacity-*' | head -1)
cp "$bounded/.budget-index" "$test_directory/budget-before"
find "$bounded" -maxdepth 1 -type f -name 'capacity-*' | sort > "$test_directory/budget-files-before"
if "$test_directory/capacity-b" "$bounded" "$key" 4096 400 1 interrupt > "$test_directory/capacity-interrupt.log" 2>&1; then
    fail "interrupted publisher ran its normal checkpoint"
fi
grep -q 'interrupted before store index checkpoint' "$test_directory/capacity-interrupt.log" ||
    fail "publisher failed before the intended interruption"
cmp "$bounded/.budget-index" "$test_directory/budget-before"
find "$bounded" -maxdepth 1 -type f -name 'capacity-*' | sort > "$test_directory/budget-files-after"
orphan=$(comm -13 "$test_directory/budget-files-before" "$test_directory/budget-files-after")
[[ -n "$orphan" && -f "$orphan" ]] || fail "interrupted publisher left no owned orphan"
"$test_directory/capacity-b" "$bounded" "$key" 4096 500 1 > "$test_directory/capacity-recovered.log"
[[ ! -e "$orphan" ]] || fail "writer retained an unindexed owned orphan"
cp "$bounded/.budget-index" "$test_directory/budget-before"
if "$test_directory/capacity-b" "$bounded" "$key" 4096 300 1 interrupt 1536 > "$test_directory/capacity-stale-size.log" 2>&1; then
    fail "enlarged publisher ran its normal checkpoint"
fi
grep -q 'interrupted before store index checkpoint' "$test_directory/capacity-stale-size.log" ||
    fail "enlarged publisher failed before the intended interruption"
cmp "$bounded/.budget-index" "$test_directory/budget-before"
(( $(wc -c < "$stale_entry") > 1536 )) || fail "stale-size fixture did not grow"
"$test_directory/capacity-b" "$bounded" "$key" 4096 600 1 > "$test_directory/capacity-size-recovered.log"
[[ ! -e "$stale_entry" ]] || fail "writer trusted stale byte accounting for a larger pack"
bounded_bytes=$(find "$bounded" -maxdepth 1 -type f -printf '%s\n' | awk '{ sum += $1 } END { print sum+0 }')
(( bounded_bytes <= 4096 )) || fail "checkpoint interruption escaped the publication bound"
"$compiler" build main.lkj --source-dir "$source_directory" --store "$bounded" --store-key "$key" --store-bytes 4096 -o "$test_directory/capacity-result"
cmp "$test_directory/expected" "$test_directory/capacity-result"
bounded_bytes=$(find "$bounded" -maxdepth 1 -type f -printf '%s\n' | awk '{ sum += $1 } END { print sum+0 }')
(( bounded_bytes <= 4096 )) || fail "ordinary build exceeded its explicit disposable-store bound"

# Hostile FIFOs must miss without waiting for a reader or writer, including
# old-generation metadata that this compilation would not otherwise request.
fifo_build() {
    local label=$1 executable=$2
    if ! timeout --kill-after=2s 30s "$executable" build main.lkj --source-dir "$source_directory" \
        --store "$store" --store-key "$key" -o "$test_directory/result" \
        > "$test_directory/fifo-$label.out" 2> "$test_directory/fifo-$label.log"; then
        fail "FIFO $label blocked or changed an ordinary build"
    fi
    cmp "$test_directory/expected" "$test_directory/result"
}
for attack in raw index lock old-generation; do
    restore
    without_image
    executable="$compiler"
    case "$attack" in
        index) rm -f "$store/.budget-index"; mkfifo "$store/.budget-index" ;;
        lock) rm -f "$store/.budget-lock"; mkfifo "$store/.budget-lock" ;;
        raw|old-generation)
            entry=$(find "$store" -maxdepth 1 -type f -name 'native-*' | head -1)
            [[ -n "$entry" ]] || fail "FIFO fixture found no native pack"
            rm "$entry"
            mkfifo "$entry"
            if [[ "$attack" == old-generation ]]; then
                cp "$compiler" "$test_directory/fifo-compiler"
                printf 'different FIFO-scan producer\n' >> "$test_directory/fifo-compiler"
                executable="$test_directory/fifo-compiler"
            fi
            ;;
    esac
    fifo_build "$attack" "$executable"
done

# Remove the directory after the compiler has actually opened its capability.
# Bodies are deliberately independent; the test exercises store disappearance,
# not large-program acceptance limits.
awk 'BEGIN { for (i=0;i<1000;i++) print "fn f" i "() -> i64 { " i " }"; print "pub fn probe() -> i64 { f0() + 42 }" }' > "$source_directory/many.lkj"
rm -rf "$store"
mkdir "$store"
absolute_store=$(realpath "$store")
"$compiler" check many.lkj --source-dir "$source_directory" --store "$store" --store-key "$key" \
    > "$test_directory/disappeared.out" 2> "$test_directory/disappeared.log" &
children=("$!")
observed=0
while kill -0 "${children[0]}" 2>/dev/null; do
    for descriptor in /proc/"${children[0]}"/fd/*; do
        if [[ $(readlink "$descriptor" 2>/dev/null || :) == "$absolute_store" ]]; then
            rm -rf "$store"
            observed=1
            break 2
        fi
    done
done
wait "${children[0]}"
children=()
(( observed == 1 )) || fail "disappearance test did not observe the open store"
printf 'ok authenticated store (identity, byte reproduction, rejection, hostile entries, unavailable storage, concurrent publication)\n'
