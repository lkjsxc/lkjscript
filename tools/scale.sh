#!/usr/bin/env bash
# Native scale generator and ordinary wall/RSS measurement; never a pass/fail budget.
set -euo pipefail
cd "$(dirname "$0")/.."
binary=${1:-target/lkjscript}
if (($#)); then shift; fi
sizes=8000,16000,32000
families=funcs,ifs,match,boxes,vecs
repeats=3
verb=check
engine=native
out=target/scale
while (($#)); do
    option=$1
    shift
    if (($# == 0)); then printf 'missing value for %s\n' "$option" >&2; exit 2; fi
    case "$option" in
        --sizes) sizes=$1;;
        --families) families=$1;;
        --repeat) repeats=$1;;
        --verb) verb=$1;;
        --engine) engine=$1;;
        --out) out=$1;;
        *) printf 'unknown option: %s\n' "$option" >&2; exit 2;;
    esac
    shift
done
[[ $repeats =~ ^[1-9][0-9]*$ ]] || { printf 'repeat must be positive\n' >&2; exit 2; }
case "$verb" in check|build|run) ;; *) printf 'verb must be check, build or run\n' >&2; exit 2;; esac
mkdir -p "$out"
measure="$out/.measure-$$"
trap 'rm -f "$measure.time" "$measure.stderr" "$measure.samples"' EXIT
IFS=, read -r -a family_list <<< "$families"
IFS=, read -r -a size_list <<< "$sizes"
printf 'family size source_bytes median_seconds median_rss_kib emitted_bytes\n'
for family in "${family_list[@]}"; do
    for size in "${size_list[@]}"; do
        [[ $size =~ ^[1-9][0-9]*$ ]] || { printf 'size must be positive\n' >&2; exit 2; }
        "$binary" generate "scale-$family" 0 "$size" -o "$out" >/dev/null
        source="$out/$family-$size.lkj"
        command=("$binary" "$verb" "$source" --stats)
        if [[ $verb == run ]]; then command+=(--entry probe --engine "$engine"); fi
        if [[ $verb == build ]]; then command+=(-o "$out/$family-$size.elf"); fi
        : > "$measure.samples"
        for ((repeat=0; repeat<repeats; repeat++)); do
            if ! /usr/bin/time -f '%e %M' -o "$measure.time" "${command[@]}" >/dev/null 2>"$measure.stderr"; then
                cat "$measure.stderr" >&2
                exit 1
            fi
            cat "$measure.time" >> "$measure.samples"
        done
        median=$((repeats / 2 + 1))
        seconds=$(sort -n -k1,1 "$measure.samples" | awk -v line="$median" 'NR==line {print $1}')
        rss=$(sort -n -k2,2 "$measure.samples" | awk -v line="$median" 'NR==line {print $2}')
        emitted=0
        if [[ $verb == build ]]; then emitted=$(wc -c < "$out/$family-$size.elf"); fi
        printf '%s %s %s %s %s %s\n' "$family" "$size" "$(wc -c < "$source")" "$seconds" "$rss" "$emitted"
        # Keep the compiler's actual phase statistics visible, from the final run.
        cat "$measure.stderr" >&2
    done
done
