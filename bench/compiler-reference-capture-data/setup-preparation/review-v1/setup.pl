#!/usr/bin/env perl
use strict;
use warnings;
use Cwd qw(abs_path);
use Fcntl qw(:DEFAULT :mode);
use Digest::SHA qw(sha1_hex sha256_hex);
use JSON::PP;
use Archive::Tar;
use IO::Compress::Gzip qw(gzip $GzipError);
use IO::Uncompress::Gunzip qw(gunzip $GunzipError);
use File::Find;
use File::Basename qw(dirname);

my ($mode, $root, $config_path, $phase_name, $phase_status) = @ARGV;
die "usage: setup.pl recover|factory|freeze ROOT CONTROLS | validate ROOT CONTROLS PHASE STATUS\n"
    unless defined($mode) && ((@ARGV == 3 && $mode =~ /\A(?:recover|factory|freeze)\z/)
        || (@ARGV == 5 && $mode eq 'validate'));
my $json = JSON::PP->new->canonical->pretty;
sub read_regular {
    my ($path) = @_;
    sysopen my $file, $path, O_RDONLY | O_NOFOLLOW or die "open $path: $!\n";
    my @st = stat $file;
    die "not a bounded regular file: $path\n" unless @st && S_ISREG($st[2]) && $st[7] <= 134217728;
    binmode $file;
    local $/;
    my $bytes = <$file>;
    $bytes = '' unless defined $bytes;
    close $file or die "close $path: $!\n";
    die "short or changed read: $path\n" unless length($bytes) == $st[7];
    return $bytes;
}
sub new_file {
    my ($path, $bytes, $permissions) = @_;
    sysopen my $file, $path, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW, 0600 or die "create $path: $!\n";
    binmode $file;
    print {$file} $bytes or die "write $path: $!\n";
    close $file or die "close $path: $!\n";
    chmod($permissions // 0600, $path) == 1 or die "chmod $path: $!\n";
}
sub write_json { new_file($_[0], $json->encode($_[1])); }
sub digest_record { return { bytes => length($_[0]), sha256 => sha256_hex($_[0]) }; }
sub assert_pin {
    my ($bytes, $pin, $label) = @_;
    die "$label digest mismatch\n" unless sha256_hex($bytes) eq $pin->{sha256};
    die "$label size mismatch\n" if defined($pin->{bytes}) && length($bytes) != $pin->{bytes};
}
my $config = decode_json(read_regular($config_path));
die "wrong scope\n" unless $root eq '/home/coder/workspace/lkjscript/target/compiler-reference-capture-measurement-1'
    && $config->{run_root} eq $root && $config->{repository} eq '/home/coder/workspace/lkjscript';
my $repo = $config->{repository};
my @root_st = lstat $root;
die "unsafe owned root\n" unless @root_st && S_ISDIR($root_st[2]) && $root_st[4] == $> && abs_path($root) eq $root;
die "comparison is not part of setup\n" if $config->{comparison_authorized};
my $art = "$root/artifacts";
sub note_command {
    my (@argv) = @_;
    sysopen my $file, "$root/records/metadata-subcommands.jsonl", O_WRONLY | O_APPEND | O_CREAT | O_NOFOLLOW, 0600 or die $!;
    print {$file} JSON::PP->new->canonical->encode({ mode => $mode, argv => \@argv }), "\n" or die $!;
    close $file or die $!;
}
sub capture {
    my ($allowed, @argv) = @_;
    note_command(@argv);
    open my $pipe, '-|', @argv or die "start command: $!\n";
    binmode $pipe;
    local $/;
    my $bytes = <$pipe>;
    $bytes = '' unless defined $bytes;
    close $pipe;
    my $status = $?;
    die "metadata command failed ($status)\n" unless $status == ($allowed << 8);
    return $bytes;
}
sub git_blob {
    my ($revision, $path) = @_;
    die "unsafe git revision\n" unless $revision =~ /\A[0-9a-f]{40}\z/;
    return capture(0, 'git', '-C', $repo, 'show', "$revision:$path");
}
sub safe_name {
    my ($name, $directory) = @_;
    $name =~ s{\A\./}{};
    $name =~ s{/\z}{} if $directory;
    return '' if $directory && ($name eq '' || $name eq '.');
    die "unsafe archive/tree path\n" if $name eq '' || $name =~ m{\A/|\\|[\x00-\x1f\x7f]};
    for my $part (split m{/}, $name, -1) { die "unsafe path component\n" if $part eq '' || $part eq '.' || $part eq '..'; }
    return $name;
}
sub ensure_dir {
    my ($base, $relative) = @_;
    my $at = $base;
    for my $part (split m{/}, $relative) {
        next if $part eq '';
        $at .= "/$part";
        my @st = lstat $at;
        if (@st) { die "unsafe existing directory $at\n" unless S_ISDIR($st[2]) && $st[4] == $>; }
        else { mkdir($at, 0700) or die "mkdir $at: $!\n"; }
    }
}
sub expected_git {
    my ($revision) = @_;
    my $raw = capture(0, 'git', '-C', $repo, 'ls-tree', '-r', '-z', '--full-tree', $revision, '--', 'src', 'library', 'tests');
    my (%files, %directories);
    for my $row (split /\0/, $raw) {
        $row =~ /\A(100644|100755) blob ([0-9a-f]{40})\t(.+)\z/s or die "nonregular Git member\n";
        my ($permissions, $oid, $path) = ($1, $2, safe_name($3, 0));
        die "unexpected Git root\n" unless $path =~ m{\A(?:src|library|tests)/};
        die "duplicate Git member\n" if exists $files{$path};
        $files{$path} = { git_mode => $permissions, git_blob => $oid };
        my @parts = split m{/}, $path; pop @parts;
        while (@parts) { $directories{join('/', @parts)} = 1; pop @parts; }
    }
    die "wrong source file population\n" unless keys(%files) == $config->{compiler_input}{regular_members};
    return (\%files, \%directories);
}
sub load_tar {
    my ($path, $compressed) = @_;
    my $tar = Archive::Tar->new;
    $tar->read($path, $compressed) or die "archive read failed: " . Archive::Tar->error . "\n";
    return $tar;
}
sub source_extract {
    my ($destination) = @_;
    die "destination already exists\n" if lstat $destination;
    my $source = read_regular("$art/compiler-source.tar");
    assert_pin($source, $config->{compiler_input}, 'compiler archive');
    my ($expected, $directories) = expected_git($config->{compiler_input}{revision});
    my $tar = load_tar("$art/compiler-source.tar", 0);
    my (%seen, %seen_dirs, @files, $global);
    for my $member ($tar->get_files) {
        if ($member->type eq 'g') {
            die "unexpected global PAX metadata\n" if $global++ || $member->name ne 'pax_global_header'
                || $member->get_content ne ("52 comment=" . $config->{compiler_input}{revision} . "\n");
            next;
        }
        my $is_dir = $member->is_dir;
        die "special archive member\n" unless $is_dir || $member->is_file;
        my $path = safe_name($member->full_path, $is_dir);
        die "unexpected archive root member\n" if $path eq '';
        if ($is_dir) {
            die "unexpected/duplicate archive directory\n" unless $directories->{$path} && !$seen_dirs{$path}++;
            next;
        }
        die "unexpected/duplicate archive file\n" unless exists($expected->{$path}) && !$seen{$path}++;
        my $bytes = $member->get_content;
        my $oid = sha1_hex("blob " . length($bytes) . "\0" . $bytes);
        die "Git content mismatch: $path\n" unless $oid eq $expected->{$path}{git_blob};
        die "unsafe archive permissions\n" if $member->mode & 07000;
        my $exec = ($member->mode & 0111) != 0;
        die "archive executable-mode mismatch\n" unless $exec == ($expected->{$path}{git_mode} eq '100755');
        push @files, { path => $path, %{digest_record($bytes)}, %{$expected->{$path}}, archive_mode => $member->mode, data => $bytes };
    }
    die "incomplete source archive\n" unless keys(%seen) == keys(%$expected) && keys(%seen_dirs) == keys(%$directories);
    mkdir($destination, 0700) or die $!;
    for my $directory (sort { length($a) <=> length($b) || $a cmp $b } keys %$directories) { ensure_dir($destination, $directory); }
    my @manifest;
    for my $file (sort { $a->{path} cmp $b->{path} } @files) {
        my %entry = %$file;
        my $bytes = delete $entry{data};
        new_file("$destination/$entry{path}", $bytes, $entry{git_mode} eq '100755' ? 0700 : 0600);
        push @manifest, \%entry;
    }
    assert_pin(read_regular("$art/compiler-source.tar"), $config->{compiler_input}, 'compiler archive after extraction');
    return \@manifest;
}
sub tree_inventory {
    my ($base) = @_;
    my (@files, @directories);
    my @root = lstat $base;
    die "missing regular directory\n" unless @root && S_ISDIR($root[2]) && $root[4] == $> && abs_path($base) eq $base;
    find({ no_chdir => 1, follow => 0, wanted => sub {
        my $path = $File::Find::name;
        return if $path eq $base;
        my @st = lstat $path;
        die "foreign or missing owned member\n" unless @st && $st[4] == $>;
        my $relative = substr($path, length($base) + 1);
        if (S_ISDIR($st[2])) { push @directories, safe_name($relative, 1); return; }
        die "special or hardlinked member\n" unless S_ISREG($st[2]) && $st[3] == 1;
        $relative = safe_name($relative, 0);
        my $bytes = read_regular($path);
        push @files, { path => $relative, %{digest_record($bytes)}, mode => $st[2] & 07777 };
    }}, $base);
    return { files => [sort { $a->{path} cmp $b->{path} } @files], directories => [sort @directories] };
}
sub equal_inventory {
    my ($actual, $expected, $label) = @_;
    die "$label changed\n" unless $json->encode($actual) eq $json->encode($expected);
}
sub verify_gzip {
    my ($archive, $expected, $label) = @_;
    my $compressed = read_regular($archive);
    my $decoded = '';
    gunzip(\$compressed => \$decoded) or die "gunzip $label: $GunzipError\n";
    die "$label gzip recovery mismatch\n" unless $decoded eq $expected;
}
sub gzip_bytes {
    my ($bytes, $path) = @_;
    my $encoded = '';
    gzip(\$bytes => \$encoded, Time => 0, Name => '', Comment => '', Level => 9) or die "gzip: $GzipError\n";
    new_file($path, $encoded);
    verify_gzip($path, $bytes, $path);
}
sub write_hashes {
    my ($path, $entries) = @_;
    new_file($path, join('', map { "$_->{sha256}  $_->{path}\n" } @$entries));
}
if ($mode eq 'validate') {
    die "invalid phase record selector\n" unless $phase_name =~ /\A(?:recover-inputs|prepare-factory|generator-build|generate-100k|freeze-artifacts)\z/
        && $phase_status =~ /\A[0-9]+\z/ && $phase_status <= 255;
    my $record = "$root/records/$phase_name";
    die "recorded wrapper status differs\n" unless read_regular("$record/status") eq "$phase_status\n";
    my %timing;
    for my $line (split /\n/, read_regular("$record/time")) {
        next unless $line =~ /\A(wall_s|user_s|system_s|max_rss_kib|exit)=([0-9]+(?:\.[0-9]+)?)\z/;
        die "repeated GNU time field\n" if exists $timing{$1};
        $timing{$1} = $2;
    }
    die "incomplete GNU time record\n" unless keys(%timing) == 5;
    die "GNU/wrapper exit mismatch\n" unless $timing{exit} =~ /\A[0-9]+\z/ && $timing{exit} == $phase_status;
    die "invalid RSS\n" unless $timing{max_rss_kib} =~ /\A[0-9]+\z/;
    my $start = read_regular("$record/start.ns"); chomp $start;
    my $end = read_regular("$record/end.ns"); chomp $end;
    die "invalid/backward capture timestamps\n" unless $start =~ /\A[0-9]{19}\z/ && $end =~ /\A[0-9]{19}\z/ && $end >= $start;
    for my $name (qw(start.utc end.utc)) {
        die "incomplete UTC capture\n" unless read_regular("$record/$name") =~ /\A[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z\n\z/;
    }
    for my $name (qw(load-before load-after)) {
        die "incomplete load capture\n" unless read_regular("$record/$name") =~ /\A[0-9]+(?:\.[0-9]+)? [0-9]+(?:\.[0-9]+)? [0-9]+(?:\.[0-9]+)?\n\z/;
    }
    die "actual cwd differs\n" unless read_regular("$record/cwd") eq read_regular("$record/cwd.actual");
    read_regular("$record/stdout");
    read_regular("$record/stderr");
    print $json->encode({ phase => $phase_name, status => 0 + $phase_status, complete_metrics => JSON::PP::true,
        timing => \%timing, outer_start_ns => $start, outer_end_ns => $end });
    exit 0;
}
if ($mode eq 'recover') {
    assert_pin(read_regular($config->{accepted_plan}{path}), $config->{accepted_plan}, 'accepted plan');
    my $seed = git_blob($config->{baseline}{recovery}{commit}, $config->{baseline}{recovery}{path});
    assert_pin($seed, { sha256 => $config->{baseline}{recovery}{archive_sha256}, bytes => $config->{baseline}{recovery}{archive_bytes} }, 'seed archive');
    new_file("$art/baseline-A.gz", $seed);
    my $baseline = '';
    gunzip(\$seed => \$baseline) or die "seed gunzip: $GunzipError\n";
    assert_pin($baseline, $config->{baseline}, 'baseline A');
    new_file("$root/producer-A", $baseline, 0500);
    my $source = read_regular($config->{compiler_input}{path});
    assert_pin($source, $config->{compiler_input}, 'input source archive');
    new_file("$art/compiler-source.tar", $source);
    my $generator = git_blob($config->{generator_source}{revision}, $config->{generator_source}{path});
    assert_pin($generator, $config->{generator_source}, 'generator source');
    new_file("$art/scale-generator-source.lkj", $generator);
    my $shape = git_blob($config->{shape}{revision}, $config->{shape}{path});
    assert_pin($shape, $config->{shape}, 'shape');
    new_file("$art/shape", $shape);
    my $members = source_extract("$root/compiler-input");
    write_json("$art/compiler-archive-members.json", $members);
    write_json("$art/compiler-inputs.json", tree_inventory("$root/compiler-input"));
    assert_pin(read_regular("$root/compiler-input/library/runtime.lkj"),
        { sha256 => $config->{compiler_input}{runtime_sha256} }, 'frozen runtime');
    mkdir("$root/generated-100k", 0700) or die $!;
    write_json("$art/recovery.json", { baseline => digest_record($baseline), source_archive => digest_record($source), source_files => scalar(@$members), extraction => 'content matches exact Git blobs; private owner-only modes' });
    print "Recovered exact A and all $config->{compiler_input}{regular_members} source files; no compiler executed.\n";
} elsif ($mode eq 'factory') {
    my $members = source_extract("$root/factory");
    write_json("$art/factory-archive-members.json", $members);
    my $before = tree_inventory("$root/factory");
    write_json("$art/factory-before.json", $before);
    my $original = read_regular("$root/factory/src/main.lkj");
    new_file("$art/factory-main.original.lkj", $original);
    my $generator = read_regular("$art/scale-generator-source.lkj");
    assert_pin($generator, $config->{generator_source}, 'generator source');
    sysopen my $main, "$root/factory/src/main.lkj", O_WRONLY | O_TRUNC | O_NOFOLLOW or die $!;
    binmode $main;
    print {$main} $generator or die $!;
    close $main or die $!;
    my $after = tree_inventory("$root/factory");
    my @changed;
    die "factory population changed\n" unless @{$before->{files}} == @{$after->{files}}
        && $json->encode($before->{directories}) eq $json->encode($after->{directories});
    for my $i (0 .. $#{$before->{files}}) {
        my ($a, $b) = ($before->{files}[$i], $after->{files}[$i]);
        die "factory path/mode changed\n" unless $a->{path} eq $b->{path} && $a->{mode} == $b->{mode};
        push @changed, $a->{path} if $a->{sha256} ne $b->{sha256} || $a->{bytes} != $b->{bytes};
    }
    die "factory overlay is not exactly src/main.lkj\n" unless @changed == 1 && $changed[0] eq 'src/main.lkj';
    write_json("$art/factory-after.json", $after);
    my $diff = capture(1, 'diff', '-u', '--label', 'src/main.lkj', '--label', 'src/main.lkj', "$art/factory-main.original.lkj", "$art/scale-generator-source.lkj");
    new_file("$art/factory-main.diff", $diff);
    print "Factory differs from exact source only at src/main.lkj; no compiler executed.\n";
} else {
    equal_inventory(tree_inventory("$root/compiler-input"), decode_json(read_regular("$art/compiler-inputs.json")), 'compiler input');
    equal_inventory(tree_inventory("$root/factory"), decode_json(read_regular("$art/factory-after.json")), 'factory');
    assert_pin(read_regular("$root/producer-A"), $config->{baseline}, 'baseline A after setup');
    assert_pin(read_regular("$art/shape"), $config->{shape}, 'shape after setup');
    my $build_log = read_regular("$root/records/generator-build/stderr");
    my @stats = $build_log =~ /^LKJSCRIPT_STATS (.+)$/mg;
    die "missing/repeated generator build stats\n" unless @stats == 1;
    my $stats = decode_json($stats[0]);
    die "generator build not complete\n" unless $stats->{route} eq 'compiler' && $stats->{stopped_stage} eq 'done';
    for my $name (qw(front_reused checked_reused checked_local_reused emitted_reused build_reused inline_attempts)) {
        die "unexpected generator build reuse/pass: $name\n" unless defined($stats->{$name}) && $stats->{$name} == 0;
    }
    my $generator = read_regular("$root/generator");
    die "generator image byte count mismatch\n" unless $stats->{emitted_bytes} == length($generator);
    write_json("$art/generator-build-stats.json", $stats);
    gzip_bytes($generator, "$art/generator.gz");
    my $inventory = tree_inventory("$root/generated-100k");
    my %fields;
    my $report = read_regular("$root/generated-100k/workload.txt");
    die "generated stdout/report mismatch\n" unless $report eq read_regular("$root/records/generate-100k/stdout");
    for my $line (split /\n/, $report) {
        $line =~ /\A([a-z_]+) (\S+)\z/ or die "malformed workload report\n";
        die "duplicate workload field\n" if exists $fields{$1};
        $fields{$1} = $2;
    }
    die "wrong generation seed/lines\n" unless ($fields{seed} // '') eq '20261116' && ($fields{lines} // '') eq '100000';
    my ($lines, $bytes, $files) = (0, 0, 0);
    for my $file (@{$inventory->{files}}) {
        next if $file->{path} eq 'workload.txt';
        die "unexpected generated file\n" unless $file->{path} =~ /\.lkj\z/;
        my $source = read_regular("$root/generated-100k/$file->{path}");
        $lines += ($source =~ tr/\n/\n/);
        ++$lines if length($source) && $source !~ /\n\z/;
        $bytes += length($source);
        ++$files;
    }
    die "generated source census mismatch\n" unless $lines == 100000 && $bytes == $fields{source_bytes} && $files == $fields{files};
    new_file("$art/workload.txt", $report);
    write_json("$art/generated-members.json", $inventory);
    write_hashes("$art/generated-members.sha256", $inventory->{files});
    my $tar_path = "$art/generated-100k.tar";
    die "archive already exists\n" if lstat $tar_path;
    my @tar = ('tar', '--sort=name', '--format=ustar', '--mtime=@0', '--owner=0', '--group=0', '--numeric-owner',
        '-cf', $tar_path, '-C', "$root/generated-100k", '.');
    note_command(@tar);
    system @tar;
    die "generated archive command failed ($?)\n" unless $? == 0;
    my $tar_bytes = read_regular($tar_path);
    gzip_bytes($tar_bytes, "$art/generated-100k.tar.gz");
    my $archive = load_tar("$art/generated-100k.tar.gz", 1);
    my %expected = map { $_->{path} => $_ } @{$inventory->{files}};
    my %expected_dirs = map { $_ => 1 } @{$inventory->{directories}};
    my (%seen, %seen_dirs, $root_seen);
    for my $member ($archive->get_files) {
        die "special generated archive member\n" unless $member->is_file || $member->is_dir;
        my $path = safe_name($member->full_path, $member->is_dir);
        if ($member->is_dir) {
            if ($path eq '') { die "duplicate root directory\n" if $root_seen++; next; }
            die "extra/duplicate generated directory\n" unless $expected_dirs{$path} && !$seen_dirs{$path}++;
            next;
        }
        die "extra/duplicate generated file\n" unless exists($expected{$path}) && !$seen{$path}++;
        my $content = $member->get_content;
        die "archive member bytes differ: $path\n" unless $content eq read_regular("$root/generated-100k/$path");
    }
    die "incomplete generated archive\n" unless $root_seen && keys(%seen) == keys(%expected) && keys(%seen_dirs) == keys(%expected_dirs);
    equal_inventory(tree_inventory("$root/generated-100k"), $inventory, 'generated input after archive');
    my @artifact_files;
    opendir my $directory, $art or die $!;
    for my $name (sort readdir $directory) {
        next if $name eq '.' || $name eq '..';
        my $content = read_regular("$art/$name");
        push @artifact_files, { path => "artifacts/$name", %{digest_record($content)} };
    }
    closedir $directory or die $!;
    write_json("$root/setup-summary.json", {
        schema => 'compiler-reference-capture-setup-result-1',
        status => 'setup-complete-not-an-ab-release', comparison_executed => JSON::PP::false,
        baseline => digest_record(read_regular("$root/producer-A")),
        generator => digest_record($generator),
        generated_archive => digest_record(read_regular("$art/generated-100k.tar.gz")),
        generated => { lines => $lines, source_bytes => $bytes, source_files => $files, total_regular_members => scalar(@{$inventory->{files}}) },
        source_revision => $config->{compiler_input}{revision},
        archive_member_equality => 'all generated regular member bytes and directory names verified',
        factory_recovery => 'compiler-source.tar plus exact scale-generator-source.lkj; only src/main.lkj differs; full before/after manifests and diff retained',
        candidate => 'not used or authorized by setup',
        artifacts => \@artifact_files
    });
    new_file("$root/artifacts.sha256", join('', map { "$_->{sha256}  $_->{path}\n" } @artifact_files)
        . sha256_hex(read_regular("$root/setup-summary.json")) . "  setup-summary.json\n");
    print "Frozen $files generated source files/$lines lines; full archive member equality passed. No A/B build ran.\n";
}
