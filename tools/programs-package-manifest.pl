#!/usr/bin/env perl
use strict;
use warnings;
use Archive::Tar;
use Cwd qw(abs_path);
use Digest::SHA qw(sha1_hex sha256_hex);
use Encode qw(decode encode FB_CROAK);
use Fcntl qw(O_RDONLY O_WRONLY O_CREAT O_EXCL O_NOFOLLOW O_NONBLOCK);
use Getopt::Long qw(GetOptions);
use IO::Uncompress::Gunzip qw(gunzip $GunzipError);
use JSON::PP;

# This trusted producer enumerates an exact Git tree or a checksum-pinned archive.
# It does not resolve Cargo, execute package code, or authenticate its publisher.
# The first profile admits original regular files only. Filesystem namespaces must
# be quiescent while this producer reads them; the reader later verifies owned bytes.
Getopt::Long::Configure(qw(no_auto_abbrev no_ignore_case));
my ($source, $git, $archive, $revision, $checksum, $name, $version, $origin, $output);
my (@notices, @dependency_pins);
GetOptions('source=s' => \$source, 'git' => \$git, 'archive=s' => \$archive,
    'revision=s' => \$revision, 'checksum=s' => \$checksum, 'name=s' => \$name,
    'version=s' => \$version, 'origin=s' => \$origin, 'output=s' => \$output,
    'notice=s@' => \@notices, 'dependency-pin=s@' => \@dependency_pins) or die "invalid arguments\n";
die "use --source ROOT (--git --revision COMMIT | --archive FILE --checksum SHA256) --name NAME --version VERSION --origin ORIGIN --notice PATH --output PREFIX\n"
    if @ARGV || !defined($source) || !defined($output) || !defined($name) || !defined($version)
        || !defined($origin) || !$git == !defined($archive) || !@notices;
die "invalid package name\n" unless $name =~ /\A[A-Za-z0-9_-]+\z/;
for ($name, $version, $origin) { die "invalid identity text\n" if !length($_) || /[\x00-\x1f\x7f]/; }
$source = abs_path($source) // die "cannot locate source root\n";
die "source is not a directory\n" unless -d $source;
my $json = JSON::PP->new->canonical(1)->utf8(1)->allow_nonref(0);

sub bytes {
    my ($path) = @_;
    sysopen(my $file, $path, O_RDONLY | O_NOFOLLOW | O_NONBLOCK) or die "open $path: $!\n";
    binmode($file);
    my @stat = stat($file);
    die "not a regular file: $path\n" unless @stat && -f $file;
    local $/;
    my $result = scalar <$file>;
    defined($result) or $result = '';
    close($file) or die "close $path: $!\n";
    return $result;
}
sub path_text {
    my ($raw) = @_;
    my $path = utf8::is_utf8($raw) ? $raw : decode('UTF-8', $raw, FB_CROAK);
    die "noncanonical source path\n" if !length($path) || $path =~ m{\A/|[\\\x00-\x1f\x7f]};
    for my $part (split m{/}, $path, -1) { die "noncanonical path component\n" if $part eq '' || $part eq '.' || $part eq '..'; }
    return $path;
}
sub source_bytes {
    my ($text) = @_;
    my @parts = split m{/}, encode('UTF-8', $text, FB_CROAK);
    my $path = $source;
    for my $index (0 .. $#parts) {
        $path .= '/' . $parts[$index];
        my @stat = lstat($path);
        die "missing source node: $text\n" unless @stat;
        die "symlink source node: $text\n" if -l _;
        die "non-directory source parent: $text\n" if $index < $#parts && !-d _;
        die "nonregular source leaf: $text\n" if $index == $#parts && !-f _;
    }
    return bytes($path);
}
sub git_output {
    my (@args) = @_;
    local %ENV = %ENV;
    delete @ENV{qw(GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_COMMON_DIR GIT_NAMESPACE GIT_CONFIG_PARAMETERS GIT_CONFIG_COUNT)};
    $ENV{GIT_CONFIG_NOSYSTEM} = '1';
    $ENV{GIT_CONFIG_GLOBAL} = '/dev/null';
    $ENV{GIT_OPTIONAL_LOCKS} = '0';
    open(my $pipe, '-|', 'git', '--no-replace-objects', '-c', 'core.fsmonitor=false',
        '-c', 'core.hooksPath=/dev/null', '-C', $source, @args) or die "start git: $!\n";
    binmode($pipe);
    local $/;
    my $result = scalar <$pipe>;
    defined($result) or $result = '';
    close($pipe) or die "git command failed\n";
    return $result;
}
sub tar_framing {
    my ($bytes) = @_;
    die "tar length is not block-aligned\n" if length($bytes) % 512;
    my ($at, $headers) = (0, 0);
    while ($at + 512 <= length($bytes)) {
        my $header = substr($bytes, $at, 512);
        if ($header eq "\0" x 512) {
            die "tar lacks two final zero blocks\n" if $at + 1024 > length($bytes);
            die "trailing tar payload\n" unless substr($bytes, $at) =~ /\A\0*\z/;
            return $headers;
        }
        my $size_text = substr($header, 124, 12);
        my $check_text = substr($header, 148, 8);
        die "non-octal tar framing\n" unless $size_text =~ /\A *[0-7]+[\0 ]*\z/ && $check_text =~ /\A *[0-7]+[\0 ]*\z/;
        $size_text =~ s/[\0 ]//g; $check_text =~ s/[\0 ]//g;
        die "empty tar numeric field\n" unless length($size_text) && length($check_text);
        my $size = oct($size_text);
        my $checksum = oct($check_text);
        substr($header, 148, 8) = ' ' x 8;
        die "tar header checksum mismatch\n" unless unpack('%32C*', $header) == $checksum;
        my $type = substr($header, 156, 1);
        die "unsupported tar node/extension\n" unless $type eq "\0" || $type eq '0' || $type eq '5';
        die "directory with tar payload\n" if $type eq '5' && $size != 0;
        $at += 512 + int(($size + 511) / 512) * 512;
        die "truncated tar payload\n" if $at > length($bytes);
        ++$headers;
    }
    die "tar has no terminal framing\n";
}
my (%records, $acquisition);
if ($git) {
    die "Git requires an exact hexadecimal revision\n" unless defined($revision) && $revision =~ /\A[0-9a-f]{40}(?:[0-9a-f]{24})?\z/;
    my $head = git_output('rev-parse', '--verify', 'HEAD^{commit}'); chomp($head);
    die "source HEAD differs from requested revision\n" unless $head eq $revision;
    my %tree;
    for my $record (split /\0/, git_output('ls-tree', '-r', '-z', '--full-tree', 'HEAD')) {
        my ($mode, $kind, $object, $raw) = $record =~ /\A([0-7]+) (\S+) ([0-9a-f]+)\t(.*)\z/s;
        die "unsupported Git node\n" unless defined($raw) && $kind eq 'blob' && ($mode eq '100644' || $mode eq '100755');
        my $path = path_text($raw);
        die "duplicate Git path\n" if exists($tree{$path});
        $tree{$path} = [$mode, $object];
    }
    my %index;
    for my $record (split /\0/, git_output('ls-files', '--stage', '-z')) {
        my ($mode, $object, $stage, $raw) = $record =~ /\A([0-7]+) ([0-9a-f]+) (\d+)\t(.*)\z/s;
        die "unmerged or malformed Git index\n" unless defined($raw) && $stage eq '0';
        my $path = path_text($raw);
        die "duplicate index path\n" if exists($index{$path});
        $index{$path} = [$mode, $object];
    }
    die "Git index is not the exact HEAD tree\n" unless $json->encode(\%index) eq $json->encode(\%tree);
    my $format = git_output('rev-parse', '--show-object-format'); chomp($format);
    die "unsupported Git object hash\n" unless $format eq 'sha1' || $format eq 'sha256';
    for my $path (sort keys %tree) {
        my $content = source_bytes($path);
        my $git_bytes = 'blob ' . length($content) . "\0" . $content;
        my $object = $format eq 'sha1' ? sha1_hex($git_bytes) : sha256_hex($git_bytes);
        die "tracked source differs from Git object: $path\n" unless $object eq $tree{$path}[1];
        $records{$path} = [$path, length($content), sha256_hex($content)];
    }
    $acquisition = { kind => 'git', revision => $revision, inventory => 'exact HEAD regular-file tree; index equals HEAD; raw worktree bytes match Git objects' };
} else {
    die "archive requires lowercase SHA256\n" unless defined($checksum) && $checksum =~ /\A[0-9a-f]{64}\z/;
    my $archive_bytes = bytes($archive);
    die "archive checksum mismatch\n" unless sha256_hex($archive_bytes) eq $checksum;
    my ($tar_bytes, $trailing);
    gunzip(\$archive_bytes => \$tar_bytes, Strict => 1, Transparent => 0, MultiStream => 0, TrailingData => $trailing)
        or die "cannot decompress pinned archive: $GunzipError\n";
    die "trailing compressed payload\n" if length($trailing // '');
    my $headers = tar_framing($tar_bytes);
    open(my $archive_input, '<', \$tar_bytes) or die "open owned archive bytes: $!\n";
    binmode($archive_input);
    my $tar = Archive::Tar->new;
    $tar->read($archive_input) or die "cannot read package archive\n";
    close($archive_input) or die "close owned archive bytes: $!\n";
    my $prefix = "$name-$version/";
    my @archive_entries = $tar->get_files;
    die "tar parser omitted a framed node\n" unless @archive_entries == $headers;
    for my $entry (@archive_entries) {
        my $raw = $entry->full_path;
        if ($entry->is_dir) {
            $raw =~ s{/$}{};
            next if $raw eq "$name-$version";
            die "archive directory has wrong package prefix\n" unless index($raw, $prefix) == 0;
            path_text(substr($raw, length($prefix)));
            next;
        }
        die "nonregular archive node\n" unless $entry->is_file && !$entry->is_symlink && !$entry->is_hardlink;
        die "archive member has wrong package prefix\n" unless index($raw, $prefix) == 0;
        my $path = path_text(substr($raw, length($prefix)));
        die "duplicate archive member\n" if exists($records{$path});
        my $content = $entry->get_content;
        die "extracted source differs from archive member: $path\n" unless source_bytes($path) eq $content;
        $records{$path} = [$path, length($content), sha256_hex($content)];
    }
    $revision = "sha256:$checksum";
    $acquisition = { kind => 'registry-archive', archive_sha256 => $checksum,
        inventory => 'one fully consumed gzip stream and tar archive, regular files/directories without extension records; exact regular-file members match extracted bytes; extra acquisition state is not admitted' };
}
die "empty source inventory\n" unless keys %records;
my @files = map { $records{$_} } sort keys %records;
my %notice_seen;
@notices = sort map { path_text($_) } @notices;
for my $notice (@notices) {
    die "duplicate or unlisted notice: $notice\n" if $notice_seen{$notice}++ || !exists($records{$notice});
}
my @edges;
for my $path (@dependency_pins) {
    my $raw = bytes($path);
    my $pin = $json->decode($raw);
    die "invalid dependency pin\n" unless ref($pin) eq 'ARRAY' && @$pin == 3 && $pin->[0] eq 'lkjscript-package-pin-1'
        && ref($pin->[1]) eq 'ARRAY' && @{$pin->[1]} == 5 && ref($pin->[2]) eq 'ARRAY' && $json->encode($pin) eq $raw;
    push @edges, $pin->[1];
}
@edges = sort { $a->[0] cmp $b->[0] } @edges;
for my $index (0 .. $#edges) {
    my $edge = $edges[$index];
    for (@$edge) { die "invalid dependency identity field\n" if ref($_) || !defined($_) || !length($_) || /[\x00-\x1f\x7f]/; }
    die "invalid dependency identity\n" unless $edge->[0] =~ /\A[A-Za-z0-9_-]+\z/ && $edge->[4] =~ /\A[0-9a-f]{64}\z/;
    die "duplicate dependency name\n" if $index && $edges[$index - 1][0] eq $edge->[0];
}
my $tree = sha256_hex("lkjscript-source-tree-1\0" . $json->encode(\@files));
my $identity = [$name, $version, $origin, $revision, $tree];
my @outputs = (
    ["$output.manifest.json", $json->encode(['lkjscript-package-source-1-regular', $identity, \@edges, \@files, \@notices])],
    ["$output.pin.json", $json->encode(['lkjscript-package-pin-1', $identity, \@edges])],
);
my @created;
eval {
    for my $item (@outputs) {
        sysopen(my $file, $item->[0], O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW, 0600) or die "create $item->[0]: $!\n";
        push @created, $item->[0];
        binmode($file);
        print {$file} $item->[1] or die "write $item->[0]: $!\n";
        close($file) or die "close $item->[0]: $!\n";
    }
    1;
} or do { my $error = $@; unlink($_) for @created; die $error; };
print $json->encode({ identity => $identity, source_root => $source, files => scalar(@files),
    notices => \@notices, dependencies => \@edges, acquisition => $acquisition,
    configuration => 'build target, features and test selection are separate caller policy',
    authentication => 'local issuer only; this inventory producer has no key and makes no publisher-signature or execution-authority claim' }), "\n";
