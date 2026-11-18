#!/usr/bin/env bash
# Core source identities, honest zero-storage markers and checked edits.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
witness=${2:-}
[[ -z "$witness" || "$witness" == --witness ]]
mkdir -p target
fixture=$(mktemp -d "$PWD/target/core-facade.XXXXXX")
cleanup() {
    status=$?
    if ((status)); then
        for file in "$fixture"/*.log "$fixture"/*.query; do
            [[ ! -f "$file" ]] || cat "$file" >&2
        done
    fi
    if [[ ${LKJSCRIPT_CORE_KEEP:-0} != 1 ]]; then rm -rf "$fixture"; fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
mkdir "$fixture/source" "$fixture/store" "$fixture/layout"
cp -a library "$fixture/library"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/source/main.lkj" <<'SOURCE'
use core::marker::PhantomData;
use core::num::NonZeroUsize;
mod local {
    pub mod usize { pub const MAX: i64 = 40i64; }
    pub fn value() -> i64 { usize::MAX }
}
struct Packet { amount: NonZeroUsize, marker: PhantomData<String> }
fn mark() -> PhantomData<String> { PhantomData }
fn mark_byte() -> PhantomData<u8> { PhantomData }
fn primitive_limit() -> usize { usize::MAX }
fn answer() -> i64 {
    let packet = Packet { amount: NonZeroUsize::new(2usize).unwrap(), marker: mark() };
    local::value() + packet.amount.get() as i64
}
fn main() -> i64 {
    let marker = mark();
    drop(marker); drop(marker); drop(mark_byte());
    assert_eq!(primitive_limit(), 18446744073709551615usize);
    answer()
}
SOURCE
common=(main.lkj --source-dir "$fixture/source" --library-dir "$fixture/library"
    --store "$fixture/store" --store-key "$fixture/key")
publish() {
    "$compiler" build "${common[@]}" --index --stats -o "$fixture/program" > "$fixture/$1.log" 2>&1
    snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/$1.log")
    [[ ${#snapshot} == 64 ]]
}
query() {
    "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$1" --id "$2" \
        --limit 128 --bytes 65536 > "$fixture/$3.query"
}
execute() {
    for engine in native interpreter; do
        [[ $("$compiler" run "${common[@]}" --no-store --engine "$engine") == "$1" ]]
    done
}
publish cold
first_snapshot=$snapshot
execute 42
query references primitive_limit primitive-limit
query references local::value local-limit
query type mark marker-type
query type mark_byte byte-marker-type
grep -Fq PhantomData "$fixture/marker-type.query"
grep -Fq String "$fixture/marker-type.query"
grep -Fq 'PhantomData<u8>' "$fixture/byte-marker-type.query"
if grep -Eq '\[\(\); ?0\]' "$fixture/marker-type.query"; then
    echo 'source marker type was replaced by its physical array storage' >&2; exit 1
fi
perl -MJSON::PP - "$fixture/source/main.lkj" "$fixture/primitive-limit.query" "$fixture/local-limit.query" <<'PERL'
use strict; use warnings;
my ($path,$primitive,$local)=@ARGV;
open my $input,'<:raw',$path or die $!; local $/; my $source=<$input>; close $input;
for my $entry ([$primitive,'std::integer::__lkjscript_usize_max','alias-value'],
               [$local,'local::usize::MAX','value']) {
    open my $facts,'<',$entry->[0] or die $!;
    my $text=<$facts>; close $facts;
    my $seen=0;
    for my $line (split /\n/,$text) {
        next unless $line =~ /^("(?:\\.|[^"])*")\s+level=source-claim\s+fact=references\s+from=("(?:\\.|[^"])*")\s+file=("(?:\\.|[^"])*")\s+start=(\d+)\s+end=(\d+)\s+kind=(\S+)/;
        my ($target,$start,$end,$kind)=(decode_json($1),$4,$5,$6);
        next unless $target eq $entry->[1];
        die "limit reference lost its spelling/alias identity\n"
            unless $kind eq $entry->[2] && substr($source,$start,$end-$start) eq 'MAX';
        ++$seen;
    }
    die "missing or duplicated limit reference\n" unless $seen==1;
}
PERL
publish warm
[[ $snapshot == "$first_snapshot" ]]
grep -Eq '"build_reused":1' "$fixture/warm.log"

# Rename an authored shadow without rewriting the primitive's stable MAX spelling.
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"local::usize::MAX","name":"TOP"}]}
REQUEST
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rename.log" 2>&1
grep -Fq 'usize::TOP' "$fixture/source/main.lkj"
grep -Fq 'fn primitive_limit() -> usize { usize::MAX }' "$fixture/source/main.lkj"
execute 42
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"answer","source":"{ local::value() + NonZeroUsize::new(3usize).unwrap().get() as i64 }"}]}
REQUEST
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/body-edit.log" 2>&1
execute 43
publish edited
valid_snapshot=$snapshot
cp "$fixture/source/main.lkj" "$fixture/before-invalid"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"mark","source":"{ [] }"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rejected-array.log" 2>&1; then
    echo 'an array literal forged the nominal marker during edit' >&2; exit 1
fi
grep -Eq 'E_EDIT_CHECK|E_TYPE' "$fixture/rejected-array.log"
cmp "$fixture/before-invalid" "$fixture/source/main.lkj"
test ! -e "$fixture/source/.lkjscript-edit.pending"
publish rollback
[[ $snapshot == "$valid_snapshot" ]]
grep -Eq '"build_reused":1' "$fixture/rollback.log"

# An immutable snapshot cannot turn a modified marker declaration into the old ABI.
cp "$fixture/library/marker.lkj" "$fixture/marker-original"
for change in field bound derive predicate; do
    cp "$fixture/marker-original" "$fixture/library/marker.lkj"
    CORE_MARKER_CHANGE="$change" perl -0pi -e '
        my $change=$ENV{CORE_MARKER_CHANGE};
        if ($change eq "field") {
            s/pub struct PhantomData<T: \?Sized>;/pub struct PhantomData<T: ?Sized> { value: T }/ or die "field anchor";
        } elsif ($change eq "bound") {
            s/pub struct PhantomData<T: \?Sized>;/pub struct PhantomData<T>;/ or die "bound anchor";
        } elsif ($change eq "derive") {
            s/Copy, Clone/Clone/ or die "derive anchor";
        } elsif ($change eq "predicate") {
            s/pub struct PhantomData<T: \?Sized>;/pub struct PhantomData<T: ?Sized> where T: Copy;/ or die "predicate anchor";
        } else { die "unknown mutation"; }
    ' "$fixture/library/marker.lkj"
    if "$compiler" build "${common[@]}" --index --stats -o "$fixture/program" > "$fixture/rejected-marker-$change.log" 2>&1; then
        echo "modified PhantomData shape was accepted: $change" >&2; exit 1
    fi
    grep -Eq 'E_TYPE|E_TRAIT|E_UNSUPPORTED' "$fixture/rejected-marker-$change.log"
done
cp "$fixture/marker-original" "$fixture/library/marker.lkj"
publish restored
[[ $snapshot == "$valid_snapshot" ]]
execute 43
cat > "$fixture/source/no-library.lkj" <<'SOURCE'
pub fn main() -> usize { usize::MAX }
SOURCE
if "$compiler" check no-library.lkj --source-dir "$fixture/source" --no-library --no-store \
    > "$fixture/no-library.log" 2>&1; then
    echo 'primitive source limit acquired an ambient provider' >&2; exit 1
fi
grep -Fq 'primitive limits require the source integer library' "$fixture/no-library.log"

# Inferred and recursive arguments must reach the marker's region boundary.
# A generic recursive carrier avoids unrelated written-lifetime restrictions.
for shape in inferred recursive; do
    cp "tests/conformance/fail/core_phantom_${shape}_borrow_boundary.lkj" "$fixture/source/marker-$shape.lkj"
    printf '\nfn main() -> i64 { probe() }\n' >> "$fixture/source/marker-$shape.lkj"
    if "$compiler" check "marker-$shape.lkj" --source-dir "$fixture/source" --library-dir "$fixture/library" \
        --no-store > "$fixture/marker-$shape.log" 2>&1; then
        echo "reference-bearing marker was accepted: $shape" >&2; exit 1
    fi
    grep -Fq 'reference-bearing PhantomData requires a phantom region proof' "$fixture/marker-$shape.log"
done

# Inspect actual source-produced checked IR with both existing layout algorithms.
cat > "$fixture/source/layout.lkj" <<'SOURCE'
use core::marker::PhantomData;
struct Packet { first: u8, marker: PhantomData<u64>, last: u8 }
fn marker_u8() -> PhantomData<u8> { PhantomData }
fn marker_u64() -> PhantomData<u64> { PhantomData }
fn marker_string() -> PhantomData<String> { PhantomData }
fn marker_str() -> PhantomData<str> { PhantomData }
fn marker_slice() -> PhantomData<[u8]> { PhantomData }
fn packet() -> Packet { Packet { first: 20u8, marker: PhantomData, last: 22u8 } }
pub fn main() -> i64 {
    drop(marker_u8()); drop(marker_u64()); drop(marker_string());
    drop(marker_str()); drop(marker_slice());
    let value = packet(); value.first as i64 + value.last as i64
}
SOURCE
for engine in native interpreter; do
    [[ $("$compiler" run layout.lkj --source-dir "$fixture/source" --library-dir "$fixture/library" \
        --no-store --engine "$engine") == 42 ]]
done
"$compiler" ir layout.lkj --source-dir "$fixture/source" --library-dir "$fixture/library" \
    --no-store -o "$fixture/layout/program.lkjir" > "$fixture/layout-ir.log" 2>&1
cp src/ir.lkj src/ir_text.lkj src/diagnostic.lkj "$fixture/layout/"
cp src/native/layout.lkj "$fixture/layout/native_layout.lkj"
mkdir "$fixture/layout/checked"
cp src/check/units/layout.lkj "$fixture/layout/checked/layout.lkj"
sha256sum src/ir.lkj src/ir_text.lkj src/diagnostic.lkj src/native/layout.lkj \
    src/check/units/layout.lkj src/check.lkj > "$fixture/layout-sources.sha256"
cat > "$fixture/layout/checked.lkj" <<'SOURCE'
pub mod layout;
use crate::diagnostic::{Diagnostic, Span};
fn problem(message: &str) -> Diagnostic { Diagnostic::new(Span::default(), "E_UNIT", message) }
pub fn layouts(program: &crate::ir::Program) -> Vec<layout::Layout> {
    layout::derive(program).unwrap()
}
SOURCE
cat > "$fixture/layout/main.lkj" <<'SOURCE'
mod diagnostic;
mod ir;
mod ir_text;
mod native_layout;
mod checked;
use diagnostic::{Diagnostic, Span};
use ir::{Program, TypeKind};
fn result_type(program: &Program, name: &str) -> usize {
    let mut selected: Option<usize> = None;
    for function in &program.functions {
        if function.name == name { assert!(selected.is_none()); selected = Some(function.result); }
    }
    selected.unwrap()
}
pub fn main() -> i64 {
    let program = ir_text::read(include_str!("program.lkjir")).unwrap();
    let native = native_layout::Layouts::new(&program).unwrap();
    let checked = checked::layouts(&program);
    let names: Vec<String> = vec!["marker_u8".to_string(), "marker_u64".to_string(),
        "marker_string".to_string(), "marker_str".to_string(), "marker_slice".to_string()];
    let mut selected = Vec::<usize>::new();
    for name in names {
        let ty = result_type(&program, &name);
        assert!(!selected.contains(&ty)); selected.push(ty);
        assert!(program.types[ty].name.contains("PhantomData"));
        assert!(matches!(program.types[ty].kind, TypeKind::Array { inner, len } if inner == ir::UNIT && len == 0usize));
        let storage = native.storage(ty);
        assert_eq!(storage.bytes, 0u64); assert_eq!(storage.alignment, 1u8); assert!(!storage.cleanup);
        assert_eq!(checked[ty].bytes, 0u64); assert_eq!(checked[ty].alignment, 1u8); assert!(!checked[ty].cleanup);
        for (owner, _) in &program.destructors { assert!(*owner != ty); }
    }
    let packet = result_type(&program, "packet");
    let storage = native.storage(packet);
    assert_eq!(storage.bytes, 2u64); assert_eq!(storage.alignment, 1u8);
    assert_eq!(storage.fields, vec![0u64, 1u64, 1u64]);
    assert_eq!(checked[packet].bytes, 2u64); assert_eq!(checked[packet].alignment, 1u8);
    assert_eq!(checked[packet].fields, storage.fields);
    42
}
SOURCE
# Copy the production checker's graph traversal verbatim; only module/privacy
# wrappers differ. Neither layout algorithm nor the trusted source is modified.
perl -0ne '
    my @functions = /^(fn direct_children\(kind: &TypeKind\) -> Vec<usize> \{.*?^\})/msg;
    die "expected one production direct_children helper\n" unless @functions==1;
    print "\n",$functions[0],"\n";
' src/check.lkj >> "$fixture/layout/main.lkj"
cmp src/native/layout.lkj "$fixture/layout/native_layout.lkj"
cmp src/check/units/layout.lkj "$fixture/layout/checked/layout.lkj"
"$compiler" run main.lkj --source-dir "$fixture/layout" --library-dir "$fixture/library" \
    --no-store --engine native > "$fixture/layout-native.log" 2>&1
[[ $(cat "$fixture/layout-native.log") == 42 ]]
[[ $("$compiler" run main.lkj --source-dir "$fixture/layout" --library-dir "$fixture/library" \
    --no-store --engine interpreter) == 42 ]]
# Unit headers preserve the nominal argument even when all layouts are empty.
mkdir "$fixture/unit-source" "$fixture/unit-store"
cat > "$fixture/unit-source/units.lkj" <<'SOURCE'
const UNITS: &[&str] = &["base=base.lkj", "app=app.lkj"];
const DEPENDS: &[&str] = &["app=base"];
const ENTRY: &str = "app::main";
SOURCE
cat > "$fixture/unit-source/base.lkj" <<'SOURCE'
use core::marker::PhantomData;
pub fn marker() -> PhantomData<u8> { PhantomData }
pub fn consume(value: PhantomData<u8>) -> i64 { drop(value); 42i64 }
SOURCE
cat > "$fixture/unit-source/app.lkj" <<'SOURCE'
use core::marker::PhantomData;
pub fn main(mut out: Stdout) {
    let value = crate::base::consume(crate::base::marker());
    out.write(format!("{value}\n").as_bytes());
}
SOURCE
unit_options=(units.lkj --source-dir "$fixture/unit-source" --library-dir "$fixture/library"
    --store "$fixture/unit-store" --store-key "$fixture/key" --stats)
"$compiler" build-units "${unit_options[@]}" --grant out -o "$fixture/unit-program" > "$fixture/unit-cold.log" 2>&1
[[ $("$fixture/unit-program") == 42 ]]
"$compiler" build-units "${unit_options[@]}" --grant out -o "$fixture/unit-warm" > "$fixture/unit-warm.log" 2>&1
grep -Fq 'checked=0 reused=2 emitted=0' "$fixture/unit-warm.log"
cmp "$fixture/unit-program" "$fixture/unit-warm"
cp "$fixture/unit-source/app.lkj" "$fixture/unit-app-original"
perl -0pi -e 's/crate::base::consume\(crate::base::marker\(\)\)/crate::base::consume(PhantomData::<u16>)/ or die "unit argument anchor"' \
    "$fixture/unit-source/app.lkj"
if "$compiler" build-units "${unit_options[@]}" --grant out -o "$fixture/unit-program" \
    > "$fixture/unit-rejected.log" 2>&1; then
    echo 'different marker arguments crossed an authenticated unit boundary' >&2; exit 1
fi
grep -Fq E_TYPE "$fixture/unit-rejected.log"
cmp "$fixture/unit-program" "$fixture/unit-warm"
cp "$fixture/unit-app-original" "$fixture/unit-source/app.lkj"
perl -0pi -e 's/PhantomData<u8>/PhantomData<u16>/g or die "unit type anchor"' "$fixture/unit-source/base.lkj"
"$compiler" build-units "${unit_options[@]}" --grant out -o "$fixture/unit-new-argument" \
    > "$fixture/unit-new-argument.log" 2>&1
grep -Fq 'checked=2 reused=0' "$fixture/unit-new-argument.log"
[[ $("$fixture/unit-new-argument") == 42 ]]
if [[ "$witness" == --witness ]]; then
    cat > "$fixture/layout.rs" <<'RUST'
use std::marker::PhantomData;
use std::mem::{align_of, size_of};
struct Packet { first: u8, marker: PhantomData<u64>, last: u8 }
fn marker<T: ?Sized>() {
    assert_eq!(size_of::<PhantomData<T>>(), 0);
    assert_eq!(align_of::<PhantomData<T>>(), 1);
}
fn main() {
    marker::<u8>(); marker::<u64>(); marker::<String>(); marker::<str>(); marker::<[u8]>();
    assert_eq!(size_of::<Packet>(), 2); assert_eq!(align_of::<Packet>(), 1);
    let value = Packet { first: 20, marker: PhantomData, last: 22 };
    println!("{}", value.first + value.last);
}
RUST
    "${RUSTC:-rustc}" --edition=2021 "$fixture/layout.rs" -o "$fixture/layout-rust" > "$fixture/layout-rust.log" 2>&1
    [[ $("$fixture/layout-rust") == 42 ]]
fi
printf 'ok core facade: exact primitive/shadow references, nominal type queries, edits and rollback, library shape invalidation, zero bytes/alignment one through both production layouts, distinct authenticated unit headers, native/interpreter and optional Rust\n'
