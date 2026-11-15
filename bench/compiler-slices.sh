#!/usr/bin/env bash
# The twenty compiler subjects, followed by each subject's bounded depth-one slice.
set -euo pipefail
(($# == 6)) || { echo 'usage: compiler-slices ROOT COMPILER STORE KEY SNAPSHOT RESULTS' >&2; exit 2; }
source_root=$(realpath "$1")
compiler=$(realpath "$2")
store=$(realpath "$3")
key=$(realpath "$4")
snapshot=$5
results=$(realpath -m "$6")
[[ ! -e $results && $snapshot =~ ^[0-9a-f]{64}$ ]] || exit 2
mkdir -p "$(dirname "$results")"
work=$(mktemp -d "$results.work.XXXXXX")
trap 'rm -rf -- "$work"' EXIT
trap 'exit 130' HUP INT TERM
common=(query src/main.lkj --source-dir "$source_root" --library-dir "$source_root/library" --store "$store" --store-key "$key" --store-bytes 2147483648 --snapshot "$snapshot" --depth 1 --limit 96 --bytes 32768)
printf 'phase\tkind\tidentity\told_span_bytes\tanswer_bytes\tfacts\tcontinuation\twall_ms\tcompiler_ms\tproducer_ms\tindex_ms\tuser_s\tkernel_s\trss_kib\tload_before\tload_after\n' > "$results"
while IFS=$'\t' read -r question identity; do
    prlimit --as=2147483648 -- timeout --kill-after=10s 30s "$compiler" "${common[@]}" --kind meta --id "$identity" > "$work/meta"
    for phase in question slice; do
        kind=$question
        if [[ $phase == slice ]]; then kind=slice; fi
        before=$(cut -d ' ' -f 1-3 /proc/loadavg)
        started=$(date +%s%N)
        /usr/bin/time -f '%e %U %S %M' -o "$work/time" prlimit --as=2147483648 -- timeout --kill-after=10s 30s \
            "$compiler" "${common[@]}" --kind "$kind" --id "$identity" --stats > "$work/answer" 2> "$work/stats"
        finished=$(date +%s%N)
        after=$(cut -d ' ' -f 1-3 /proc/loadavg)
        perl - "$phase" "$kind" "$identity" "$started" "$finished" "$before" "$after" "$work" <<'PERL' >> "$results"
use strict; use warnings; use JSON::PP;
my ($phase,$kind,$identity,$start,$finish,$before,$after,$work)=@ARGV;
sub read_text { open my $f,'<:raw',shift or die $!; local $/; return <$f>; }
my $meta=read_text("$work/meta"); my $answer=read_text("$work/answer"); my $stats=read_text("$work/stats");
my ($from,$to)=$meta=~/fact=item\t[^\n]*? start=(\d+) end=(\d+)/;
my ($cursor,$facts)=$answer=~/fact=continuation\tcursor=([^ ]+) facts=(\d+)/;
die "missing metadata/continuation for $identity\n" unless defined($from) && defined($facts);
my ($json)=$stats=~/LKJSCRIPT_STATS (\{[^\n]*\})/; my $timing=decode_json($json);
my ($wall,$user,$kernel,$rss)=read_text("$work/time")=~/([\d.]+) ([\d.]+) ([\d.]+) (\d+)\n?\z/;
die "missing completed timing\n" unless defined $rss;
$before=~s/ /\//g; $after=~s/ /\//g;
printf "%s\t%s\t%s\t%d\t%d\t%d\t%s\t%.3f\t%.3f\t%.3f\t%.3f\t%s\t%s\t%s\t%s\t%s\n",
 $phase,$kind,encode_json($identity),$to-$from,length($answer),$facts,$cursor eq 'none'?'none':'available',
 ($finish-$start)/1e6,$timing->{total_ms},$timing->{producer_hash_ms},$timing->{semantic_query_ms},
 $user,$kernel,$rss,$before,$after;
PERL
    done
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
perl - "$results" <<'PERL'
use strict; use warnings;
open my $f,'<',$ARGV[0] or die $!; <$f>;
my @bytes; my ($continuations,$slow,$questions)=(0,0,0);
while (<$f>) {
    chomp; my @v=split /\t/;
    if ($v[0] eq 'slice') { push @bytes,$v[4]; $continuations++ if $v[6] ne 'none'; }
    $slow++ if $v[7]>=100; $questions++;
}
@bytes=sort {$a<=>$b} @bytes;
die "wrong subject count\n" unless @bytes==20 && $questions==40;
printf "slices=%d median_bytes=%.1f min_bytes=%d max_bytes=%d continuation=%d queries_at_or_above_100ms=%d/%d\n",
 scalar(@bytes),($bytes[9]+$bytes[10])/2,$bytes[0],$bytes[-1],$continuations,$slow,$questions;
PERL
