use strict;
use warnings;
use Fcntl qw(:DEFAULT :mode);
use File::Find;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use Errno qw(ENOENT);

# Prepared only. Execute --remove only after root releases these exact bytes.
# Cooperative closure, not an atomic filesystem transaction or a race defence.
my $ROOT = '/home/coder/workspace/lkjscript';
my $PREP = "$ROOT/target/compiler-scalar-cohort-cleanup-preparation-1";
my $COMMIT = '0b0b27803d27b932a5b91cde2d766bd3c8bb82b9';
my $PACKAGE = 'bench/compiler-scalar-simplify-data/cohort-package';
my $EMITTER = 'bench/compiler-scalar-simplify-data/preparation/artifacts/f85624e3cc694f6a3a3e10c33f822789ae6084e2d5a5b41c3b1fc8fcc9bd2d64.compiler.gz';
my @ROOTS = ('target/compiler-scalar-simplify.20261116-1',
             'target/compiler-scalar-trial-capture.20261116-1');
my $JSON = JSON::PP->new->canonical;
$| = 1;
die "usage: perl guard.pl --check|--remove\n" unless @ARGV == 1 && ($ARGV[0] eq '--check' || $ARGV[0] eq '--remove');
my $remove = $ARGV[0] eq '--remove';
chdir $ROOT or die "cwd: $!\n";

sub emit { print $JSON->encode(shift), "\n" or die "progress write: $!\n"; }
sub safe_relative {
    my ($p) = @_;
    die "unsafe relative path\n" unless defined($p) && $p =~ m{\A[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*\z}
        && !grep { $_ eq '.' || $_ eq '..' } split m{/}, $p;
}
sub in_scope {
    my ($p) = @_;
    safe_relative($p);
    my @matches = grep { index($p, "$_/") == 0 } @ROOTS;
    die "unknown removal scope: $p\n" unless @matches == 1;
    return $matches[0];
}
sub parent_components {
    my ($path) = @_;
    die "absolute path required\n" unless $path =~ m{\A/};
    my @parts = split m{/}, $path;
    pop @parts;
    my $p = '';
    for my $part (@parts) {
        next unless length $part;
        die "unsafe absolute component\n" if $part eq '.' || $part eq '..';
        $p .= "/$part";
        my @s = lstat($p);
        die "parent is not a real directory: $p\n" unless @s && S_ISDIR($s[2]);
    }
}
sub regular {
    my ($path) = @_;
    parent_components($path);
    my @before = lstat($path);
    die "not a regular file: $path\n" unless @before && S_ISREG($before[2]);
    sysopen my $f, $path, O_RDONLY | O_NOFOLLOW or die "open $path: $!\n";
    binmode $f;
    my @opened = stat($f);
    die "opened file differs: $path\n" unless @opened && S_ISREG($opened[2])
        && $opened[0] == $before[0] && $opened[1] == $before[1];
    return ($f, \@opened);
}
sub hash_file {
    my ($path) = @_;
    my ($f, $s) = regular($path);
    my $sha = Digest::SHA->new(256);
    my $git = Digest::SHA->new(1);
    $git->add('blob ' . $s->[7] . "\0");
    my $bytes = 0;
    while (1) {
        my $n = read($f, my $b, 65536);
        die "read $path: $!\n" unless defined $n;
        last unless $n;
        $bytes += $n; $sha->add($b); $git->add($b);
    }
    my @end = stat($f);
    close $f or die "close $path: $!\n";
    my @leaf = lstat($path);
    die "file changed during read: $path\n" unless @end && @leaf && S_ISREG($leaf[2])
        && $bytes == $s->[7] && $end[0] == $s->[0] && $end[1] == $s->[1]
        && $end[7] == $s->[7] && $end[9] == $s->[9] && $end[10] == $s->[10]
        && $leaf[0] == $s->[0] && $leaf[1] == $s->[1] && $leaf[7] == $s->[7]
        && $leaf[9] == $s->[9] && $leaf[10] == $s->[10];
    # Digest::SHA finalizers consume/reset state: store each result once.
    my $h = $sha->hexdigest; my $g = $git->hexdigest;
    return { sha256 => $h, git_blob => $g, bytes => $bytes, dev => $s->[0],
        ino => $s->[1], mode => sprintf('%04o', $s->[2] & 07777), mtime => $s->[9] };
}
sub raw_pinned {
    my ($path, $pin) = @_;
    my ($f, $s) = regular($path);
    die "metadata cap: $path\n" if $s->[7] > 16777216;
    local $/;
    my $b = <$f>; $b = '' unless defined $b;
    close $f or die "close $path: $!\n";
    die "metadata pin: $path\n" unless length($b) == $s->[7] && sha256_hex($b) eq $pin;
    return $b;
}
sub json_pinned { return decode_json(raw_pinned(@_)); }
sub match_file {
    my ($path, $e, $identity) = @_;
    my $a = hash_file($path);
    die "changed file: $path\n" unless $a->{sha256} eq $e->{sha256} && $a->{bytes} == $e->{bytes};
    if ($identity) {
        die "changed file identity: $path\n" unless $a->{dev} == $e->{dev} && $a->{ino} == $e->{ino}
            && $a->{mode} eq $e->{mode} && (!exists($e->{mtime}) || $a->{mtime} == $e->{mtime});
    }
    return $a;
}

my $snap = json_pinned("$PREP/snapshot.json", '529395e9b74464531137d27bc05b72df3d368bbe0c77edfbea97f68149efe132');
die "snapshot root contract\n" unless $snap->{repo} eq $ROOT && $snap->{commit} eq $COMMIT
    && join("\0", @{$snap->{roots}}) eq join("\0", @ROOTS) && $snap->{files} == 337
    && $snap->{bytes} == 207027139 && @{$snap->{directories}} == 22;
for my $p (@{$snap->{parents}}) {
    parent_components("$p->{path}/entry");
    my @s = lstat($p->{path});
    die "parent identity changed: $p->{path}\n" unless @s && S_ISDIR($s[2])
        && $s[0] == $p->{dev} && $s[1] == $p->{ino};
}
my %directories;
for my $d (@{$snap->{directories}}) {
    safe_relative($d->{path});
    die "unknown directory root\n" unless grep { $d->{path} eq $_ || index($d->{path}, "$_/") == 0 } @ROOTS;
    die "duplicate directory\n" if exists $directories{$d->{path}};
    $directories{$d->{path}} = $d;
}
sub check_directory {
    my ($r) = @_;
    my $e = $directories{$r} or die "directory not enumerated: $r\n";
    parent_components("$ROOT/$r/entry");
    my @s = lstat("$ROOT/$r");
    die "directory identity changed: $r\n" unless @s && S_ISDIR($s[2])
        && $s[0] == $e->{dev} && $s[1] == $e->{ino} && sprintf('%04o', $s[2] & 07777) eq $e->{mode};
}
sub check_removal_parents {
    my ($r) = @_;
    my $scope = in_scope($r);
    my $p = $r;
    while ($p ne $scope) {
        $p =~ s{/[^/]+\z}{} or die "parent chain\n";
        check_directory($p);
    }
}
my $canonical = $snap->{canonical};
die "canonical contract\n" unless $canonical->{path} eq 'target/lkjscript'
    && $canonical->{sha256} eq '6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc'
    && $canonical->{bytes} == 32584542;
match_file("$ROOT/$canonical->{path}", $canonical, 1);

# Every recovery file must match both the accepted SHA-256 manifest and its
# exact committed Git blob. No decompression or new recovery representation.
my $manifest = raw_pinned("$ROOT/$PACKAGE/manifest.sha256", '3bea8d64d065041a61c44e654c7e65315f4a6f28ddd0d8124310d9b69e602438');
my %recovery;
for my $line (split /\n/, $manifest) {
    die "recovery manifest framing\n" unless $line =~ /\A([0-9a-f]{64})  (.+)\z/;
    my ($h, $rel) = ($1, $2); safe_relative($rel);
    die "duplicate recovery member\n" if exists $recovery{"$PACKAGE/$rel"};
    $recovery{"$PACKAGE/$rel"} = $h;
}
die "recovery manifest count\n" unless keys(%recovery) == 70;
$recovery{"$PACKAGE/manifest.sha256"} = '3bea8d64d065041a61c44e654c7e65315f4a6f28ddd0d8124310d9b69e602438';
$recovery{$EMITTER} = 'ec4d686eb3a296c97757c479ce308a7c6388f3da8d30dca0079af9cb5684e7c2';
open my $git, '-|', '/usr/bin/git', '-C', $ROOT, 'ls-tree', '-r', '-z', '--full-tree',
    $COMMIT, '--', $PACKAGE, $EMITTER or die "git ls-tree: $!\n";
binmode $git;
my $tree; { local $/; $tree = <$git>; }
close $git or die "committed recovery lookup failed: $?\n";
my %committed;
for my $row (split /\0/, $tree) {
    die "git tree framing\n" unless $row =~ /\A(100644|100755) blob ([0-9a-f]{40})\t(.+)\z/s;
    my ($blob, $p) = ($2, $3); safe_relative($p);
    die "unexpected committed recovery: $p\n" unless exists $recovery{$p} && !exists $committed{$p};
    $committed{$p} = $blob;
}
die "committed recovery population\n" unless keys(%committed) == 72 && keys(%recovery) == 72;
my %recovered;
for my $p (sort keys %recovery) {
    my $a = hash_file("$ROOT/$p");
    die "recovery not committed/exact: $p\n" unless $a->{sha256} eq $recovery{$p}
        && $a->{git_blob} eq $committed{$p};
    $recovered{$p} = $a;
}
die "existing emitter recovery contract\n" unless $recovered{$EMITTER}{bytes} == 6582463
    && $committed{$EMITTER} eq '95a68fd1c3c298e3686138bb28ad6bf63036e8e2';

my $inv = json_pinned("$ROOT/$PACKAGE/retention-proposal/inventory.json", '235e976799d2640bf6599068204d7955bd359673696b3e657387b021f822a070');
my $policy = json_pinned("$ROOT/$PACKAGE/retention-proposal/later-artifacts.json", '5f99c5746ce18811468904ff4ff6a979fa994634f242df518a2076abaed6e1de');
my $proof = json_pinned("$ROOT/$PACKAGE/verification-corrected.json", '1df0de7ff739413788067a4c131a7d74889f3a7a2b56784baeb349052e7fd8da');
die "inventory/proof contract\n" unless $inv->{root} eq $ROOT && $inv->{total_files} == 337
    && $inv->{total_bytes} == 207027139 && @{$inv->{files}} == 337
    && $proof->{all_original_inputs_unchanged} && $proof->{proposal_copy_files} == 18
    && @{$proof->{metadata_correction}{corrected_fields}} == 7;
die "scope list\n" unless join("\0", map { $_->{path} } @{$inv->{scopes}}) eq join("\0", @ROOTS);
my (%all, %mapped, %class_counts);
my $total = 0;
for my $e (@{$inv->{files}}) {
    my $scope = in_scope($e->{path});
    die "inventory scope/path mismatch\n" unless $e->{scope} eq $scope && "$scope/$e->{relative}" eq $e->{path};
    die "duplicate source leaf\n" if exists $all{$e->{path}};
    die "bad digest or length\n" unless $e->{sha256} =~ /\A[0-9a-f]{64}\z/ && $e->{bytes} =~ /\A[0-9]+\z/;
    $all{$e->{path}} = $e; $class_counts{$e->{classification}}++; $total += $e->{bytes};
}
die "inventory total\n" unless keys(%all) == 337 && $total == 207027139;
sub map_original {
    my ($p, $class, $h, $n, $recovery) = @_;
    my $e = $all{$p} or die "recovery maps outside original population: $p\n";
    die "duplicate original recovery: $p\n" if exists $mapped{$p};
    die "original recovery mismatch: $p\n" unless $e->{classification} eq $class
        && $e->{sha256} eq $h && $e->{bytes} == $n;
    $mapped{$p} = $recovery;
}
sub artifact {
    my ($rel, $h, $n) = @_;
    safe_relative($rel);
    my $a = $recovered{"$PACKAGE/$rel"} or die "uncommitted recovery artifact: $rel\n";
    die "artifact identity: $rel\n" unless $a->{sha256} eq $h && $a->{bytes} == $n;
}
my $raw = $proof->{raw};
die "raw proof\n" unless $raw->{artifact} eq 'raw-cohort-records.tar.gz' && $raw->{members} == 154
    && @{$raw->{rows}} == 154 && $raw->{member_bytes} == 87098 && $raw->{all_members_byte_equal};
artifact($raw->{artifact}, $raw->{sha256}, $raw->{bytes});
for my $r (@{$raw->{rows}}) {
    die "raw member byte proof\n" unless $r->{byte_equal};
    map_original($r->{path}, 'retain-raw-record', $r->{sha256}, $r->{bytes},
        "$PACKAGE/$raw->{artifact}::$r->{path}");
}
my %gzips;
for my $g (@{$proof->{new_gzips}}) {
    die "duplicate gzip proof\n" if exists $gzips{$g->{artifact}};
    die "gzip byte proof\n" unless $g->{full_bytes_equal};
    artifact($g->{artifact}, $g->{compressed_sha256}, $g->{compressed_bytes});
    $gzips{$g->{artifact}} = $g;
}
die "gzip proof count\n" unless keys(%gzips) == 3 && @{$policy->{new_gzips}} == 3;
my %new_classes = (
    'target/compiler-scalar-simplify.20261116-1/fixture/producer-A' => 'retain-producer-A',
    'target/compiler-scalar-simplify.20261116-1/fixture/producer-B' => 'retain-producer-B',
    'target/compiler-scalar-simplify.20261116-1/fixture/source.tar' => 'retain-source-archive',
);
for my $p (@{$policy->{new_gzips}}) {
    my $g = $gzips{$p->{artifact}} or die "missing gzip proof\n";
    die "gzip policy\n" unless $g->{original_sha256} eq $p->{original_sha256}
        && $g->{original_bytes} == $p->{original_bytes};
    my $class = $new_classes{$p->{source}} or die "unknown gzip source\n";
    map_original($p->{source}, $class, $g->{original_sha256}, $g->{original_bytes}, "$PACKAGE/$p->{artifact}");
}
my $source = $proof->{source_archive};
die "source-member proof\n" unless $source->{members} == 172 && @{$source->{rows}} == 172
    && $source->{member_bytes} == 7280497 && $source->{all_members_byte_equal}
    && $source->{artifact} eq $policy->{shared_output_recovery}{artifact};
artifact($source->{artifact}, $source->{sha256}, $source->{bytes});
for my $r (@{$source->{rows}}) {
    safe_relative($r->{path});
    die "source member byte proof\n" unless $r->{byte_equal};
    map_original("target/compiler-scalar-simplify.20261116-1/fixture/source/$r->{path}",
        'source-tree-duplicate', $r->{sha256}, $r->{bytes}, "$PACKAGE/$source->{artifact}::$r->{path}");
}
my $e = $proof->{existing_emitter};
die "emitter original proof\n" unless $e->{full_bytes_equal} && !$e->{copied}
    && $e->{existing_path} eq $EMITTER && $e->{compressed_sha256} eq $recovery{$EMITTER}
    && $e->{git_blob} eq $committed{$EMITTER} && $e->{compressed_bytes} == $recovered{$EMITTER}{bytes};
map_original('target/compiler-scalar-simplify.20261116-1/fixture/emitter',
    'existing-emitter-duplicate', $e->{original_sha256}, $e->{original_bytes}, $EMITTER);
my $seed = $proof->{shared_seed}; my $sp = $policy->{shared_output_recovery};
die "shared seed proof\n" unless $seed->{all_seven_byte_equal} && @{$seed->{outputs}} == 7
    && $seed->{no_expanded_seed_or_output_copy} && $seed->{bytes} == $sp->{original_bytes}
    && $seed->{sha256} eq $sp->{original_sha256}
    && $seed->{embedded_compressed_sha256} eq $sp->{member_sha256}
    && $sp->{member} eq 'seed/lkjscript.gz'
    && join("\0", @{$seed->{outputs}}) eq join("\0", @{$sp->{paths}});
my $seed_member = $all{'target/compiler-scalar-simplify.20261116-1/fixture/source/seed/lkjscript.gz'};
die "seed member not retained\n" unless $seed_member && $mapped{$seed_member->{path}}
    && $seed_member->{sha256} eq $seed->{embedded_compressed_sha256};
for my $p (@{$seed->{outputs}}) {
    map_original($p, 'seed-equal-output-duplicate', $seed->{sha256}, $seed->{bytes},
        "$PACKAGE/$source->{artifact}::seed/lkjscript.gz (expanded)");
}
my %expected_classes = ('retain-raw-record'=>154, 'retain-producer-A'=>1, 'retain-producer-B'=>1,
    'retain-source-archive'=>1, 'source-tree-duplicate'=>172, 'existing-emitter-duplicate'=>1,
    'seed-equal-output-duplicate'=>7);
die "incomplete recovery map\n" unless keys(%mapped) == 337 && keys(%class_counts) == keys(%expected_classes);
for my $class (keys %expected_classes) {
    die "recovery class count: $class\n" unless ($class_counts{$class} // 0) == $expected_classes{$class};
}

# Check the complete current population before the first possible unlink.
my (%seen_files, %seen_dirs);
for my $scope (@ROOTS) {
    check_directory($scope);
    find({ no_chdir=>1, follow=>0, wanted=>sub {
        my $path = $File::Find::name;
        my @s = lstat($path); die "lstat current entry: $path\n" unless @s;
        my $r = substr($path, length($ROOT)+1);
        if (S_ISDIR($s[2])) {
            die "extra/duplicate directory: $r\n" unless exists $directories{$r} && !$seen_dirs{$r}++;
            check_directory($r); return;
        }
        die "symlink/special entry: $r\n" unless S_ISREG($s[2]);
        die "extra/duplicate file: $r\n" unless exists $all{$r} && !$seen_files{$r}++;
        match_file($path, $all{$r}, 1);
    }}, "$ROOT/$scope");
}
die "missing current entry\n" unless keys(%seen_files) == 337 && keys(%seen_dirs) == 22;
emit({event=>'preflight-pass', mode=>$ARGV[0], commit=>$COMMIT, files=>337, bytes=>207027139,
    directories=>22, committed_recovery_files=>72, canonical_sha256=>$canonical->{sha256},
    recovery_classes=>\%class_counts, no_process_or_fd_observation=>JSON::PP::true});
if (!$remove) {
    emit({event=>'check-only-complete', removed_files=>0, removed_directories=>0}); exit 0;
}
my $removed_files = 0; my $removed_dirs = 0;
for my $r (sort keys %all) {
    check_removal_parents($r);
    match_file("$ROOT/$r", $all{$r}, 1);
    unlink("$ROOT/$r") == 1 or die "unlink $r: $!\n";
    $removed_files++;
    emit({event=>'unlinked', path=>$r, count=>$removed_files, recovery=>$mapped{$r}});
}
for my $r (sort { length($b) <=> length($a) || $b cmp $a } keys %directories) {
    check_directory($r);
    rmdir("$ROOT/$r") or die "rmdir nonempty/changed $r: $!\n";
    $removed_dirs++;
    emit({event=>'removed-empty-directory', path=>$r, count=>$removed_dirs});
}
for my $r (@ROOTS) {
    $! = 0; my @s = lstat("$ROOT/$r"); my $error = 0 + $!;
    die "required ENOENT failed: $r errno=$error\n" unless !@s && $error == ENOENT;
    emit({event=>'root-absent', path=>$r, errno=>'ENOENT'});
}
match_file("$ROOT/$canonical->{path}", $canonical, 1);
emit({event=>'cleanup-complete', removed_files=>$removed_files, removed_directories=>$removed_dirs,
    canonical_sha256=>$canonical->{sha256}, unrelated_paths_mutated=>0,
    contract=>'cooperative non-atomic closure; any failure stops without rollback or automatic retry'});
