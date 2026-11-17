#!/usr/bin/env perl
use strict;
use warnings;
use JSON::PP;
use Encode qw(encode decode FB_CROAK);
use Digest::SHA qw(sha256_hex);
use File::Path qw(make_path remove_tree);
use File::Copy qw(copy);
use File::Find qw(find);
use Cwd qw(abs_path);
use Time::HiRes qw(time sleep);
use IPC::Open3;
use IO::Select;
use Symbol qw(gensym);
use POSIX qw(setsid WNOHANG);

my $json = JSON::PP->new->canonical->utf8;
my $script = abs_path($0);
(my $root = $script) =~ s{/bench/[^/]+$}{};
sub load_json { my ($path)=@_; open my $f,'<:raw',$path or die "$path: $!"; local $/; return $json->decode(<$f>); }
sub save_json { my ($path,$value)=@_; open my $f,'>:raw',$path or die "$path: $!"; print $f $json->encode($value),"\n"; close $f or die $!; }
sub read_bytes { my ($path)=@_; open my $f,'<:raw',$path or die "$path: $!"; local $/; return <$f>; }
sub read_text { return decode('UTF-8',read_bytes($_[0]),FB_CROAK); }
sub write_text { write_bytes($_[0],encode('UTF-8',$_[1])); }
sub write_bytes { my ($path,$bytes)=@_; open my $f,'>:raw',$path or die "$path: $!"; print $f $bytes; close $f or die $!; }
sub capture {
    my ($cwd,$stdin,@command)=@_;
    my $old=Cwd::getcwd(); chdir $cwd or die "$cwd: $!";
    my ($in,$out); my $err=gensym;
    my $pid=open3($in,$out,$err,@command);
    chdir $old or die $!;
    binmode $in; binmode $out; binmode $err;
    print $in $stdin if defined $stdin;
    close $in;
    my $select=IO::Select->new($out,$err);
    my ($stdout,$stderr)=('','');
    my $out_fileno=fileno($out);
    while ($select->count) {
        for my $handle ($select->can_read) {
            my $count=sysread($handle,my $chunk,65536);
            die "read child: $!" unless defined $count;
            if (!$count) {$select->remove($handle);close $handle;next;}
            if (fileno($handle)==$out_fileno) {$stdout.=$chunk;} else {$stderr.=$chunk;}
        }
    }
    waitpid($pid,0);
    return {exit=>($?&127 ? 128+($?&127) : $?>>8),stdout=>$stdout,stderr=>$stderr};
}
sub require_ok { my ($r,$what)=@_; die "$what failed: $r->{stderr}\n$r->{stdout}" if $r->{exit}; return $r->{stdout}; }
sub source_files {
    my ($dir)=@_; my @files;
    find({no_chdir=>1,wanted=>sub {return unless -f $_ && !-l $_; (my $path=$_) =~ s{^\Q$dir\E/}{}; push @files,$path if $path =~ m{^(src|library|tools|tests)/};}},$dir);
    return [sort @files];
}
sub source_path {
    my ($run,$path)=@_;
    die "path outside granted source trees" unless defined $path && $path =~ m{^(src|library|tools|tests)/} && $path !~ m{(?:^|/)\.\.?(?:/|$)|//|/$} && $path !~ /\0/;
    my $full="$run/source/$path";
    my $check=$full;
    while ($check ne "$run/source") {die "symlink path refused" if -l $check; $check =~ s{/[^/]+$}{};}
    return $full;
}
sub archive {
    my ($repository,$revision,$destination)=@_;
    make_path($destination);
    my $r=capture($repository,undef,'git','archive','--format=tar',$revision,'src','library','tools','tests');
    require_ok($r,"archive $revision");
    require_ok(capture($destination,$r->{stdout},'tar','-xf','-'),"extract $revision");
    my $files=source_files($destination);
    for my $path (@$files) {die "historical .rs needs an explicit projection policy" if $path =~ /\.rs$/ && $path =~ m{^(src|library)/};}
}
sub selector {
    my ($run)=@_;
    return ('--source-dir',"$run/source",'--library-dir',"$run/source/library",
        '--store',"$run/store",'--store-key',"$run/key",'--store-bytes','2147483648');
}
sub compiler_run {
    my ($run,$state,$command,@args)=@_;
    my $r=$command eq '--help' ? capture("$run/source",undef,$state->{binary},'--help')
        : capture("$run/source",undef,$state->{binary},$command,'src/main.lkj',selector($run),@args);
    $r->{stdout}=decode('UTF-8',$r->{stdout},FB_CROAK);
    $r->{stderr}=decode('UTF-8',$r->{stderr},FB_CROAK);
    return $r;
}
sub prepare {
    my ($repository,$binary,$run,$case,$mode,$model,$max_calls)=@_;
    die "existing run $run" if -e $run;
    make_path("$run/agent","$run/store","$run/logs");
    my $parent=require_ok(capture($repository,undef,'git','rev-parse',"$case->{revision}^"),'parent'); chomp $parent;
    archive($repository,$parent,"$run/source");
    archive($repository,$case->{revision},"$run/desired");
    open my $random,'<:raw','/dev/urandom' or die $!;
    my $count=read($random,my $key,32); die "short entropy read" unless $count==32; close $random;
    write_bytes("$run/key",$key); chmod 0600,"$run/key";
    my %baseline=map {$_=>sha256_hex(read_bytes("$run/source/$_"))} @{source_files("$run/source")};
    my $state={version=>1,case=>$case,mode=>$mode,model=>$model,parent=>$parent,
        binary=>$binary,binary_sha256=>sha256_hex(read_bytes($binary)),harness_sha256=>sha256_hex(read_bytes($script)),
        calls=>0,payload_bytes=>0,response_bytes=>0,gates=>[],max_calls=>$max_calls,
        setup_started=>time,baseline=>\%baseline,full_repository_gate=>"not-run"};
    save_json("$run/state.json",$state);
    if ($mode eq 'commands') {
        my $r=compiler_run($run,$state,'build','--index','-o',"$run/index-image",'--stats');
        save_json("$run/logs/setup-index.json",$r);
        if (!$r->{exit} && $r->{stdout}=~/LKJSCRIPT_SNAPSHOT ([0-9a-f]{64})/) {$state->{initial_snapshot}=$1;}
        else {$state->{setup_error}="index failed";$state->{setup_result}=$r;}
    }
    $state->{setup_seconds}=time-$state->{setup_started};
    $state->{started}=time;
    save_json("$run/state.json",$state);
    return $state;
}
sub candidate_changes {
    my ($run,$state)=@_;my %changed;
    for my $path (@{source_files("$run/source")}) {
        my $bytes=read_bytes("$run/source/$path");
        $changed{$path}=decode('UTF-8',$bytes,FB_CROAK)
            if !exists $state->{baseline}{$path} || sha256_hex(read_bytes("$run/source/$path")) ne $state->{baseline}{$path};
    }
    for my $path (keys %{$state->{baseline}}) {$changed{$path}=undef unless -e "$run/source/$path";}
    return \%changed;
}
sub gate {
    my ($run,$state)=@_;
    save_json(sprintf("$run/gate-%02d-files.json",scalar(@{$state->{gates}})+1),candidate_changes($run,$state));
    my $check=compiler_run($run,$state,'check');
    my $result={checker_pass=>$check->{exit}==0?JSON::PP::true:JSON::PP::false,
        checker=>$check,canonical_target_pass=>undef,full_repository_gate=>"not-run",
        scope=>"current producer checks complete historical selected root; canonical target equality is stricter than behavior"};
    if (!$check->{exit}) {
        my $compare=compiler_run($run,$state,'compare','--against','src/main.lkj',
            '--against-source-dir',"$run/desired",'--against-library-dir',"$run/desired/library",'--limit','8');
        $result->{comparison}=$compare;
        $result->{canonical_target_pass}=JSON::PP::false if !$compare->{exit};
        if (!$compare->{exit} && $compare->{stdout}=~/identity-domain=lkjscript-checked-meaning-1 before=([0-9a-f]{64}) after=([0-9a-f]{64})/ && $1 eq $2) {
            $result->{canonical_target_pass}=JSON::PP::true;
        }
    }
    push @{$state->{gates}},$result;
    return $result;
}
sub perform {
    my ($run,$state,$tool,$a)=@_;
    die "experiment tool-call allowance exhausted ($state->{max_calls})" if $state->{calls}>$state->{max_calls};
    if ($tool eq 'gate') {return gate($run,$state);}
    if ($tool eq 'finish') {$state->{finished}=time;$state->{reason}=$a->{reason}//'';return {finished=>JSON::PP::true};}
    if ($tool eq 'compiler') {
        die "compiler commands unavailable in files arm" unless $state->{mode} eq 'commands';
        my @args=@{$a->{argv}//[]};
        my $command=shift(@args)//'--help';
        die "only --help, query and edit are available" unless $command=~/^(--help|query|edit)$/;
        if ($command eq '--help') {die "help takes no arguments" if @args; return compiler_run($run,$state,'--help');}
        my %values=map {$_=>1} qw(--kind --id --snapshot --cursor --depth --limit --bytes);
        my %flags=map {$_=>1} qw(--help --dry-run --recover --stats);
        for (my $i=0;$i<@args;$i++) {
            if ($values{$args[$i]}) {die "missing option value" if ++$i>=@args; next;}
            die "option outside experiment: $args[$i]" unless $flags{$args[$i]};
        }
        if (exists $a->{request}) {
            die "request only applies to edit" unless $command eq 'edit';
            my $raw=ref($a->{request})?$json->encode($a->{request}):$a->{request};
            write_text("$run/request.json",$raw);
            push @args,'--request',"$run/request.json";
        }
        my $r=compiler_run($run,$state,$command,@args);
        if ($command eq 'edit') {push @{$state->{edit_exits}},$r->{exit};}
        return $r;
    }
    die "file tools unavailable in commands arm" unless $state->{mode} eq 'files';
    if ($tool eq 'list') {
        my $prefix=$a->{prefix}//'';
        return {paths=>[grep {index($_,$prefix)==0} @{source_files("$run/source")}]};
    }
    if ($tool eq 'read') {
        my $bytes=read_bytes(source_path($run,$a->{path}));
        my $offset=$a->{offset}//0; my $length=$a->{length}//length($bytes);
        die "invalid range" if $offset<0 || $length<0;
        return {path=>$a->{path},offset=>$offset,total_bytes=>length($bytes),text=>decode('UTF-8',substr($bytes,$offset,$length),FB_CROAK)};
    }
    if ($tool eq 'search') {
        my $needle=$a->{literal}//'';die "empty search" if !length($needle);
        my $limit=$a->{limit}//40;die "invalid limit" if $limit<1;
        my @hits;my $prefix=$a->{prefix}//'';
        FILE: for my $path (@{source_files("$run/source")}) {
            next if index($path,$prefix)!=0;
            my @lines=split /\n/,read_text("$run/source/$path"),-1;
            for my $i (0..$#lines) {
                next if index($lines[$i],$needle)<0;
                push @hits,{path=>$path,line=>$i+1,text=>$lines[$i]};
                last FILE if @hits >= $limit;
            }
        }
        return {hits=>\@hits,limit=>$limit};
    }
    if ($tool eq 'replace' || $tool eq 'write') {die "writes are limited to the src/library task scope" unless ($a->{path}//'') =~ m{^(src|library)/};}
    if ($tool eq 'replace') {
        my $path=source_path($run,$a->{path});
        my $text=read_text($path); my $old=$a->{old}//'';my $new=$a->{new}//'';
        die "replacement requires nonempty exact old text" unless length($old);
        my $at=index($text,$old);die "old text absent" if $at<0;
        die "old text is ambiguous" if index($text,$old,$at+length($old))>=0;
        substr($text,$at,length($old),$new);
        write_text($path,$text);
        return {path=>$a->{path},written_bytes=>length($text)};
    }
    if ($tool eq 'write') {
        my $path=source_path($run,$a->{path});
        my $parent=$path;$parent=~s{/[^/]+$}{};
        die "parent directory absent" unless -d $parent;
        my $text=$a->{text}//'';
        write_text($path,$text);
        return {path=>$a->{path},written_bytes=>length($text)};
    }
    die "unknown experiment tool";
}
sub schema {
    my ($properties,@required)=@_;
    return {type=>'object',properties=>$properties,required=>\@required,additionalProperties=>JSON::PP::false};
}
sub tool_definitions {
    my ($state)=@_;
    my $s={type=>'string'}; my $n={type=>'integer',minimum=>0};
    my @tools=(
        {name=>'gate',description=>'Check the current candidate and compare its canonical checked meaning to the hidden historical target. This is a historical case check, not the full repository gate.',inputSchema=>schema({})},
        {name=>'finish',description=>'Finish this attempt, explaining what succeeded or blocked you.',inputSchema=>schema({reason=>$s},'reason')});
    if ($state->{mode} eq 'commands') {
        unshift @tools,{name=>'compiler',description=>'Run the supplied binary with raw --help, query, or edit arguments. The source selector, library, store and private key are injected. request, if supplied, is the complete raw request file content passed to --request; its schema is determined only by the binary. No source files are accessible by other means.',
            inputSchema=>schema({argv=>{type=>'array',items=>$s},request=>$s},'argv')};
    } else {
        unshift @tools,
            {name=>'list',description=>'List source file paths with an optional prefix.',inputSchema=>schema({prefix=>$s})},
            {name=>'read',description=>'Read exact file text, optionally selecting a byte range.',inputSchema=>schema({path=>$s,offset=>$n,length=>$n},'path')},
            {name=>'search',description=>'Search source lines for a literal string.',inputSchema=>schema({literal=>$s,prefix=>$s,limit=>{type=>'integer',minimum=>1}},'literal')},
            {name=>'replace',description=>'Replace one exact unique piece of file text.',inputSchema=>schema({path=>$s,old=>$s,new=>$s},qw(path old new))},
            {name=>'write',description=>'Write the complete text of a source file beneath an existing directory.',inputSchema=>schema({path=>$s,text=>$s},qw(path text))};
    }
    return \@tools;
}
sub serve {
    my ($run)=@_;
    $|=1;
    while (my $line=<STDIN>) {
        my $request=eval {$json->decode($line)};next unless $request && exists $request->{id};
        my $id=$request->{id};my $method=$request->{method}//'';
        my $state=load_json("$run/state.json");
        my $response;
        if ($method eq 'initialize') {
            $response={protocolVersion=>'2024-11-05',capabilities=>{tools=>{}},serverInfo=>{name=>'lkjscript-stranger',version=>'1'}};
        } elsif ($method eq 'ping') {$response={};}
        elsif ($method eq 'tools/list') {$response={tools=>tool_definitions($state)};}
        elsif ($method eq 'tools/call') {
            $state->{calls}++;
            my $t0=time;my $tool=$request->{params}{name};my $args=$request->{params}{arguments}//{};
            my $result=eval {perform($run,$state,$tool,$args)};
            my $error=$@;
            $result={error=>$error} if $error;
            my $payload=$json->encode($result);
            $response={content=>[{type=>'text',text=>$payload}],isError=>$error?JSON::PP::true:JSON::PP::false};
            my $response_bytes=length($json->encode($response));
            $state->{payload_bytes}+=length($payload);
            $state->{response_bytes}+=$response_bytes;
            my $record={number=>$state->{calls},tool=>$tool,arguments=>$args,result=>$result,seconds=>time-$t0,
                payload_bytes=>length($payload),response_bytes=>$response_bytes};
            save_json(sprintf("$run/logs/tool-%03d.json",$state->{calls}),$record);
            save_json("$run/state.json",$state);
        } else {
            print $json->encode({jsonrpc=>'2.0',id=>$id,error=>{code=>-32601,message=>'method not found'}}),"\n";next;
        }
        print $json->encode({jsonrpc=>'2.0',id=>$id,result=>$response}),"\n";
    }
}
sub finish_run {
    my ($run)=@_;my $state=load_json("$run/state.json");
    $state->{finished}//=time;$state->{wall_seconds}=$state->{finished}-$state->{started};
    my %changed;
    for my $path (@{source_files("$run/source")}) {
        my $bytes=read_bytes("$run/source/$path");
        $changed{$path}=decode('UTF-8',$bytes,FB_CROAK) if !exists $state->{baseline}{$path} || sha256_hex($bytes) ne $state->{baseline}{$path};
    }
    for my $path (keys %{$state->{baseline}}) {$changed{$path}=undef unless -e "$run/source/$path";}
    save_json("$run/final-files.json",\%changed);
    $state->{changed_files}=[sort keys %changed];
    if (-f "$run/agent-trace.jsonl") {
        my $raw=read_bytes("$run/agent-trace.jsonl");my ($attempted,$outside)=(0,0);
        for my $line (split /\n/,$raw) {
            my $event=eval {$json->decode($line)};next unless $event && ($event->{type}//'') eq 'item.started';
            my $item=$event->{item}//{};
            if (($item->{type}//'') eq 'mcp_tool_call') {$attempted++;$outside++ if ($item->{server}//'') ne 'stranger';}
            elsif (($item->{type}//'') =~ /^(command_execution|file_change|web_search)$/) {$outside++;}
        }
        $state->{attempted_tool_calls}=$attempted;
        $state->{outside_tool_calls}=$outside;
        if (!$state->{calls} && $raw =~ /MCP tool call requires approval/) {
            $state->{setup_error}='CLI rejected fixture tool approval before execution';
        }
    }
    delete $state->{baseline};
    save_json("$run/state.json",$state);
    return $state;
}
sub prompt {
    my ($state)=@_;
    my $c=$state->{case};my $paths=join(', ',@{$c->{paths}});
    my $access=$state->{mode} eq 'commands'
        ? "Only the compiler MCP tool is available for reading/changing the program. Learn from what the binary itself prints; no documentation or source filesystem tools are available. Initial indexed snapshot: ".($state->{initial_snapshot}//'unavailable')."."
        : "Only ordinary file listing, reading, searching and text editing MCP tools are available for the program. Do not use structural query/edit commands.";
    return "You are an independent programmer seeing this program and language for the first time. Complete this real historical change.\n\nTASK: $c->{task}\nLikely affected files: $paths\n\n$access\nThe surrounding source tree and library have already been selected. Their configuration is infrastructure, not a hint about the edit. No desired implementation is visible.\nUse the common gate tool when your first complete candidate is ready; its checker result and exact historical target equality are separate measurements. Exact equality is stricter than behavioral equivalence. You may make a correction after a failed check, but every attempt is recorded. The full tools/check.sh repository gate is a separate not-run metric unless the coordinator runs it.\nCall finish with a concise reason when done or blocked. Do not ask anyone for hints. This experiment grants at most $state->{max_calls} tool calls and a finite external wall-time allowance. Use no tools other than this experiment's MCP tools. Do not use filesystem, shell, web, other agents, prior project instructions or external documentation. A blocked attempt is useful evidence; do not invent success.\n";
}
sub run_agent {
    my ($run,$state,$seconds)=@_;
    my $text=prompt($state);write_text("$run/prompt.txt",$text);
    $state->{agent_allowance_seconds}=$seconds;
    $state->{task_bytes}=length($text);save_json("$run/state.json",$state);
    my $pid=fork();die "fork: $!" unless defined $pid;
    if (!$pid) {
        setsid()>=0 or die "setsid: $!";
        open STDIN,'<:raw',"$run/prompt.txt" or die $!;
        open STDOUT,'>:raw',"$run/agent-trace.jsonl" or die $!;
        open STDERR,'>:raw',"$run/agent-stderr.txt" or die $!;
        exec 'codex','exec','--ephemeral','--skip-git-repo-check',
            '--model',$state->{model},'-c','model_reasoning_effort="medium"',
            '--approve-for-me','--cd',"$run/agent",'--trust','--json','--color','never',
            '--disable','shell_tool','--disable','unified_exec','--disable','apps',
            '--disable','browser_use','--disable','computer_use','--disable','skill_search',
            '--enable','skip_host_skill_discovery','-c','web_search="disabled"',
            '-c','mcp_servers.stranger.command="perl"',
            '-c','mcp_servers.stranger.args='.$json->encode([$script,'serve',$run]),
            '-c','mcp_servers.stranger.tool_timeout_sec=900',
            '-o',"$run/agent-final.txt",'-';
        die "exec codex: $!";
    }
    my $started=time;my $timed_out=0;
    while (waitpid($pid,WNOHANG)==0) {
        if (time-$started>$seconds) {$timed_out=1;kill 'TERM',-$pid;sleep 1;kill 'KILL',-$pid;waitpid($pid,0);last;}
        sleep 0.25;
    }
    my $exit=$?&127 ? 128+($?&127) : $?>>8;
    $state=load_json("$run/state.json");
    $state->{agent_exit}=$exit;$state->{timed_out}=$timed_out?JSON::PP::true:JSON::PP::false;
    save_json("$run/state.json",$state);
    return finish_run($run);
}
sub report {
    my ($out)=@_;
    my @rows;
    for my $path (sort glob("$out/*/state.json")) {
        my $s=load_json($path);my $g=$s->{gates}[0]//{};
        my $target=$g->{comparison} && $g->{comparison}{exit} ? undef : $g->{canonical_target_pass};
        push @rows,{case=>$s->{case}{id},mode=>$s->{mode},model=>$s->{model},calls=>$s->{calls},
            task_bytes=>$s->{task_bytes}//0,payload_bytes=>$s->{payload_bytes},response_bytes=>$s->{response_bytes},
            wall_seconds=>$s->{wall_seconds}//0,setup_seconds=>$s->{setup_seconds},
            checker_first=>$g->{checker_pass},target_first=>$target,
            attempted_tool_calls=>$s->{attempted_tool_calls},outside_tool_calls=>$s->{outside_tool_calls},setup_error=>$s->{setup_error},
            gate_called=>@{$s->{gates}}?JSON::PP::true:JSON::PP::false,full_repository_gate=>$s->{full_repository_gate},
            first_edit_exit=>$s->{edit_exits}[0],reason=>$s->{reason}//'',timed_out=>$s->{timed_out}//JSON::PP::false,
            path=>$path};
    }
    save_json("$out/results.json",\@rows);
    print $json->encode(\@rows),"\n";
}
sub recover_first_candidate {
    my ($repository,$run,$state)=@_;
    my $path="$run/gate-01-files.json";
    return load_json($path) if -f $path;
    die "no first candidate was submitted" unless @{$state->{gates}};
    die "missing command-arm snapshot requires inspection" unless $state->{mode} eq 'files';
    my %files;my @used;my $saw_gate=0;
    for my $log (sort glob("$run/logs/tool-*.json")) {
        my $record=load_json($log);
        if ($record->{tool} eq 'gate') {$saw_gate=1;last;}
        next unless $record->{tool} eq 'replace' || $record->{tool} eq 'write';
        next if exists $record->{result}{error};
        my $a=$record->{arguments};my $name=$a->{path};
        die "candidate change outside language scope" unless $name=~m{^(src|library)/} && $name!~m{(?:^|/)\.\.?(?:/|$)|//|/$};
        if ($record->{tool} eq 'write') {$files{$name}=$a->{text};}
        else {
            if (!exists $files{$name}) {
                my $r=capture($repository,undef,'git','show',"$state->{parent}:$name");
                $files{$name}=decode('UTF-8',require_ok($r,'recover original source'),FB_CROAK);
            }
            my $at=index($files{$name},$a->{old});
            die "recorded replacement cannot be replayed uniquely" if $at<0 || index($files{$name},$a->{old},$at+length($a->{old}))>=0;
            substr($files{$name},$at,length($a->{old}),$a->{new});
        }
        push @used,$record->{number};
    }
    die "first gate record is missing" unless $saw_gate;
    save_json($path,\%files);
    save_json("$run/gate-01-recovery.json",{method=>'deterministic replay of recorded successful text edits up to first gate',
        parent=>$state->{parent},tool_records=>\@used,snapshot_sha256=>sha256_hex(read_bytes($path))});
    return \%files;
}
sub full_gate {
    my ($repository,$run,$out)=@_;
    my $state=load_json("$run/state.json");
    my $files=recover_first_candidate($repository,$run,$state);
    die "full-gate output already exists" if -e $out;
    make_path($out);$out=abs_path($out);
    my $tree="$out/worktree";
    require_ok(capture($repository,undef,'git','worktree','add','--detach',$tree,$state->{case}{revision}),'create historical gate tree');
    require_ok(capture($tree,undef,'git','restore','--source',$state->{parent},'--worktree','--','src','library'),'restore pre-change language sources');
    for my $path (sort keys %$files) {
        die "candidate change outside language scope" unless $path=~m{^(src|library)/} && $path!~m{(?:^|/)\.\.?(?:/|$)|//|/$};
        my $file="$tree/$path";
        if (!defined $files->{$path}) {unlink $file or die "$file: $!" if -e $file;next;}
        my $parent=$file;$parent=~s{/[^/]+$}{};make_path($parent);
        die "candidate symlink refused" if -l $file;
        write_text($file,$files->{$path});
    }
    my $started=time;
    my $result=capture($tree,undef,'env','LKJSCRIPT_GATE_JOBS=4','bash','tools/check.sh');
    my $evidence={version=>1,case=>$state->{case}{id},arm=>$state->{mode},candidate=>1,
        source_parent=>$state->{parent},test_revision=>$state->{case}{revision},
        candidate_snapshot_sha256=>sha256_hex(read_bytes("$run/gate-01-files.json")),
        started=>$started,seconds=>time-$started,exit=>$result->{exit},
        stdout=>$result->{stdout},stderr=>$result->{stderr},
        compiler_policy=>'historical tools/check.sh bootstraps the saved candidate compiler and runs its own tests/fixed point/mutants; frozen trial executor is not used'};
    if (-f "$tree/target/lkjscript") {$evidence->{candidate_compiler_sha256}=sha256_hex(read_bytes("$tree/target/lkjscript"));}
    save_json("$out/gate.json",$evidence);
    $state->{full_repository_gate}=$result->{exit}?'failed':'passed';
    $state->{full_gate_evidence}="$out/gate.json";
    $state->{full_gate_seconds}=$evidence->{seconds};
    save_json("$run/state.json",$state);
    require_ok(capture($repository,undef,'git','worktree','remove','--force',$tree),'remove owned historical gate tree');
    print $json->encode({case=>$state->{case}{id},arm=>$state->{mode},candidate=>1,
        full_repository_gate=>$state->{full_repository_gate},seconds=>$evidence->{seconds},evidence=>"$out/gate.json"}),"\n";
}

sub process_info {
    my ($pid)=@_;
    my $raw=eval {read_bytes("/proc/$pid/stat")};return unless defined $raw;
    return unless $raw =~ /^\d+ \((.*)\) (.*)$/s;
    my ($name,$rest)=($1,$2);my @parts=split / /,$rest;
    return {pid=>$pid,name=>$name,state=>$parts[0],start=>$parts[19]};
}
sub coordinator {
    my ($out)=@_;
    my @pids;
    if (-f "$out/coordinator.pid") {my $pid=read_bytes("$out/coordinator.pid");chomp $pid;push @pids,$pid if $pid=~/^\d+$/;}
    else {
        for my $path (glob('/proc/[0-9]*/cmdline')) {
            my $raw=eval {read_bytes($path)};next unless defined $raw;
            my @args=split /\0/,$raw;
            next unless grep {$_ eq $out} @args;
            next unless grep {$_ eq 'campaign'} @args;
            next unless grep {m{(?:^|/)compiler-stranger\.pl$}} @args;
            $path=~m{/proc/(\d+)/};push @pids,$1;
        }
    }
    die "expected one live campaign coordinator" unless @pids==1;
    my $raw=read_bytes("/proc/$pids[0]/cmdline");my @args=split /\0/,$raw;
    die "coordinator identity mismatch" unless grep {$_ eq $out} @args;
    write_bytes("$out/coordinator.pid","$pids[0]\n");
    return ($pids[0],\@args);
}
sub children {
    my ($pid)=@_;
    my $raw=eval {read_bytes("/proc/$pid/task/$pid/children")}//'';
    return grep {/^\d+$/} split /\s+/,$raw;
}
sub pause_campaign {
    my ($out)=@_;my ($pid,$args)=coordinator($out);
    write_bytes("$out/pause","requested ".time."\n");
    my %known;
    while (1) {
        my @children=children($pid);
        my @live=grep {my $p=process_info($_);$p && $p->{state} ne 'Z'} @children;
        my $busy=grep {my $p=process_info($_);$p && $p->{name} !~ /^(compiler|codex)$/} @live;
        if (!$busy) {
            kill 'STOP',$pid or die "stop coordinator: $!";
            @children=children($pid);
            $busy=grep {my $p=process_info($_);$p && $p->{state} ne 'Z' && $p->{name} !~ /^(compiler|codex)$/} @children;
            if (!$busy) {for my $child (@children) {$known{$child}=process_info($child);}last;}
            kill 'CONT',$pid or die "resume coordinator: $!";
        }
        sleep 0.1;
    }
    my $seconds=$args->[-2];
    while (1) {
        for my $parent (keys %known) {
            for my $child (children($parent)) {$known{$child}//=process_info($child);}
        }
        my @live=grep {my $p=process_info($_);$p && $known{$_} && $p->{start} eq $known{$_}{start} && $p->{state} ne 'Z'} keys %known;
        last unless @live;
        for my $child (@live) {
            next unless $known{$child}{name} eq 'codex';
            my @states=sort {(-M $a)<=> (-M $b)} glob("$out/*/state.json");
            my $s=@states?load_json($states[0]):{};
            if ($s->{started} && !$s->{finished} && $seconds=~/^\d+$/ && time-$s->{started}>$seconds) {
                kill 'TERM',-$child;sleep 1;kill 'KILL',-$child;
                $s->{pause_timeout}=JSON::PP::true;$s->{finished}=time;
                save_json($states[0],$s);
            }
        }
        sleep 0.25;
    }
    save_json("$out/pause-state.json",{coordinator=>$pid,paused=>JSON::PP::true,drained=>JSON::PP::true,at=>time});
    print $json->encode({paused=>JSON::PP::true,drained=>JSON::PP::true,flag=>"$out/pause",coordinator=>$pid}),"\n";
}
sub resume_campaign {
    my ($out)=@_;my ($pid)=coordinator($out);
    unlink "$out/pause";
    kill 'CONT',$pid or die "resume coordinator: $!";
    save_json("$out/pause-state.json",{coordinator=>$pid,paused=>JSON::PP::false,at=>time});
    print $json->encode({paused=>JSON::PP::false,coordinator=>$pid}),"\n";
}

my $action=shift(@ARGV)//'help';
if ($action eq 'full-gate') {full_gate(abs_path($ARGV[0]),abs_path($ARGV[1]),$ARGV[2]);exit;}
if ($action eq 'recover-candidate') {my $s=load_json("$ARGV[1]/state.json");recover_first_candidate(abs_path($ARGV[0]),abs_path($ARGV[1]),$s);exit;}
if ($action eq 'pause') {pause_campaign(abs_path($ARGV[0]));exit;}
if ($action eq 'resume') {resume_campaign(abs_path($ARGV[0]));exit;}
if ($action eq 'serve') {serve($ARGV[0]);exit;}
if ($action eq 'probe') {
    my ($binary,$run,$model)=@ARGV;
    die "probe path exists" if -e $run;
    make_path("$run/agent","$run/source","$run/desired","$run/store","$run/logs");
    my $state={version=>1,case=>{id=>0,paths=>[],task=>'Read the compiler help once and then finish. Do not edit or run the gate.'},
        mode=>'commands',model=>$model,binary=>abs_path($binary),calls=>0,payload_bytes=>0,response_bytes=>0,
        gates=>[],max_calls=>2,baseline=>{},started=>time,setup_seconds=>0,full_repository_gate=>'not-run'};
    save_json("$run/state.json",$state);
    print $json->encode(run_agent($run,$state,120)),"\n";exit;
}
if ($action eq 'report') {report(abs_path($ARGV[0]));exit;}
if ($action eq 'campaign') {
    my ($repository,$binary,$out,$model,$selection,$modes,$seconds,$calls)=@ARGV;
    die "usage: $0 campaign REPOSITORY BINARY OUTPUT MODEL CASES|all files,commands SECONDS CALLS\n" unless defined $calls;
    $repository=abs_path($repository);$binary=abs_path($binary);
    die "missing binary" unless -x $binary;
    make_path($out);$out=abs_path($out);
    write_bytes("$out/coordinator.pid","$$\n");
    my $frozen="$out/compiler";
    if (!-e $frozen) {copy($binary,$frozen) or die $!;chmod 0755,$frozen;}
    die "compiler differs from frozen campaign" unless sha256_hex(read_bytes($binary)) eq sha256_hex(read_bytes($frozen));
    my %selected=map {$_=>1} split /,/,$selection;
    my $manifest=load_json("$root/bench/compiler-stranger-cases.json");
    for my $case (@{$manifest->{cases}}) {
        next if $selection ne 'all' && !$selected{$case->{id}};
        for my $mode (split /,/,$modes) {
            while (-e "$out/pause") {sleep 0.25;}
            die "unknown arm" unless $mode eq 'files' || $mode eq 'commands';
            my $run=sprintf("%s/%02d-%s",$out,$case->{id},$mode);
            if (-e "$run/state.json") {
                my $old=load_json("$run/state.json");
                die "unfinished prior run requires inspection: $run" unless exists $old->{wall_seconds};
                next;
            }
            my $s=prepare($repository,$frozen,$run,$case,$mode,$model,$calls);
            if (!$s->{setup_error}) {$s=run_agent($run,$s,$seconds);}
            else {$s->{reason}=$s->{setup_error};save_json("$run/state.json",$s);$s=finish_run($run);}
            print $json->encode({case=>$case->{id},mode=>$mode,calls=>$s->{calls},reason=>$s->{reason}//'',wall_seconds=>$s->{wall_seconds}}),"\n";
            remove_tree("$run/source","$run/desired","$run/store");
            unlink "$run/key","$run/index-image","$run/request.json";
        }
    }
    report($out);exit;
}
die "usage: $0 campaign REPOSITORY BINARY OUTPUT MODEL CASES|all files,commands SECONDS CALLS\n       $0 report OUTPUT\n       $0 pause|resume OUTPUT\n       $0 full-gate REPOSITORY RUN OUTPUT\n       $0 recover-candidate REPOSITORY RUN\n";
