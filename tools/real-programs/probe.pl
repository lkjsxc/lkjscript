#!/usr/bin/env perl
# Observe complete upstream roots through an unchanged suffix projection.
# This is an untrusted harness, not a Cargo loader or an admission certificate.
use strict;
use warnings;
use Cwd qw(abs_path);
use Digest::SHA qw(sha256_hex);
use Encode qw(decode encode FB_CROAK LEAVE_SRC);
use File::Basename qw(dirname);
use File::Temp qw(tempdir tempfile);
use FindBin;
use Getopt::Long qw(GetOptions);
use JSON::PP;
use MIME::Base64 qw(encode_base64);
use POSIX qw(strftime);
use Time::HiRes qw(clock_gettime CLOCK_MONOTONIC);

sub fail { die "real-programs-probe: $_[0]\n" }
sub text_value {
    my ($raw, $description) = @_;
    my $text = eval { decode('UTF-8', $raw, FB_CROAK | LEAVE_SRC) };
    fail("non-UTF-8 $description") if $@;
    return $text;
}
sub filesystem_bytes {
    my ($text, $description) = @_;
    my $raw = eval { encode('UTF-8', $text, FB_CROAK | LEAVE_SRC) };
    fail("invalid Unicode $description") if $@;
    return $raw;
}
sub bytes {
    open my $f, '<:raw', $_[0] or fail("read $_[0]: $!");
    my $value = do { local $/; <$f> };
    close $f or fail("close $_[0]: $!");
    return defined $value ? $value : '';
}
sub load_average {
    my $text = bytes('/proc/loadavg');
    $text =~ /\A([0-9.]+) / or fail('unexpected /proc/loadavg');
    return $1;
}
sub library_identity {
    my ($root) = @_;
    my @files;
    my $visit;
    $visit = sub {
        my ($relative) = @_;
        my $relative_text = text_value($relative, 'library path');
        my $path = length($relative) ? "$root/$relative" : $root;
        fail("symlink in library input: $path") if -l $path;
        if (-d $path) {
            opendir my $dir, $path or fail("list library: $!");
            my @names = sort grep { $_ ne '.' && $_ ne '..' } readdir($dir);
            closedir $dir or fail("close library directory: $!");
            $visit->(length($relative) ? "$relative/$_" : $_) for @names;
        } elsif (-f $path) {
            my $raw = bytes($path);
            push @files, { path => $relative_text, bytes => length($raw), sha256 => sha256_hex($raw) };
        } else {
            fail("nonregular library input: $path");
        }
    };
    $visit->('');
    undef $visit;
    return { files => \@files,
        manifest_sha256 => sha256_hex(JSON::PP->new->canonical->ascii->encode(\@files)) };
}
sub stream {
    my ($path) = @_;
    my $raw = bytes($path);
    my $decoded = eval { decode('UTF-8', $raw, FB_CROAK | LEAVE_SRC) };
    return { bytes => length($raw), sha256 => sha256_hex($raw),
        defined($decoded) ? (text => $decoded) : (base64 => encode_base64($raw, '')) };
}
sub run_observed {
    my ($temporary, $index, $command, $time_program) = @_;
    my ($out, $err, $rss) = map { "$temporary/$index.$_" } qw(stdout stderr rss);
    my $load_before = load_average();
    my $start = clock_gettime(CLOCK_MONOTONIC);
    my $pid = fork();
    fail("fork: $!") unless defined $pid;
    if (!$pid) {
        open STDIN, '<', '/dev/null' or die "stdin: $!";
        open STDOUT, '>:raw', $out or die "stdout: $!";
        open STDERR, '>:raw', $err or die "stderr: $!";
        exec {$time_program} $time_program, '-f', '%M', '-o', $rss, @$command;
        die "exec $time_program: $!";
    }
    waitpid($pid, 0) == $pid or fail("waitpid: $!");
    my $status = $?;
    my $elapsed = clock_gettime(CLOCK_MONOTONIC) - $start;
    my $load_after = load_average();
    my $rss_text = bytes($rss);
    $rss_text =~ /(?:\A|\n)([0-9]+)\n\z/ or fail('missing GNU-time peak RSS');
    my $peak_rss = 0 + $1;
    return { command => [map { text_value($_, 'command argument') } @$command],
        exit_status => $status >> 8, launcher_signal => $status & 127,
        wall_seconds => 0 + sprintf('%.6f', $elapsed), peak_rss_kib => $peak_rss,
        load_before => $load_before, load_after => $load_after,
        stdout => stream($out), stderr => stream($err), time_report => stream($rss) };
}
sub command_json {
    my (@command) = @_;
    open my $pipe, '-|', @command or fail("start verifier: $!");
    my $raw = do { local $/; <$pipe> };
    close $pipe or fail("projection verification failed: $?");
    return decode_json($raw);
}

my ($compiler, $library, $source, $projection, $output, $package, $version, $help);
my $time_program = '/usr/bin/time';
my @entries;
Getopt::Long::Configure(qw(no_auto_abbrev no_ignore_case));
GetOptions('compiler=s' => \$compiler, 'library=s' => \$library,
    'source=s' => \$source, 'projection=s' => \$projection,
    'output=s' => \$output, 'package=s' => \$package, 'version=s' => \$version,
    'entry=s@' => \@entries, 'time=s' => \$time_program, 'help' => \$help) or fail('use --help');
if ($help) {
    print <<'HELP';
Usage: perl probe.pl --compiler COMPILER --library LIBRARY --source SOURCE
       --projection PROJECTION --package NAME --version EXACT --entry src/lib.rs
       [--entry another/root.rs ...] --output NEW_RECEIPT.json [--time GNU_TIME]
SOURCE and PROJECTION must already pass project.pl --verify. Each entry is checked
at its original .rs path, then its byte-identical .lkj alias with the complete
tracked module/asset context preserved. No source normalization is performed.
Receipts retain exact commands, all diagnostic bytes, compiler/root identities,
load, elapsed wall time and peak process RSS. Entries must name tracked .rs files.
These are source-root observations under the current compiler's configuration,
not Cargo feature/test configurations or a claim of complete crate admission.
HELP
    exit 0;
}
fail('all named arguments and at least one --entry are required')
    if @ARGV || !@entries || grep { !defined($_) || !length($_) }
        ($compiler, $library, $source, $projection, $output, $package, $version);
# OS calls keep byte paths. Manifest keys and receipt strings use decoded text.
my @entry_text = map { text_value($_, 'entry path') } @entries;
$package = text_value($package, 'package name');
$version = text_value($version, 'package version');
text_value($output, 'output path');
for ($compiler, $library, $source, $projection, $time_program) {
    text_value($_, 'input path');
    $_ = abs_path($_) // fail('input path does not exist');
    text_value($_, 'resolved input path');
}
fail('compiler is not executable') unless -f $compiler && -x $compiler;
fail('GNU-time-compatible launcher is not executable') unless -f $time_program && -x $time_program;
fail('output already exists') if lstat($output);
my $projector = "$FindBin::Bin/project.pl";
fail('project.pl must be adjacent to probe.pl') unless -f $projector;
my $before = command_json($^X, $projector, '--verify', $source, $projection);
my $manifest_raw = bytes("$projection/projection.json");
fail('projection manifest differs from the verified bytes')
    unless sha256_hex($manifest_raw) eq $before->{manifest_sha256};
my $manifest = decode_json($manifest_raw);
my %files = map { $_->{path} => $_ } @{$manifest->{files}};
my %aliases = map { $_->{from} => $_->{to} } @{$manifest->{aliases}};
my %seen;
for my $index (0 .. $#entries) {
    my $entry = $entry_text[$index];
    fail("entry must be a unique tracked Rust path: $entries[$index]")
        unless exists($files{$entry}) && exists($aliases{$entry}) && !$seen{$entry}++;
}
my $producer = sha256_hex(bytes($compiler));
my $library_before = library_identity($library);
my $temporary = tempdir('root-probe-XXXXXX', DIR => dirname($output), CLEANUP => 1);
my @observations;
for my $index (0 .. $#entries) {
    my $entry = $entry_text[$index];
    for my $route ('original_suffix', 'suffix_projection') {
        my ($root, $path) = $route eq 'original_suffix'
            ? ($source, $entries[$index])
            : ($projection, filesystem_bytes($aliases{$entry}, 'manifest alias path'));
        my $result = run_observed($temporary, scalar(@observations),
            [$compiler, 'check', $path, '--source-dir', $root, '--library-dir', $library], $time_program);
        push @observations, { route => $route, upstream_entry => $entry,
            entry_sha256 => $files{$entry}{sha256}, entry_bytes => $files{$entry}{bytes},
            %$result };
    }
}
my $after = command_json($^X, $projector, '--verify', $source, $projection);
fail('projection manifest changed during observations')
    unless $before->{manifest_sha256} eq $after->{manifest_sha256};
fail('compiler bytes changed during observations') unless sha256_hex(bytes($compiler)) eq $producer;
my $library_after = library_identity($library);
fail('library bytes changed during observations')
    unless $library_before->{manifest_sha256} eq $library_after->{manifest_sha256};
my $receipt = { schema => 1, kind => 'unmodified_real_program_root_observations',
    timestamp_utc => strftime('%Y-%m-%dT%H:%M:%SZ', gmtime),
    package => { name => $package, version => $version, %{$manifest->{source}} },
    compiler => { path => text_value($compiler, 'compiler path'), sha256 => $producer },
    time_launcher => { path => text_value($time_program, 'time launcher path'), sha256 => sha256_hex(bytes($time_program)) },
    library => { root => text_value($library, 'library root'), %$library_before },
    projection_manifest_sha256 => sha256_hex($manifest_raw),
    verification_before => $before, verification_after => $after,
    interpretation => 'Every source byte is unchanged. Suffix projection preserves tracked module/asset context. The compiler has no Cargo package/feature/test configuration here. A refusal is a root observation, not an exhaustive blocker count; acceptance would not establish Cargo-equivalent configuration or executed upstream tests. Exit status and launcher_signal belong to the time launcher; the complete separate time_report retains its abnormal-exit/signal diagnostic as well as peak RSS.',
    observations => \@observations };
my ($fh, $pending) = tempfile('.root-probe-XXXXXX', DIR => $temporary, UNLINK => 0);
binmode $fh;
print {$fh} JSON::PP->new->canonical->pretty->ascii->encode($receipt) or fail("write receipt: $!");
close $fh or fail("close receipt: $!");
if (!link($pending, $output)) {
    my $error = $!;
    unlink $pending;
    fail("publish new receipt: $error");
}
unlink $pending or fail("remove receipt temporary: $!");
print JSON::PP->new->canonical->ascii->encode({ receipt => text_value($output, 'receipt path'), observations => scalar @observations,
    compiler_sha256 => $producer, source_unchanged => JSON::PP::true }), "\n";
