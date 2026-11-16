#!/usr/bin/env perl
# Read-only guards for the one released scale-refusal fixture and its evidence mirror.
use strict;
use warnings;
use Cwd qw(realpath);
use Digest::SHA qw(sha256_hex);
use Errno qw(ESRCH);
use File::Find;
use JSON::PP;
use POSIX qw(strftime);

my $mode = shift // '';
die "usage: $0 before|mirror\n" unless @ARGV == 0 && ($mode eq 'before' || $mode eq 'mirror');
my $repo = '/home/coder/workspace/lkjscript-compiler-scale-refusal-20261116';
die "wrong repository\n" unless realpath('.') eq $repo;
my $run = "$repo/target/compiler-scale-refusal-run.tCSZMe";
my $key = '/tmp/compiler-scale-refusal-key.aD8jW6';
my $evidence = "$repo/target/compiler-scale-refusal-evidence.62fa76f4-1";
my $raw = "$repo/bench/compiler-scale-refusal-data/measurement-1";
my $saved = "$repo/bench/compiler-scale-refusal-cleanup-data/frozen-cleanup";
my $result_commit = '379f456840d21f7a1441fb3fd0e8cf3a08f90acf';
my $result_manifest = 'bench/compiler-scale-refusal-results.sha256';
my $result_manifest_sha = '24574f4eb2e85e21e5a8f3d190c7aa143c45012af140a5773e1230fa99bf3085';
my $frozen_cleanup_sha = '3953f766928e37ceb6321d90210e63c827c31821357c14fd73c033efa62f1a70';
my $acceptance_commit = 'dc0a325e19367b3563bc2d35316804be8ee2fa51';
my $acceptance_sha = 'ffbdd3def42c20a19d1dc3a4c06c6cb90f428550ec2d21385d4a868adc96bc1a';
my $acceptance_manifest_sha = 'c2ac940ba56471f9f779d871ea21c559c5574c590eb91655ccbc8403d2a782e8';

sub read_bytes {
    my ($path) = @_;
    open my $file, '<:raw', $path or die "read $path: $!\n";
    local $/;
    my $text = <$file>;
    close $file or die "close $path: $!\n";
    return defined($text) ? $text : '';
}
sub command {
    open my $pipe, '-|', @_ or die "launch $_[0]: $!\n";
    local $/;
    my $text = <$pipe>;
    close $pipe or die "command failed: $_[0]\n";
    return defined($text) ? $text : '';
}
sub digest { return sha256_hex(read_bytes($_[0])); }
sub tree {
    my ($root) = @_;
    die "not a literal directory: $root\n" unless -d $root && !-l $root && realpath($root) eq $root;
    my @root_stat = lstat($root);
    my (%files, $directories);
    $directories = 0;
    find({no_chdir => 1, wanted => sub {
        my $path = $File::Find::name;
        my @s = lstat($path);
        die "unstatable entry: $path\n" unless @s;
        die "symlink: $path\n" if -l _;
        die "cross-device entry: $path\n" unless $s[0] == $root_stat[0];
        die "foreign-owned entry: $path\n" unless $s[4] == $<;
        if (-d _) { ++$directories; return; }
        die "nonregular entry: $path\n" unless -f _;
        die "multiply linked file: $path\n" unless $s[3] == 1;
        my $relative = substr($path, length($root) + 1);
        $files{$relative} = $s[7];
    }}, $root);
    return {files => \%files, directories => $directories};
}
sub same_paths {
    my ($a, $b, $label) = @_;
    die "$label path set mismatch\n" unless join("\0", sort keys %$a) eq join("\0", sort keys %$b);
}
sub same_files {
    my ($a, $b, $paths) = @_;
    for my $relative (sort keys %$paths) {
        die "byte mismatch: $relative\n" unless read_bytes("$a/$relative") eq read_bytes("$b/$relative");
    }
}
sub committed_tree {
    my ($absolute, $inventory) = @_;
    my $relative = substr($absolute, length($repo) + 1);
    my %tracked = map { substr($_, length($relative) + 1) => 1 }
        grep { length } split /\0/, command('git', 'ls-files', '-z', '--', $relative);
    same_paths($inventory->{files}, \%tracked, "committed $relative");
    command('git', 'diff', '--quiet', 'HEAD', '--', $relative);
}
die "changed result manifest\n" unless digest($result_manifest) eq $result_manifest_sha;
die "changed frozen cleanup\n" unless digest('bench/compiler-scale-refusal.sh') eq $frozen_cleanup_sha;
command('git', 'diff', '--quiet', $result_commit, 'HEAD', '--',
    'bench/compiler-scale-refusal-data', 'bench/compiler-scale-refusal-62fa76f4.md', $result_manifest);
command('sha256sum', '-c', $result_manifest);
die "root acceptance identity mismatch\n" unless
    sha256_hex(command('git', 'show', "$acceptance_commit:bench/compiler-scale-refusal.acceptance.md")) eq $acceptance_sha;
die "root acceptance manifest identity mismatch\n" unless
    sha256_hex(command('git', 'show', "$acceptance_commit:bench/compiler-scale-refusal-root-audit-data/manifest.sha256")) eq $acceptance_manifest_sha;
my $original = tree($raw);
die "expected exactly 92 flat raw files\n" unless keys(%{$original->{files}}) == 92 && $original->{directories} == 1;
committed_tree($raw, $original);
my $live = tree($evidence);
my %live_original = map { $_ => $live->{files}{$_} }
    grep { !m{\Acleanup/} } keys %{$live->{files}};
same_paths($original->{files}, \%live_original, 'raw evidence mirror');
same_files($raw, $evidence, $original->{files});
my $completion = read_bytes("$evidence/completion");
die "unexpected supervisor completion\n" unless $completion eq "supervisor_exit=1\ncompleted_utc=2026-11-16T11:55:57Z\n";

# Use the same narrow own-process/ancestor exclusion as the frozen cleanup.
my (%parents, %args);
for my $line (split /\n/, command('ps', '-eo', 'pid=,ppid=,args=')) {
    $line =~ /^\s*(\d+)\s+(\d+)\s+(.*)$/ or die "unparsed process inventory\n";
    ($parents{$1}, $args{$1}) = ($2, $3);
}
my %ancestors;
my $cursor = $$;
while ($cursor > 0) {
    die "invalid process ancestry\n" if $ancestors{$cursor}++ || !exists $parents{$cursor};
    $cursor = $parents{$cursor};
}
my @live_references = sort { $a <=> $b } grep {
    !$ancestors{$_} && (index($args{$_}, $run) >= 0 || index($args{$_}, $key) >= 0 || index($args{$_}, $evidence) >= 0)
} keys %args;
die "live ownership reference PIDs: @live_references\n" if @live_references;
for my $line (split /\n/, read_bytes('/proc/self/mountinfo')) {
    my @fields = split / /, $line;
    my $mount = $fields[4];
    $mount =~ s/\\([0-7]{3})/chr(oct($1))/ge;
    for my $root ($run, $key, $evidence) {
        die "nested mount: $mount\n" if $mount eq $root || index($mount, "$root/") == 0;
    }
}
my %out = (
    mode => $mode, utc => strftime('%Y-%m-%dT%H:%M:%SZ', gmtime()),
    repository => $repo, result_commit => $result_commit, result_manifest_sha256 => $result_manifest_sha,
    root_acceptance_commit => $acceptance_commit, root_acceptance_sha256 => $acceptance_sha,
    root_acceptance_manifest_sha256 => $acceptance_manifest_sha,
    fixture => $run, key_directory => $key, evidence => $evidence,
    original_raw_files => 92, raw_mirror_bytes_identical => JSON::PP::true,
    committed_originals => JSON::PP::true, live_reference_count => 0, nested_mounts => 0,
    symlinks => 0, multiply_linked_regular_files => 0, special_files => 0,
    cleanup_authorized_by_this_check => JSON::PP::false,
);
if ($mode eq 'before') {
    die "cleanup already started\n" if -e "$evidence/cleanup" || -l "$evidence/cleanup";
    die "unexpected evidence directories\n" unless $live->{directories} == 1;
    my $fixture_tree = tree($run);
    my $key_tree = tree($key);
    my %key_names = (key => 1, 'owner-run' => 1);
    same_paths($key_tree->{files}, \%key_names, 'key directory');
    die "unexpected private-key length\n" unless $key_tree->{files}{key} == 32;
    die "key owner mismatch\n" unless read_bytes("$key/owner-run") eq "$run\n";
    for my $name (qw(owner-repository owner-pid owner-key owner-evidence)) {
        die "owner record mismatch: $name\n" unless read_bytes("$run/$name") eq read_bytes("$repo/bench/compiler-scale-refusal-data/launch-1/ownership/$name");
    }
    die "wrong repository owner\n" unless read_bytes("$run/owner-repository") eq "$repo\n";
    die "wrong evidence owner\n" unless read_bytes("$run/owner-evidence") eq "$evidence\n";
    die "wrong key owner\n" unless read_bytes("$run/owner-key") eq "$key\n";
    my $pid = read_bytes("$run/owner-pid");
    die "unexpected supervisor PID\n" unless $pid eq "2699229\n";
    die "supervisor still live\n" if kill(0, 2699229);
    die "cannot establish supervisor absence: $!\n" unless $! == ESRCH;
    my @pending = grep { m{\Astore-[^/]+/(?:.*/)?(?:\.tmp-|[^/]*pending)} } keys %{$fixture_tree->{files}};
    # Staging directories can be empty, so inspect directory names too.
    find({no_chdir => 1, wanted => sub {
        push @pending, $File::Find::name if -d $_ && $File::Find::name =~ m{/store-[^/]+/(?:.*/)?(?:\.tmp-|[^/]*pending)};
    }}, $run);
    die "pending store publication metadata\n" if @pending;
    for my $store ('store-1000000-build', 'store-1000000-check') {
        my $inventory = tree("$run/$store");
        die "refusal store is not empty: $store\n" unless keys(%{$inventory->{files}}) == 0 && $inventory->{directories} == 1;
    }
    my @summary = grep { /^LKJSCRIPT_STORE_IO / && /"category":"summary"/ } split /\n/, read_bytes("$raw/100000-cold.stderr");
    die "missing unique 100k store summary\n" unless @summary == 1;
    $summary[0] =~ s/^LKJSCRIPT_STORE_IO //;
    my $store_summary = decode_json($summary[0]);
    die "100k pending checkpoint\n" unless defined($store_summary->{pending_checkpoint}) && $store_summary->{pending_checkpoint} == 0;
    $out{fixture_regular_files} = scalar keys %{$fixture_tree->{files}};
    $out{fixture_directories} = $fixture_tree->{directories};
    $out{key_regular_files} = scalar keys %{$key_tree->{files}};
    $out{pending_store_metadata} = 0;
    $out{supervisor_absent} = JSON::PP::true;
    $out{cross_worktree_dependency} = 'Root source/archive acceptance completed; root reports no live fixture dependency. Explicit cleanup release remains required.';
} else {
    die "fixture or key still exists\n" if -e $run || -l $run || -e $key || -l $key;
    my $cleanup = tree("$evidence/cleanup");
    my $retained = tree($saved);
    my %expected = map { $_ => 1 } qw(owner-repository owner-pid owner-key owner-evidence action store-apparent-bytes);
    same_paths($cleanup->{files}, \%expected, 'frozen cleanup records');
    same_paths($retained->{files}, \%expected, 'retained cleanup records');
    die "unexpected cleanup directories\n" unless $cleanup->{directories} == 1 && $retained->{directories} == 1;
    same_files("$evidence/cleanup", $saved, \%expected);
    committed_tree($saved, $retained);
    for my $name (qw(owner-repository owner-pid owner-key owner-evidence)) {
        die "retained owner mismatch: $name\n" unless read_bytes("$saved/$name") eq read_bytes("$repo/bench/compiler-scale-refusal-data/launch-1/ownership/$name");
    }
    my $action = read_bytes("$saved/action");
    die "incomplete cleanup action\n" unless $action =~ /\Acleanup_started_utc=(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ)\nrun=\Q$run\E\nkey_directory=\Q$key\E\ncleanup_completed_utc=(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ)\n\z/ && $2 ge $1;
    my %stores;
    for my $line (split /\n/, read_bytes("$saved/store-apparent-bytes")) {
        $line =~ /\A\d+\s+\Q$run\E\/(store-100000-build|store-1000000-build|store-1000000-check)\z/ or die "unexpected cleanup store record\n";
        die "duplicate cleanup store record\n" if $stores{$1}++;
    }
    die "missing cleanup store record\n" unless keys(%stores) == 3;
    die "unexpected final evidence entries\n" unless keys(%{$live->{files}}) == 98 && $live->{directories} == 2;
    $out{cleanup_records} = 6;
    $out{committed_cleanup_records} = JSON::PP::true;
    $out{fixture_and_key_absent} = JSON::PP::true;
    $out{total_mirror_files} = 98;
}
print JSON::PP->new->canonical->pretty->encode(\%out);
