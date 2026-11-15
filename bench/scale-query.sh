#!/usr/bin/env bash
# Bounded questions over the exact indexed ladder snapshot. No source reads.
set -euo pipefail
(($# == 9)) || { echo 'usage: scale-query COMPILER TREE STORE KEY LINES MEMORY_MIB DEADLINE STORE_BYTES SNAPSHOT' >&2; exit 2; }
compiler=$1 tree=$2 store=$3 key=$4 rung=$5 memory_mib=$6 deadline=$7 store_bytes=$8 snapshot=$9
[[ $rung =~ ^[0-9]+$ && $memory_mib =~ ^[1-9][0-9]*$ && $deadline =~ ^[1-9][0-9]*$ && $snapshot =~ ^[0-9a-f]{64}$ ]] || exit 2
query_work=$(mktemp -d "$(dirname "$store")/scale-query.XXXXXX")
trap 'rm -rf -- "$query_work"' EXIT
trap 'exit 130' HUP INT TERM
common=(query main.lkj --source-dir "$tree" --store "$store" --store-key "$key" --store-bytes "$store_bytes" --snapshot "$snapshot" --bytes 32768 --stats)
load() { cut -d ' ' -f 1-3 /proc/loadavg; }
sequence=0
printf 'rung\tphase\tkind\tidentity\tdepth\tlimit\tstatus\twall_ms\tuser_s\tkernel_s\trss_kib\tanswer_bytes\tfacts\tcontinuation\tpages\tcompiler_ms\tproducer_ms\tindex_ms\tload_before\tload_after\tanswer_sha256\n'
ask() {
    local phase=$1 kind=$2 identity=$3 depth=$4 limit=$5 cursor=$6
    local before after started finished status=0
    sequence=$((sequence + 1))
    answer_path="$query_work/answer-$sequence"
    before=$(load)
    started=$(date +%s%N)
    local continuation=()
    if [[ -n $cursor ]]; then continuation=(--cursor "$cursor"); fi
    /usr/bin/time -f '%e %U %S %M' -o "$query_work/time" \
        prlimit --as=$((memory_mib * 1024 * 1024)) -- timeout --kill-after=10s "${deadline}s" \
        "$compiler" "${common[@]}" --kind "$kind" --id "$identity" --depth "$depth" --limit "$limit" "${continuation[@]}" \
        > "$answer_path" 2> "$query_work/stats" || status=$?
    finished=$(date +%s%N)
    after=$(load)
    perl - "$rung" "$phase" "$kind" "$identity" "$depth" "$limit" "$status" "$started" "$finished" "$before" "$after" "$query_work" "$answer_path" <<'PERL'
use strict; use warnings; use JSON::PP; use Digest::SHA qw(sha256_hex);
my ($rung,$phase,$kind,$identity,$depth,$limit,$status,$started,$finished,$before,$after,$dir,$path)=@ARGV;
sub read_text { open my $f, '<:raw', shift or die $!; local $/; return <$f>; }
my $answer=read_text($path); my $stats=read_text("$dir/stats"); my $time=read_text("$dir/time");
my ($wall,$user,$kernel,$rss)=$time=~/([\d.]+) ([\d.]+) ([\d.]+) (\d+)\n?\z/;
die "missing completed timing\n" unless defined $rss;
my ($facts,$cursor,$pages,$total,$producer,$index)=('-', '-', '-', '-', '-', '-');
my $outcome=$status==0?'ok':"exit_$status";
if ($status==0) {
    my ($token,$count,$read)=$answer=~/\tfact=continuation\tcursor=(\S+) facts=(\d+) pages-read=(\d+)/;
    die "missing bounded continuation\n" unless defined $read;
    die "answer exceeds bytes bound\n" if length($answer)>32768;
    ($facts,$cursor,$pages)=($count,$token eq 'none'?'none':'available',$read);
    my @records=$stats=~/LKJSCRIPT_STATS (\{[^\n]*\})/g;
    for my $raw (@records) {
        my $value=decode_json($raw);
        next unless exists $value->{semantic_query_ms};
        ($total,$producer,$index)=@{$value}{qw(total_ms producer_hash_ms semantic_query_ms)};
    }
    die "query phase timings missing\n" if $index eq '-';
} elsif ($stats=~/R0006|R0008|ENOMEM|allocation failed|Cannot allocate memory/) {
    $outcome="memory_refusal_$status";
}
$before=~s/ /\//g; $after=~s/ /\//g;
printf "%s\t%s\t%s\t%s\t%s\t%s\t%s\t%.3f\t%s\t%s\t%s\t%d\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n",
    $rung,$phase,$kind,encode_json($identity),$depth,$limit,$outcome,($finished-$started)/1e6,
    $user,$kernel,$rss,length($answer),$facts,$cursor,$pages,$total,$producer,$index,$before,$after,sha256_hex($answer);
PERL
    if ((status)); then
        printf 'query failure kind=%s id=%s depth=%s limit=%s status=%s\n' "$kind" "$identity" "$depth" "$limit" "$status" >&2
        cat "$query_work/stats" >&2
        if [[ -s $answer_path ]]; then
            printf 'failed query response:\n' >&2
            cat "$answer_path" >&2
        fi
        return "$status"
    fi
}
# This reads only a bounded authenticated answer, never the source tree.
names() {
    perl - "$answer_path" <<'PERL'
use strict; use warnings; use JSON::PP;
my ($function,$generic,$type,$module,$trait)=('','','','','');
open my $file, '<:raw', $ARGV[0] or die $!;
while (my $line=<$file>) {
    my ($id,$level,$fact,$value)=split /\t/,$line,4;
    next unless $level eq 'level=source-claim' && $fact eq 'fact=signature';
    my ($kind,$text)=$value=~/^kind=(\S+) text=("(?:\\.|[^"])*")/;
    next unless defined $text;
    my $name=decode_json($id); my $signature=decode_json($text);
    $function=$name if !$function && $kind eq 'function' && $name=~/::f\d+$/;
    $generic=$name if !$generic && $kind eq 'function' && $signature=~/\bfn\s+\w+\s*</;
    $type=$name if !$type && $kind eq 'struct';
    $module=$name if !$module && $kind eq 'module';
    $trait=$name if !$trait && $kind eq 'trait';
}
print "$function\n$generic\n$type\n$module\n$trait\n";
PERL
}
missing() {
    printf '%s\tquestion\t%s\t""\t1\t30\tidentity_not_found_within_probe_bound\t-\t-\t-\t-\t0\t-\t-\t-\t-\t-\t-\t-\t-\t-\n' "$rung" "$1"
}
first_function= generic_function= record_type= first_module=
# Bound discovery to eight generated modules and four nesting levels per module.
# Missing IDs and a continuation remain coverage results, never invented facts.
ask discovery outline crate 1 96 ""
mapfile -t modules < <(perl - "$answer_path" <<'PERL'
use strict; use warnings; use JSON::PP;
open my $f,'<:raw',$ARGV[0] or die $!;
my $count=0;
while (<$f>) {
    my ($id,$level,$fact,$value)=split /\t/,$_,4;
    next unless $level eq 'level=source-claim' && $fact eq 'fact=signature' && $value=~/^kind=module /;
    my $name=decode_json($id); next unless $name=~/^m\d+$/;
    print "$name\n"; last if ++$count==8;
}
PERL
)
for selected in "${modules[@]}"; do
    for nesting in 0 1 2 3; do
        ask discovery outline "$selected" 1 96 ""
        mapfile -t observed < <(names)
        if [[ -z $first_function && -n ${observed[0]} ]]; then first_function=${observed[0]}; first_module=$selected; fi
        if [[ -z $generic_function && -n ${observed[1]} ]]; then generic_function=${observed[1]}; fi
        if [[ -z $record_type && -n ${observed[2]} ]]; then record_type=${observed[2]}; fi
        if [[ -n ${observed[0]} || -z ${observed[3]} ]]; then break; fi
        selected=${observed[3]}
    done
    if [[ -n $generic_function && -n $first_function && -n $record_type ]]; then break; fi
done
ask discovery outline std::iter 1 96 ""
mapfile -t observed < <(names)
trait_identity=${observed[4]}
ask question outline crate 1 30 ""
if [[ -n $first_module ]]; then ask question outline "$first_module" 1 30 ""; else missing outline-module; fi
if [[ -n $record_type ]]; then ask question outline "$record_type" 1 30 ""; else missing outline-type; fi
ask question item edit::target 1 30 ""
ask question slice edit::target 1 30 ""
if [[ -n $first_function ]]; then ask question slice "$first_function" 2 30 ""; else missing slice-function; fi
ask question uses edit::target 1 30 ""
ask question callers edit::target 1 30 ""
ask question callers edit::target 3 30 ""
ask question callees main 1 30 ""
ask question callees main 3 30 ""
if [[ -n $record_type ]]; then ask question impls "$record_type" 1 30 ""; else missing impls-type; fi
if [[ -n $trait_identity ]]; then ask question impls "$trait_identity" 1 30 ""; else missing impls-trait; fi
if [[ -n $generic_function ]]; then ask question instances "$generic_function" 1 30 ""; else missing instances; fi
ask question type edit::target::param:value 1 30 ""
if [[ -n $record_type ]]; then
    ask question type "$record_type" 1 30 ""
    ask question layout "$record_type" 1 30 ""
else missing type; missing layout; fi
ask question authority main 1 30 ""
ask question authority edit::target 1 30 ""
ask question dependents edit::target 1 30 ""
ask question dependents edit::target 3 30 ""
ask question outline crate 1 3 ""
cursor=$(sed -n 's/.*fact=continuation[[:space:]]cursor=\([^ ]*\).*/\1/p' "$answer_path")
if [[ $cursor != none ]]; then ask continuation outline crate 1 3 "$cursor"; fi
