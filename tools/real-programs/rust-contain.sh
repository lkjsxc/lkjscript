#!/usr/bin/env bash
# A disposable Linux build namespace. Only the explicitly bound directories are visible.
# The host-network mode is restricted to one reviewed, already-built Sōzu RTT test.
set -euo pipefail
if test "$#" -lt 7; then
    printf '%s\n' 'usage: rust-contain.sh [--host-network-readonly] SOURCE WORK CACHE TOOLCHAIN PROTOC OPT COMMAND [ARG...]' >&2
    exit 2
fi
network_mode=isolated
forward_mode=()
if test "$1" = --host-network-readonly; then
    network_mode=host-readonly
    forward_mode=(--host-network-readonly)
    shift
fi
if test "$1" != --namespace; then
    namespace_args=(--user --map-root-user --mount --pid --fork --ipc --uts --kill-child=KILL)
    if test "$network_mode" = isolated; then namespace_args+=(--net); fi
    exec unshare "${namespace_args[@]}" /usr/bin/env -i PATH=/usr/bin:/bin \
        /bin/bash "$0" "${forward_mode[@]}" --namespace "$@"
fi
shift
source_root=$1
work_root=$2
cache_root=$3
toolchain_root=$4
protoc_root=$5
optimization=$6
shift 6
if test "$network_mode" = host-readonly; then
    test "$#" -eq 5
    [[ "$1" =~ ^/target/debug/deps/sozu_lib-[0-9a-f]{16}$ ]]
    test "$2" = --exact
    test "$3" = socket::stats::test_rtt
    test "$4" = --nocapture
    test "$5" = --test-threads=1
fi
for path in "$source_root" "$work_root" "$cache_root" "$toolchain_root" "$protoc_root"; do
    case "$path" in /*) ;; *) printf 'path must be absolute: %s\n' "$path" >&2; exit 2;; esac
    case "$path" in *[!A-Za-z0-9_./-]*) printf 'unsupported mount path: %s\n' "$path" >&2; exit 2;; esac
done
case "$optimization" in 0|3) ;; *) exit 2;; esac
rootfs="$work_root/root"
mount --make-rprivate /
mount --bind "$rootfs" "$rootfs"
mount --bind /usr "$rootfs/usr"
mount -o remount,bind,ro /usr "$rootfs/usr"
mount --bind /etc/alternatives "$rootfs/etc/alternatives"
mount -o remount,bind,ro /etc/alternatives "$rootfs/etc/alternatives"
mount --bind /etc/ld.so.cache "$rootfs/etc/ld.so.cache"
mount -o remount,bind,ro /etc/ld.so.cache "$rootfs/etc/ld.so.cache"
mount --bind "$toolchain_root" "$rootfs/toolchain"
mount -o remount,bind,ro "$toolchain_root" "$rootfs/toolchain"
mount --bind "$protoc_root" "$rootfs/protoc"
mount -o remount,bind,ro "$protoc_root" "$rootfs/protoc"
mount --bind "$work_root/target" "$rootfs/target"
if test "$network_mode" = host-readonly; then
    mount -o remount,bind,ro "$work_root/target" "$rootfs/target"
fi
mount --bind "$work_root/tmp" "$rootfs/tmp"
mount --bind "$work_root/cargo-home" "$rootfs/cargo-home"
mount --bind "$cache_root/registry" "$rootfs/cargo-home/registry"
mount -o remount,bind,ro "$cache_root/registry" "$rootfs/cargo-home/registry"
mount -t overlay overlay -o "lowerdir=$source_root,upperdir=$work_root/upper,workdir=$work_root/overlay-work,userxattr" "$rootfs/src"
if test "$network_mode" = host-readonly; then
    mount -o remount,ro "$rootfs/src"
    # The driver supplied a digest-verified resolver copy; the rootfs is sealed below.
    test -s "$rootfs/etc/resolv.conf"
    mount -o remount,bind,ro "$work_root/cargo-home" "$rootfs/cargo-home"
fi
mount -t proc -o ro,nosuid,nodev,noexec proc "$rootfs/proc"
for device in null zero random urandom; do
    mount --bind "/dev/$device" "$rootfs/dev/$device"
done
if test "$network_mode" = isolated; then ip link set lo up; fi
mount -o remount,bind,ro "$rootfs" "$rootfs"
# A rootless namespace still initially has namespace capabilities. Remove them
# before any build script, compiler plugin, test, or descendant process executes.
cd /
exec /usr/sbin/chroot "$rootfs" /usr/bin/setpriv --bounding-set=-all --inh-caps=-all \
    --ambient-caps=-all --no-new-privs /usr/bin/env -i \
    PATH=/toolchain/bin:/protoc/bin:/usr/bin:/bin \
    CARGO_HOME=/cargo-home CARGO_TARGET_DIR=/target CARGO_NET_OFFLINE=true \
    CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_OPT_LEVEL="$optimization" \
    RUSTC=/toolchain/bin/rustc RUSTDOC=/toolchain/bin/rustdoc \
    RUSTFLAGS='-C codegen-units=1' RUSTDOCFLAGS='-C codegen-units=1' \
    PROTOC=/protoc/bin/protoc PROTOC_INCLUDE=/protoc/include \
    RUST_TEST_THREADS=1 TOKIO_WORKER_THREADS=1 \
    TMPDIR=/tmp LC_ALL=C TZ=UTC \
    GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null \
    "$@"
