#!/usr/bin/env bash
# Forty real compiler questions against an explicit authenticated v2 snapshot.
# One cold indexed build and one unchanged build precede the timed requests.
set -euo pipefail
cd "$(dirname "$0")/.."
benchmark_root=$PWD
if [[ ${1-} == --cleanup ]]; then
    (($# == 2)) || exit 2
    abandoned=$(realpath -e "$2")
    [[ $abandoned == "$benchmark_root"/target/compiler-query-v2.* && ${abandoned#"$benchmark_root/target/"} != */* && ! -L $2 ]] || exit 2
    [[ -f $abandoned/owner-repository && $(cat "$abandoned/owner-repository") == "$benchmark_root" ]] || exit 2
    owner_pid=$(cat "$abandoned/owner-pid")
    [[ $owner_pid =~ ^[1-9][0-9]*$ ]] || exit 2
    if kill -0 "$owner_pid" 2>/dev/null; then echo 'benchmark owner is alive; cleanup refused' >&2; exit 2; fi
    if ps -eo pid=,args= | awk -v run="$abandoned" -v self="$$" -v parent="$PPID" '$1!=self && $1!=parent && index($0,run)>0 && $0!~/awk -v run=/ {found=1} END {exit !found}'; then
        echo 'a command still refers to the benchmark tree; cleanup refused' >&2; exit 2
    fi
    evidence="$benchmark_root/target/${abandoned##*/}.recovery"
    mkdir "$evidence"
    cp "$abandoned/checkpoint" "$evidence/checkpoint"
    rm -rf -- "$abandoned"
    printf 'removed interrupted tree; checkpoint=%s; timed logs remain beside results\n' "$evidence"
    exit 0
fi
compiler=target/lkjscript
revision=20d5f7e98fe4501d441b3ac357733a8a7ba942e7
expected_producer=
results=target/compiler-query-v2.tsv
while (($#)); do
    case "$1" in
        --compiler|--revision|--producer-sha256|--results)
            (($# >= 2)) || exit 2
            case "$1" in
                --compiler) compiler=$2 ;;
                --revision) revision=$2 ;;
                --producer-sha256) expected_producer=$2 ;;
                --results) results=$2 ;;
            esac
            shift 2 ;;
        *) echo 'usage: bash bench/compiler-query-v2.sh --compiler FILE --revision COMMIT --producer-sha256 SHA256 --results FILE' >&2; exit 2 ;;
    esac
done
[[ $revision =~ ^[0-9a-f]{7,40}$ && $expected_producer =~ ^[0-9a-f]{64}$ ]] || exit 2
revision=$(git rev-parse --verify "$revision^{commit}")
compiler=$(realpath "$compiler")
results=$(realpath -m "$results")
[[ -x $compiler && ! -e $results && ! -e $results.raw && ! -e $results.meta && ! -e $results.build.tsv ]] || exit 2
mkdir -p target "$(dirname "$results")"
free_kib=$(df -Pk target | awk 'NR==2 {print $4}')
if ((free_kib < 4 * 1024 * 1024)); then echo 'E_QUERY_BENCH_SPACE need 4 GiB free' >&2; exit 1; fi
umask 077
run=$(mktemp -d "$benchmark_root/target/compiler-query-v2.XXXXXX")
printf '%s\n' "$benchmark_root" > "$run/owner-repository"
printf '%s\n' "$BASHPID" > "$run/owner-pid"
printf 'phase=prepare\nresults=%s\n' "$results" > "$run/checkpoint"
printf 'run_directory=%s\n' "$run"
trap 'rm -rf -- "$run"' EXIT
trap 'exit 130' HUP INT TERM
mkdir "$run/source" "$run/store" "$results.raw"
git archive "$revision" src library tools tests | tar -x -C "$run/source"
cp "$compiler" "$run/producer"
chmod 700 "$run/producer"
actual_producer=$(sha256sum "$run/producer" | cut -d ' ' -f 1)
[[ $actual_producer == "$expected_producer" ]] || { echo 'E_QUERY_BENCH_PRODUCER copied bytes differ' >&2; exit 1; }
head -c 32 /dev/urandom > "$run/key"
chmod 600 "$run/key"
{
    printf 'revision=%s\nsource_tree=%s\nlibrary_tree=%s\n' "$revision" "$(git rev-parse "$revision:src")" "$(git rev-parse "$revision:library")"
    printf 'producer_sha256=%s\nproducer_bytes=%s\nreader_sha256=%s\n' "$actual_producer" "$(stat -c %s "$run/producer")" "$actual_producer"
    printf 'measured_harness_sha256=%s\n' "$(sha256sum "$benchmark_root/bench/compiler-query-v2.sh" | cut -d ' ' -f 1)"
    printf 'source_root=%s\nstarted_utc=%s\nfree_kib_before=%s\n' "$run/source" "$(date -u +%FT%TZ)" "$free_kib"
    printf 'index_memory_bytes=6442450944\nquery_memory_bytes=2147483648\nstore_budget_bytes=2147483648\nindex_deadline_s=900\nquery_deadline_s=30\nworkers=1\n'
    printf 'native_inline_limit=0\ndepth=1\nfact_limit=96\nreply_byte_limit=32768\nmetadata_prequeries=0\nunchanged_indexed_builds=1\nfile_cache_reset=false\n'
} > "$results.meta"
printf 'phase\tstatus\tprotocol_valid\twall_ms\tsubprocess_wall_s\tcompiler_ms\tproducer_setup_ms\tsource_read_ms\tload_parse_ms\tresolve_ms\ttype_ms\tbuild_ir_ms\tcheck_body_ms\tsemantic_catalog_ms\tsemantic_index_ms\tstore_io_ms\tuser_s\tkernel_s\trss_kib\timage_bytes\timage_sha256\tstore_bytes\tbuild_reused\tstore_hits\tstore_writes\tir_functions\tir_instructions\tsnapshot_sha256\tsource_sha256\tchecked_ir_sha256\tload_before\tload_after\terrors\n' > "$results.build.tsv"
printf 'phase\tkind\tidentity\tstatus\tprotocol_valid\twall_ms\tsubprocess_wall_s\tcompiler_ms\tsnapshot_auth_ms\tsemantic_query_ms\tproducer_setup_ms\tproducer_bytes_hashed\tanswer_bytes\tfacts\tcontinuation\tpages_read\ttraversal_steps\tuser_s\tkernel_s\trss_kib\tproducer_sha256\tsource_sha256\tchecked_ir_sha256\tsnapshot_sha256\tanswer_sha256\tload_before\tload_after\terrors\n' > "$results"
load() { cut -d ' ' -f 1-3 /proc/loadavg; }
measure() {
    local tag=$1 memory=$2 deadline=$3
    shift 3
    local before after started finished status=0
    before=$(load)
    started=$(date +%s%N)
    printf 'phase=%s\nload_before=%s\nstarted_ns=%s\nresults=%s\n' "$tag" "$before" "$started" "$results" > "$run/checkpoint"
    /usr/bin/time -f '%e %U %S %M' -o "$results.raw/$tag.time" \
        prlimit --as="$memory" -- timeout --kill-after=10s "${deadline}s" "$@" \
        > "$results.raw/$tag.stdout" 2> "$results.raw/$tag.stderr" || status=$?
    finished=$(date +%s%N)
    after=$(load)
    printf '%s\t%s\t%s\t%s\t%s\n' "$status" "$started" "$finished" "$before" "$after" > "$results.raw/$tag.capture"
}
build_common=(build src/main.lkj --source-dir "$run/source" --library-dir "$run/source/library"
    --runtime-source "$run/source/library/runtime.lkj" --store "$run/store" --store-key "$run/key"
    --store-bytes 2147483648 --index --inline-limit 0 --stats -o "$run/image")
for phase in cold-index unchanged-index; do
    measure "$phase" 6442450944 900 "$run/producer" "${build_common[@]}"
    perl - "$phase" "$results" "$run" <<'PERL' >> "$results.build.tsv"
use strict; use warnings; use JSON::PP; use Digest::SHA qw(sha256_hex);
my ($phase,$results,$run)=@ARGV;
sub read_text { open my $f,'<:raw',shift or die $!; local $/; return <$f>; }
my ($status,$start,$finish,$before,$after)=split /\t/,read_text("$results.raw/$phase.capture");
chomp $after; $before=~s/ /\//g; $after=~s/ /\//g;
my $out=read_text("$results.raw/$phase.stdout"); my $err=read_text("$results.raw/$phase.stderr");
my ($json)=$err=~/LKJSCRIPT_STATS (\{[^\n]*\})/; my $stats=defined($json)?decode_json($json):{};
my ($wall,$user,$kernel,$rss)=read_text("$results.raw/$phase.time")=~/([\d.]+) ([\d.]+) ([\d.]+) (\d+)\n?\z/;
my ($snapshot,$source,$ir)=$out=~/LKJSCRIPT_SNAPSHOT ([0-9a-f]{64}) source=([0-9a-f]{64}) checked-ir=([0-9a-f]{64})/;
my @errors; push @errors,"exit=$status" if $status; push @errors,"missing stats" unless defined $json;
push @errors,"missing timing" unless defined $rss; push @errors,"missing snapshot" unless defined $ir;
my ($image_bytes,$image_hash)=('NA','NA');
if (-f "$run/image") { my $image=read_text("$run/image"); $image_bytes=length($image); $image_hash=sha256_hex($image); }
else { push @errors,"missing image"; }
if ($phase eq 'cold-index' && !@errors) {
    open my $pins,'>',"$run/pins" or die $!; print $pins join("\t",$snapshot,$source,$ir,$image_hash),"\n"; close $pins;
} elsif ($phase eq 'unchanged-index') {
    my @pins=split /\t/,read_text("$run/pins"); chomp $pins[-1];
    push @errors,"unchanged identity mismatch" unless defined($ir) && join("\t",$snapshot,$source,$ir,$image_hash) eq join("\t",@pins);
    push @errors,"unchanged build was not reused" unless ($stats->{build_reused}//0)==1;
    push @errors,"unchanged build wrote store" unless exists($stats->{store_writes}) && $stats->{store_writes}==0;
}
my $store_bytes='NA';
use File::Find;
if ($phase eq 'cold-index') {
    $store_bytes=0;
    find({no_chdir=>1,wanted=>sub { $store_bytes+=-s $_ if -f $_ && !-l $_; }},"$run/store");
}
sub value { my ($key)=@_; return exists($stats->{$key})?$stats->{$key}:'NA'; }
print join("\t",$phase,$status,@errors?'no':'yes',sprintf("%.3f",($finish-$start)/1e6),$wall//'NA',
    map({value($_)} qw(total_ms producer_hash_ms source_read_ms load_parse_ms resolve_ms type_ms build_ir_ms check_body_ms semantic_catalog_ms semantic_index_ms store_io_ms)),
    $user//'NA',$kernel//'NA',$rss//'NA',$image_bytes,$image_hash,$store_bytes,
    map({value($_)} qw(build_reused store_hits store_writes ir_functions ir_instructions)),
    $snapshot//'NA',$source//'NA',$ir//'NA',$before,$after,encode_json(\@errors)),"\n";
if (@errors) { print STDERR join("; ",@errors),"\n",$err; exit 1; }
PERL
    tail -n 1 "$results.build.tsv"
done
IFS=$'\t' read -r snapshot source_digest checked_ir image_digest < "$run/pins"
printf 'snapshot_sha256=%s\nsource_sha256=%s\nchecked_ir_sha256=%s\nimage_sha256=%s\n' "$snapshot" "$source_digest" "$checked_ir" "$image_digest" >> "$results.meta"
query_common=(query src/main.lkj --source-dir "$run/source" --library-dir "$run/source/library"
    --store "$run/store" --store-key "$run/key" --store-bytes 2147483648
    --snapshot "$snapshot" --depth 1 --limit 96 --bytes 32768 --stats)
ordinal=0
while IFS=$'\t' read -r question identity; do
    for phase in question slice; do
        kind=$question
        if [[ $phase == slice ]]; then kind=slice; fi
        ordinal=$((ordinal + 1))
        tag=$(printf 'query-%02d' "$ordinal")
        measure "$tag" 2147483648 30 "$run/producer" "${query_common[@]}" --kind "$kind" --id "$identity"
        perl - "$phase" "$kind" "$identity" "$tag" "$results" "$actual_producer" "$source_digest" "$checked_ir" "$snapshot" <<'PERL' >> "$results"
use strict; use warnings; use JSON::PP; use Digest::SHA qw(sha256_hex);
my ($phase,$kind,$identity,$tag,$results,$expected_producer,$expected_source,$expected_ir,$expected_snapshot)=@ARGV;
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
    && $header=~/\bselection=explicit$/ && $header=~/\bkind=\Q$kind\E depth=1 /;
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
sub value { my ($key)=@_; return exists($stats->{$key})?$stats->{$key}:'NA'; }
print join("\t",$phase,$kind,encode_json($identity),$status,@errors?'no':'yes',sprintf("%.3f",($finish-$start)/1e6),$wall//'NA',
    map({value($_)} qw(total_ms snapshot_auth_ms semantic_query_ms producer_hash_ms producer_bytes_hashed)),
    length($answer),$facts//'NA',defined($cursor)?($cursor eq 'none'?'none':'available'):'NA',$pages//'NA',$steps//'NA',
    $user//'NA',$kernel//'NA',$rss//'NA',$producer//'NA',$source//'NA',$ir//'NA',$snapshot//'NA',sha256_hex($answer),
    $before,$after,encode_json(\@errors)),"\n";
if (@errors) { print STDERR "$tag $identity: ",join("; ",@errors),"\n",$err; exit 1; }
PERL
        tail -n 1 "$results"
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
[[ $ordinal == 40 && $(sha256sum "$run/producer" | cut -d ' ' -f 1) == "$actual_producer" ]] || exit 1
printf 'store_bytes_after_queries=%s\n' "$(find "$run/store" -type f -printf '%s\n' | awk '{n+=$1} END {printf "%.0f",n}')" >> "$results.meta"
printf 'completed_utc=%s\nqueries=40\n' "$(date -u +%FT%TZ)" >> "$results.meta"
printf 'completed forty requests; results=%s\n' "$results"
