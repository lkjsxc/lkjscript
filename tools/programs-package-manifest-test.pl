#!/usr/bin/env perl
use strict;
use warnings;
use Archive::Tar;
use Cwd qw(abs_path);
use Digest::SHA qw(sha256_hex);
use Errno qw(EINTR);
use File::Temp qw(tempdir);
use IO::Compress::Gzip qw(gzip $GzipError);
use JSON::PP;
use POSIX qw(mkfifo);
my ($producer, $parent) = @ARGV;
die "usage: manifest-test PRODUCER TEMP-PARENT\n" unless @ARGV == 2;
$producer = abs_path($producer) // die "missing producer\n";
my $work = tempdir('package-inventory-XXXXXX', DIR => $parent, CLEANUP => 1);
mkdir "$work/source" or die $!;
mkdir "$work/source/src" or die $!;
$ENV{GIT_CONFIG_NOSYSTEM} = 1;
$ENV{GIT_CONFIG_GLOBAL} = '/dev/null';
sub put {
    my ($path, $bytes) = @_;
    open my $file, '>:raw', $path or die "$path: $!";
    print {$file} $bytes or die $!;
    close $file or die $!;
}
sub read_bytes {
    my ($path) = @_;
    open my $file, '<:raw', $path or die $!;
    local $/; my $bytes = scalar <$file>; close $file or die $!;
    return defined($bytes) ? $bytes : '';
}
my $commands = 0;
my $watchdog = 0;
sub run {
    my ($want, @command) = @_;
    my $pid = fork(); defined($pid) or die $!;
    if (!$pid) {
        open STDOUT, '>:raw', "$work/stdout" or die $!;
        open STDERR, '>:raw', "$work/stderr" or die $!;
        exec @command; die "exec: $!";
    }
    my $timed_out = 0;
    local $SIG{ALRM} = sub { $timed_out = 1; kill 'KILL', $pid; };
    alarm($watchdog) if $watchdog;
    my $wait;
    do { $wait = waitpid($pid, 0); } while $wait < 0 && $! == EINTR;
    my $status = $?;
    alarm(0);
    die "control timed out; no refusal credit\n" if $timed_out;
    $wait == $pid or die $!;
    die "command died by signal; no refusal credit\n" if $status & 127;
    die "unexpected exit $status: @command\n" . read_bytes("$work/stdout") . read_bytes("$work/stderr")
        if ($status == 0) != $want;
    ++$commands;
    return read_bytes("$work/stdout");
}
sub git { run(1, 'git', '-c', 'core.hooksPath=/dev/null', '-c', 'core.fsmonitor=false',
    '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', '-C', "$work/source", @_); }
my $code = "pub fn answer() -> u8 { 7 }\n";
put("$work/source/LICENSE", "permission\n");
put("$work/source/src/lib.rs", $code);
git('init', '-q'); git('add', 'LICENSE', 'src/lib.rs'); git('commit', '-qm', 'fixture');
my $revision = git('rev-parse', 'HEAD'); chomp $revision;
my $case = 0;
sub produce {
    my ($want, @route) = @_;
    my $output = "$work/package-" . ++$case;
    my $result = run($want, $^X, $producer, '--source', "$work/source", '--name', 'fixture', '--version', '1.0',
        '--origin', 'https://example.invalid/fixture', '--notice', 'LICENSE', '--output', $output, @route);
    die "rejected producer left a pin\n" if !$want && (-e "$output.pin.json" || -e "$output.manifest.json");
    return $result;
}
my $git_report = decode_json(produce(1, '--git', '--revision', $revision));
die "wrong exact Git inventory\n" unless $git_report->{files} == 2;
put("$work/source/src/lib.rs", "pub fn answer() -> u8 { 8 }\n");
produce(0, '--git', '--revision', $revision);
git('add', 'src/lib.rs'); produce(0, '--git', '--revision', $revision);
git('reset', '-q', 'HEAD', '--', 'src/lib.rs'); put("$work/source/src/lib.rs", $code);
unlink "$work/source/src/lib.rs" or die $!;
symlink '../LICENSE', "$work/source/src/lib.rs" or die $!;
produce(0, '--git', '--revision', $revision);
unlink "$work/source/src/lib.rs" or die $!; put("$work/source/src/lib.rs", $code);
my $tar = Archive::Tar->new;
$tar->add_data('fixture-1.0/LICENSE', "permission\n");
$tar->add_data('fixture-1.0/src/lib.rs', $code);
my $tar_bytes = $tar->write;
sub zipped { my ($input) = @_; my $output; gzip(\$input => \$output) or die $GzipError; return $output; }
my $archive = zipped($tar_bytes);
sub archive_case {
    my ($want, $bytes, $checksum) = @_;
    put("$work/package.crate", $bytes);
    return produce($want, '--archive', "$work/package.crate", '--checksum', $checksum // sha256_hex($bytes));
}
my $archive_report = decode_json(archive_case(1, $archive));
die "Git and archive logical trees differ\n" unless $git_report->{identity}[4] eq $archive_report->{identity}[4];
archive_case(0, $archive, '0' x 64);
defined(mkfifo("$work/archive-fifo", 0600)) or die $!;
$watchdog = 5;
produce(0, '--archive', "$work/archive-fifo", '--checksum', '0' x 64);
$watchdog = 0;
die "FIFO failed for the wrong reason\n" unless read_bytes("$work/stderr") =~ /not a regular file/;
archive_case(0, $archive . $archive);
archive_case(0, $archive . 'trailing compressed payload');
archive_case(0, zipped($tar_bytes . $tar_bytes));
archive_case(0, zipped(substr($tar_bytes, 0, length($tar_bytes) - 1)));
my $header_change = $tar_bytes; substr($header_change, 0, 1) = 'X';
archive_case(0, zipped($header_change));
put("$work/source/src/lib.rs", "different bytes\n"); archive_case(0, $archive);
git('add', 'src/lib.rs'); git('commit', '-qm', 'substitute');
my $substitute = git('rev-parse', 'HEAD'); chomp $substitute;
git('reset', '--soft', $revision); git('replace', $revision, $substitute);
produce(0, '--git', '--revision', $revision);
print "ok package inventory: $case controls, exact Git/archive trees and compressed/tar framing refusals\n";
