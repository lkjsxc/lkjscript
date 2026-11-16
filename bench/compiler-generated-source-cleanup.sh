#!/usr/bin/env bash
# Remove only the seven accepted generated-source repair artifacts.
set -euo pipefail
root=/home/coder/workspace/lkjscript-compiler-generated-source-20261116
cd "$root"
data=bench/compiler-generated-source-cleanup-data
[[ -d $data && ! -L $data ]]
[[ $(git rev-parse HEAD) == f6e5ea361dd9051da026ca0bf2bae71b2d874563 ]]
sha256sum --status -c bench/compiler-generated-source-manifest.sha256
printf 'committed_entries_before=437 verified\n'
cat > "$data/expected.tsv" <<'EXPECTED'
path	bytes	mode	sha256
/home/coder/workspace/lkjscript-compiler-generated-source-20261116/target/compiler-generated-source-baseline	31724996	700	ab2fa80787261e2fec6d872d989099d3b1dd0984c26ac59bf89c7237d9e97504
/home/coder/workspace/lkjscript-compiler-generated-source-20261116/target/compiler-generated-source-candidate	31981829	755	b722fb766f1c2f68808f55ebc56b26656de5241c4b60470b86f1bfabaa9f9ff4
/home/coder/workspace/lkjscript-compiler-generated-source-20261116/target/compiler-generated-source-candidate2	31994217	755	2f9fca2dbc0722fea49395f37d38973170af68ecefa6cbf62ac5db90fdbaff9e
/home/coder/workspace/lkjscript-compiler-generated-source-20261116/target/compiler-generated-source-candidate3	31994217	755	a43c0ee337599a4c9ebf6d2c02b27b74f949c20206b4134e7775ebf2d123859d
/home/coder/workspace/lkjscript-compiler-generated-source-20261116/target/compiler-generated-source-candidate4	32002610	755	408e30e5679ca684d4a66e501d16854a41ea6ae6d62a7c07a415b1e7df8c1b3a
/home/coder/workspace/lkjscript-compiler-generated-source-20261116/target/compiler-generated-source-key-path	68	600	a4f1072d5c58cdff970d1f94a18a471cc560422dc8824b759af1bfe8544bb258
/home/coder/workspace/compiler-generated-source-key-20261116.lCc7UT	32	600	e7f38244bc49a03e9d960fcf353673734ffec39bea9bdd48d4221fe0f0d6d8bc
EXPECTED
sha256sum .lkjscript-edit.lock .lkjscript-edit.generation > "$data/preserved.before.sha256"
[[ $(stat -c %s .lkjscript-edit.lock) == 0 ]]
[[ $(stat -c %s .lkjscript-edit.generation) == 64 ]]
[[ $(sha256sum .lkjscript-edit.lock | cut -d ' ' -f1) == e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 ]]
[[ $(sha256sum .lkjscript-edit.generation | cut -d ' ' -f1) == b79e9acd141ea40b4e77e31ccc9735c0849a197a59d1d735aeff6545ac8aaa07 ]]
[[ $(cat target/compiler-generated-source-key-path) == /home/coder/workspace/compiler-generated-source-key-20261116.lCc7UT ]]
[[ $(sha256sum src/structure_write.lkj | cut -d ' ' -f1) == 96408572b6a687d3906abd946b9644fb11175f03b354c65cc1ec67f94fd26afc ]]
[[ $(git show 830896f51b449532e99c203a6531844fa399fb50:src/structure_write.lkj | sha256sum | cut -d ' ' -f1) == 96408572b6a687d3906abd946b9644fb11175f03b354c65cc1ec67f94fd26afc ]]
find . -path './.git' -prune -o \( -name '.lkjscript-edit.pending' -o -name '*.lkjscript-next' \) -print > "$data/pending-and-stages.before"
[[ ! -s $data/pending-and-stages.before ]]
printf 'path\tbytes\tmode\towner\tsha256\n' > "$data/before.tsv"
paths=()
total=0
while IFS=$'\t' read -r path bytes mode digest; do
    [[ $path == path ]] && continue
    [[ -f $path && ! -L $path && $(realpath -e "$path") == "$path" ]]
    [[ $(stat -c %U "$path") == coder && $(stat -c %s "$path") == "$bytes" && $(stat -c %a "$path") == "$mode" ]]
    actual=$(sha256sum -- "$path" | cut -d ' ' -f1)
    [[ $actual == "$digest" ]]
    printf '%s\t%s\t%s\tcoder\t%s\n' "$path" "$bytes" "$mode" "$actual" >> "$data/before.tsv"
    paths+=("$path")
    total=$((total + bytes))
done < "$data/expected.tsv"
[[ ${#paths[@]} == 7 && $total == 159697969 ]]

# Inspect same-user executables and open descriptors by inode, without reading
# process-owned file contents. A live inaccessible descriptor inventory refuses.
perl - "$data/expected.tsv" > "$data/process-references.txt" <<'PROCESS'
use strict; use warnings;
my $file=shift;
open my $f,'<:raw',$file or die $!;
my %owned;
while (<$f>) {
    next if /^path\t/;
    my ($path)=split /\t/;
    my @s=stat($path); @s or die "target disappeared before process scan\n";
    $owned{"$s[0]:$s[1]"}=$path;
}
close $f;
my %ancestors=($$=>1);
my $parent=getppid();
while ($parent>0 && !$ancestors{$parent}++) {
    open my $s,'<',"/proc/$parent/status" or die "cannot inspect parent status\n";
    my $next=0; while (<$s>) { $next=$1 if /^PPid:\s+(\d+)/; }
    close $s; $parent=$next;
}
my $uid=$<; my $inspected=0;
for my $dir (glob '/proc/[0-9]*') {
    $dir =~ m{/(\d+)$} or next; my $pid=$1;
    next if $ancestors{$pid};
    my @p=stat($dir); next unless @p && $p[4]==$uid;
    my @e=stat("$dir/exe");
    if (@e && exists $owned{"$e[0]:$e[1]"}) { die "live executable reference pid=$pid path=$owned{\"$e[0]:$e[1]\"}\n"; }
    if (!opendir(my $fds,"$dir/fd")) {
        next unless -d $dir;
        die "cannot inspect live same-user descriptor inventory pid=$pid\n";
    } else {
        while (my $fd=readdir($fds)) {
            next unless $fd =~ /^\d+$/;
            my @s=stat("$dir/fd/$fd"); next unless @s;
            if (exists $owned{"$s[0]:$s[1]"}) { die "live descriptor reference pid=$pid fd=$fd path=$owned{\"$s[0]:$s[1]\"}\n"; }
        }
        closedir($fds);
    }
    $inspected++;
}
print "same_user_processes_inspected=$inspected\nlive_owned_executable_or_descriptor_references=none\n";
PROCESS

processes=$(ps -eo pid=,ppid=,args=)
ancestors=" $$ "
ancestor=$PPID
while [[ -n $ancestor && $ancestor != 0 ]]; do
    ancestors+="$ancestor "
    ancestor=$(printf '%s\n' "$processes" | awk -v pid="$ancestor" '$1==pid {print $2; exit}')
done
if printf '%s\n' "$processes" | awk -v root="$root" \
    -v key=/home/coder/workspace/compiler-generated-source-key-20261116.lCc7UT \
    -v ancestors="$ancestors" \
    '!index(ancestors," "$1" ") && (index($0,root) || index($0,key)) {print; found=1} END {exit !found}' > "$data/process-arguments.txt"; then
    printf 'live command references an owned path; refusing cleanup\n' >&2
    exit 1
fi

# Last checks immediately precede the single exact-path unlink. No recursive
# removal, directory removal, compiler invocation, or source mutation occurs.
find . -path './.git' -prune -o \( -name '.lkjscript-edit.pending' -o -name '*.lkjscript-next' \) -print > "$data/pending-and-stages.final"
[[ ! -s $data/pending-and-stages.final ]]
while IFS=$'\t' read -r path bytes mode digest; do
    [[ $path == path ]] && continue
    [[ -f $path && ! -L $path && $(stat -c %U "$path") == coder ]]
    [[ $(stat -c %s "$path") == "$bytes" && $(stat -c %a "$path") == "$mode" ]]
    [[ $(sha256sum -- "$path" | cut -d ' ' -f1) == "$digest" ]]
done < "$data/expected.tsv"
rm -- "${paths[@]}"
printf 'path\tabsent\n' > "$data/after.tsv"
for path in "${paths[@]}"; do
    [[ ! -e $path && ! -L $path ]]
    printf '%s\tyes\n' "$path" >> "$data/after.tsv"
done
sha256sum .lkjscript-edit.lock .lkjscript-edit.generation > "$data/preserved.after.sha256"
cmp "$data/preserved.before.sha256" "$data/preserved.after.sha256"
sha256sum --status -c bench/compiler-generated-source-manifest.sha256
printf 'removed_files=7\nremoved_bytes=159697969\npreserved_metadata=byte-identical\ncommitted_entries_after=437 verified\ncompiler_executions=0\nreplay_executions=0\n'
