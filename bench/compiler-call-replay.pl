#!/usr/bin/env perl
# One outside-gate command-only attempt; the retained stranger adapter is unchanged.
use strict;
use warnings;
use JSON::PP;
use Encode qw(encode decode FB_CROAK);
use Digest::SHA qw(sha256_hex);
use File::Path qw(make_path);
use File::Copy qw(copy);
use File::Find qw(find);
use Cwd qw(abs_path);
use Time::HiRes qw(time sleep);
use POSIX qw(setsid WNOHANG);

my $json = JSON::PP->new->canonical->utf8;
sub read_bytes { open my $f, '<:raw', $_[0] or die "$_[0]: $!"; local $/; return <$f>; }
sub write_bytes {
    my ($path, $bytes) = @_;
    open my $f, '>:raw', $path or die "$path: $!";
    print {$f} $bytes or die "$path: $!"; close $f or die "$path: $!";
}
sub load_json { return $json->decode(read_bytes($_[0])); }
sub save_json {
    my ($path, $value) = @_;
    write_bytes("$path.tmp.$$", $json->encode($value)."\n");
    rename "$path.tmp.$$", $path or die "$path: $!";
}
sub text { my $data = $_[0]; return decode('UTF-8', $data, FB_CROAK); }
sub bytes { return encode('UTF-8', $_[0]); }
sub exit_code { return $_[0] & 127 ? 128 + ($_[0] & 127) : $_[0] >> 8; }

die "usage: $0 BINARY OUTPUT [MODEL]\n" unless @ARGV == 2 || @ARGV == 3;
my ($binary, $output, $model) = @ARGV;
$model //= 'gpt-6-luna';
$binary = abs_path($binary);
die "missing executable binary\n" unless defined $binary && -f $binary && -x $binary;
die "existing output requires inspection: $output\n" if -e $output || -l $output;
my $script = abs_path($0);
(my $repository = $script) =~ s{/bench/[^/]+$}{};
my $adapter = "$repository/bench/compiler-stranger.pl";
my $specimen = "$repository/tests/conformance/pass/const_function_calls.lkj";
my $started_setup = time;
umask 0077;
make_path(map {"$output/$_"} qw(agent source/src source/library desired/src desired/library store logs));
my $run = abs_path($output);
my $frozen = "$run/compiler";
copy($binary, $frozen) or die "freeze compiler: $!";
chmod 0700, $frozen or die "compiler mode: $!";
my $producer = sha256_hex(read_bytes($binary));
die "compiler changed while freezing\n" unless $producer eq sha256_hex(read_bytes($frozen));
my $original = read_bytes($specimen);
my $desired = $original;
my $changed = $desired =~ s/add\(self\.get\(\), self\.get\(\)\)/add(self.get(), add(self.get(), 1i64))/g;
die "specimen must contain exactly one selected pair\n" unless $changed == 1;
write_bytes("$run/source/src/main.lkj", $original);
write_bytes("$run/desired/src/main.lkj", $desired);
write_bytes("$run/source/library/lib.lkj", '');
write_bytes("$run/desired/library/lib.lkj", '');
save_json("$run/initial-files.json", {'src/main.lkj'=>text($original), 'library/lib.lkj'=>''});
open my $random, '<:raw', '/dev/urandom' or die $!;
read($random, my $key, 32) == 32 or die "short entropy read\n";
close $random;
write_bytes("$run/key", $key); chmod 0600, "$run/key" or die "key mode: $!";
undef $key;

my $task = "In Number::doubled, pass the result of the second self.get() call, together with 1i64, to the existing add function. Leave the first self.get() call and all other declarations unchanged. Choose the smallest supported checked edit.";
my $state = {
    version=>1, case=>{id=>'call-expression', paths=>['src/main.lkj'], task=>$task,
        scope=>'one source fixture; complete selected root', population=>'one focused unfamiliar-agent attempt'},
    mode=>'commands', model=>$model, reasoning_effort=>'medium', parent=>'fixed-specimen',
    binary=>$frozen, binary_sha256=>$producer, harness_sha256=>sha256_hex(read_bytes($adapter)),
    runner_sha256=>sha256_hex(read_bytes($script)),
    fixture_sha256=>sha256_hex($original), desired_sha256=>sha256_hex($desired),
    calls=>0, payload_bytes=>0, response_bytes=>0, gates=>[], max_calls=>24,
    setup_started=>$started_setup, baseline=>{'src/main.lkj'=>sha256_hex($original), 'library/lib.lkj'=>sha256_hex('')},
    full_repository_gate=>'not-run',
};
save_json("$run/state.json", $state);

# These selectors are identical to compiler-stranger.pl::selector.
my @index = ($frozen, 'build', 'src/main.lkj', '--source-dir', "$run/source",
    '--library-dir', "$run/source/library", '--store', "$run/store",
    '--store-key', "$run/key", '--store-bytes', '2147483648',
    '--index', '-o', "$run/index-image", '--stats');
my $pid = fork(); die "fork index: $!" unless defined $pid;
if (!$pid) {
    chdir "$run/source" or die $!;
    open STDOUT, '>:raw', "$run/logs/setup-index.stdout" or die $!;
    open STDERR, '>:raw', "$run/logs/setup-index.stderr" or die $!;
    exec @index; die "exec index: $!";
}
waitpid($pid, 0);
my $setup = {exit=>exit_code($?), stdout=>text(read_bytes("$run/logs/setup-index.stdout")),
    stderr=>text(read_bytes("$run/logs/setup-index.stderr"))};
save_json("$run/logs/setup-index.json", $setup);
if (!$setup->{exit} && $setup->{stdout} =~ /LKJSCRIPT_SNAPSHOT ([0-9a-f]{64})/) {
    $state->{initial_snapshot} = $1;
} else {
    $state->{setup_error} = 'index failed';
}
$state->{setup_seconds} = time - $started_setup;
$state->{preparation_seconds} = $state->{setup_seconds};

my $prompt = "You are an independent programmer seeing this program and language for the first time.\n\nTASK: $task\nLikely affected file: src/main.lkj\n\nOnly the compiler MCP tool is available for reading and changing the program. Learn from what the binary itself prints. No documentation or source filesystem tools are available. Initial indexed snapshot: ".($state->{initial_snapshot}//'unavailable').".\nThe source tree and library have already been selected. Their configuration is infrastructure, not a hint about the edit. No desired implementation is visible.\nUse the common gate tool when your first complete proposal is ready, then finish with a concise reason. The checker result and exact checked-meaning equality to the hidden desired program are separate measurements; equality is stricter than behavioral equivalence. The full tools/check.sh repository gate is a separate not-run metric.\nUse only this experiment's MCP tools: compiler help, query, edit, gate and finish. Do not use filesystem, shell, web, other agents, prior project instructions or external documentation. Do not ask for hints. You have at most 24 tool calls and an external 900-second allowance. A blocked attempt is useful evidence; do not invent success.\n";
write_bytes("$run/prompt.txt", bytes($prompt));
$state->{task_bytes} = length(bytes($prompt));
$state->{agent_allowance_seconds} = 900;
$state->{started} = time;
save_json("$run/state.json", $state);
my $raw_seconds = 0;
unless ($state->{setup_error}) {
    my $agent_started = time;
    $pid = fork(); die "fork agent: $!" unless defined $pid;
    if (!$pid) {
        setsid() >= 0 or die "setsid: $!";
        open STDIN, '<:raw', "$run/prompt.txt" or die $!;
        open STDOUT, '>:raw', "$run/agent-trace.jsonl" or die $!;
        open STDERR, '>:raw', "$run/agent-stderr.txt" or die $!;
        # Keep the existing run_agent restrictions and approval settings verbatim.
        exec 'codex', 'exec', '--ephemeral', '--skip-git-repo-check',
            '--model', $model, '-c', 'model_reasoning_effort="medium"',
            '--approve-for-me', '--cd', "$run/agent", '--trust', '--json', '--color', 'never',
            '--disable', 'shell_tool', '--disable', 'unified_exec', '--disable', 'apps',
            '--disable', 'browser_use', '--disable', 'computer_use', '--disable', 'skill_search',
            '--enable', 'skip_host_skill_discovery', '-c', 'web_search="disabled"',
            '-c', 'mcp_servers.stranger.command="perl"',
            '-c', 'mcp_servers.stranger.args='.$json->encode([$adapter, 'serve', $run]),
            '-c', 'mcp_servers.stranger.tool_timeout_sec=900',
            '-o', "$run/agent-final.txt", '-';
        die "exec codex: $!";
    }
    my ($timed_out, $interrupted) = (0, 0);
    local $SIG{INT} = local $SIG{TERM} = sub { $interrupted = 1; };
    while (waitpid($pid, WNOHANG) == 0) {
        if ($interrupted || time - $agent_started > 900) {
            $timed_out = !$interrupted;
            kill 'TERM', -$pid; sleep 1; kill 'KILL', -$pid; waitpid($pid, 0); last;
        }
        sleep 0.25;
    }
    my $agent_exit = exit_code($?);
    $raw_seconds = time - $agent_started;
    $state = load_json("$run/state.json");
    $state->{agent_exit} = $agent_exit;
    $state->{timed_out} = $timed_out ? JSON::PP::true : JSON::PP::false;
    $state->{interrupted} = $interrupted ? JSON::PP::true : JSON::PP::false;
}
$state->{finished} //= time;
$state->{wall_seconds} = $state->{finished} - $state->{started};
$state->{raw_wall_seconds} = $raw_seconds;
my %final;
find({no_chdir=>1, wanted=>sub {
    return unless -f $_ && !-l $_;
    (my $path = $_) =~ s{^\Q$run/source/\E}{};
    return unless $path =~ m{^(src|library|tools|tests)/};
    my $data = read_bytes($_);
    $final{$path} = text($data) if !exists $state->{baseline}{$path}
        || sha256_hex($data) ne $state->{baseline}{$path};
}}, "$run/source");
for my $path (keys %{$state->{baseline}}) { $final{$path} = undef unless -e "$run/source/$path"; }
save_json("$run/final-files.json", \%final);
$state->{changed_files} = [sort keys %final];

my ($attempted, $outside) = (0, 0);
if (-f "$run/agent-trace.jsonl") {
    my $trace = read_bytes("$run/agent-trace.jsonl");
    if (!$state->{calls} && $trace =~ /MCP tool call requires approval/) {
        $state->{setup_error} = 'CLI rejected fixture tool approval before execution';
    } elsif (!$state->{calls} && ($state->{agent_exit}//0) && !$state->{timed_out} && !$state->{interrupted}) {
        $state->{setup_error} //= 'CLI exited before any experiment tool call; inspect trace and stderr';
    }
    for my $line (split /\n/, $trace) {
        my $event = eval { $json->decode($line) };
        next unless $event && ($event->{type}//'') eq 'item.started';
        my $item = $event->{item}//{};
        if (($item->{type}//'') eq 'mcp_tool_call') { $attempted++; $outside++ if ($item->{server}//'') ne 'stranger'; }
        elsif (($item->{type}//'') =~ /^(command_execution|file_change|web_search)$/) { $outside++; }
    }
}
$state->{attempted_tool_calls} = $attempted; $state->{outside_tool_calls} = $outside;
save_json("$run/state.json", $state);

my (%operations, %applied, %before_gate_operations, @edits);
my ($before_gate_requests, $before_gate_source_bytes) = (0, 0);
my ($request_bytes, $source_bytes, $applied_source_bytes, $requests, $first_gate) = (0, 0, 0, 0, undef);
for my $path (sort glob("$run/logs/tool-*.json")) {
    my $record = load_json($path);
    if ($record->{tool} eq 'gate') { $first_gate //= $record->{number}; next; }
    next unless $record->{tool} eq 'compiler';
    my $args = $record->{arguments}; my @argv = @{$args->{argv}//[]};
    next unless @argv && $argv[0] eq 'edit' && exists $args->{request};
    next if grep { $_ eq '--help' || $_ eq '--recover' } @argv;
    $requests++;
    my $raw = ref($args->{request}) ? $json->encode($args->{request}) : bytes($args->{request});
    $request_bytes += length($raw);
    my $request = eval { $json->decode($raw) };
    my @kinds; my $edit_source_bytes = 0;
    my $accepted = defined($record->{result}{exit}) && $record->{result}{exit} == 0
        && !grep { $_ eq '--dry-run' } @argv;
    if (ref($request) eq 'HASH' && ref($request->{operations}) eq 'ARRAY') {
        for my $op (@{$request->{operations}}) {
            next unless ref($op) eq 'HASH';
            my $kind = $op->{op}//'<missing>'; push @kinds, $kind; $operations{$kind}++;
            my $size = defined($op->{source}) && !ref($op->{source}) ? length(bytes($op->{source})) : 0;
            $source_bytes += $size; $edit_source_bytes += $size;
            unless (defined $first_gate) { $before_gate_operations{$kind}++; $before_gate_source_bytes += $size; }
            if ($accepted) { $applied{$kind}++; $applied_source_bytes += $size; }
        }
    }
    $before_gate_requests++ unless defined $first_gate;
    push @edits, {tool_record=>$record->{number}, operations=>\@kinds,
        request_bytes=>length($raw), operation_source_bytes=>$edit_source_bytes,
        exit=>$record->{result}{exit}, applied=>$accepted?JSON::PP::true:JSON::PP::false};
}
my $proposal;
if (-f "$run/gate-01-files.json") {
    my $files = load_json("$run/gate-01-files.json");
    my $total = 0; for my $data (values %$files) { $total += length(bytes($data)) if defined $data; }
    $proposal = {path=>'gate-01-files.json', sha256=>sha256_hex(read_bytes("$run/gate-01-files.json")),
        paths=>[sort keys %$files], changed_files=>scalar(keys %$files), submitted_file_bytes=>$total};
}
my $first = $state->{gates}[0]//{};
my $summary = {
    version=>1, scope=>'single focused command-only unfamiliar-agent replay; no population comparison',
    model=>$model, reasoning_effort=>'medium', binary_sha256=>$producer,
    adapter_sha256=>$state->{harness_sha256}, runner_sha256=>$state->{runner_sha256},
    fixture_sha256=>$state->{fixture_sha256}, desired_sha256=>$state->{desired_sha256},
    initial_snapshot=>$state->{initial_snapshot}, setup_seconds=>$state->{setup_seconds},
    raw_wall_seconds=>$raw_seconds, wall_to_finish_seconds=>$state->{wall_seconds},
    calls=>$state->{calls}, attempted_tool_calls=>$attempted, outside_tool_calls=>$outside,
    payload_bytes=>$state->{payload_bytes}, response_bytes=>$state->{response_bytes},
    task_bytes=>$state->{task_bytes}, agent_exit=>$state->{agent_exit},
    timed_out=>$state->{timed_out}//JSON::PP::false, interrupted=>$state->{interrupted}//JSON::PP::false,
    setup_error=>$state->{setup_error}, reason=>$state->{reason}//'',
    first_gate_tool_record=>$first_gate, first_proposal=>$proposal,
    checker_first=>$first->{checker_pass}, checked_meaning_equal_first=>$first->{canonical_target_pass},
    behavioral_equivalence=>'not separately tested',
    equivalence_evidence=>$first->{canonical_target_pass} ? 'equal canonical checked meaning' : 'not established',
    native_interpreter_execution=>'not-run', full_repository_gate=>'not-run',
    edit_request_count=>$requests, request_bytes=>$request_bytes,
    operation_counts=>\%operations, submitted_operation_source_bytes=>$source_bytes,
    applied_operation_counts=>\%applied, applied_operation_source_bytes=>$applied_source_bytes,
    source_byte_scope=>'UTF-8 operation.source strings; repeated requests count again',
    requests_before_first_gate=>$before_gate_requests, operations_before_first_gate=>\%before_gate_operations,
    operation_source_bytes_before_first_gate=>$before_gate_source_bytes,
    edit_attempts=>\@edits, final_changed_files=>$state->{changed_files},
    evidence=>['prompt.txt','initial-files.json','state.json','agent-trace.jsonl',
        'agent-stderr.txt','agent-final.txt','logs/tool-*.json','final-files.json'],
};
save_json("$run/summary.json", $summary);
print $json->encode($summary), "\n";
exit($state->{setup_error} ? 2 : 0);
