#!/usr/bin/env bash
# Separately released continuation: only the fifty never-executed questions.
# No build, prequery, repair, map walk, census, retry, or cleanup.
set -euo pipefail
cd "$(dirname "$0")/.."
benchmark_root=$(pwd -P)
checkpoint= results=
while (($#)); do
    case "$1" in
        --checkpoint) (($# >= 2)) || exit 2; checkpoint=$2; shift 2 ;;
        --results) (($# >= 2)) || exit 2; results=$2; shift 2 ;;
        *) echo 'usage: compiler-query-current-continue.sh --checkpoint COMMITTED_JSON --results DECLARED_NEW_DIRECTORY' >&2; exit 2 ;;
    esac
done
[[ -n $checkpoint && -n $results ]] || exit 2
checkpoint=$(realpath -e "$checkpoint")
results=$(realpath -m "$results")
[[ $checkpoint == "$benchmark_root/bench/compiler-query-current-continuation-62fa76f4.json" ]] || exit 2
[[ -f $checkpoint && ! -L $checkpoint && ! -e $results && ! -L $results ]] || exit 2
[[ $results == "$benchmark_root/target/compiler-query-current-continuation-62fa76f4" ]] || exit 2
validator="$benchmark_root/bench/compiler-query-current-continuation-validate.pl"
questions="$benchmark_root/bench/compiler-query-current-questions.tsv"
umask 077
mkdir "$results" "$results/raw"
fixture= keydir= original= imported= snapshot=
phase=preflight
finish() {
    local status=$?
    trap - EXIT
    printf 'phase=%s\nstatus=%s\nfixture=%s\nkey_directory=%s\noriginal_attempt=%s\nfinished_utc=%s\n' \
        "$phase" "$status" "$fixture" "$keydir" "$original" "$(date -u +%FT%TZ)" > "$results/completion"
    printf 'retained continuation=%s fixture=%s key_directory=%s status=%s\n' "$results" "$fixture" "$keydir" "$status"
    exit "$status"
}
trap finish EXIT
trap 'exit 130' HUP INT TERM
cp "$checkpoint" "$results/checkpoint.submitted.json"
printf '%s\n' "$benchmark_root" > "$results/owner-repository"
printf '%s\n' "$BASHPID" > "$results/owner-pid"
git ls-files --error-unmatch bench/compiler-query-current-continuation-62fa76f4.json \
    bench/compiler-query-current-continue.sh bench/compiler-query-current-continuation-validate.pl > "$results/tracked-controls"
git diff --quiet HEAD -- bench/compiler-query-current-continuation-62fa76f4.json \
    bench/compiler-query-current-continue.sh bench/compiler-query-current-continuation-validate.pl \
    bench/compiler-query-current-questions.tsv
perl -MJSON::PP -MDigest::SHA - "$checkpoint" "$benchmark_root" "$results" > "$results/bindings" 2> "$results/preparation.stderr" <<'PERL'
use strict;
use warnings;
sub raw {
    my($p)=@_;
    -f $p && !-l $p or die "regular record required: $p\n";
    open my $f,'<:raw',$p or die "$p: $!\n";
    local $/;return <$f>;
}
sub object { my $v=decode_json(raw($_[0]));ref($v) eq 'HASH' or die "object required\n";return $v; }
sub digest {
    my($p)=@_;
    -f $p && !-l $p or die "regular digest input required\n";
    open my $f,'<:raw',$p or die "$p: $!\n";
    return Digest::SHA->new(256)->addfile($f)->hexdigest;
}
sub equal { defined($_[0]) && defined($_[1]) && !ref($_[0]) && !ref($_[1]) && $_[0] eq $_[1] or die "$_[2]\n"; }
my($path,$repo,$results)=@ARGV;
my $c=object($path);
equal($c->{version},1,'checkpoint version');
equal($c->{protocol},'lkjscript-current50-checkpoint-continuation-1','checkpoint protocol');
equal($c->{results},$results,'single declared result destination');
equal($c->{repository},$repo,'owned repository differs');
equal($c->{prior_query_launches},0,'prior query count differs');
equal($c->{query_launches},50,'query count differs');
for my $r(@{$c->{records}}) {
    ref($r) eq 'HASH' && ($r->{sha256}//'')=~/\A[0-9a-f]{64}\z/ or die "record shape\n";
    my $relative=$r->{path}//'';
    $relative=~m{\Abench/[A-Za-z0-9_./-]+\z} && $relative!~m{(?:\A|/)\.\.(?:/|\z)} or die "record path\n";
    equal(digest("$repo/$relative"),$r->{sha256},"changed record $relative");
}
my $original=$c->{original_evidence};
my $frozen="$repo/$c->{imported_attempt}";
my $manifest=raw("$repo/$c->{imported_attempt_manifest}");
for my $line(split /\n/,$manifest) {
    $line=~/\A([0-9a-f]{64})  (.+)\z/ or die "attempt manifest framing\n";
    my($hash,$record)=($1,$2);
    my $prefix="$c->{imported_attempt}/";
    index($record,$prefix)==0 or die "attempt manifest path\n";
    my $suffix=substr($record,length($prefix));
    equal(digest("$repo/$record"),$hash,"frozen attempt changed");
    equal(digest("$original/$suffix"),$hash,"live attempt changed");
}
my @prior_queries=glob("$original/raw/query-*");
!@prior_queries && !-e "$original/timed-completion" or die "questions already started\n";
raw("$original/completion")=~/\Aphase=unchanged-index\nstatus=1\n/ or die "original stop differs\n";
my $imported="$repo/$c->{validated_directory}";
my $builds=object("$imported/builds.json");
$builds->{valid} && ref($builds->{rows}) eq 'ARRAY' && @{$builds->{rows}}==2 or die "two accepted imported builds required\n";
my $pin=object("$imported/pins.json");
for my $key(qw(snapshot source checked_ir producer image_sha256)) { equal($pin->{$key},$c->{pins}{$key},"pin $key differs"); }
my $fixture=$c->{fixture};
my $keydir=$c->{key_directory};
for my $v($fixture,$keydir,$original,$imported,$pin->{snapshot}) { defined($v) && !ref($v) && $v!~/[\r\n\0]/ or die "invalid binding\n"; }
equal(raw("$fixture/owner-evidence"),"$original\n",'fixture evidence ownership');
equal(raw("$fixture/owner-repository"),"$repo\n",'fixture repository ownership');
equal(raw("$fixture/owner-key"),"$keydir\n",'fixture key ownership');
equal(raw("$keydir/owner-run"),"$fixture\n",'key run ownership');
-f "$keydir/key" && !-l "$keydir/key" && -s "$keydir/key"==32 or die "private key unavailable\n";
equal(digest("$fixture/bin/producer"),$c->{pins}{producer},'producer differs');
equal(-s "$fixture/bin/producer",$c->{producer_bytes},'producer bytes differ');
for my $name(qw(cold-index unchanged-index)) {
    equal(digest("$fixture/output/$name.image"),$c->{pins}{producer},"$name image differs");
    equal(-s "$fixture/output/$name.image",$c->{producer_bytes},"$name image size differs");
}
print "$fixture\n$keydir\n$pin->{snapshot}\n$original\n$imported\n";
PERL
{
    read -r fixture
    read -r keydir
    read -r snapshot
    read -r original
    read -r imported
} < "$results/bindings"
source_root="$fixture/source"
key="$keydir/key"
[[ -d $source_root && ! -L $source_root && -d $fixture/store && ! -L $fixture/store ]] || exit 1
case "$key" in "$source_root"/*) echo 'key entered source grant' >&2; exit 1 ;; esac
perl "$validator" questions "$questions" > "$results/questions-validation.stdout" 2> "$results/questions-validation.stderr"
perl "$validator" identity "$original/identities.json" "$benchmark_root" "$fixture/bin/producer" > "$results/identities.json" 2> "$results/identity-validation.stderr"
cp "$imported/pins.json" "$results/pins.json"
cp "$imported/builds.json" "$results/imported-builds.json"
cp "$questions" "$results/requests.tsv"
cp "$original/inputs-before.sha256" "$results/imported-inputs-before.sha256"
cmp "$fixture/bin/producer" "$fixture/output/cold-index.image"
cmp "$fixture/bin/producer" "$fixture/output/unchanged-index.image"
printf 'offline cmp: cold=unchanged=gated canonical; no build repeated\n' > "$results/imported-image-parity"
sha256sum "$checkpoint" "$validator" "$questions" "$benchmark_root/bench/compiler-query-current-continue.sh" > "$results/control-inputs.sha256"
{
    printf 'protocol=lkjscript-current50-checkpoint-continuation-1\n'
    printf 'original_attempt=%s\nfixture=%s\nsource_root=%s\nsource_grant=.\n' "$original" "$fixture" "$source_root"
    printf 'snapshot=%s\nbuilds_repeated=0\nprior_query_launches=0\nquery_launches_planned=50\n' "$snapshot"
    printf 'warm_sequence=cold_index,unchanged_index,validation_stop,offline_inspection_pause,first_fifty_queries\n'
    printf 'pause_and_cache_state_change=true\nmetadata_prequeries=0\nmap_walks=0\ncensus=0\nretries=0\nfile_cache_reset=false\n'
    printf 'query_memory_bytes=2147483648\nquery_deadline_s=30\nkill_grace_s=10\nquery_reader_cap=default_per_extent\n'
    printf 'wrapper_includes=subprocess_resource_launch_and_stdout_stderr_capture\nwrapper_excludes=prior_command_cwd_load_metadata_and_subsequent_hash_validation\n'
    printf 'historical_v2_causal_latency_comparison=false\nstarted_utc=%s\n' "$(date -u +%FT%TZ)"
} > "$results/protocol.meta"
load() { cut -d ' ' -f 1-3 /proc/loadavg | tr ' ' ','; }
measure() {
    local tag=$1 memory=$2 deadline=$3
    shift 3
    local before after started finished status=0
    printf '%s\n' "$source_root" > "$results/raw/$tag.cwd"
    perl -MJSON::PP -e 'print JSON::PP->new->canonical->encode(\@ARGV),"\n"' "$@" > "$results/raw/$tag.argv.json"
    before=$(load)
    printf 'phase=%s\ntag=%s\nload_before=%s\n' "$phase" "$tag" "$before" > "$results/checkpoint"
    started=$(date +%s%N)
    (
        cd "$source_root"
        /usr/bin/time -f '%e %U %S %M' -o "$results/raw/$tag.time" \
            prlimit --as="$memory" -- timeout --kill-after=10s "${deadline}s" "$@"
    ) > "$results/raw/$tag.stdout" 2> "$results/raw/$tag.stderr" || status=$?
    finished=$(date +%s%N)
    after=$(load)
    perl -MJSON::PP -e 'my($status,$start,$finish,$before,$after)=@ARGV;print JSON::PP->new->canonical->encode({status=>0+$status,start_ns=>0+$start,finish_ns=>0+$finish,wrapper_wall_ns=>$finish-$start,load_before=>$before,load_after=>$after}),"\n"' \
        "$status" "$started" "$finished" "$before" "$after" > "$results/raw/$tag.capture.json"
    printf '%s\t%s\t%s\t%s\t%s\n' "$tag" "$status" "$((finished-started))" "$before" "$after" >> "$results/phase-order.tsv"
    return "$status"
}
printf 'tag\tstatus\twrapper_wall_ns\tload_before\tload_after\n' > "$results/phase-order.tsv"
query_argv=(query src/main.lkj --library-dir library --store "$fixture/store" --store-key "$key" --snapshot "$snapshot" --stats)
phase=queries
count=0
while IFS=$'\t' read -r ordinal group kind identity depth limit bytes; do
    [[ $ordinal == ordinal ]] && continue
    count=$((count + 1))
    [[ $ordinal == "$count" ]] || exit 1
    tag=$(printf 'query-%02d' "$ordinal")
    # A refusal remains a row; no pin repair, rerun or substitute is permitted.
    measure "$tag" 2147483648 30 env -u LKJSCRIPT_STORE_IO "$fixture/bin/producer" \
        "${query_argv[@]}" --kind "$kind" --id "$identity" --depth "$depth" --limit "$limit" --bytes "$bytes" || true
done < "$results/requests.tsv"
[[ $count == 50 ]] || exit 1
printf 'timed_queries=50\ncompleted_utc=%s\n' "$(date -u +%FT%TZ)" > "$results/timed-completion"
phase=postprocessing
validation=0
perl "$validator" queries "$results" "$results/requests.tsv" > "$results/query-validation.stdout" 2> "$results/query-validation.stderr" || validation=$?
(
    cd "$source_root"
    find src library tools tests -type f -print0 | sort -z | xargs -0 sha256sum
) > "$results/inputs-after.sha256"
sha256sum "$fixture/bin/producer" > "$results/producer-after.sha256"
LC_ALL=C sort "$original/inputs-before.sha256" > "$results/inputs-before.canonical.sha256"
LC_ALL=C sort "$results/inputs-after.sha256" > "$results/inputs-after.canonical.sha256"
cmp "$results/inputs-before.canonical.sha256" "$results/inputs-after.canonical.sha256"
cmp "$original/producer-before.sha256" "$results/producer-after.sha256"
printf 'frozen source path/digest mappings and executable identity unchanged\n' > "$results/final-inventory-parity"
phase=complete
exit "$validation"
