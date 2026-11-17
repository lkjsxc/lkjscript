#!/usr/bin/env perl
# Adapter/accounting regressions only; no model or compiler is invoked.
use strict;
use warnings;
use JSON::PP;
use Encode qw(encode decode);
use File::Temp qw(tempdir);
use File::Path qw(make_path);
use File::Copy qw(copy);
use Cwd qw(abs_path);
use FindBin;
use IPC::Open3;
use Symbol qw(gensym);
use Time::HiRes qw(time sleep);
use POSIX qw(WNOHANG);

my $json=JSON::PP->new->canonical->utf8;
my $text_json=JSON::PP->new->canonical;
my $harness=abs_path("$FindBin::Bin/compiler-stranger.pl");
my $fixture=tempdir('lkjscript-stranger-adapter.XXXXXX',TMPDIR=>1,CLEANUP=>1);
sub bytes {open my $f,'<:raw',$_[0] or die $!;local $/;return <$f>;}
sub write_bytes {open my $f,'>:raw',$_[0] or die $!;print $f $_[1];close $f or die $!;}
sub put {write_bytes($_[0],$json->encode($_[1])."\n");}
sub get {return $json->decode(bytes($_[0]));}
sub run {
    my ($input,@argv)=@_;my ($in,$out);my $err=gensym;
    my $pid=open3($in,$out,$err,@argv);binmode $in;binmode $out;binmode $err;
    print $in $input;close $in;
    local $/;my $stdout=<$out>//'';my $stderr=<$err>//'';
    close $out;close $err;waitpid($pid,0);
    die "child failed: $stderr" if $?;
    return $stdout;
}
sub require_true {die "$_[1]\n" unless $_[0];}
my $run="$fixture/utf8";
make_path("$run/source/src","$run/logs");
put("$run/state.json",{mode=>'files',calls=>0,max_calls=>2,payload_bytes=>0,response_bytes=>0,gates=>[],baseline=>{}});
my $sample=pack('U*',0x3c0,0x1f642)."\n";
write_bytes("$run/source/src/main.lkj",encode('UTF-8',$sample));
my @requests=(
    {jsonrpc=>'2.0',id=>1,method=>'tools/call',params=>{name=>'read',arguments=>{path=>'src/main.lkj'}}},
    {jsonrpc=>'2.0',id=>2,method=>'tools/call',params=>{name=>'write',arguments=>{path=>'src/main.lkj',text=>$sample}}},
    {jsonrpc=>'2.0',id=>3,method=>'tools/call',params=>{name=>'gate',arguments=>{}}},
);
my $wire=run(join('',map {$json->encode($_)."\n"} @requests),$^X,$harness,'serve',$run);
my @replies=map {$json->decode($_)} grep {length} split /\n/,$wire;
require_true(@replies==3,'response count');
my $read=$text_json->decode($replies[0]{result}{content}[0]{text});
my $write=$text_json->decode($replies[1]{result}{content}[0]{text});
require_true($read->{text} eq $sample,'non-ASCII source changed in MCP transit');
require_true($write->{written_bytes}==length(encode('UTF-8',$sample)),'written byte count');
require_true($replies[2]{result}{isError},'exhausted gate must remain refused');
require_true($text_json->decode($replies[2]{result}{content}[0]{text})->{error}=~/allowance exhausted/,'gate refusal reason');
require_true(get("$run/gate-01-files.json")->{'src/main.lkj'} eq $sample,'first attempted gate candidate lost after budget refusal');
for my $i (0..2) {
    my $record=get(sprintf("$run/logs/tool-%03d.json",$i+1));
    require_true($record->{payload_bytes}==length(encode('UTF-8',$replies[$i]{result}{content}[0]{text})),'payload byte count');
    require_true($record->{response_bytes}==length($json->encode($replies[$i]{result})),'MCP result byte count');
    require_true($record->{adapter_sha256}=~/^[0-9a-f]{64}$/,'adapter identity missing');
}

# Successful help must not become successful edit acceptance in the report.
my $cohort="$fixture/report";my $arm="$cohort/01-commands";
make_path("$arm/logs");
put("$arm/state.json",{case=>{id=>1},mode=>'commands',model=>'synthetic-adapter-test',
    calls=>4,gates=>[],wall_seconds=>1,setup_seconds=>2,payload_bytes=>0,response_bytes=>0,
    timed_out=>JSON::PP::false,pause_timeout=>JSON::PP::true,full_repository_gate=>'not-run'});
my @events=(
    {number=>1,tool=>'compiler',arguments=>{argv=>['edit','--help']},result=>{exit=>0,stdout=>'help'}},
    {number=>2,tool=>'compiler',arguments=>{argv=>['edit'],request=>'{}'},result=>{exit=>1,stderr=>'E_EDIT_VERSION expected version 1'}},
    {number=>3,tool=>'compiler',arguments=>{argv=>['edit','--dry-run'],request=>'{"version":1,"operations":[{"op":"replace-body","id":"f","source":"{ 1 }"}]}'},result=>{exit=>0}},
    {number=>4,tool=>'gate',arguments=>{},result=>{error=>'experiment allowance exhausted'}},
);
for my $event (@events) {put(sprintf("$arm/logs/tool-%03d.json",$event->{number}),$event);}
put("$arm/coordinator-pauses.json",[{stage=>'setup',seconds=>0.25}]);
my $rows=$json->decode(run('',$^X,$harness,'report',$cohort));
my $metrics=$rows->[0]{edit_metrics};
require_true($metrics->{first_edit_invocation}{kind} eq 'help' && $metrics->{first_edit_invocation}{exit}==0,'help attribution');
require_true($metrics->{first_edit_request}{exit}==1 && $metrics->{first_edit_request}{diagnostic} eq 'E_EDIT_VERSION','request attribution');
require_true($metrics->{edit_request_count}==2 && $metrics->{requests_before_first_gate}==2,'request counts');
require_true($metrics->{gate_invocation_count}==1 && !$rows->[0]{gate_called},'refused gate attribution');
require_true($rows->[0]{timed_out},'pause timeout lost from report');
require_true($rows->[0]{setup_seconds}==2 && $rows->[0]{setup_pause_seconds}==0.25 && $rows->[0]{setup_active_seconds}==1.75,'raw setup and idle pause were conflated');
require_true(!defined($rows->[0]{preparation_active_seconds}),'missing complete preparation became a measured value');

# Pause synthetic coordinators during an agent and an index setup. No real
# compiler, agent or campaign is signalled. Preserve agent exit time and record
# setup idle time separately before the coordinator resumes.
my $coordinator_code=<<'COORDINATOR';
use strict;use warnings;use Time::HiRes qw(time);
my ($out,$child)=@ARGV;
open my $pid,'>',"$out/coordinator.pid" or die $!;print $pid "$$\n";close $pid;
my $p=fork();die $! unless defined $p;if (!$p) {exec $child,'0.8';die $!;}
open my $ready,'>',"$out/ready" or die $!;print $ready "$p\n";close $ready;
waitpid($p,0);
open my $done,'>',"$out/resumed" or die $!;print $done time;close $done;
COORDINATOR
my $own_coordinator;
END {
    if ($own_coordinator && waitpid($own_coordinator,WNOHANG)==0) {
        kill 'CONT',$own_coordinator;kill 'TERM',$own_coordinator;waitpid($own_coordinator,0);
    }
}
for my $stage ('agent','setup') {
    my $pause_cohort="$fixture/pause-$stage";my $pause_arm="$pause_cohort/01-files";
    make_path($pause_arm);
    put("$pause_arm/state.json",$stage eq 'agent'?{started=>time,calls=>1}:{setup_started=>time,calls=>0});
    my $child="$fixture/".($stage eq 'agent'?'codex':'compiler');
    copy('/bin/sleep',$child) or die $!;chmod 0755,$child;
    my ($coordinator_in,$coordinator_out);my $coordinator_err=gensym;
    my $coordinator_pid=open3($coordinator_in,$coordinator_out,$coordinator_err,
        $^X,'-e',$coordinator_code,$pause_cohort,$child,10,24);
    close $coordinator_in;$own_coordinator=$coordinator_pid;
    my $deadline=time+5;
    while (!-f "$pause_cohort/ready") {die "synthetic coordinator not ready" if time>$deadline;sleep 0.01;}
    my $paused=$json->decode(run('',$^X,$harness,'pause',$pause_cohort));
    require_true($paused->{drained},'pause did not drain');
    my $paused_state=get("$pause_arm/state.json");
    my $observed=$paused_state->{finished};
    if ($stage eq 'agent') {require_true($observed && $paused_state->{finish_observation},'drained exit timestamp missing');}
    else {require_true(!defined($observed),'setup was mistaken for a finished agent');}
    sleep 0.1;
    run('',$^X,$harness,'resume',$pause_cohort);
    waitpid($coordinator_pid,0);$own_coordinator=undef;
    require_true(!$?,'synthetic coordinator failed');
    if ($stage eq 'agent') {
        require_true(get("$pause_arm/state.json")->{finished}==$observed,'resume changed observed finish time');
        require_true(bytes("$pause_cohort/resumed")>$observed,'finish was timestamped at resume');
    } else {
        my $intervals=get("$pause_arm/coordinator-pauses.json");
        require_true(@$intervals==1 && $intervals->[0]{stage} eq 'setup' && $intervals->[0]{seconds}>=0.09,'setup idle interval missing');
        require_true(!get("$pause_arm/state.json")->{started},'resume invented an agent start');
    }
    close $coordinator_out;close $coordinator_err;
}
print "ok stranger adapter: UTF-8 bytes, request/help attribution, budget-refused candidate, drained wall clock, setup idle and timeout report\n";
