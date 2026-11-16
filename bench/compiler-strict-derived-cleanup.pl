#!/usr/bin/env perl
# DESTRUCTIVE only after explicit root release; exact accepted fixture, no glob.
use strict; use warnings; use FindBin; use Cwd qw(realpath);
use Time::HiRes qw(time); use POSIX qw(strftime);
do "$FindBin::Bin/compiler-strict-derived-cleanup-data/inventory.pl" or die $@ || $!;
@ARGV==1 && $ARGV[0] eq '--execute-after-root-release' or die "explicit root cleanup release is required\n";
my $root='/home/coder/workspace/lkjscript-compiler-strict-derived-20261116';
my $fixture="$root/target/compiler-strict-derived.ZQv7cu";
my $data="$root/bench/compiler-strict-derived-cleanup-data";
realpath("$FindBin::Bin/..") eq $root or die "wrong worktree\n";
chdir $root or die $!;
$|=1;
my $start=time;
my $load_before=derived_bytes('/proc/loadavg'); $load_before=~s/\n\z//;
print "utc_before=".strftime('%Y-%m-%dT%H:%M:%SZ',gmtime)." load_before=$load_before\n";
for my $manifest ('bench/compiler-strict-derived.results.sha256',
                  'bench/compiler-strict-derived.prepared.sha256',
                  'bench/compiler-strict-derived.producer-evidence.sha256',
                  'bench/compiler-strict-derived-cleanup.prepared.sha256') {
    system('sha256sum','--status','-c',$manifest)==0 or die "retained evidence differs: $manifest\n";
}
derived_inventory($fixture) eq derived_bytes("$data/fixture.snapshot") or die "live fixture differs from prepared ownership/content snapshot\n";
derived_bytes("$fixture/owner") eq "compiler-strict-derived-v1\n" or die "owner marker differs\n";
my $owner=derived_bytes("$fixture/pid");
$owner=~/^([1-9][0-9]*)\n$/ or die "invalid owner pid\n"; $owner=$1;
!-e "/proc/$owner" or die "original owner PID still exists\n";
for my $pair (['.lkjscript-edit.lock','before.lock'],['.lkjscript-edit.generation','before.generation']) {
    derived_bytes("$fixture/case-003/before/$pair->[0]") eq derived_bytes("$data/metadata/$pair->[1]")
        or die "completed metadata differs\n";
}
my @worktree_metadata=glob "$root/.lkjscript-edit*";
!@worktree_metadata or die "unexpected worktree edit metadata; leave it untouched\n";
my %ancestors; my $p=$$;
while ($p) {
    !$ancestors{$p}++ or die "process ancestry cycle\n";
    my $status=derived_bytes("/proc/$p/status");
    $status=~/^PPid:\s+([0-9]+)$/m or die "cannot read process ancestry\n";
    $p=$1;
}
my ($processes,$descriptors)=(0,0);
for my $process (sort glob '/proc/[0-9]*') {
    my ($pid)=$process=~m{/([0-9]+)$};
    next if $ancestors{$pid};
    my @s=stat $process; next unless @s && $s[4]==$<;
    $processes++;
    for my $name ('cmdline','maps') {
        my $path="$process/$name";
        if (open my $f,'<:raw',$path) {
            local $/; my $value=<$f>//''; close $f;
            index($value,$fixture)<0 or die "live process reference: PID $pid ($name)\n";
        } elsif (-d $process) {die "cannot inspect live process $pid/$name\n";}
    }
    for my $name ('cwd','root','exe') {
        my $link=readlink "$process/$name";
        next unless defined $link;
        index($link,$fixture)<0 or die "live process reference: PID $pid ($name)\n";
    }
    my $fd="$process/fd";
    if (opendir my $directory,$fd) {
        my @fds=grep {/^[0-9]+$/} readdir $directory; closedir $directory;
        for my $n (@fds) {
            my $link=readlink "$fd/$n"; next unless defined $link; $descriptors++;
            index($link,$fixture)<0 or die "live descriptor reference: PID $pid\n";
        }
    } elsif (-d $process) {die "cannot inspect live descriptors for PID $pid\n";}
}
print "guards=passed same_user_processes=$processes descriptors=$descriptors pending=absent staging=absent key_content=not-read\n";
print "remove_fixture=$fixture files=3706 logical_bytes=71949767\n";
system('rm','-rf','--one-file-system','--',$fixture)==0 or die "fixture removal failed\n";
!-e $fixture && !-l $fixture or die "fixture still exists\n";
for my $manifest ('bench/compiler-strict-derived.results.sha256',
                  'bench/compiler-strict-derived-cleanup.prepared.sha256') {
    system('sha256sum','--status','-c',$manifest)==0 or die "preserved evidence changed: $manifest\n";
}
my $load_after=derived_bytes('/proc/loadavg');$load_after=~s/\n\z//;
printf "removed=true elapsed_seconds=%.9f utc_after=%s load_after=%s\n",time-$start,strftime('%Y-%m-%dT%H:%M:%SZ',gmtime),$load_after;
