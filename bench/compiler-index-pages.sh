#!/usr/bin/env bash
# Fixed two-producer index experiment; controls are compiler-index-pages-plan.md.
set -euo pipefail
export LC_ALL=C
cd "$(dirname "$0")/.."
root=$PWD
baseline=$(realpath "${1:-target/compiler-index-baseline}")
candidate=$(realpath "${2:-target/compiler-index-candidate}")
evidence=${3:-target/compiler-index-pages-measurement}
[[ ! -e $evidence ]] || { printf 'refusing existing evidence directory: %s\n' "$evidence" >&2; exit 2; }
mkdir -p "$evidence"
evidence=$(realpath "$evidence")
budget=2147483648
free=$(df -Pk "$root/target" | awk 'NR==2 {printf "%.0f",$4*1024}')
printf 'free_bytes=%s required_bytes=%s\n' "$free" "$((4*budget+536870912))" > "$evidence/preflight"
((free >= 4*budget+536870912)) || { echo 'free-space refusal' >&2; exit 1; }
fixture=$(mktemp -d "$root/target/compiler-index-pages-run.XXXXXX")
printf '%s\n' "$fixture" > "$evidence/fixture.path"
printf '%s\n' "$root" > "$fixture/owner-repository"
printf '%s\n' "$BASHPID" > "$fixture/owner-pid"
printf '%s\n' "$evidence" > "$fixture/owner-evidence"
trap 'status=$?; printf "supervisor_status=%s\n" "$status" >> "$evidence/completion"; exit "$status"' EXIT
trap 'exit 130' HUP INT TERM
umask 077
key_directory=$(mktemp -d /tmp/compiler-index-pages-key.XXXXXX)
key="$key_directory/key"
printf '%s\n' "$fixture" > "$key_directory/owner-run"
printf '%s\n' "$key_directory" > "$fixture/owner-key"
head -c 32 /dev/urandom > "$key"
a=$(sha256sum "$baseline" | awk '{print $1}')
b=$(sha256sum "$candidate" | awk '{print $1}')
[[ $a == 71b2fc066cd74e9e37cb3134173903c9b322b83472b9293f1a49df7c1d18f3b7 ]]
[[ $b == 40464dda053795b93dbba1af93e41633b915b2b625b0007a7b95ec4d22f0f200 ]]
printf 'baseline=%s\ncandidate=%s\nshape_revision=0b30ca8987bd4ba343ab45ab01751b6721c7151a\nseed=20261116\nmemory_limit_bytes=4294967296\ntimeout_seconds=900\nstore_budget_bytes=%s\n' "$a" "$b" "$budget" > "$evidence/protocol.meta"
sha256sum bench/compiler-index-pages-plan.md bench/compiler-index-pages.sh bench/compiler-index-pages-compare.pl tools/semantic-index-page-test.sh src/semantic_store.lkj > "$evidence/implementation.sha256"
load() { awk '{printf "%s,%s,%s",$1,$2,$3}' /proc/loadavg; }
measure() {
    local label=$1; shift
    local before after status=0
    before=$(load)
    printf '%q ' "$@" > "$evidence/$label.command"
    printf '\n' >> "$evidence/$label.command"
    printf 'load_before=%s\n' "$before" > "$evidence/$label.meta"
    /usr/bin/time -f '%e\t%U\t%S\t%M\t%x' -o "$evidence/$label.time" \
        prlimit --as=4294967296 -- timeout --kill-after=10s 900 "$@" > "$evidence/$label.out" 2> "$evidence/$label.err" || status=$?
    after=$(load)
    printf 'load_after=%s\nstatus=%s\n' "$after" "$status" >> "$evidence/$label.meta"
    printf '%s\t%s\t%s\t%s\n' "$label" "$(cat "$evidence/$label.time")" "$before" "$after" >> "$evidence/timings.tsv"
    printf '%s status=%s time=%s load=%s->%s\n' "$label" "$status" "$(cat "$evidence/$label.time")" "$before" "$after"
    ((status == 0)) || { tail -n 15 "$evidence/$label.err" >&2; return "$status"; }
}
printf 'phase\twall_s\tuser_s\tkernel_s\trss_kib\texit\tload_before\tload_after\n' > "$evidence/timings.tsv"
mkdir "$fixture/frozen"
git archive 0b30ca8987bd4ba343ab45ab01751b6721c7151a src library | tar -x -C "$fixture/frozen"
cp -a src "$fixture/analysis"
cp bench/scale.lkj "$fixture/analysis/main.lkj"
measure generator "$baseline" build analysis/main.lkj --source-dir "$fixture" --no-store --inline-limit 0 -o "$fixture/generator"
measure census "$fixture/generator" shape "$fixture/frozen"
cp "$evidence/census.out" "$evidence/shape"
[[ $(sha256sum "$evidence/shape" | awk '{print $1}') == 0457f8f9424882d2e3160a568f280193d49b9b046a5fae2d6e2d0b6da047072c ]]
for rung in 10000 100000; do
    tree="$fixture/source-$rung"
    mkdir "$tree"
    measure "$rung-generate" "$fixture/generator" generate "$tree" "$rung" 20261116 < "$evidence/shape"
    (cd "$tree" && find . -type f -name '*.lkj' -print0 | sort -z | xargs -0 sha256sum) > "$evidence/$rung-source.sha256"
    for mode in baseline candidate; do
        if [[ $mode == baseline ]]; then compiler=$baseline; else compiler=$candidate; fi
        store="$fixture/store-$rung-$mode"
        mkdir "$store"
        common=(main.lkj --source-dir "$tree" --library-dir "$fixture/frozen/library" --runtime-source "$fixture/frozen/library/runtime.lkj" --store "$store" --store-key "$key" --store-bytes "$budget" --inline-limit 0 --stats -o "$fixture/image-$rung-$mode")
        measure "$rung-$mode-cold" "$compiler" build "${common[@]}"
        measure "$rung-$mode-index" "$compiler" build "${common[@]}" --index
        snapshot=$(sed -n 's/^LKJSCRIPT_SNAPSHOT \([0-9a-f]\{64\}\) .*/\1/p' "$evidence/$rung-$mode-index.out")
        [[ ${#snapshot} == 64 ]]
        printf '%s\n' "$snapshot" > "$evidence/$rung-$mode.snapshot"
        find "$store" -maxdepth 1 -type f -printf '%s\n' | awk '{sum+=$1;n++} END{printf "files=%d\nstored_bytes=%.0f\n",n,sum}' > "$evidence/$rung-$mode-store.meta"
        sha256sum "$fixture/image-$rung-$mode" > "$evidence/$rung-$mode-image.sha256"
    done
    cmp "$fixture/image-$rung-baseline" "$fixture/image-$rung-candidate"
    sa=$(cat "$evidence/$rung-baseline.snapshot"); sb=$(cat "$evidence/$rung-candidate.snapshot")
    measure "$rung-payload-oracle" perl bench/compiler-index-pages-compare.pl "$fixture/store-$rung-baseline" "$fixture/store-$rung-candidate" "$key" "$a" "$b" "$sa" "$sb"
    kinds=(outline signature text slice uses callers instances authority type layout)
    ids=(edit edit::target edit::target edit::target edit::target edit::target edit::target edit::target edit::target u64)
    for index in "${!kinds[@]}"; do
        kind=${kinds[index]}; identity=${ids[index]}
        for mode in baseline candidate; do
            if [[ $mode == baseline ]]; then compiler=$baseline; snapshot=$sa; else compiler=$candidate; snapshot=$sb; fi
            prlimit --as=4294967296 -- timeout --kill-after=10s 900 "$compiler" query main.lkj --source-dir "$tree" --library-dir "$fixture/frozen/library" \
                --store "$fixture/store-$rung-$mode" --store-key "$key" --store-bytes "$budget" \
                --snapshot "$snapshot" --kind "$kind" --id "$identity" --depth 1 --limit 96 --bytes 32768 \
                > "$evidence/$rung-$mode-query-$kind.out" 2> "$evidence/$rung-$mode-query-$kind.err"
        done
        perl - "$evidence/$rung-baseline-query-$kind.out" "$evidence/$rung-candidate-query-$kind.out" "$a" "$b" "$sa" "$sb" <<'PERL'
use strict; use warnings;
my($left,$right,$a,$b,$sa,$sb)=@ARGV;
sub normalized {
 my($path,$producer,$snapshot)=@_; open my $f,'<:raw',$path or die $!;local $/;my $out=<$f>;
 die "query byte bound\n" if length($out)>32768;
 die "query incomplete\n" unless $out=~/\tfact=continuation\tcursor=none /;
 my($header,$body)=split /\n/,$out,2;
 die "query provenance\n" unless $header=~/\tlevel=protocol\tfact=query\tversion=2 producer=\Q$producer\E validation=authenticated-producing-compiler current-check=not-performed snapshot=\Q$snapshot\E / && $header=~/ freshness=not-asserted / && $header=~/ selection=explicit$/;
 $header=~s/producer=\Q$producer\E /producer=VERIFIED_PRODUCER /;
 $header=~s/snapshot=\Q$snapshot\E /snapshot=VERIFIED_SNAPSHOT /;
 return "$header\n$body";
}
die "query bytes differ beyond producer/snapshot\n" unless normalized($left,$a,$sa) eq normalized($right,$b,$sb);
PERL
        printf '%s\t%s\t%s\texact-except-verified-producer-and-snapshot\n' "$rung" "$kind" "$identity" >> "$evidence/query-parity.tsv"
    done
done
printf 'completed=true\n' >> "$evidence/completion"
