#!/usr/bin/env perl
# Historical gate orchestration controls. The fixture gate is a shell comparison;
# no model, real compiler or main repository gate is invoked.
use strict;
use warnings;
use JSON::PP;
use Digest::SHA qw(sha256_hex);
use File::Temp qw(tempdir);
use File::Path qw(make_path);
use Cwd qw(abs_path);
use FindBin;
use IPC::Open3;
use IO::Select;
use Symbol qw(gensym);
my $json=JSON::PP->new->canonical->utf8;
my $harness=abs_path("$FindBin::Bin/compiler-stranger.pl");
my $fixture=tempdir('lkjscript-stranger-gates.XXXXXX',TMPDIR=>1,CLEANUP=>1);
sub bytes {open my $f,'<:raw',$_[0] or die $!;local $/;return <$f>;}
sub write_bytes {open my $f,'>:raw',$_[0] or die $!;print $f $_[1];close $f or die $!;}
sub put {write_bytes($_[0],$json->encode($_[1])."\n");}
sub get {return $json->decode(bytes($_[0]));}
sub require_true {die "$_[1]\n" unless $_[0];}
sub run {
    my (@args)=@_;my ($in,$out);my $err=gensym;
    my $pid=open3($in,$out,$err,@args);close $in;
    my $selector=IO::Select->new($out,$err);my $output='';
    while ($selector->count) {
        for my $handle ($selector->can_read) {
            my $n=sysread($handle,my $chunk,65536);die $! unless defined $n;
            if (!$n) {$selector->remove($handle);close $handle;} else {$output.=$chunk;}
        }
    }
    waitpid($pid,0);return {exit=>($?&127?128+($?&127):$?>>8),output=>$output};
}
sub ok {my $result=run(@_);die $result->{output} if $result->{exit};return $result->{output};}
my $repository="$fixture/repository";
make_path("$repository/src","$repository/library","$repository/tools","$repository/fixtures");
ok('git','init','-q',$repository);
ok('git','-C',$repository,'config','user.name','Stranger gate fixture');
ok('git','-C',$repository,'config','user.email','stranger-gate-fixture@example.invalid');
write_bytes("$repository/src/main.lkj","parent\n");
write_bytes("$repository/library/core.lkj","parent library\n");
write_bytes("$repository/tools/check.sh", "#!/bin/sh\nexit 99\n");
ok('git','-C',$repository,'add','.');
ok('git','-C',$repository,'commit','-q','-m','Fixture parent');
my $parent=ok('git','-C',$repository,'rev-parse','HEAD');chomp $parent;
write_bytes("$repository/src/main.lkj","target\n");
write_bytes("$repository/src/target-only.lkj","must be removed by parent restoration\n");
write_bytes("$repository/library/core.lkj","target library\n");
write_bytes("$repository/fixtures/expected-source","first proposal\n");
write_bytes("$repository/fixtures/expected-library","parent library\n");
write_bytes("$repository/tools/check.sh", <<'GATE');
#!/bin/sh
set -eu
printf 'visit\n' >> "$LKJSCRIPT_STORE/visits"
test "$LKJSCRIPT_GATE_JOBS" = 4
test ! -e src/target-only.lkj
cmp library/core.lkj fixtures/expected-library
cmp src/main.lkj fixtures/expected-source
printf 'ok synthetic historical gate\n'
GATE
ok('git','-C',$repository,'add','.');
ok('git','-C',$repository,'commit','-q','-m','Fixture target tests');
my $revision=ok('git','-C',$repository,'rev-parse','HEAD');chomp $revision;
my $cohort="$fixture/cohort";my @arms=('01-files','01-commands','02-files');
for my $index (0..$#arms) {
    my $run="$cohort/$arms[$index]";make_path($run);
    put("$run/state.json",{case=>{id=>$index<2?1:2,revision=>$revision},parent=>$parent,
        mode=>$index==1?'commands':'files',model=>'synthetic-gate-control',
        wall_seconds=>3,calls=>7,payload_bytes=>123,gates=>[{checker_pass=>JSON::PP::true}],
        full_repository_gate=>'not-run'});
    my $candidate={'src/main.lkj'=>$index==2?"wrong proposal\n":"first proposal\n"};
    put("$run/gate-01-files.json",$candidate);
    # Distinct serialized snapshots can describe exactly the same source input.
    if ($index==1) {write_bytes("$run/gate-01-files.json",$json->pretty->encode($candidate));$json->pretty(0);}
}
my $plan_path="$fixture/plan.json";my $output="$fixture/gates";
my $store="$fixture/store";make_path($store);
my $key="$fixture/key";write_bytes($key,'x'x32);chmod 0600,$key;
ok($^X,$harness,'gate-inputs',$repository,$plan_path,$cohort);
my $plan=get($plan_path);
require_true(@{$plan->{groups}}==2,'identical first language inputs did not share one identity');
make_path($output);write_bytes("$output/pause","pause at an idle gate boundary\n");
ok($^X,$harness,'full-gates',$repository,$plan_path,$output,$store,$key);
require_true(!-e "$store/visits",'paused population started a gate');
require_true(get("$cohort/01-files/state.json")->{full_repository_gate} eq 'not-run','pause invented gate evidence');
unlink "$output/pause" or die $!;
ok($^X,$harness,'full-gates',$repository,$plan_path,$output,$store,$key);
require_true(bytes("$store/visits") eq "visit\nvisit\n",'expected exactly two fixture gate executions');
my $files=get("$cohort/01-files/state.json");my $commands=get("$cohort/01-commands/state.json");
my $failed=get("$cohort/02-files/state.json");
require_true($files->{full_repository_gate} eq 'passed' && $commands->{full_repository_gate} eq 'passed','shared input result missing');
require_true($files->{full_gate_evidence} eq $commands->{full_gate_evidence},'identical inputs acquired distinct proof');
require_true(($files->{full_gate_shared}?1:0)+($commands->{full_gate_shared}?1:0)==1,'shared proof claimed as two executions');
require_true($failed->{full_repository_gate} eq 'failed','failed fixture became a pass');
require_true($files->{wall_seconds}==3 && $files->{calls}==7 && $files->{payload_bytes}==123,'gate association rewrote agent metrics');
my $summary="$output/summary-".sha256_hex(bytes($plan_path)).".json";
require_true(get($summary)->{new_executions}==2 && get($summary)->{complete},'complete gate population missing');
for my $group (@{$plan->{groups}}) {require_true(!-e "$output/$group->{gate_inputs_sha256}/worktree",'completed worktree retained');}
ok($^X,$harness,'full-gates',$repository,$plan_path,$output,$store,$key);
require_true(bytes("$store/visits") eq "visit\nvisit\n",'resume repeated an already complete gate');
require_true(get($summary)->{new_executions}==0 && get($summary)->{reused_proofs}==2,'resume proof accounting');
my $before=bytes("$cohort/01-files/gate-01-files.json");
put("$cohort/01-files/gate-01-files.json",{'src/main.lkj'=>"changed later\n"});
my $result=run($^X,$harness,'full-gates',$repository,$plan_path,$output,$store,$key);
require_true($result->{exit} && $result->{output}=~/first candidate snapshot changed/,'stale plan accepted');
write_bytes("$cohort/01-files/gate-01-files.json",$before);
my $first=$plan->{groups}[0]{gate_inputs_sha256};
my $proof_path="$output/$first/gate.json";my $proof=bytes($proof_path);
my $wrong=get($proof_path);$wrong->{gate_inputs_sha256}='0'x64;put($proof_path,$wrong);
$result=run($^X,$harness,'full-gates',$repository,$plan_path,$output,$store,$key);
require_true($result->{exit} && $result->{output}=~/stored gate evidence does not match/,'wrong input proof accepted');
write_bytes($proof_path,$proof);
my $incomplete="$fixture/incomplete";make_path("$incomplete/$first");
$result=run($^X,$harness,'full-gates',$repository,$plan_path,$incomplete,$store,$key);
require_true($result->{exit} && $result->{output}=~/incomplete gate output requires inspection/,'uncertain gate was restarted');
require_true(bytes("$store/visits") eq "visit\nvisit\n",'refusal executed a gate');
my $unfinished=get($plan_path);$unfinished->{unfinished}=[{run=>'unfinished'}];put("$fixture/unfinished.json",$unfinished);
$result=run($^X,$harness,'full-gates',$repository,"$fixture/unfinished.json",$output,$store,$key);
require_true($result->{exit} && $result->{output}=~/unfinished agents/,'unfinished population admitted');
my $empty={version=>1,groups=>[],unfinished=>[]};put("$fixture/empty.json",$empty);
ok($^X,$harness,'full-gates',$repository,"$fixture/empty.json","$fixture/empty-gates",$store,$key);
my @empty_summaries=glob("$fixture/empty-gates/summary-*.json");
require_true(@empty_summaries==1 && get($empty_summaries[0])->{complete},'empty submitted population missing');
print "ok stranger gate population: exact first source, historical tests, shared proof, failure, idle pause, resume, stale and incomplete refusal, preserved agent metrics\n";
