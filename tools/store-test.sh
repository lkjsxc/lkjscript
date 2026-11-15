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
source_file="$source_directory/main.rs"
mkdir "$store" "$source_directory"
head -c 32 /dev/urandom > "$key"
cp "$key" "$test_directory/key-copy"
cat > "$source_file" <<'SOURCE'
mod dependency;
fn add(value: i64) -> i64 { value + 2 }
pub fn probe() -> i64 { add(dependency::answer()) }
SOURCE
printf 'pub fn answer() -> i64 { 40 }\n' > "$source_directory/dependency.rs"
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
    "$compiler" build main.rs --source-dir "$source_directory" --store "$store" --store-key "$key" --stats \
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
"$compiler" build main.rs --source-dir "$source_directory" -o "$test_directory/expected"
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

for attack in truncated corrupted forged; do
    restore
    while IFS= read -r entry; do
        case "$attack" in
            truncated) truncate -s 1 "$entry" ;;
            corrupted) printf '\377untrusted suffix\000' >> "$entry" ;;
            forged) printf 'checked\n42\nvalid\n' > "$entry" ;;
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
printf 'pub fn answer() -> i64 { 41 }\n' > "$source_directory/dependency.rs"
"$compiler" build main.rs --source-dir "$source_directory" -o "$test_directory/changed-expected"
cached_build replayed
cmp "$test_directory/changed-expected" "$test_directory/result"
"$compiler" run main.rs --source-dir "$source_directory" --store "$store" --store-key "$key" > "$test_directory/changed-value"
[[ $(cat "$test_directory/changed-value") == 43 ]] || fail "changed dependency replayed old native code"
printf 'pub fn answer() -> i64 { 40 }\n' > "$source_directory/dependency.rs"

# A previously absent alternate module is also an input dependency.
restore
mkdir "$source_directory/dependency"
printf 'pub fn answer() -> i64 { 9 }\n' > "$source_directory/dependency/mod.rs"
if "$compiler" check main.rs --source-dir "$source_directory" > "$test_directory/ambiguous.out" 2> "$test_directory/ambiguous.expected"; then
    fail "alternate module fixture was not ambiguous"
fi
if cached_build absent-alternative; then fail "cached source plan ignored a newly present module"; fi
sed '/^LKJSCRIPT_STATS /d' "$test_directory/absent-alternative.log" > "$test_directory/ambiguous.actual"
cmp "$test_directory/ambiguous.expected" "$test_directory/ambiguous.actual"
rm "$source_directory/dependency/mod.rs"
rmdir "$source_directory/dependency"

# A cached build never expands the source Directory grant. This sibling is
# harmless fixture data; the signing key is never included in source.
printf 'outside source grant\\n' > "$test_directory/outside"
cat > "$source_directory/escape.rs" <<'SOURCE'
pub fn probe() -> i64 { include_bytes!("../outside")[0] as i64 }
SOURCE
if "$compiler" check escape.rs --source-dir "$source_directory" --store "$store" --store-key "$key" \
    > "$test_directory/escape.out" 2> "$test_directory/escape.log"; then
    fail "source include escaped its Directory"
fi

# Options also identify code. Use the same output path because profile output
# embeds its sample-file prefix.
restore
"$compiler" build main.rs --source-dir "$source_directory" --profile -o "$test_directory/profile"
cp "$test_directory/profile" "$test_directory/profile-expected"
"$compiler" build main.rs --source-dir "$source_directory" --profile --store "$store" --store-key "$key" \
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
"$test_directory/another-compiler" build main.rs --source-dir "$source_directory" --store "$store" --store-key "$key" --stats \
    -o "$test_directory/result" > "$test_directory/producer.out" 2> "$test_directory/producer.log"
cmp "$test_directory/expected" "$test_directory/result"
(( $(field "$test_directory/producer.log" store_hits) == 0 )) || fail "another producer reused entries"
cached_build producer-a-repeat
cmp "$test_directory/expected" "$test_directory/result"
(( $(field "$test_directory/producer-a-repeat.log" build_reused) > 0 )) ||
    fail "another producer displaced the original producer's image"
"$test_directory/another-compiler" build main.rs --source-dir "$source_directory" --store "$store" --store-key "$key" --stats \
    -o "$test_directory/result" > "$test_directory/producer-b-repeat.out" 2> "$test_directory/producer-b-repeat.log"
cmp "$test_directory/expected" "$test_directory/result"
(( $(field "$test_directory/producer-b-repeat.log" build_reused) > 0 )) ||
    fail "producer entries do not coexist"


# Refused writes are merely misses. Ignore SIGXFSZ so the kernel returns EFBIG
# instead of killing the process; this command publishes no ordinary output.
rm -rf "$store"
mkdir "$store"
( trap '' XFSZ; ulimit -f 0; "$compiler" check main.rs --source-dir "$source_directory" --store "$store" --store-key "$key" )
same_build after-refusal

rm -rf "$store"
printf 'not a directory\n' > "$store"
same_build unavailable
rm "$store"
mkdir "$store"

# Both writers publish exactly the same independently checked bytes.
"$compiler" build main.rs --source-dir "$source_directory" --store "$store" --store-key "$key" -o "$test_directory/race-a" \
    > "$test_directory/race-a.out" 2> "$test_directory/race-a.log" &
children+=("$!")
"$compiler" build main.rs --source-dir "$source_directory" --store "$store" --store-key "$key" -o "$test_directory/race-b" \
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
cat > "$source_directory/cases/pure.rs" <<'SOURCE'
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
sed -i 's/42/43/g' "$source_directory/cases/pure.rs"
"$compiler" test "$source_directory/cases" --jobs 2 --store "$store" --store-key "$key" \
    > "$test_directory/test-changed.out" 2> "$test_directory/test-changed.log"
sed -i 's/expect: value 43/expect: value 44/' "$source_directory/cases/pure.rs"
if "$compiler" test "$source_directory/cases" --jobs 2 --store "$store" --store-key "$key" \
    > "$test_directory/test-expectation.out" 2> "$test_directory/test-expectation.log"; then
    fail "changed test expectation replayed an old pass"
fi

# Closed language traps are deterministic observations on both execution routes.
# Resource/host failures remain observations to rerun, even with trap-looking paths.
mkdir "$source_directory/traps"
cp tests/conformance/trap/integer_shift_width.rs "$source_directory/traps/overflow.rs"
cat > "$source_directory/traps/division.rs" <<'SOURCE'
// expect: trap R0002
pub fn probe() -> i64 {
    let divisor = 0i64;
    7i64 / divisor
}
SOURCE
cat > "$source_directory/traps/bounds.rs" <<'SOURCE'
// expect: trap R0003
pub fn probe() -> i64 {
    let values = vec![1i64];
    values[1]
}
SOURCE
cat > "$source_directory/traps/assertion.rs" <<'SOURCE'
// expect: trap R0005
pub fn probe() { assert!(false); }
SOURCE
cat > "$source_directory/traps/explicit.rs" <<'SOURCE'
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

cat > "$source_directory/R0003-native-resource.rs" <<'SOURCE'
pub fn probe() -> i64 { 42 }
SOURCE
printf 'run\tprobe\t%s/R0003-native-resource.rs\n' "$source_directory" > "$test_directory/resource-batch"
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
        --runtime-source library/runtime.rs --store "$store" --store-key "$key" --stats \
        > "$test_directory/runtime-$pass.out" 2> "$test_directory/runtime-$pass.log"
    grep -q '"checkStatus":0,"status":0,"stdoutHex":"34320a"' "$test_directory/runtime-$pass.out" ||
        fail "explicit runtime probe did not complete"
    (( $(field "$test_directory/runtime-$pass.log" test_reused) == 0 )) ||
        fail "explicit runtime execution was reused"
done
(( $(field "$test_directory/runtime-warm.log" checked_reused) > 0 )) ||
    fail "explicit runtime disabled independent body reuse"

# Whole-image identity includes exact runtime-source bytes, even a harmless edit.
cp library/runtime.rs "$test_directory/runtime.rs"
cached_build runtime-image-cold --runtime-source "$test_directory/runtime.rs"
cp "$test_directory/result" "$test_directory/runtime-image-expected"
cached_build runtime-image-warm --runtime-source "$test_directory/runtime.rs"
(( $(field "$test_directory/runtime-image-warm.log" build_reused) > 0 )) ||
    fail "explicit runtime image was not reused"
printf '\n// changed runtime input closure\n' >> "$test_directory/runtime.rs"
cached_build runtime-image-changed --runtime-source "$test_directory/runtime.rs"
cmp "$test_directory/runtime-image-expected" "$test_directory/result"
(( $(field "$test_directory/runtime-image-changed.log" build_reused) == 0 )) ||
    fail "runtime-source edit replayed an old image"

# A World entry must run even if the source and expected output are unchanged.
cat > "$source_directory/effect.rs" <<SOURCE
// expect: output
pub fn main(mut world: World) {
    let mut directory = world.directory("$test_directory").unwrap();
    let mut file = directory.create("effect").unwrap();
    file.write_all(b"executed").unwrap();
}
SOURCE
for pass in cold warm; do
    rm -f "$test_directory/effect"
    "$compiler" test "$source_directory/effect.rs" --jobs 1 --store "$store" --store-key "$key" \
        > "$test_directory/effect-$pass.out" 2> "$test_directory/effect-$pass.log"
    [[ $(cat "$test_directory/effect") == executed ]] || fail "$pass effectful test was skipped"
done

# Function fragments remain relocatable across mixed hits, changing earlier
# control flow, literal interning and demanded owning cleanup.
relocation_store="$test_directory/relocation-store"
mkdir "$relocation_store"
cat > "$source_directory/relocation.rs" <<'SOURCE'
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
    "$compiler" build relocation.rs --source-dir "$source_directory" --store "$relocation_store" --store-key "$key" --stats \
        -o "$test_directory/relocation-result" > "$test_directory/relocation-$label.out" 2> "$test_directory/relocation-$label.log"
    cmp "$test_directory/relocation-expected" "$test_directory/relocation-result"
}
"$compiler" build relocation.rs --source-dir "$source_directory" --no-store -o "$test_directory/relocation-expected"
relocation_build cold
relocation_build warm
warm_emitted=$(field "$test_directory/relocation-warm.log" emitted_reused)
(( warm_emitted > 1 )) || fail "relocation fixture did not reuse multiple functions"
emission_entry=0
for entry in "$relocation_store"/native-*; do
    emission_entry=$((emission_entry+1))
    if (( emission_entry % 2 == 1 )); then rm "$entry"; fi
done
relocation_build mixed
mixed_emitted=$(field "$test_directory/relocation-mixed.log" emitted_reused)
(( mixed_emitted > 0 && mixed_emitted < warm_emitted )) || fail "mixed emission did not combine hits and misses"
# Preserve line numbers so unchanged bodies retain their exact diagnostic input.
sed -i 's/if x > 2 { shared.len() as i64 } else { x }/if x > 2 { if x > 3 { let extra = "earlier extra literal"; shared.len() as i64 + extra.len() as i64 - extra.len() as i64 } else { x } } else { x }/' "$source_directory/relocation.rs"
"$compiler" build relocation.rs --source-dir "$source_directory" --no-store -o "$test_directory/relocation-expected"
relocation_build changed
changed_emitted=$(field "$test_directory/relocation-changed.log" emitted_reused)
(( changed_emitted > 0 && changed_emitted < warm_emitted )) || fail "body change did not retain unchanged function fragments"
"$compiler" run relocation.rs --source-dir "$source_directory" --engine native --store "$relocation_store" --store-key "$key" \
    > "$test_directory/relocation-native"
"$compiler" run relocation.rs --source-dir "$source_directory" --engine interpreter --no-store \
    > "$test_directory/relocation-interpreted"
cmp "$test_directory/relocation-native" "$test_directory/relocation-interpreted"
[[ $(cat "$test_directory/relocation-native") == 56 ]] || fail "relocated fragments changed observable behavior"

# Remove the directory after the compiler has actually opened its capability.
# Bodies are deliberately independent; the test exercises store disappearance,
# not large-program acceptance limits.
awk 'BEGIN { for (i=0;i<1000;i++) print "fn f" i "() -> i64 { " i " }"; print "pub fn probe() -> i64 { f0() + 42 }" }' > "$source_directory/many.rs"
rm -rf "$store"
mkdir "$store"
absolute_store=$(realpath "$store")
"$compiler" check many.rs --source-dir "$source_directory" --store "$store" --store-key "$key" \
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
