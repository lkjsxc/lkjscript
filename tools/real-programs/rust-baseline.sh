#!/usr/bin/env bash
# Original Rust suites, with initial compilation and test-driver phases measured.
# Test profile defaults are preserved except opt-level and codegen-units=1.
# Test and doc phases include Cargo verification and any additional compilation.
set -euo pipefail
if test "$#" -lt 8; then
    printf '%s\n' 'usage: rust-baseline.sh SOURCE WORK EVIDENCE CACHE TOOLCHAIN PROTOC probe|build|test|doc|network-rtt 0|3 [--package NAME] [--reviewed-native-config SHA256]' >&2
    exit 2
fi
source_root=$(realpath "$1")
work_root=$(realpath "$2")
evidence_root=$(realpath "$3")
cache_root=$(realpath "$4")
toolchain_root=$(realpath "$5")
protoc_root=$(realpath "$6")
phase=$7
optimization=$8
shift 8
package_name=
reviewed_config_sha=
while test "$#" -gt 0; do
    case "$1" in
        --package)
            test "$#" -ge 2
            test -z "$package_name"
            package_name=$2
            [[ "$package_name" =~ ^[A-Za-z0-9_-]+$ ]]
            shift 2;;
        --reviewed-native-config)
            test "$#" -ge 2
            test -z "$reviewed_config_sha"
            reviewed_config_sha=$2
            [[ "$reviewed_config_sha" =~ ^[0-9a-f]{64}$ ]]
            shift 2;;
        *) printf 'unknown baseline option: %s\n' "$1" >&2; exit 2;;
    esac
done
cargo_scope=(--workspace)
if test -n "$package_name"; then cargo_scope=(-p "$package_name"); fi
case "$phase" in probe|build|test|doc|network-rtt) ;; *) exit 2;; esac
case "$optimization" in 0|3) ;; *) exit 2;; esac
script_root=$(cd -- "$(dirname -- "$0")" && pwd)
for path in "$source_root" "$work_root" "$evidence_root" "$cache_root" "$toolchain_root" "$protoc_root" "$script_root"; do
    case "$path" in *[!A-Za-z0-9_./-]*) printf 'unsupported path: %s\n' "$path" >&2; exit 2;; esac
done
test -f "$source_root/Cargo.lock"
test -x "$toolchain_root/bin/cargo"
test -x "$toolchain_root/bin/rustc"
test -x "$protoc_root/bin/protoc"
test -d "$cache_root/registry"
# Config files require a digest-pinned review. The one admitted file configures
# only a Wasm runner; native host tests never select that target or runner.
mapfile -d '' cargo_configs < <(find "$source_root" \( -path '*/.cargo/config' -o -path '*/.cargo/config.toml' \) -print0)
if test "${#cargo_configs[@]}" -gt 0; then
    test "${#cargo_configs[@]}" -eq 1
    test "${cargo_configs[0]}" = "$source_root/.cargo/config.toml"
    test "$package_name" = dom_query
    test "$reviewed_config_sha" = c61514b5b0144ce3ff2b63c490f51b38db47d553feaadad3047c83afaf2928af
    test "$(sha256sum "${cargo_configs[0]}" | cut -d ' ' -f1)" = "$reviewed_config_sha"
elif test -n "$reviewed_config_sha"; then
    printf '%s\n' 'reviewed configuration is absent' >&2
    exit 2
fi
# env -i removes target overrides; the default target is this pinned native host.
"$toolchain_root/bin/rustc" -Vv | grep -x 'host: x86_64-unknown-linux-gnu' > /dev/null
git -C "$source_root" diff --quiet HEAD --
if test -n "$(git -C "$source_root" ls-files --others --exclude-standard)"; then
    printf '%s\n' 'specimen has untracked files' >&2
    exit 2
fi
package_commit=$(git -C "$source_root" rev-parse HEAD)
package_lock=$(sha256sum "$source_root/Cargo.lock" | cut -d ' ' -f1)
marker="$work_root/.rust-baseline-owner"
identity=$(printf '%s\n' "$source_root" "$package_commit" "$package_lock" "$toolchain_root" "$protoc_root" "$package_name" "$reviewed_config_sha")
if test -e "$marker"; then
    test "$(cat "$marker")" = "$identity" || { printf '%s\n' 'baseline work identity differs' >&2; exit 2; }
else
    printf '%s\n' "$identity" > "$marker"
fi
run_root="$work_root/opt$optimization"
rootfs="$run_root/root"
mkdir -p "$run_root/upper" "$run_root/overlay-work" "$run_root/target" \
    "$run_root/tmp" "$run_root/cargo-home/registry" \
    "$rootfs/usr" "$rootfs/etc/alternatives" "$rootfs/toolchain" \
    "$rootfs/protoc" "$rootfs/target" "$rootfs/tmp" "$rootfs/cargo-home" \
    "$rootfs/src" "$rootfs/proc" "$rootfs/dev"
for pair in bin:usr/bin sbin:usr/sbin lib:usr/lib lib64:usr/lib64; do
    link_name=$(printf '%s' "$pair" | cut -d: -f1)
    link_target=$(printf '%s' "$pair" | cut -d: -f2)
    test -L "$rootfs/$link_name" || ln -s "$link_target" "$rootfs/$link_name"
done
for device in null zero random urandom; do
    test -f "$rootfs/dev/$device" || : > "$rootfs/dev/$device"
done
for pair in fd:/proc/self/fd stdin:/proc/self/fd/0 stdout:/proc/self/fd/1 stderr:/proc/self/fd/2; do
    link_name=$(printf '%s' "$pair" | cut -d: -f1)
    link_target=$(printf '%s' "$pair" | cut -d: -f2)
    test -L "$rootfs/dev/$link_name" || ln -s "$link_target" "$rootfs/dev/$link_name"
done
: > "$rootfs/etc/ld.so.cache"
printf 'root:x:0:0:baseline:/:/bin/bash\n' > "$rootfs/etc/passwd"
printf 'root:x:0:\n' > "$rootfs/etc/group"
printf '127.0.0.1 localhost\n::1 localhost\n' > "$rootfs/etc/hosts"
printf 'hosts: files\npasswd: files\ngroup: files\n' > "$rootfs/etc/nsswitch.conf"
: > "$rootfs/etc/resolv.conf"
record="$evidence_root/opt$optimization-$phase"
if test -e "$record.meta.json" || test -e "$record.stdout" || test -e "$record.stderr" || test -e "$record.time"; then
    printf 'evidence already exists: %s\n' "$record" >&2
    exit 2
fi
containment_mode=()
network_scope=isolated-loopback
binary_sha=
resolver_sha=
phase_timeout=3600s
if test "$phase" = probe; then
    set -- /bin/bash -c '
set -euo pipefail
cd /src
test -z "$(printenv HOME || true)"
test ! -e /home/coder
test ! -e /root
test ! -e /run/docker.sock
for directory in /usr /toolchain /protoc /cargo-home/registry; do
    if (: > "$directory/.baseline-write-probe") 2>/dev/null; then
        printf "unexpected writable mount: %s\n" "$directory" >&2
        exit 1
    fi
done
: > /src/.baseline-copyup-probe
rm /src/.baseline-copyup-probe
: > /tmp/.baseline-private-probe
rm /tmp/.baseline-private-probe
test -z "$(ip route show)"
test "$(ip -o link show | wc -l)" = 1
ip -o link show dev lo
perl -MSocket -e '"'"'socket(my $s,AF_INET,SOCK_STREAM,0) or die $!; bind($s,sockaddr_in(0,inet_aton("127.0.0.1"))) or die $!; my($port)=sockaddr_in(getsockname($s)); print "loopback-bind=$port\n";'"'"'
grep -E "^(Uid|Gid|CapInh|CapPrm|CapEff|CapBnd|CapAmb|NoNewPrivs):" /proc/self/status
test "$(awk "/^CapEff:/ {print \$2}" /proc/self/status)" = 0000000000000000
test "$(awk "/^CapBnd:/ {print \$2}" /proc/self/status)" = 0000000000000000
/toolchain/bin/rustc -Vv
/toolchain/bin/cargo -V
/protoc/bin/protoc --version
/usr/bin/cc --version | sed -n 1p
printf "containment-probe=passed\n"
'
elif test "$phase" = build; then
    probe_record="$evidence_root/opt$optimization-probe.meta.json"
    test -f "$probe_record" || probe_record="$evidence_root/opt0-probe.meta.json"
    perl -MJSON::PP -e 'local $/; exit(decode_json(<>)->{exit_status} == 0 ? 0 : 1)' "$probe_record"
    set -- /bin/bash -c 'cd /src && exec "$@"' baseline-cargo /toolchain/bin/cargo test "${cargo_scope[@]}" --locked --offline --no-run --verbose
elif test "$phase" = network-rtt; then
    # This is the sole reviewed host-network grant. It never invokes Cargo.
    test "$package_commit" = c5ff06a12bd514e06ab28965052d0ad1e19c45b3
    perl -MJSON::PP -e 'local $/; exit(decode_json(<>)->{exit_status} == 0 ? 0 : 1)' "$evidence_root/opt$optimization-build.meta.json"
    perl -MJSON::PP -e 'local $/; exit(decode_json(<>)->{exit_status} == 101 ? 0 : 1)' "$evidence_root/opt$optimization-test.meta.json"
    selected_binary=$(perl -ne 'if (/Running `(\/target\/debug\/deps\/sozu_lib-[0-9a-f]{16})(?: |\x60)/) {$seen{$1}=1} END {die "expected one observed Sōzu test binary\n" unless keys(%seen)==1; print((keys %seen)[0])}' "$evidence_root/opt$optimization-test.stderr")
    binary_host="$run_root/target/${selected_binary#/target/}"
    test -x "$binary_host"
    binary_sha=$(sha256sum "$binary_host" | cut -d ' ' -f1)
    resolver_sha=$(sha256sum /etc/resolv.conf | cut -d ' ' -f1)
    cp /etc/resolv.conf "$rootfs/etc/resolv.conf"
    test "$(sha256sum "$rootfs/etc/resolv.conf" | cut -d ' ' -f1)" = "$resolver_sha"
    containment_mode=(--host-network-readonly)
    network_scope=host-network-exact-rtt-case
    phase_timeout=120s
    printf 'hosts: files dns\npasswd: files\ngroup: files\n' > "$rootfs/etc/nsswitch.conf"
    set -- "$selected_binary" --exact socket::stats::test_rtt --nocapture --test-threads=1
else
    perl -MJSON::PP -e 'local $/; exit(decode_json(<>)->{exit_status} == 0 ? 0 : 1)' "$evidence_root/opt$optimization-build.meta.json"
    if test "$phase" = doc; then
        set -- /bin/bash -c 'cd /src && exec "$@"' baseline-cargo /toolchain/bin/cargo test "${cargo_scope[@]}" --locked --offline --doc --verbose -- --test-threads=1
    else
        set -- /bin/bash -c 'cd /src && exec "$@"' baseline-cargo /toolchain/bin/cargo test "${cargo_scope[@]}" --locked --offline --verbose -- --test-threads=1
    fi
fi
harness_sha=$(sha256sum "$script_root/rust-baseline.sh" | cut -d ' ' -f1)
containment_sha=$(sha256sum "$script_root/rust-contain.sh" | cut -d ' ' -f1)
started=$(date -u +%Y-%m-%dT%H:%M:%SZ)
load_before=$(cat /proc/loadavg)
set +e
/usr/bin/time -v -o "$record.time" timeout --kill-after=15s "$phase_timeout" \
    flock --close --nonblock "$work_root/.run.lock" \
    /bin/bash "$script_root/rust-contain.sh" "${containment_mode[@]}" "$source_root" "$run_root" \
    "$cache_root" "$toolchain_root" "$protoc_root" "$optimization" "$@" \
    </dev/null > "$record.stdout" 2> "$record.stderr"
status=$?
set -e
finished=$(date -u +%Y-%m-%dT%H:%M:%SZ)
load_after=$(cat /proc/loadavg)
perl -MJSON::PP -e '
my($phase,$opt,$status,$started,$finished,$before,$after,$source,$commit,$lock,$toolchain,$protoc,$harness_sha,$containment_sha,$script_root,$run_root,$cache_root,$evidence_root,$network_scope,$binary_sha,$resolver_sha,$phase_timeout,$package_name,$reviewed_config_sha,@argv)=@ARGV;
print JSON::PP->new->canonical->pretty->encode({
 phase=>$phase,opt_level=>0+$opt,exit_status=>0+$status,started=>$started,finished=>$finished,
 load_before=>$before,load_after=>$after,source=>$source,commit=>$commit,lock_sha256=>$lock,
 toolchain=>$toolchain,protoc=>$protoc,build_jobs=>2,codegen_units=>1,
 harness_sha256=>$harness_sha,containment_sha256=>$containment_sha,
 command_argv=>\@argv,
 containment_argv=>["/bin/bash","$script_root/rust-contain.sh",($network_scope eq "host-network-exact-rtt-case" ? ("--host-network-readonly") : ()),$source,$run_root,$cache_root,$toolchain,$protoc,join("",$opt),@argv],
 binary_sha256=>length($binary_sha) ? $binary_sha : undef,
 resolver_sha256=>length($resolver_sha) ? $resolver_sha : undef,
 timeout=>$phase_timeout,
 cargo_package=>length($package_name) ? $package_name : undef,
 cargo_scope=>length($package_name) ? "selected package" : "workspace",
 reviewed_native_config_sha256=>length($reviewed_config_sha) ? $reviewed_config_sha : undef,
 native_host=>"x86_64-unknown-linux-gnu",
 source_mode=>$network_scope eq "host-network-exact-rtt-case" ? "read-only overlay" : "private writable overlay",
 target_mode=>$network_scope eq "host-network-exact-rtt-case" ? "read-only" : "private writable",
 toolchain_version_evidence=>"$evidence_root/opt0-probe.stdout",
 profile=>"test",debug_info=>2,debug_assertions=>JSON::PP::true,overflow_checks=>JSON::PP::true,
 lto=>JSON::PP::false,incremental=>JSON::PP::false,default_features=>JSON::PP::true,
 test_threads=>1,tokio_workers=>1,network=>$network_scope,
 test_timing=>$network_scope eq "host-network-exact-rtt-case" ? "Only the already-built exact test case; no Cargo or build scripts" : "Cargo verification, any additional compilation, and test execution",
 rustdoc_scope=>"Cargo rustdoc defaults plus codegen-units=1; dependency libraries use selected test profile"
});' "$phase" "$optimization" "$status" "$started" "$finished" "$load_before" "$load_after" \
    "$source_root" "$package_commit" "$package_lock" "$toolchain_root" "$protoc_root" \
    "$harness_sha" "$containment_sha" "$script_root" "$run_root" "$cache_root" "$evidence_root" \
    "$network_scope" "$binary_sha" "$resolver_sha" "$phase_timeout" \
    "$package_name" "$reviewed_config_sha" "$@" > "$record.meta.json"
if test -n "$binary_sha"; then
    test "$(sha256sum "$binary_host" | cut -d ' ' -f1)" = "$binary_sha"
fi
git -C "$source_root" diff --quiet HEAD --
test "$(sha256sum "$source_root/Cargo.lock" | cut -d ' ' -f1)" = "$package_lock"
test ! -e "$source_root/.baseline-copyup-probe"
printf 'phase=%s opt=%s exit=%s evidence=%s\n' "$phase" "$optimization" "$status" "$record"
exit "$status"
