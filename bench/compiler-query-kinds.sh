#!/usr/bin/env bash
# New-reader timings on one existing historical snapshot; old-reader parity last.
# The fixture owner retains responsibility for source/store/key lifetime.
set -euo pipefail
cd "$(dirname "$0")/.."
benchmark_root=$PWD
reader= producer= source_root= library_root= store= key= snapshot= source_digest= checked_ir=
expected_reader= expected_producer=
results=target/compiler-query-kinds.tsv
while (($#)); do
    case "$1" in
        --reader|--producer|--source-root|--library-root|--store|--key|--snapshot|--source-sha256|--checked-ir-sha256|--reader-sha256|--producer-sha256|--results)
            (($# >= 2)) || exit 2
            case "$1" in
                --reader) reader=$2 ;;
                --producer) producer=$2 ;;
                --source-root) source_root=$2 ;;
                --library-root) library_root=$2 ;;
                --store) store=$2 ;;
                --key) key=$2 ;;
                --snapshot) snapshot=$2 ;;
                --source-sha256) source_digest=$2 ;;
                --checked-ir-sha256) checked_ir=$2 ;;
                --reader-sha256) expected_reader=$2 ;;
                --producer-sha256) expected_producer=$2 ;;
                --results) results=$2 ;;
            esac
            shift 2 ;;
        *) echo 'usage: compiler-query-kinds --reader FILE --reader-sha256 SHA --producer FILE --producer-sha256 SHA --source-root ROOT --library-root LIB --store STORE --key KEY --snapshot SHA --source-sha256 SHA --checked-ir-sha256 SHA --results FILE' >&2; exit 2 ;;
    esac
done
for digest in "$expected_reader" "$expected_producer" "$snapshot" "$source_digest" "$checked_ir"; do
    [[ $digest =~ ^[0-9a-f]{64}$ ]] || exit 2
done
reader=$(realpath "$reader")
producer=$(realpath "$producer")
source_root=$(realpath "$source_root")
library_root=$(realpath "$library_root")
store=$(realpath "$store")
key=$(realpath "$key")
results=$(realpath -m "$results")
[[ -x $reader && -x $producer && -d $source_root && -d $library_root && -d $store && -f $key ]] || exit 2
[[ ! -e $results && ! -e $results.raw && ! -e $results.meta && ! -e $results.parity.tsv ]] || exit 2
mkdir -p target "$(dirname "$results")"
umask 077
run=$(mktemp -d "$benchmark_root/target/compiler-query-kinds.XXXXXX")
printf 'owned_readers=%s\n' "$run"
trap 'rm -rf -- "$run"' EXIT
trap 'exit 130' HUP INT TERM
mkdir "$results.raw"
cp "$reader" "$run/reader"
cp "$producer" "$run/producer"
chmod 700 "$run/reader" "$run/producer"
actual_reader=$(sha256sum "$run/reader" | cut -d ' ' -f 1)
actual_producer=$(sha256sum "$run/producer" | cut -d ' ' -f 1)
[[ $actual_reader == "$expected_reader" && $actual_producer == "$expected_producer" ]] || exit 1
{
    printf 'reader_sha256=%s\nreader_bytes=%s\n' "$actual_reader" "$(stat -c %s "$run/reader")"
    printf 'snapshot_producer_sha256=%s\nproducer_bytes=%s\n' "$actual_producer" "$(stat -c %s "$run/producer")"
    printf 'source_root=%s\nlibrary_root=%s\nsnapshot_sha256=%s\nsource_sha256=%s\nchecked_ir_sha256=%s\n' "$source_root" "$library_root" "$snapshot" "$source_digest" "$checked_ir"
    printf 'measured_harness_sha256=%s\nstarted_utc=%s\n' "$(sha256sum "$benchmark_root/bench/compiler-query-kinds.sh" | cut -d ' ' -f 1)" "$(date -u +%FT%TZ)"
    printf 'query_memory_bytes=2147483648\nstore_budget_bytes=2147483648\nquery_deadline_s=30\nworkers=1\n'
    printf 'depth=2\nfact_limit=96\nreply_byte_limit=32768\nmetadata_prequeries=0\nindex_builds_here=0\nfile_cache_reset=false\n'
    printf 'oracle=ten old-reader requests after all ten timed new-reader requests; exact stdout parity; not benchmark samples\n'
} > "$results.meta"
printf 'phase\tkind\tidentity\tstatus\tprotocol_valid\tcoverage\tfact_levels\tfact_kinds\twall_ms\tsubprocess_wall_s\tcompiler_ms\tsnapshot_auth_ms\tsemantic_query_ms\tproducer_setup_ms\tproducer_bytes_hashed\tanswer_bytes\tfacts\tcontinuation\tpages_read\ttraversal_steps\tuser_s\tkernel_s\trss_kib\treader_sha256\tproducer_sha256\tsource_sha256\tchecked_ir_sha256\tsnapshot_sha256\tanswer_sha256\tload_before\tload_after\terrors\n' > "$results"
printf 'phase\tkind\tidentity\tstatus\texact_stdout\tcandidate_answer_sha256\toracle_answer_sha256\tload_before\tload_after\n' > "$results.parity.tsv"
load() { cut -d ' ' -f 1-3 /proc/loadavg; }
measure() {
    local tag=$1 memory=$2 deadline=$3
    shift 3
    local before after started finished status=0
    before=$(load)
    started=$(date +%s%N)
    printf 'phase=%s\nload_before=%s\nstarted_ns=%s\nresults=%s\n' "$tag" "$before" "$started" "$results" > "$results.raw/checkpoint"
    /usr/bin/time -f '%e %U %S %M' -o "$results.raw/$tag.time" \
        prlimit --as="$memory" -- timeout --kill-after=10s "${deadline}s" "$@" \
        > "$results.raw/$tag.stdout" 2> "$results.raw/$tag.stderr" || status=$?
    finished=$(date +%s%N)
    after=$(load)
    printf '%s\t%s\t%s\t%s\t%s\n' "$status" "$started" "$finished" "$before" "$after" > "$results.raw/$tag.capture"
}
query_common=(query src/main.lkj --source-dir "$source_root" --library-dir "$library_root"
    --store "$store" --store-key "$key" --store-bytes 2147483648
    --snapshot "$snapshot" --depth 2 --limit 96 --bytes 32768 --stats)
ordinal=0
while IFS=$'\t' read -r kind identity; do
    phase=supplemental
    ordinal=$((ordinal + 1))
        tag=$(printf 'query-%02d' "$ordinal")
        printf '%s\t%s\t%s\t%s\n' "$tag" "$phase" "$kind" "$identity" >> "$results.raw/requests.tsv"
        measure "$tag" 2147483648 30 "$run/reader" "${query_common[@]}" --kind "$kind" --id "$identity"
        perl - "$phase" "$kind" "$identity" "$tag" "$results" "$actual_producer" "$source_digest" "$checked_ir" "$snapshot" "$actual_reader" <<'PERL' >> "$results"
use strict; use warnings; use JSON::PP; use Digest::SHA qw(sha256_hex);
my ($phase,$kind,$identity,$tag,$results,$expected_producer,$expected_source,$expected_ir,$expected_snapshot,$reader_digest)=@ARGV;
sub read_text { open my $f,'<:raw',shift or die $!; local $/; return <$f>; }
my ($status,$start,$finish,$before,$after)=split /\t/,read_text("$results.raw/$tag.capture");
chomp $after; $before=~s/ /\//g; $after=~s/ /\//g;
my $answer=read_text("$results.raw/$tag.stdout"); my $err=read_text("$results.raw/$tag.stderr");
my ($json)=$err=~/LKJSCRIPT_STATS (\{[^\n]*\})/; my $stats=defined($json)?decode_json($json):{};
my ($wall,$user,$kernel,$rss)=read_text("$results.raw/$tag.time")=~/([\d.]+) ([\d.]+) ([\d.]+) (\d+)\n?\z/;
my ($header)=$answer=~/\A([^\n]*)\n/; $header//='';
my ($producer)=$header=~/\bproducer=([0-9a-f]{64})(?: |$)/;
my ($source)=$header=~/\bsource=([0-9a-f]{64})(?: |$)/;
my ($ir)=$header=~/\bchecked-ir=([0-9a-f]{64})(?: |$)/;
my ($snapshot)=$header=~/\bsnapshot=([0-9a-f]{64})(?: |$)/;
my ($cursor,$facts,$pages,$steps)=$answer=~/\tfact=continuation\tcursor=([^ ]+) facts=(\d+) pages-read=(\d+) traversal-steps=(\d+)\n\z/;
my @errors; push @errors,"exit=$status" if $status; push @errors,"missing stats" unless defined $json;
push @errors,"missing timing" unless defined $rss;
my ($header_identity,$level,$fact)=split /\t/,$header;
push @errors,"header identity" unless defined($header_identity) && $header_identity eq encode_json($identity);
push @errors,"header protocol" unless ($level//'') eq 'level=protocol' && ($fact//'') eq 'fact=query'
    && $header=~/\bversion=2 / && $header=~/\bvalidation=authenticated-producing-compiler /
    && $header=~/\bcurrent-check=not-performed / && $header=~/\bfreshness=not-asserted /
    && $header=~/\bselection=explicit$/ && $header=~/\bkind=\Q$kind\E depth=2 /;
push @errors,"producer provenance" unless ($producer//'') eq $expected_producer;
push @errors,"source provenance" unless ($source//'') eq $expected_source;
push @errors,"IR provenance" unless ($ir//'') eq $expected_ir;
push @errors,"snapshot provenance" unless ($snapshot//'') eq $expected_snapshot;
push @errors,"reply byte bound" if length($answer)>32768;
push @errors,"fact bound or missing continuation" unless defined($facts) && $facts<=96;
push @errors,"unexpected producer byte hashing" unless exists($stats->{producer_bytes_hashed}) && $stats->{producer_bytes_hashed}==0;
push @errors,"missing snapshot timings" unless exists($stats->{snapshot_auth_ms}) && exists($stats->{semantic_query_ms});
my @lines=split /\n/,$answer;
push @errors,"fact count mismatch" if defined($facts) && @lines!=$facts+2;
my @body_lines=@lines>2?@lines[1..$#lines-1]:();
my (%levels,%kinds); my $substantive=0; my $mislabelled=0;
my %expected=(
    uses=>{'source-claim:uses'=>1},
    callers=>{'source-claim:callers'=>1,'checked-ir:callers'=>1},
    callees=>{'source-claim:callees'=>1,'checked-ir:callees'=>1},
    impls=>{'source-claim:impls'=>1},
    instances=>{'checked-ir:instance'=>1},
    type=>{'source-claim:declared-type'=>1},
    layout=>{'checked-ir:layout'=>1,'checked-ir:variant-layout'=>1},
    dependents=>{'source-claim:dependents'=>1},
    text=>{'source-claim:text'=>1}
);
for my $line (@body_lines) {
    my ($level,$fact)=$line=~/\tlevel=([^\t]+)\tfact=([^\t]+)\t/;
    next if defined($level) && $level eq 'protocol';
    $substantive++; $levels{$level//'missing'}=1; $kinds{$fact//'missing'}=1;
    $mislabelled++ unless defined($level) && defined($fact) && $expected{$kind}{"$level:$fact"};
}
my $coverage=$status?'request-refused':!$substantive?'empty':$mislabelled?'mislabelled':'nonempty';
push @errors,"fact label mismatch" if $mislabelled;
sub value { my ($key)=@_; return exists($stats->{$key})?$stats->{$key}:'NA'; }
print join("\t",$phase,$kind,encode_json($identity),$status,@errors?'no':'yes',$coverage,join(",",sort keys %levels),join(",",sort keys %kinds),sprintf("%.3f",($finish-$start)/1e6),$wall//'NA',
    map({value($_)} qw(total_ms snapshot_auth_ms semantic_query_ms producer_hash_ms producer_bytes_hashed)),
    length($answer),$facts//'NA',defined($cursor)?($cursor eq 'none'?'none':'available'):'NA',$pages//'NA',$steps//'NA',
    $user//'NA',$kernel//'NA',$rss//'NA',$reader_digest,$producer//'NA',$source//'NA',$ir//'NA',$snapshot//'NA',sha256_hex($answer),
    $before,$after,encode_json(\@errors)),"\n";
if (@errors) { print STDERR "$tag $identity: ",join("; ",@errors),"\n",$err; }
PERL
        tail -n 1 "$results"
done <<'QUESTIONS'
uses	driver::Memo
callers	driver::compile
callees	driver::execute_source
impls	store::QueryAccess
impls	store::QueryReader
instances	semantic_store::open
type	driver::Options::field:store_bytes
layout	driver::Options
dependents	semantic_store::Snapshot
text	driver::compile
QUESTIONS

[[ $ordinal == 10 ]] || exit 1
printf 'timed_queries_complete_utc=%s\n' "$(date -u +%FT%TZ)" >> "$results.meta"
# No oracle request can warm a timed candidate request: all timings are complete.
parity_failed=0
while IFS=$'\t' read -r tag phase kind identity; do
    before=$(load)
    status=0
    prlimit --as=2147483648 -- timeout --kill-after=10s 30s "$run/producer" \
        "${query_common[@]}" --kind "$kind" --id "$identity" \
        > "$results.raw/$tag.oracle.stdout" 2> "$results.raw/$tag.oracle.stderr" || status=$?
    after=$(load)
    equal=no
    if ((status == 0)) && cmp -s "$results.raw/$tag.stdout" "$results.raw/$tag.oracle.stdout"; then equal=yes; fi
    candidate_hash=$(sha256sum "$results.raw/$tag.stdout" | cut -d ' ' -f 1)
    oracle_hash=$(sha256sum "$results.raw/$tag.oracle.stdout" | cut -d ' ' -f 1)
    printf 'oracle-not-benchmark\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$kind" "$identity" "$status" "$equal" "$candidate_hash" "$oracle_hash" \
        "${before// /\/}" "${after// /\/}" >> "$results.parity.tsv"
    if [[ $equal != yes ]]; then
        printf 'E_QUERY_BENCH_PARITY %s %s status=%s\n' "$kind" "$identity" "$status" >&2
        parity_failed=1
    fi
done < "$results.raw/requests.tsv"
[[ $(sha256sum "$run/reader" | cut -d ' ' -f 1) == "$actual_reader" \
    && $(sha256sum "$run/producer" | cut -d ' ' -f 1) == "$actual_producer" ]] || exit 1
printf 'completed_utc=%s\ntimed_queries=10\noracle_validations=10\n' "$(date -u +%FT%TZ)" >> "$results.meta"
printf 'completed ten timed queries and ten subsequent exact-stdout oracle validations; results=%s\n' "$results"
exit "$parity_failed"
