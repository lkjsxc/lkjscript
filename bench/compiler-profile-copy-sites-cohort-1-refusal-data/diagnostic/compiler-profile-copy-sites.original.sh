#!/usr/bin/env bash
# One prospective fixed-witness cohort. Requires a committed, explicitly released JSON.
set -euo pipefail
export LC_ALL=C
umask 077
[[ ${1-} == run && $# == 2 ]] || {
    echo 'usage: bash bench/compiler-profile-copy-sites.sh run bench/RELEASE.json' >&2
    exit 2
}
script=$(realpath -e "${BASH_SOURCE[0]}")
repository=$(git -C "$(dirname "$script")" rev-parse --show-toplevel)
repository=$(realpath -e "$repository")
cd "$repository"
release=$(realpath -e "$2")
[[ -d $repository/target && ! -L $repository/target && ! -L $2 &&
   $release == "$repository"/bench/* && $script == "$repository"/bench/* ]] || {
    echo 'E_COPY_RELEASE require committed bench files and a physical target directory' >&2
    exit 2
}
# Read only path syntax before atomically claiming this attempt. Full pin checks follow.
paths=$(perl -MJSON::PP -e '
    use strict; use warnings;
    my ($file,$repo)=@ARGV;
    open my $f,"<:raw",$file or die "$file: $!";
    local $/; my $r=decode_json(<$f>);
    die "release schema\n" unless ref($r) eq "HASH" &&
        ($r->{schema}//"") eq "compiler-copy-sites-release-v1" &&
        ($r->{repository}//"") eq $repo;
    for my $pair (["evidence_directory","compiler-copy-sites-evidence."],
                  ["run_directory","compiler-copy-sites-run."]) {
        my $v=$r->{$pair->[0]};
        die "release path\n" if !defined($v) || ref($v) ||
            $v !~ m{\Atarget/\Q$pair->[1]\E[A-Za-z0-9][A-Za-z0-9._-]*\z};
        print "$v\n";
    }
' "$release" "$repository") || exit 2
mapfile -t paths_array <<< "$paths"
[[ ${#paths_array[@]} == 2 ]] || exit 2
evidence="$repository/${paths_array[0]}"
run="$repository/${paths_array[1]}"
[[ ! -e $evidence && ! -L $evidence && ! -e $run && ! -L $run ]] || {
    echo 'E_COPY_ONCE existing evidence or run; no retry' >&2
    exit 2
}
mkdir "$evidence"
phase=preflight
stop_reason=unclassified_exit
completed=false
finish() {
    local status=$?
    trap - EXIT HUP INT TERM
    set +e
    if [[ $completed != true && $status == 0 ]]; then status=125; fi
    printf 'supervisor_exit=%s\ncompleted=%s\nlast_phase=%s\nreason=%s\nfinished_utc=%s\nrun=%s\n' \
        "$status" "$completed" "$phase" "$stop_reason" "$(date -u +%FT%TZ)" "$run" > "$evidence/completion"
    local record_status=$?
    : > "$evidence/not-run.tsv"
    local label
    for label in 01-construct-ordinary 02-construct-profile 03-execute-ordinary 04-execute-profile \
                 05-summary 06-copy-detail 07-accounting; do
        if [[ ! -d $evidence/$label ]]; then
            printf '%s\tnot_run\t%s\n' "$label" "$stop_reason" >> "$evidence/not-run.tsv"
        fi
    done
    if ((record_status != 0)); then status=125; fi
    printf 'completed=%s status=%s phase=%s evidence=%s\n' "$completed" "$status" "$phase" "$evidence"
    exit "$status"
}
trap finish EXIT
trap 'stop_reason=supervisor_signal; exit 130' HUP INT TERM
stop() { stop_reason=$1; echo "E_COPY_STOP $stop_reason" >&2; exit 1; }
printf '%s\n' "$repository" > "$evidence/owner-repository"
printf '%s\n' "$BASHPID" > "$evidence/owner-pid"
awk '{print $22}' "/proc/$BASHPID/stat" > "$evidence/owner-start"
printf '%s\n' "$run" > "$evidence/fixture.path"
cp -- "$release" "$evidence/release.json"
sha256sum "$release" > "$evidence/release.sha256"
printf '%s\n' "$script" > "$evidence/launcher.path"
printf '%s\n' "$repository" > "$evidence/cwd"
# This validator checks identities, not the semantic content of root's acceptance.
perl - "$repository" "$script" "$release" "$evidence" > "$evidence/preflight.json" \
    2> "$evidence/preflight.stderr" <<'PERL_PREFLIGHT' || stop release_preflight_failed
use strict; use warnings;
use JSON::PP (); use Digest::SHA (); use Cwd qw(abs_path);
my ($repo,$script,$release,$out)=@ARGV;
sub die_if { die "$_[1]\n" if $_[0]; }
sub raw {
    my ($p)=@_; my @s=lstat($p);
    die_if(!@s || !-f _ || -l _ || $s[3]!=1,"not a single-link regular file: $p");
    open my $f,"<:raw",$p or die "$p: $!"; local $/; return <$f>;
}
sub sha_file {
    my ($p,$allow_links)=@_; my @s=lstat($p);
    die_if(!@s || !-f _ || -l _ || (!$allow_links && $s[3]!=1),"not an admissible regular file: $p");
    open my $f,"<:raw",$p or die "$p: $!";
    return Digest::SHA->new(256)->addfile($f)->hexdigest;
}
sub git {
    open my $f,"-|","git",@_ or die "git: $!"; binmode $f; local $/;
    my $v=<$f>; close $f or die "git failed: @_\n"; return $v;
}
sub hash { die_if(!defined($_[0]) || ref($_[0]) || $_[0]!~/\A[0-9a-f]{64}\z/,"invalid sha256"); return $_[0]; }
sub oid { die_if(!defined($_[0]) || ref($_[0]) || $_[0]!~/\A[0-9a-f]{40}\z/,"invalid commit/tree"); return $_[0]; }
sub rel {
    my ($p)=@_;
    die_if(!defined($p) || ref($p) || $p!~m{\A[A-Za-z0-9._/-]+\z} ||
        $p=~m{(?:\A|/)\.\.?(?:/|\z)} || $p=~m{//} || $p=~m{\A/},"unsafe relative path");
    return $p;
}
sub exact_keys {
    my ($v,@allowed)=@_; die_if(ref($v) ne "HASH","expected object");
    my %ok=map {$_=>1} @allowed;
    die_if(scalar(grep(!$ok{$_},keys %$v)),"unknown release field");
    die_if(scalar(grep(!exists($v->{$_}),@allowed)),"missing release field");
}
sub committed {
    my ($path,$want,$commit)=@_; rel($path); hash($want); oid($commit);
    die_if(sha_file("$repo/$path") ne $want,"working file hash differs: $path");
    die_if(Digest::SHA::sha256_hex(git("show","$commit:$path")) ne $want,
        "committed bytes differ: $path");
}
my $bytes=raw($release);
my $r=JSON::PP->new->utf8->decode($bytes);
exact_keys($r,qw(schema cohort repository release_path evidence_directory run_directory files producer evidence witness bounds decode workers tools));
die_if($r->{schema} ne "compiler-copy-sites-release-v1" || $r->{repository} ne $repo,"schema/repository");
die_if($r->{cohort}!~/\Acompiler-copy-sites-[a-z0-9][a-z0-9._-]*\z/,"cohort");
my $head=git("rev-parse","HEAD"); chomp $head; oid($head);
die_if("$repo/".rel($r->{release_path}) ne $release,"release path mismatch");
die_if(Digest::SHA::sha256_hex(git("show","$head:$r->{release_path}")) ne
    Digest::SHA::sha256_hex($bytes),"release is not exact committed HEAD bytes");
die_if(raw("$out/release.json") ne $bytes,"release copy mismatch");
die_if($r->{evidence_directory}!~m{\Atarget/compiler-copy-sites-evidence\.[A-Za-z0-9][A-Za-z0-9._-]*\z} ||
       "$repo/$r->{evidence_directory}" ne $out ||
       $r->{run_directory}!~m{\Atarget/compiler-copy-sites-run\.[A-Za-z0-9][A-Za-z0-9._-]*\z},"result/run paths");
exact_keys($r->{files},qw(plan launcher accounting));
for my $kind (qw(plan launcher accounting)) {
    my $v=$r->{files}{$kind}; exact_keys($v,qw(path sha256));
    die_if(rel($v->{path})!~m{\Abench/[^/]+\z},"bench file required");
    committed($v->{path},$v->{sha256},$head);
}
die_if("$repo/$r->{files}{launcher}{path}" ne $script,"launcher path mismatch");
die_if($r->{workers} ne "1","one worker required");
exact_keys($r->{bounds},qw(address_space_bytes deadline_seconds grace_seconds free_space_bytes));
my %bounds=(address_space_bytes=>4294967296,deadline_seconds=>900,grace_seconds=>10,free_space_bytes=>3221225472);
for my $k (keys %bounds) { die_if($r->{bounds}{$k} ne "$bounds{$k}","bound mismatch: $k"); }
exact_keys($r->{decode},qw(limit bytes));
die_if($r->{decode}{limit} ne "200" || $r->{decode}{bytes} ne "1048576","detail bounds");
my $w=$r->{witness};
exact_keys($w,qw(revision source_tree library_tree runtime_sha256 optimizer_sha256 seed_archive_sha256 seed_archive_bytes expected_sha256 expected_bytes input_files));
my %witness=(
 revision=>"26bf2c3c021dd79810a81fa1d9054d59f3768b06",
 source_tree=>"3431671024d0a550b3130f2401bde067026d9bd3",
 library_tree=>"e442fb3920db7eda2377f92fde0d92761f0562be",
 runtime_sha256=>"eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14",
 optimizer_sha256=>"961d59858c83afd8c5e8e2f9ec00d0f3a90cc91953ba8903528589f41a9da705",
 seed_archive_sha256=>"85b4e70de34dfa759aaf3cfb7e1009ba6b49b57499e01e4a3d49ae9182a6193c",
 seed_archive_bytes=>"3145429",expected_sha256=>"8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132",
 expected_bytes=>"16228028",input_files=>"172");
for my $k (keys %witness) { die_if($w->{$k} ne $witness{$k},"witness mismatch: $k"); }
for my $pair (["src","source_tree"],["library","library_tree"]) {
    my $v=git("rev-parse","$w->{revision}:$pair->[0]"); chomp $v;
    die_if($v ne $w->{$pair->[1]},"witness tree mismatch");
}
for my $pair (["library/runtime.lkj","runtime_sha256"],["src/optimizer.lkj","optimizer_sha256"],["seed/lkjscript.gz","seed_archive_sha256"]) {
    die_if(Digest::SHA::sha256_hex(git("show","$w->{revision}:$pair->[0]")) ne $w->{$pair->[1]},"witness blob mismatch");
}
my $p=$r->{producer};
exact_keys($p,qw(path sha256 bytes source_commit source_tree library_tree runtime_sha256 seed_commit seed_archive_sha256 seed_archive_bytes bootstrap_revision bootstrap_path bootstrap_sha256));
die_if(!defined($p->{path}) || $p->{path}!~m{\A/[A-Za-z0-9._/-]+\z} ||
       abs_path($p->{path}) ne $p->{path},"producer path must be absolute/canonical");
hash($p->{sha256}); oid($p->{source_commit}); oid($p->{seed_commit}); oid($p->{bootstrap_revision});
die_if($p->{bytes}!~/\A[1-9][0-9]*\z/ || -s $p->{path} != $p->{bytes} ||
       sha_file($p->{path}) ne $p->{sha256},"producer identity");
die_if(!-x $p->{path},"producer not executable");
for my $pair (["src","source_tree"],["library","library_tree"]) {
    oid($p->{$pair->[1]}); my $v=git("rev-parse","$p->{source_commit}:$pair->[0]"); chomp $v;
    die_if($v ne $p->{$pair->[1]},"producer source tree mismatch");
}
hash($p->{runtime_sha256});
die_if(Digest::SHA::sha256_hex(git("show","$p->{source_commit}:library/runtime.lkj")) ne $p->{runtime_sha256},"producer runtime mismatch");
my $seed=git("show","$p->{seed_commit}:seed/lkjscript.gz");
die_if(length($seed) ne $p->{seed_archive_bytes} ||
       Digest::SHA::sha256_hex($seed) ne hash($p->{seed_archive_sha256}),"producer seed archive mismatch");
die_if(Digest::SHA::sha256_hex(git("show","$p->{bootstrap_revision}:".rel($p->{bootstrap_path}))) ne
       hash($p->{bootstrap_sha256}),"bootstrap recipe mismatch");
die_if(system("git","merge-base","--is-ancestor",$p->{source_commit},$head)!=0,"producer source not in release history");
die_if(system("git","diff","--quiet",$p->{source_commit},"--","src","library")!=0,"current source/library differs from gated producer");
for my $f (".lkjscript-edit.pending","..lkjscript-edit.pending.lkjscript-next","..lkjscript-edit.generation.lkjscript-next") {
    die_if(-e "$repo/$f" || -l "$repo/$f","pending source transaction");
}
die_if(ref($r->{evidence}) ne "ARRAY","evidence array required");
my %required=map {$_=>0} qw(gate fixed_point ci focused_profile);
for my $e (@{$r->{evidence}}) {
    exact_keys($e,qw(kind path commit sha256 status coverage_limits));
    die_if(!exists($required{$e->{kind}}) || $required{$e->{kind}}++,"duplicate/unknown acceptance kind");
    die_if($e->{status} ne "passed" || !defined($e->{coverage_limits}) || ref($e->{coverage_limits}),"acceptance must record passed status and limitations");
    die_if(rel($e->{path})!~m{\Abench/},"acceptance must be retained in bench");
    committed($e->{path},$e->{sha256},$e->{commit});
    die_if(system("git","merge-base","--is-ancestor",$e->{commit},$head)!=0,"acceptance commit not in release history");
}
die_if(scalar(grep($_!=1,values %required)),"missing required acceptance");
exact_keys($r->{tools},qw(time prlimit timeout perl git gzip tar bash));
for my $name (sort keys %{$r->{tools}}) {
    my $t=$r->{tools}{$name}; exact_keys($t,qw(path sha256));
    die_if($t->{path}!~m{\A/[A-Za-z0-9._/-]+\z} ||
           abs_path($t->{path}) ne $t->{path} || !-x $t->{path},"tool path: $name");
    die_if(sha_file($t->{path},1) ne hash($t->{sha256}),"tool hash: $name");
}
die_if($r->{tools}{time}{path} ne "/usr/bin/time","GNU time path differs");
my $report={schema=>"compiler-copy-sites-preflight-v1",status=>"passed",head=>$head,
    release_sha256=>Digest::SHA::sha256_hex($bytes),release=>$r};
print JSON::PP->new->canonical->pretty->encode($report);
PERL_PREFLIGHT

value() {
    perl -MJSON::PP -e '
        my ($p,@keys)=@ARGV; open my $f,"<:raw",$p or die $!;
        local $/; my $v=decode_json(<$f>);
        for my $k (@keys) { $v=$v->{$k}; }
        die "invalid scalar\n" if !defined($v) || ref($v) || $v=~/[\r\n\0]/;
        print $v;
    ' "$evidence/release.json" "$@"
}
producer_input=$(value producer path)
producer_sha=$(value producer sha256)
producer_bytes=$(value producer bytes)
helper_input="$repository/$(value files accounting path)"
helper_sha=$(value files accounting sha256)
time_tool=$(value tools time path)
prlimit_tool=$(value tools prlimit path)
timeout_tool=$(value tools timeout path)
perl_tool=$(value tools perl path)
git_tool=$(value tools git path)
gzip_tool=$(value tools gzip path)
tar_tool=$(value tools tar path)
# Match PATH-resolved tools used in setup to the pinned programs.
for pair in "perl:$perl_tool" "git:$git_tool" "gzip:$gzip_tool" "tar:$tar_tool" "prlimit:$prlimit_tool" "timeout:$timeout_tool"; do
    name=${pair%%:*}; pinned=${pair#*:}
    [[ $(realpath -e "$(command -v "$name")") == "$pinned" ]] || stop setup_tool_path_mismatch
done
[[ $(realpath -e "$BASH") == "$(value tools bash path)" ]] || stop bash_path_mismatch
load() { awk '{print $1 " " $2 " " $3}' /proc/loadavg; }
space() {
    local label=$1 needed=${2:-3221225472} free
    free=$(df -Pk "$repository/target" | awk 'NR==2 {printf "%.0f",$4*1024}')
    printf 'utc=%s\nload=%s\nfree_bytes=%s\nrequired_bytes=%s\n' \
        "$(date -u +%FT%TZ)" "$(load)" "$free" "$needed" > "$evidence/space-$label.meta"
    [[ $free =~ ^[0-9]+$ ]] && ((free >= needed))
}
space preparation || stop free_space_preparation
mkdir "$run"
cp "$evidence"/owner-* "$run/"
printf '%s\n' "$evidence" > "$run/owner-evidence"
printf '%s\n' "$repository" > "$run/owner-cwd"
cp -- "$producer_input" "$run/producer"
cp -- "$helper_input" "$run/accounting.pl"
chmod 500 "$run/producer"
chmod 400 "$run/accounting.pl"
producer="$run/producer"
helper="$run/accounting.pl"
[[ $(sha256sum "$producer" | awk '{print $1}') == "$producer_sha" &&
   $(stat -c %s "$producer") == "$producer_bytes" &&
   $(sha256sum "$helper" | awk '{print $1}') == "$helper_sha" ]] || stop immutable_copy_mismatch
sha256sum "$producer" "$helper" > "$evidence/copied-tools.sha256"
source="$run/source"
mkdir "$source"
"$git_tool" archive 26bf2c3c021dd79810a81fa1d9054d59f3768b06 src library seed/lkjscript.gz |
    "$tar_tool" -x -C "$source" || stop archive_failed
inventory() {
    "$perl_tool" - "$1" <<'PERL_INVENTORY'
use strict; use warnings; use File::Find (); use Digest::SHA ();
my ($root)=@ARGV; my @paths;
File::Find::find({no_chdir=>1,wanted=>sub {
    my $p=$File::Find::name; my @s=lstat($p); die "lstat $p: $!" unless @s;
    die "link/special input: $p\n" if -l _ || (!-d _ && !-f _);
    if (-f _) { die "hardlink: $p\n" if $s[3]!=1; push @paths,$p; }
}},$root);
for my $p (sort @paths) {
    open my $f,"<:raw",$p or die "$p: $!";
    my $relative=substr($p,length($root)+1);
    die "unsupported input path\n" if $relative=~/[\r\n\0]/;
    print Digest::SHA->new(256)->addfile($f)->hexdigest,"  $relative\n";
}
PERL_INVENTORY
}
inventory "$source" > "$evidence/source-before.sha256" || stop source_inventory_failed
[[ $(wc -l < "$evidence/source-before.sha256") == 172 &&
   $(sha256sum "$source/library/runtime.lkj" | awk '{print $1}') == eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14 &&
   $(sha256sum "$source/src/optimizer.lkj" | awk '{print $1}') == 961d59858c83afd8c5e8e2f9ec00d0f3a90cc91953ba8903528589f41a9da705 &&
   $(sha256sum "$source/seed/lkjscript.gz" | awk '{print $1}') == 85b4e70de34dfa759aaf3cfb7e1009ba6b49b57499e01e4a3d49ae9182a6193c ]] || stop frozen_source_mismatch
"$gzip_tool" -dc "$source/seed/lkjscript.gz" > "$run/expected" || stop frozen_seed_decode_failed
expected="$run/expected"
[[ $(stat -c %s "$expected") == 16228028 &&
   $(sha256sum "$expected" | awk '{print $1}') == 8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132 ]] || stop frozen_seed_mismatch
ordinary="$run/producer-ordinary64"
profiled="$run/producer-profile64"
companion="$run/selected-native.lkjir"
output_ordinary="$run/output-ordinary"
output_profile="$run/output-profile"
run_phase() {
    local label=$1; shift
    phase=$label
    local dir="$evidence/$label" status=0 after capture_status=0
    mkdir "$dir" || return 125
    printf '%s\n' "$repository" > "$dir/cwd" || return 125
    "$perl_tool" -MJSON::PP -e 'print JSON::PP->new->canonical->encode(\@ARGV),"\n";' -- "$@" > "$dir/argv.json" || return 125
    "$perl_tool" -MJSON::PP -e 'print JSON::PP->new->canonical->encode(\@ARGV),"\n";' -- \
        "$time_tool" -f '%e\t%U\t%S\t%M\t%x' -o "$dir/time" \
        "$prlimit_tool" --as=4294967296 -- "$timeout_tool" --kill-after=10s 900s "$@" > "$dir/wrapper-argv.json" || return 125
    date -u +%FT%TZ > "$dir/start.utc" || return 125
    date +%s%N > "$dir/start.ns" || return 125
    load > "$dir/load.before" || return 125
    "$time_tool" -f '%e\t%U\t%S\t%M\t%x' -o "$dir/time" \
        "$prlimit_tool" --as=4294967296 -- "$timeout_tool" --kill-after=10s 900s "$@" \
        < /dev/null > "$dir/stdout" 2> "$dir/stderr" || status=$?
    printf '%s\n' "$status" > "$dir/status" || capture_status=1
    date +%s%N > "$dir/end.ns" || capture_status=1
    date -u +%FT%TZ > "$dir/end.utc" || capture_status=1
    load > "$dir/load.after" || capture_status=1
    "$perl_tool" - "$dir" "$status" "$capture_status" > "$dir/capture.json" 2> "$dir/capture.stderr" <<'PERL_CAPTURE' || capture_status=1
use strict; use warnings; use JSON::PP (); use Math::BigInt;
my ($dir,$status,$io)=@ARGV;
my @errors;
sub read_value {
    my ($name)=@_; open my $f,"<:raw","$dir/$name" or do {push @errors,"missing:$name"; return ""};
    local $/; my $v=<$f>; $v=~s/\n\z//; return $v;
}
my $start=read_value("start.ns"); my $end=read_value("end.ns");
my $before=read_value("load.before"); my $after=read_value("load.after");
my $start_utc=read_value("start.utc"); my $end_utc=read_value("end.utc");
for my $v ($start_utc,$end_utc) { push @errors,"utc" if $v!~/\A[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z\z/; }
push @errors,"capture-io" if $io;
push @errors,"timestamps" if $start!~/\A[0-9]+\z/ || $end!~/\A[0-9]+\z/ ||
    Math::BigInt->new($end)->bcmp(Math::BigInt->new($start))<0;
for my $v ($before,$after) { push @errors,"load" if $v!~/\A[0-9]+\.[0-9]+ [0-9]+\.[0-9]+ [0-9]+\.[0-9]+\z/; }
my $time=read_value("time");
my @rows=grep { /\A[0-9]+(?:\.[0-9]+)?\t[0-9]+(?:\.[0-9]+)?\t[0-9]+(?:\.[0-9]+)?\t[0-9]+\t[0-9]+\z/ } split /\n/,$time;
push @errors,"gnu-time" unless @rows==1;
my ($wall,$user,$system,$rss,$gnu)=@rows==1 ? split(/\t/,$rows[0]) : ();
push @errors,"exit-mismatch" if defined($gnu) && $gnu ne $status;
my $outcome=$status==124 ? "watchdog" : $status==137 ? "signal_or_watchdog" :
    ($status>=125 && $status<=127) ? "launcher_failure" : $status>=128 ? "signal" :
    $status!=0 ? "exit_failure" : @errors ? "capture_incomplete" : "ok";
my $r={status=>"$status",outcome=>$outcome,capture=>@errors?"incomplete":"complete",
    wall_s=>$wall,user_s=>$user,system_s=>$system,rss_kib=>$rss,gnu_exit=>$gnu,
    start_ns=>$start,end_ns=>$end,start_utc=>$start_utc,end_utc=>$end_utc,
    load_before=>$before,load_after=>$after,errors=>\@errors};
print JSON::PP->new->canonical->pretty->encode($r);
exit(@errors ? 1 : 0);
PERL_CAPTURE
    printf '%s\n' "$capture_status" > "$dir/capture.status" || return 125
    printf 'phase=%s status=%s capture=%s load=%s -> %s\n' "$label" "$status" "$capture_status" \
        "$(cat "$dir/load.before")" "$(cat "$dir/load.after")"
    ((capture_status == 0)) || return 125
    return "$status"
}
stats() {
    "$perl_tool" - "$evidence/$1" "$evidence/$2" <<'PERL_STATS'
use strict; use warnings; use JSON::PP ();
my ($dir,$other)=@ARGV;
open my $f,"<:raw","$dir/stderr" or die $!; local $/; my $text=<$f>;
die "unexpected construction stdout\n" if -s "$dir/stdout";
die "one stats record required\n" unless $text=~/\ALKJSCRIPT_STATS (\{[^\n]+\})\n\z/;
my $v=JSON::PP->new->decode($1);
die "unsuccessful construction stats\n" unless ($v->{route}//"") eq "compiler" && ($v->{stopped_stage}//"") eq "done";
my @counts=qw(front_definitions front_functions ir_functions ir_instructions inline_attempts inline_calls inline_functions inline_accepted inline_rejected);
for my $k (@counts,qw(build_reused front_reused checked_reused emitted_reused)) {
    die "missing count:$k\n" unless defined($v->{$k}) && !ref($v->{$k}) && $v->{$k}=~/\A[0-9]+\z/;
}
die "inline fallback or no accepted rewrite\n" unless $v->{inline_accepted}>0 && $v->{inline_rejected}==0 && $v->{inline_calls}>0 && $v->{inline_functions}>0;
for my $k (qw(build_reused front_reused checked_reused emitted_reused)) { die "unexpected store reuse:$k\n" if $v->{$k}!=0; }
if ($other ne $dir) {
    open my $old,"<:raw","$other/stats.json" or die $!; my $a=JSON::PP->new->decode(<$old>);
    for my $k (@counts) { die "construction counts differ:$k\n" unless $v->{$k} eq $a->{$k}; }
}
open my $o,">:raw","$dir/stats.json" or die $!; print $o JSON::PP->new->canonical->pretty->encode($v); close $o or die $!;
PERL_STATS
}
artifact() {
    local file=$1 label=$2
    [[ -f $file && ! -L $file && $(stat -c %h "$file") == 1 ]] || return 1
    printf 'path=%s\nbytes=%s\nsha256=%s\n' "$file" "$(stat -c %s "$file")" \
        "$(sha256sum "$file" | awk '{print $1}')" > "$evidence/$label.artifact"
}
oracle() {
    local label=$1 output=$2
    [[ ! -s $evidence/$label/stdout && ! -s $evidence/$label/stderr &&
       -f $output && ! -L $output && $(stat -c %h "$output") == 1 ]] || return 1
    cmp -- "$expected" "$output" > "$evidence/$label/oracle.stdout" 2> "$evidence/$label/oracle.stderr" || return 1
    artifact "$output" "$label-output" || return 1
    printf 'byte_equal=true\nstdout_empty=true\nstderr_empty=true\n' > "$evidence/$label/oracle.meta"
}
selectors=(build src/main.lkj --source-dir "$source" --library-dir "$source/library" --runtime-source "$source/library/runtime.lkj")
space construct-ordinary || stop free_space_construct_ordinary
run_phase 01-construct-ordinary "$producer" "${selectors[@]}" --no-store --inline-limit 64 --stats -o "$ordinary" || stop construct_ordinary_failed
stats 01-construct-ordinary 01-construct-ordinary > "$evidence/01-construct-ordinary/validation.stdout" 2> "$evidence/01-construct-ordinary/validation.stderr" || stop ordinary_stats_failed
artifact "$ordinary" ordinary64 || stop ordinary_artifact_failed
space construct-profile || stop free_space_construct_profile
run_phase 02-construct-profile "$producer" "${selectors[@]}" --no-store --inline-limit 64 --profile --profile-ir "$companion" --stats -o "$profiled" || stop construct_profile_failed
stats 02-construct-profile 01-construct-ordinary > "$evidence/02-construct-profile/validation.stdout" 2> "$evidence/02-construct-profile/validation.stderr" || stop profile_stats_failed
artifact "$profiled" profile64 || stop profile_artifact_failed
artifact "$companion" selected-ir || stop selected_ir_artifact_failed
run_phase 03-execute-ordinary "$ordinary" "${selectors[@]}" --no-store --inline-limit 0 -o "$output_ordinary" || stop execute_ordinary_failed
oracle 03-execute-ordinary "$output_ordinary" || stop ordinary_output_oracle_failed
shopt -s nullglob
samples=("$profiled".samples.*)
((${#samples[@]} == 0)) || stop preexisting_sample_file
run_phase 04-execute-profile "$profiled" "${selectors[@]}" --no-store --inline-limit 0 -o "$output_profile" || stop execute_profile_failed
oracle 04-execute-profile "$output_profile" || stop profile_output_oracle_failed
samples=("$profiled".samples.*)
((${#samples[@]} == 1)) || stop sample_file_count
sample=${samples[0]}
artifact "$sample" raw-profile || stop sample_artifact_failed
# The sole foreground execution wrapper completed successfully; no child is launched
# between its completion and these read-only decodes. Ownership is cooperative metadata.
run_phase 05-summary "$producer" profile "$sample" || stop summary_failed
[[ ! -s $evidence/05-summary/stderr ]] || stop summary_diagnostics
run_phase 06-copy-detail "$producer" profile "$sample" --copy-sites --profile-ir "$companion" --limit 200 --bytes 1048576 || stop detail_failed
[[ ! -s $evidence/06-copy-detail/stderr && $(stat -c %s "$evidence/06-copy-detail/stdout") -le 1048576 ]] || stop detail_diagnostics_or_bound
run_phase 07-accounting "$perl_tool" "$helper" --profile "$sample" --companion "$companion" \
    --summary "$evidence/05-summary/stdout" --detail "$evidence/06-copy-detail/stdout" \
    --output "$evidence/accounting" || stop accounting_failed
[[ ! -s $evidence/07-accounting/stderr ]] || stop accounting_diagnostics
for file in accounting.json kinds.tsv functions.tsv function-map.tsv widths.tsv roles.tsv width-roles.tsv manifest.sha256; do
    [[ -f $evidence/accounting/$file && ! -L $evidence/accounting/$file && $(stat -c %h "$evidence/accounting/$file") == 1 ]] || stop accounting_output_missing
done
phase=preservation
for label in ordinary64 profile64 selected-ir raw-profile; do
    file=$(awk -F= '$1=="path" {print substr($0,6)}' "$evidence/$label.artifact")
    before_sha=$(awk -F= '$1=="sha256" {print $2}' "$evidence/$label.artifact")
    before_bytes=$(awk -F= '$1=="bytes" {print $2}' "$evidence/$label.artifact")
    [[ -f $file && ! -L $file && $(stat -c %h "$file") == 1 &&
       $(stat -c %s "$file") == "$before_bytes" &&
       $(sha256sum "$file" | awk '{print $1}') == "$before_sha" ]] || stop measured_artifact_changed
done
inventory "$source" > "$evidence/source-after.sha256" || stop source_recheck_failed
cmp -- "$evidence/source-before.sha256" "$evidence/source-after.sha256" || stop source_changed
sha256sum -c "$evidence/copied-tools.sha256" > "$evidence/copied-tools-recheck.stdout" 2> "$evidence/copied-tools-recheck.stderr" || stop copied_tools_changed
[[ $(sha256sum "$release" | awk '{print $1}') == $(sha256sum "$evidence/release.json" | awk '{print $1}') &&
   $(sha256sum "$script" | awk '{print $1}') == "$(value files launcher sha256)" &&
   $(sha256sum "$helper_input" | awk '{print $1}') == "$helper_sha" &&
   $(sha256sum "$producer_input" | awk '{print $1}') == "$producer_sha" ]] || stop released_input_changed
pending_bytes=$(( $(stat -c %s "$sample") + $(stat -c %s "$companion") ))
required=$((pending_bytes + 1073741824))
((required >= 3221225472)) || required=3221225472
space preservation "$required" || stop free_space_preservation
mkdir "$evidence/artifacts"
for pair in "$sample:profile.lkjprof" "$companion:selected.lkjir"; do
    input=${pair%:*}; name=${pair##*:}
    raw_sha=$(sha256sum "$input" | awk '{print $1}')
    compressed="$evidence/artifacts/$raw_sha.$name.gz"
    compression_status=0
    "$prlimit_tool" --as=4294967296 -- "$timeout_tool" --kill-after=10s 900s \
        "$gzip_tool" -n -c "$input" > "$compressed" \
        2> "$evidence/artifacts/$name.gzip.stderr" || compression_status=$?
    printf '%s\n' "$compression_status" > "$evidence/artifacts/$name.gzip.status"
    ((compression_status == 0)) || stop preservation_compression_failed
    set +e
    "$prlimit_tool" --as=4294967296 -- "$timeout_tool" --kill-after=10s 900s \
        "$gzip_tool" -dc "$compressed" 2> "$evidence/artifacts/$name.gunzip.stderr" |
        sha256sum > "$evidence/artifacts/$name.roundtrip.sha256" 2> "$evidence/artifacts/$name.sha256.stderr"
    roundtrip_status=("${PIPESTATUS[@]}")
    set -e
    printf '%s\n' "${roundtrip_status[@]}" > "$evidence/artifacts/$name.roundtrip.status"
    ((roundtrip_status[0] == 0 && roundtrip_status[1] == 0)) || stop preservation_decode_failed
    [[ $(awk '{print $1}' "$evidence/artifacts/$name.roundtrip.sha256") == "$raw_sha" ]] || stop preservation_digest_mismatch
    printf 'original_path=%s\noriginal_bytes=%s\noriginal_sha256=%s\ncompressed_path=%s\ncompressed_bytes=%s\ncompressed_sha256=%s\n' \
        "$input" "$(stat -c %s "$input")" "$raw_sha" "$compressed" \
        "$(stat -c %s "$compressed")" "$(sha256sum "$compressed" | awk '{print $1}')" \
        > "$evidence/artifacts/$name.meta"
done
# No automatic cleanup: raw originals, images and source remain for root acceptance.
completed=true
stop_reason=success
