#!/usr/bin/env bash
# Replay actual compiler-structure questions against one authenticated snapshot.
# The old route is the exact item's source-byte span, excluding search overhead.
set -euo pipefail
compiler_query_root=${1:?workspace root}
compiler_query_binary=${2:?compiler binary}
compiler_query_store=${3:?authenticated store}
compiler_query_key=${4:?private key}
compiler_query_snapshot=${5:?snapshot identity}
cd "$compiler_query_root"
compiler_query_temporary=$(mktemp -d "${TMPDIR:-/tmp}/compiler-query.XXXXXX")
trap 'rm -rf "$compiler_query_temporary"' EXIT HUP INT TERM
common=(query src/main.lkj --library-dir library --store "$compiler_query_store" --store-key "$compiler_query_key" --store-bytes 2147483648 --snapshot "$compiler_query_snapshot")
printf 'kind\tidentity\told_span\told_bytes\tquery_bytes\tfacts\tcontinuation\telapsed_ms\tproducer_ms\tindex_ms\tload_before\tload_after\n'
while IFS=$'\t' read -r kind identity; do
    "$compiler_query_binary" "${common[@]}" --kind meta --id "$identity" --limit 96 --bytes 32768 > "$compiler_query_temporary/meta"
    before=$(cut -d ' ' -f 1 /proc/loadavg)
    "$compiler_query_binary" "${common[@]}" --kind "$kind" --id "$identity" --depth 1 --limit 96 --bytes 32768 --stats > "$compiler_query_temporary/answer" 2> "$compiler_query_temporary/stats"
    after=$(cut -d ' ' -f 1 /proc/loadavg)
    perl - "$kind" "$identity" "$before" "$after" "$compiler_query_temporary" <<'PERL'
use strict;
use warnings;
use JSON::PP;
my ($kind, $identity, $before, $after, $directory) = @ARGV;
sub read_text { open my $file, '<:raw', shift or die $!; local $/; return <$file>; }
my $meta = read_text("$directory/meta");
my $answer = read_text("$directory/answer");
my $stats = read_text("$directory/stats");
my ($file, $start, $end) = $meta =~ /fact=item\t[^\n]*? file=("(?:\\.|[^"])*") start=(\d+) end=(\d+)/;
die "missing source item $identity\n" unless defined($end);
$file = decode_json($file);
my ($json) = $stats =~ /LKJSCRIPT_STATS (\{[^\n]*\})/;
my $timing = decode_json($json);
my ($cursor, $facts) = $answer =~ /fact=continuation\tcursor=([^ ]+) facts=(\d+)/;
die "missing continuation $identity\n" unless defined($facts);
printf "%s\t%s\t%s:%d..%d\t%d\t%d\t%d\t%s\t%.3f\t%.3f\t%.3f\t%s\t%s\n",
    $kind, $identity, $file, $start, $end, $end - $start, length($answer), $facts,
    $cursor eq 'none' ? 'none' : 'available', $timing->{total_ms}, $timing->{producer_hash_ms},
    $timing->{semantic_query_ms}, $before, $after;
PERL
done <<'QUESTIONS'
outline	driver::Options
outline	driver::Memo
item	driver::compile
signature	driver::execute_source
signature	driver::read_path
signature	driver::diagnostic_source_path
outline	ir::TypeKind
outline	ir::Op
outline	ir::Function
outline	ir::Program
outline	ir::Local
outline	ir::ReturnSource
outline	semantic::Catalog
outline	semantic::Item
outline	semantic::Reference
outline	reuse::AuthenticatedRoot
signature	test_runner::closed_contracts
signature	source::load_report
outline	native::PreparedImage
authority	authority::analyze
QUESTIONS
