#!/bin/sh
# Measurements are observations, never admission limits.
set -eu
cd "$(dirname "$0")/.."
root=$(pwd)
compiler=target/lkjscript
witness=
witness0=
inline_limit=
previous=
output=target/bench
rust=no
while [ "$#" -gt 0 ]; do
    case "$1" in
        --compiler|--witness|--witness0|--previous|--out|--inline-limit)
            [ "$#" -ge 2 ] || { echo "missing value for $1" >&2; exit 2; }
            case "$1" in
                --compiler) compiler=$2 ;;
                --witness) witness=$2; rust=yes ;;
                --witness0) witness0=$2; rust=yes ;;
                --previous) previous=$2 ;;
                --out) output=$2 ;;
                --inline-limit) inline_limit=$2 ;;
            esac
            shift 2 ;;
        --rust) rust=yes; shift ;;
        *) echo "usage: sh bench/run.sh [--compiler PATH] [--witness PATH] [--witness0 PATH] [--previous PATH] [--inline-limit N] [--rust] [--out PATH]" >&2; exit 2 ;;
    esac
done
mkdir -p "$output"
output=$(cd "$output" && pwd)
rows="$output/measurements.tsv"
: > "$rows"

measure() {
    measured_program=$1
    measured_route=$2
    measured_image=$3
    shift 3
    read -r load1 load5 load15 runnable last_pid < /proc/loadavg
    measured_load="$load1/$load5/$load15"
    /usr/bin/time -f '%e %U %S %M' -o "$output/time" "$@" > "$output/stdout" 2> "$output/stderr"
    if [ -s "$output/stderr" ]; then
        cat "$output/stderr" >&2
        exit 1
    fi
    read -r real user kernel rss < "$output/time"
    bytes=$(wc -c < "$measured_image")
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$measured_program" "$measured_route" "$real" "$user" "$kernel" "$rss" "$bytes" "$measured_load" >> "$rows"
}

# The same explicit pass setting reaches native and same-source witnesses.
set --
if [ -n "$inline_limit" ]; then set -- --inline-limit "$inline_limit"; fi
# This row measures the compiler code which the microbenchmarks cannot represent.
measure self_build native "$compiler" "$compiler" build src/main.lkj \
    --library-dir library --runtime-source library/runtime.lkj --no-store "$@" -o "$output/compiler-native"
if [ -n "$witness0" ]; then
    measure self_build witness0 "$witness0" "$witness0" build src/main.lkj \
        --library-dir library --runtime-source library/runtime.lkj --no-store "$@" -o "$output/compiler-witness0"
    cmp "$output/compiler-native" "$output/compiler-witness0"
fi
if [ -n "$witness" ]; then
    measure self_build witness3 "$witness" "$witness" build src/main.lkj \
        --library-dir library --runtime-source library/runtime.lkj --no-store "$@" -o "$output/compiler-witness"
    cmp "$output/compiler-native" "$output/compiler-witness"
fi
if [ -n "$previous" ]; then
    measure self_build previous "$previous" "$previous" build src/main.lkj \
        --library-dir library --runtime-source library/runtime.lkj --no-store -o "$output/compiler-previous"
fi

host=witness/bridge/output.rs
while read -r program count length seed small_count small_length; do
    [ -n "$program" ] || continue
    source="bench/$program.lkj"
    if [ "$program" = http_parse ]; then
        mkdir -p "$output/http_parse-source"
        cp "$source" "$output/http_parse-source/main.lkj"
        cp bench/common.lkj examples/http_protocol.lkj "$output/http_parse-source/"
        source="$output/http_parse-source/main.lkj"
    fi
    routes=native
    if [ -n "$inline_limit" ]; then
        "$compiler" build "$source" --inline-limit "$inline_limit" -o "$output/$program-native"
    else
        "$compiler" build "$source" -o "$output/$program-native"
    fi
    if [ -n "$previous" ]; then
        "$previous" build "$source" -o "$output/$program-previous"
        routes="$routes previous"
    fi
    if [ "$rust" = yes ]; then
        {
            printf '#[path="%s/%s"] mod __lkjscript_host;\nmod case {\n' "$root" "$host"
            sed -e "s|^mod common;$|#[path=\"$root/bench/common.lkj\"] mod common;|" \
                -e "s|^mod http_protocol;$|#[path=\"$root/examples/http_protocol.lkj\"] mod http_protocol;|" "$source"
            printf '\nuse crate::__lkjscript_host::World;\n}\nfn main(){__lkjscript_host::enter(case::main);}\n'
        } > "$output/$program-witness.rs"
        for level in 0 3; do
            rustc --edition=2024 --crate-name benchmark -C overflow-checks=yes \
                -C panic=abort -C codegen-units=1 -C "opt-level=$level" -A warnings \
                "$output/$program-witness.rs" -o "$output/$program-rustc$level"
            routes="$routes rustc$level"
        done
    fi
    for size in small measured; do
        first=yes
        for route in $routes; do
            image="$output/$program-$route"
            if [ "$length" = - ]; then
                if [ "$size" = small ]; then set -- "$small_count" "$seed"; else set -- "$count" "$seed"; fi
            else
                if [ "$size" = small ]; then set -- "$small_count" "$small_length" "$seed"; else set -- "$count" "$length" "$seed"; fi
            fi
            if [ "$size" = small ]; then
                "$image" "$@" > "$output/stdout" 2> "$output/stderr"
                [ ! -s "$output/stderr" ] || { cat "$output/stderr" >&2; exit 1; }
            else
                measure "$program" "$route" "$image" "$image" "$@"
            fi
            if [ "$first" = yes ]; then
                cp "$output/stdout" "$output/reference"
                first=no
            else
                cmp "$output/reference" "$output/stdout"
            fi
        done
    done
done <<'WORKLOADS'
aggregate64 10000000 - 7 13 -
vector64 10000000 1000000 7 127 19
aggregate 20000000 - 7 13 -
scalar 200000000 - 7 13 -
scalar_add 200000000 - 7 13 -
buffers 200000 1000 7 127 19
vector 100000000 5000000 7 127 19
http_parse 200000 - 7 13 -
string_map 500000 4096 7 13 19
WORKLOADS

printf '| Workload / route | Elapsed s | User s | Kernel s | Peak RSS KiB | Executable bytes | Load 1/5/15 min |\n'
printf '| --- | ---: | ---: | ---: | ---: | ---: | --- |\n'
awk -F '\t' '{printf "| %s / %s | %s | %s | %s | %s | %s | %s |\n", $1, $2, $3, $4, $5, $6, $7, $8}' "$rows"
rm -f "$output/time" "$output/stdout" "$output/stderr" "$output/reference"
