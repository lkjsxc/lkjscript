#!/usr/bin/env bash
# Usage: exercise-compiler-edit.sh COMPILER [FRESH_OUTPUT_DIRECTORY]
# Reproduce the checked compiler body edit from an immutable baseline archive.
set -euo pipefail
if (($# < 1 || $# > 2)); then
    printf 'usage: %s COMPILER [FRESH_OUTPUT_DIRECTORY]\n' "$0" >&2
    exit 2
fi
catalog_producer=$(realpath -- "$1")
[[ -f "$catalog_producer" && -x "$catalog_producer" ]] || {
    printf 'compiler is not executable: %s\n' "$catalog_producer" >&2
    exit 2
}
catalog_worktree=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)
catalog_baseline=7dd89b8fba48b907a1a81f387a6f34e059b81da8
umask 077
if (($# == 2)); then
    catalog_data=$(realpath -m -- "$2")
    [[ ! -e "$catalog_data" && ! -L "$catalog_data" ]] || {
        printf 'output directory must be fresh: %s\n' "$catalog_data" >&2
        exit 2
    }
    mkdir -p -- "$(dirname -- "$catalog_data")"
    mkdir -- "$catalog_data"
else
    mkdir -p -- "$catalog_worktree/target"
    catalog_data=$(mktemp -d "$catalog_worktree/target/compiler-catalog-edit.XXXXXX")
fi
cd "$catalog_worktree"
catalog_tree="$catalog_data/source"
catalog_stage=preparation
catalog_key_directory=
catalog_key=
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if [[ -n "$catalog_key_directory" ]]; then
        rm -f -- "$catalog_key"
        rmdir -- "$catalog_key_directory"
    fi
    if ((status)); then
        printf 'compiler edit failed during %s; evidence: %s\n' "$catalog_stage" "$catalog_data" >&2
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM

# The disposable signing key is never part of the source or retained evidence.
catalog_key_directory=$(mktemp -d "${TMPDIR:-/tmp}/lkjscript-catalog-key.XXXXXX")
catalog_key="$catalog_key_directory/private.key"
head -c 32 /dev/urandom > "$catalog_key"
mkdir -- "$catalog_tree"
git -C "$catalog_worktree" archive "$catalog_baseline" src library tools tests |
    tar -x -C "$catalog_tree"
{
    printf 'baseline=%s\n' "$catalog_baseline"
    printf 'producer='
    sha256sum "$catalog_producer"
} > "$catalog_data/provenance.txt"

cat > "$catalog_data/request.json" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"value@syntax::macros::expand","source":"{ let result = expand::run(program); result }"}]}
REQUEST
cp "$catalog_tree/src/syntax/macros.lkj" "$catalog_data/compiler-macros.before.lkj"
perl - "$catalog_data/compiler-macros.before.lkj" "$catalog_data/compiler-macros.expected.lkj" <<'PERL'
use strict; use warnings;
open my $input, '<:raw', shift @ARGV or die $!;
local $/; my $text = <$input>;
my $before = '{ expand::run(program) }';
my $after = '{ let result = expand::run(program); result }';
my $at = index($text, $before);
die "expected exactly one original wrapper body\n"
    unless $at >= 0 && index($text, $before, $at + length($before)) == -1;
my $declaration = 'pub(crate) fn expand(program: &mut Program) -> Result<(), Diagnostic> ';
die "literal body is not the intended function\n"
    unless $at >= length($declaration)
        && substr($text, $at - length($declaration), length($declaration)) eq $declaration;
my $modules = () = $text =~ /^mod expand;$/mg;
die "expected exactly one same-path module declaration\n" unless $modules == 1;
substr($text, $at, length($before), $after);
open my $output, '>:raw', shift @ARGV or die $!;
print {$output} $text or die $!;
close $output or die $!;
PERL

catalog_sources() (
    cd "$catalog_tree"
    rg --files --hidden --no-ignore -0 -g '*.lkj' |
        LC_ALL=C sort -z | xargs -0 sha256sum
)
catalog_sources > "$catalog_data/source.before.sha256"
catalog_stage=dry-run
/usr/bin/time -v -o "$catalog_data/dry-run.time" "$catalog_producer" edit src/main.lkj \
    --source-dir "$catalog_tree" --library-dir "$catalog_tree/library" --no-store --store-key "$catalog_key" \
    --request "$catalog_data/request.json" --dry-run --stats \
    > "$catalog_data/dry-run.out" 2> "$catalog_data/dry-run.err"
catalog_sources > "$catalog_data/source.dry-run.sha256"
cmp "$catalog_data/source.before.sha256" "$catalog_data/source.dry-run.sha256"
[[ ! -e "$catalog_tree/.lkjscript-edit.generation" && ! -e "$catalog_tree/.lkjscript-edit.pending" ]]
grep -Fq 'selected="value@syntax::macros::expand"' "$catalog_data/dry-run.out"
grep -Fxq 'transaction sites=1 files=1 identities=1' "$catalog_data/dry-run.out"
grep -Eq ' mode=dry-run$' "$catalog_data/dry-run.out"

catalog_stage=apply
/usr/bin/time -v -o "$catalog_data/apply.time" "$catalog_producer" edit src/main.lkj \
    --source-dir "$catalog_tree" --library-dir "$catalog_tree/library" --no-store --store-key "$catalog_key" \
    --request "$catalog_data/request.json" --stats \
    > "$catalog_data/apply.out" 2> "$catalog_data/apply.err"
catalog_sources > "$catalog_data/source.after.sha256"
cmp "$catalog_data/compiler-macros.expected.lkj" "$catalog_tree/src/syntax/macros.lkj"
perl - "$catalog_data/source.before.sha256" "$catalog_data/source.after.sha256" \
    > "$catalog_data/changed-sources.txt" <<'PERL'
use strict; use warnings;
sub manifest {
    open my $input, '<', shift or die $!;
    my %paths;
    while (<$input>) {
        /^([0-9a-f]{64})  (.+)\n$/ or die "bad source manifest\n";
        die "duplicate source path\n" if exists $paths{$2};
        $paths{$2} = $1;
    }
    return \%paths;
}
my $before = manifest(shift @ARGV);
my $after = manifest(shift @ARGV);
die "different source-file set\n"
    unless join("\n", sort keys %$before) eq join("\n", sort keys %$after);
my @changed = grep { $before->{$_} ne $after->{$_} } sort keys %$before;
die "unexpected edits: @changed\n"
    unless @changed == 1 && $changed[0] eq 'src/syntax/macros.lkj';
print "$changed[0]\n";
PERL
[[ -s "$catalog_tree/.lkjscript-edit.generation" && ! -e "$catalog_tree/.lkjscript-edit.pending" ]]
grep -Fq 'selected="value@syntax::macros::expand"' "$catalog_data/apply.out"
grep -Fxq 'transaction sites=1 files=1 identities=1' "$catalog_data/apply.out"
grep -Fq 'program level=checked-ir authority-delta=none' "$catalog_data/apply.out"
grep -Eq ' mode=apply$' "$catalog_data/apply.out"

catalog_stage=build
(
    cd "$catalog_tree"
    /usr/bin/time -v -o "$catalog_data/build.time" "$catalog_producer" build src/main.lkj \
        --library-dir library --runtime-source library/runtime.lkj --inline-limit 64 --no-store \
        -o "$catalog_data/edited-compiler" > "$catalog_data/build.out" 2> "$catalog_data/build.err"
)
catalog_stage=reproduce
(
    cd "$catalog_tree"
    /usr/bin/time -v -o "$catalog_data/reproduce.time" "$catalog_data/edited-compiler" build src/main.lkj \
        --library-dir library --runtime-source library/runtime.lkj --inline-limit 64 --no-store \
        -o "$catalog_data/edited-compiler-reproduced" \
        > "$catalog_data/reproduce.out" 2> "$catalog_data/reproduce.err"
)
cmp "$catalog_data/edited-compiler" "$catalog_data/edited-compiler-reproduced"
catalog_stage=execution
printf '42\n' > "$catalog_data/expected-result.txt"
for catalog_engine in native interpreter; do
    "$catalog_data/edited-compiler" run "$catalog_tree/tests/conformance/pass/const_function_calls.lkj" \
        --no-library --no-store --engine "$catalog_engine" \
        > "$catalog_data/run-$catalog_engine.out" 2> "$catalog_data/run-$catalog_engine.err"
    cmp "$catalog_data/expected-result.txt" "$catalog_data/run-$catalog_engine.out"
done
{
    printf 'baseline=%s\n' "$catalog_baseline"
    printf 'dry-run source and publication state unchanged: pass\n'
    printf 'exact intended body and all %s archived source files: pass\n' \
        "$(wc -l < "$catalog_data/source.before.sha256")"
    printf 'changed source files: src/syntax/macros.lkj\n'
    printf 'same-path module and every other source file preserved: pass\n'
    sha256sum "$catalog_producer" "$catalog_data/edited-compiler" "$catalog_data/edited-compiler-reproduced"
    printf 'native result=42\ninterpreter result=42\n'
    printf 'edited compiler fixed point and both execution routes: pass\n'
} > "$catalog_data/verification.txt"
printf 'ok compiler catalog edit; evidence: %s\n' "$catalog_data"
