#!/usr/bin/env perl
use strict;
use warnings;
use Cwd qw(abs_path);
use Digest::SHA qw(sha256_hex);
use Encode qw(decode FB_CROAK LEAVE_SRC);
use Fcntl qw(O_RDONLY O_WRONLY O_CREAT O_EXCL O_NOFOLLOW S_ISREG S_ISDIR S_ISLNK);
use File::Basename qw(dirname basename);
use File::Spec;
use File::Temp qw(tempdir);
use Getopt::Long qw(GetOptions);
use JSON::PP;

my $MANIFEST = 'projection.json';
sub fail { die "real-programs-project: $_[0]\n" }
sub json { JSON::PP->new->canonical->pretty->ascii->encode($_[0]) }
sub text_value {
    my ($bytes, $description) = @_;
    my $text = eval { decode('UTF-8', $bytes, FB_CROAK | LEAVE_SRC) };
    fail("non-UTF-8 $description") if $@;
    return $text;
}

sub git_output {
    my ($root, @args) = @_;
    local %ENV = %ENV;
    delete @ENV{qw(GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR
                   GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES
                   GIT_CONFIG_PARAMETERS GIT_CONFIG_COUNT)};
    delete $ENV{$_} for grep { /\AGIT_CONFIG_(?:KEY|VALUE)_\d+\z/ } keys %ENV;
    $ENV{GIT_OPTIONAL_LOCKS} = '0';
    $ENV{GIT_CONFIG_NOSYSTEM} = '1';
    $ENV{GIT_CONFIG_GLOBAL} = '/dev/null';
    my @command = ('git', '--no-replace-objects', '--no-optional-locks', '-c', 'core.fsmonitor=false',
                   '-c', 'core.hooksPath=/dev/null', '-C', $root, @args);
    open my $pipe, '-|', @command or fail("cannot start Git: $!");
    my $output = do { local $/; <$pipe> };
    $output = '' unless defined $output;
    close $pipe or fail("Git $args[0] failed (status $?)");
    return $output;
}

sub canonical_directory {
    my ($input, $description) = @_;
    fail("empty or NUL-containing $description") if !length($input) || index($input, "\0") >= 0;
    my $absolute = File::Spec->rel2abs($input);
    my $walk = '/';
    for my $part (split m{/+}, $absolute) {
        next if !length($part) || $part eq '.';
        if ($part eq '..') { $walk = dirname($walk); next }
        $walk = $walk eq '/' ? "/$part" : "$walk/$part";
        my @st = lstat($walk);
        fail("missing directory component in $description: $walk") unless @st;
        fail("symlink directory component in $description: $walk") if S_ISLNK($st[2]);
        fail("non-directory component in $description: $walk") unless S_ISDIR($st[2]);
    }
    my $root = abs_path($absolute);
    fail("cannot resolve $description") unless defined $root;
    return $root;
}

sub safe_path {
    my ($path) = @_;
    fail('empty tracked path') unless length $path;
    fail("absolute or ambiguous tracked path: $path")
        if $path =~ m{\A/|\A[A-Za-z]:|\\|\0};
    my @parts = split m{/}, $path, -1;
    fail("escaping or noncanonical tracked path: $path")
        if grep { !length($_) || $_ eq '.' || $_ eq '..' } @parts;
    text_value($path, 'tracked path');
    return $path;
}

sub node_path {
    my ($root, $relative, $allow_link) = @_;
    safe_path($relative);
    my @parts = split m{/}, $relative;
    my $walk = $root;
    for my $i (0 .. $#parts) {
        $walk .= '/' . $parts[$i];
        my @st = lstat($walk);
        fail("missing node: $walk") unless @st;
        if ($i == $#parts) {
            fail("nonregular file: $walk")
                unless S_ISREG($st[2]) || ($allow_link && S_ISLNK($st[2]));
        } else {
            fail("symlink parent: $walk") if S_ISLNK($st[2]);
            fail("non-directory parent: $walk") unless S_ISDIR($st[2]);
        }
    }
    return $walk;
}

sub regular_file { node_path($_[0], $_[1], 0) }
sub tracked_node { node_path($_[0], $_[1], 1) }

sub read_node {
    my ($path, $object_format, $keep_content) = @_;
    my @before = lstat($path);
    fail("not a regular file: $path") unless @before && S_ISREG($before[2]);
    sysopen my $fh, $path, O_RDONLY | O_NOFOLLOW or fail("read $path: $!");
    binmode $fh;
    my @opened = stat($fh);
    fail("file identity changed while opening: $path")
        unless @opened && S_ISREG($opened[2])
            && $before[0] == $opened[0] && $before[1] == $opened[1];
    my $sha = Digest::SHA->new(256);
    my $git = defined($object_format) ? Digest::SHA->new($object_format eq 'sha1' ? 1 : 256) : undef;
    $git->add('blob ' . $opened[7] . "\0") if $git;
    my ($size, $content) = (0, '');
    while (1) {
        my $read = sysread($fh, my $buffer, 65536);
        fail("read $path: $!") unless defined $read;
        last unless $read;
        $size += $read;
        $sha->add($buffer);
        $git->add($buffer) if $git;
        $content .= $buffer if $keep_content;
    }
    my @after = stat($fh);
    close $fh or fail("close $path: $!");
    fail("file changed while reading: $path")
        unless @after && $size == $opened[7] && $size == $after[7]
            && $opened[9] == $after[9] && $opened[10] == $after[10];
    my $record = { kind => 'regular', sha256 => $sha->hexdigest, bytes => $size,
        device => $opened[0], inode => $opened[1], mode => $opened[2], links => $opened[3] };
    $record->{git_blob} = $git->hexdigest if $git;
    $record->{content} = $content if $keep_content;
    return $record;
}

sub read_link_node {
    my ($path, $object_format) = @_;
    my @before = lstat($path);
    fail("not a symbolic link: $path") unless @before && S_ISLNK($before[2]);
    my $target = readlink($path);
    fail("readlink $path: $!") unless defined $target;
    my @after = lstat($path);
    fail("symbolic link changed while reading: $path")
        unless @after && S_ISLNK($after[2]) && $before[0] == $after[0]
            && $before[1] == $after[1] && $before[9] == $after[9] && $before[10] == $after[10];
    my $record = { kind => 'symlink', raw_target => $target, sha256 => sha256_hex($target),
        bytes => length($target), device => $before[0], inode => $before[1],
        mode => $before[2], links => $before[3] };
    if (defined $object_format) {
        my $git = Digest::SHA->new($object_format eq 'sha1' ? 1 : 256);
        $git->add('blob ' . length($target) . "\0", $target);
        $record->{git_blob} = $git->hexdigest;
    }
    return $record;
}

sub validate_link_graph {
    my ($nodes) = @_;
    my %directories = ('' => 1);
    for my $path (keys %$nodes) {
        my @parts = split m{/}, $path;
        pop @parts;
        while (@parts) { $directories{join('/', @parts)} = 1; pop @parts }
    }
    my (@links, %resolved);
    for my $path (sort keys %$nodes) {
        next unless $nodes->{$path}{kind} eq 'symlink';
        push @links, $path;
        my $target = $nodes->{$path}{raw_target};
        fail("empty, absolute, or ambiguous symlink target: $path")
            if !length($target) || $target =~ m{\A/|\A[A-Za-z]:|\\|\0};
        text_value($target, 'symlink target');
    }
    my ($resolve_link, $walk);
    $walk = sub {
        my ($base, $parts, $active) = @_;
        my @stack = @$base;
        my @pending = @$parts;
        while (@pending) {
            my $part = shift @pending;
            next if $part eq '' || $part eq '.';
            if ($part eq '..') {
                fail('symlink target escapes SOURCE') unless @stack;
                pop @stack;
                next;
            }
            my $path = join('/', @stack, $part);
            if (exists $nodes->{$path}) {
                my $target = $nodes->{$path}{kind} eq 'symlink'
                    ? $resolve_link->($path, $active) : { path => $path, kind => 'file' };
                fail("symlink target traverses a non-directory: $path")
                    if @pending && $target->{kind} ne 'directory';
                return $target unless @pending;
                @stack = length($target->{path}) ? split(m{/}, $target->{path}) : ();
            } elsif ($directories{$path}) {
                push @stack, $part;
            } else {
                fail("dangling or untracked symlink target: $path");
            }
        }
        return { path => join('/', @stack), kind => 'directory' };
    };
    $resolve_link = sub {
        my ($path, $active) = @_;
        fail("cyclic symlink chain: $path") if $active->{$path};
        return $resolved{$path} if exists $resolved{$path};
        my @base = split m{/}, $path;
        pop @base;
        my @parts = split m{/}, $nodes->{$path}{raw_target}, -1;
        my $target = $walk->(\@base, \@parts, { %$active, $path => 1 });
        $resolved{$path} = $target;
        return $target;
    };
    $resolve_link->($_, {}) for @links;
    undef $resolve_link;
    undef $walk;

    # Inventory never follows symlinks. Still reject directory-containment
    # cycles (including links to ancestors and loops between sibling trees).
    my (%edges, %state);
    for my $directory (keys %directories) {
        next unless length $directory;
        my $parent = dirname($directory);
        $parent = '' if $parent eq '.';
        $edges{$parent}{$directory} = 1;
    }
    for my $path (@links) {
        next unless $resolved{$path}{kind} eq 'directory';
        my $parent = dirname($path);
        $parent = '' if $parent eq '.';
        $edges{$parent}{$resolved{$path}{path}} = 1;
    }
    my $visit;
    $visit = sub {
        my ($directory) = @_;
        fail("cyclic directory symlink graph at: $directory") if ($state{$directory} || 0) == 1;
        return if ($state{$directory} || 0) == 2;
        $state{$directory} = 1;
        $visit->($_) for sort keys %{$edges{$directory} || {}};
        $state{$directory} = 2;
    };
    $visit->('');
    undef $visit;
    return \%resolved;
}

sub parse_index {
    my ($raw) = @_;
    my %index;
    for my $entry (split /\0/, $raw) {
        fail('malformed Git index entry') unless $entry =~ /\A([0-7]{6}) ([0-9a-f]+) ([0-3])\t(.+)\z/s;
        my ($mode, $oid, $stage, $path) = ($1, $2, $3, $4);
        safe_path($path);
        fail("unmerged or duplicate tracked path: $path") if $stage ne '0' || exists $index{$path};
        fail("gitlink or unsupported tracked mode $mode: $path")
            unless $mode eq '100644' || $mode eq '100755' || $mode eq '120000';
        $index{$path} = { mode => $mode, oid => $oid };
    }
    return \%index;
}

sub parse_head_tree {
    my ($raw) = @_;
    my %tree;
    for my $entry (split /\0/, $raw) {
        fail('malformed Git tree entry') unless $entry =~ /\A([0-7]{6}) (\w+) ([0-9a-f]+)\t(.+)\z/s;
        my ($mode, $type, $oid, $path) = ($1, $2, $3, $4);
        safe_path($path);
        fail("unsupported Git HEAD entry: $path")
            unless $type eq 'blob' && ($mode eq '100644' || $mode eq '100755' || $mode eq '120000');
        fail("duplicate Git HEAD path: $path") if exists $tree{$path};
        $tree{$path} = { mode => $mode, oid => $oid };
    }
    return \%tree;
}

sub source_snapshot {
    my ($input) = @_;
    my $root = canonical_directory($input, 'SOURCE');
    my $top = git_output($root, 'rev-parse', '--show-toplevel');
    $top =~ s/\n\z//;
    fail('SOURCE must be the Git worktree root') unless $top eq $root;
    my $head = git_output($root, 'rev-parse', '--verify', 'HEAD^{commit}');
    $head =~ s/\n\z//;
    fail('invalid Git HEAD identity') unless $head =~ /\A(?:[0-9a-f]{40}|[0-9a-f]{64})\z/;
    my $format = git_output($root, 'rev-parse', '--show-object-format');
    $format =~ s/\n\z//;
    fail("unsupported Git object format: $format") unless $format eq 'sha1' || $format eq 'sha256';
    my @origins = split /\0/, git_output($root, 'config', '--null', '--get-all', 'remote.origin.url'), -1;
    pop @origins if @origins && $origins[-1] eq '';
    fail('SOURCE needs exactly one nonempty origin URL') unless @origins == 1 && length $origins[0];
    my $index = parse_index(git_output($root, 'ls-files', '--stage', '-z'));
    my $tree = parse_head_tree(git_output($root, 'ls-tree', '-r', '-z', '--full-tree', $head));
    fail('dirty Git index: tracked entries differ from HEAD') unless json($index) eq json($tree);
    fail('SOURCE has no tracked files') unless keys %$index;
    my (@files, %nodes, %records);
    for my $path (sort keys %$index) {
        my $is_link = $index->{$path}{mode} eq '120000';
        my $node = $is_link ? read_link_node(tracked_node($root, $path), $format)
                           : read_node(regular_file($root, $path), $format, 0);
        fail("dirty tracked bytes (including filters/assume-unchanged): $path")
            unless $node->{git_blob} eq $index->{$path}{oid};
        # Git records the owner-execute bit; group/other execute bits are not tracked.
        my $mode = $is_link ? '120000' : ($node->{mode} & 0100) ? '100755' : '100644';
        fail("dirty tracked executable mode: $path") unless $mode eq $index->{$path}{mode};
        my $record = { path => text_value($path, 'tracked path'), sha256 => $node->{sha256},
            bytes => $node->{bytes}, git_blob => $node->{git_blob}, git_mode => $mode };
        if ($is_link) {
            $record->{kind} = 'symlink';
            $record->{target} = text_value($node->{raw_target}, 'symlink target');
        }
        push @files, $record;
        $records{$path} = $record;
        $nodes{$path} = $node;
    }
    my $resolved = validate_link_graph(\%nodes);
    for my $path (keys %$resolved) {
        $records{$path}{resolved_target} = text_value($resolved->{$path}{path}, 'resolved symlink target');
        $records{$path}{target_kind} = $resolved->{$path}{kind};
    }
    return { root => $root, source => { root => text_value($root, 'SOURCE path'),
        commit => $head, origin => text_value($origins[0], 'origin URL'), object_format => $format },
        files => \@files, nodes => \%nodes };
}

sub layout {
    my ($snapshot) = @_;
    my (%targets, %directories, @links, @aliases);
    my $reserve = sub {
        my ($path, $description) = @_;
        safe_path($path);
        fail("projection path collision: $path ($targets{$path}, $description)") if exists $targets{$path};
        $targets{$path} = $description;
        my @parts = split m{/}, $path;
        pop @parts;
        while (@parts) { $directories{join('/', @parts)} = 1; pop @parts }
    };
    for my $path (sort keys %{$snapshot->{nodes}}) {
        $reserve->($path, 'tracked original');
        push @links, { from => $path, to => $path };
        if ($path =~ /\.rs\z/) {
            (my $alias = $path) =~ s/\.rs\z/.lkj/;
            $reserve->($alias, 'Rust suffix alias');
            push @links, { from => $path, to => $alias };
            push @aliases, { from => text_value($path, 'alias source'), to => text_value($alias, 'alias target') };
        }
    }
    $reserve->($MANIFEST, 'projection manifest');
    for my $directory (sort keys %directories) {
        fail("projection file/directory collision: $directory") if exists $targets{$directory};
    }
    my $manifest = {
        schema => 1, kind => 'git_tracked_suffix_projection', source => $snapshot->{source},
        claim => 'Path-only projection: tracked bytes are unchanged; .rs files gain .lkj aliases. No Rust compatibility, semantic equivalence, or checker acceptance is claimed.',
        storage => 'All tracked originals and suffix aliases are hardlinks to SOURCE; no payload copies. Hardlinks share writable contents and are not a write-protection boundary.',
        provenance => 'Records Git identity and byte integrity; does not authenticate a publisher or verify signatures.',
        concurrency => 'SOURCE and DEST namespaces must remain quiescent during creation, verification and cleanup. This tool is not a boundary against concurrent host filesystem mutation.',
        clean_state => 'Index equals HEAD; every tracked file equals its raw Git blob and executable mode. Untracked files are ignored. Symlinks and gitlinks are refused.',
        files => $snapshot->{files}, aliases => \@aliases,
    };
    my $has_links = grep { $_->{kind} eq 'symlink' } values %{$snapshot->{nodes}};
    if ($has_links) {
        $manifest->{claim} = 'Path-only projection: tracked regular-file bytes and symbolic-link target bytes are unchanged; .rs paths gain .lkj aliases. No Rust compatibility, semantic equivalence, or checker/loader acceptance is claimed.';
        $manifest->{storage} = 'Tracked regular files, symbolic-link objects and suffix aliases are hardlinks to SOURCE, including symlink inodes on Linux. No target payload copies or chmod. Hardlinks share writable contents and are not a write-protection boundary.';
        $manifest->{clean_state} = 'Index equals HEAD; regular files equal their raw Git blobs and executable modes, and symlink targets equal their raw Git blobs. Untracked files are ignored. Gitlinks and nonregular nonsymlink nodes are refused.';
        $manifest->{symlink_policy} = 'Only relative links resolving entirely within the original tracked-node/directory graph are admitted. Absolute, escaping, dangling/untracked, chain-cyclic and directory-cyclic targets are refused. Inventory uses lstat/readlink and never traverses a symbolic link.';
    }
    my %kinds = ($MANIFEST => 'regular');
    $kinds{$_->{to}} = $snapshot->{nodes}{$_->{from}}{kind} for @links;
    return { manifest => $manifest, json => json($manifest),
        links => [sort { $a->{to} cmp $b->{to} } @links],
        targets => \%targets, kinds => \%kinds, directories => \%directories };
}

sub destination_path {
    my ($input, $source) = @_;
    fail('empty or NUL-containing DEST') if !length($input) || index($input, "\0") >= 0;
    $input =~ s{/+\z}{};
    my $name = basename($input);
    fail('DEST must name a fresh directory') if !length($name) || $name eq '.' || $name eq '..';
    my $parent = canonical_directory(dirname($input), 'DEST parent');
    my $dest = $parent eq '/' ? "/$name" : "$parent/$name";
    fail('DEST must be outside SOURCE') if $source eq '/' || $dest eq $source || index($dest, "$source/") == 0;
    return $dest;
}

sub visit_tree {
    my ($root, $callback, $relative) = @_;
    $relative = '' unless defined $relative;
    my $path = length($relative) ? "$root/$relative" : $root;
    my @st = lstat($path);
    fail("missing projection node: $path") unless @st;
    $callback->($relative, \@st) if length $relative;
    return unless S_ISDIR($st[2]);
    opendir my $dh, $path or fail("list $path: $!");
    my @children = sort grep { $_ ne '.' && $_ ne '..' } readdir($dh);
    closedir $dh or fail("close directory $path: $!");
    for my $name (@children) {
        visit_tree($root, $callback, length($relative) ? "$relative/$name" : $name);
    }
}

sub verify_projection {
    my ($source_input, $dest_input) = @_;
    my $snapshot = source_snapshot($source_input);
    my $plan = layout($snapshot);
    my $dest = destination_path($dest_input, $snapshot->{root});
    canonical_directory($dest, 'DEST');
    visit_tree($dest, sub {
        my ($relative, $st) = @_;
        fail("nonregular nonsymlink projection node: $relative")
            unless S_ISREG($st->[2]) || S_ISDIR($st->[2]) || S_ISLNK($st->[2]);
        if (S_ISDIR($st->[2])) {
            fail("unexpected projection directory: $relative") unless $plan->{directories}{$relative};
        } else {
            fail("unexpected projection file: $relative") unless exists $plan->{targets}{$relative};
            my $kind = S_ISLNK($st->[2]) ? 'symlink' : 'regular';
            fail("projection node type changed: $relative") unless $kind eq $plan->{kinds}{$relative};
        }
    });
    my $manifest = read_node(regular_file($dest, $MANIFEST), undef, 1);
    fail('manifest differs from current clean SOURCE identity, hashes, or prescribed layout')
        unless $manifest->{content} eq $plan->{json};
    for my $link (@{$plan->{links}}) {
        my $original = $snapshot->{nodes}{$link->{from}};
        my $node = $original->{kind} eq 'symlink'
            ? read_link_node(tracked_node($dest, $link->{to}), undef)
            : read_node(regular_file($dest, $link->{to}), undef, 0);
        fail("not an original-source hardlink: $link->{to}")
            unless $node->{device} == $original->{device} && $node->{inode} == $original->{inode};
        fail("projection bytes changed: $link->{to}")
            unless $node->{bytes} == $original->{bytes} && $node->{sha256} eq $original->{sha256};
        fail("projection symlink target changed: $link->{to}")
            if $original->{kind} eq 'symlink' && $node->{raw_target} ne $original->{raw_target};
    }
    my $result = { operation => 'verify', read_only => JSON::PP::true,
        manifest => text_value("$dest/$MANIFEST", 'manifest path'),
        manifest_sha256 => $manifest->{sha256}, commit => $snapshot->{source}{commit},
        tracked_files => scalar @{$snapshot->{files}}, aliases => scalar @{$plan->{manifest}{aliases}} };
    my $symlinks = grep { $_->{kind} eq 'symlink' } values %{$snapshot->{nodes}};
    $result->{tracked_symlinks} = $symlinks if $symlinks;
    return $result;
}

sub remove_owned_tree {
    my ($root, $identity) = @_;
    my @st = lstat($root);
    return unless @st;
    fail("refusing cleanup of replaced destination: $root")
        unless S_ISDIR($st[2]) && $st[0] == $identity->[0] && $st[1] == $identity->[1];
    my $remove;
    $remove = sub {
        my ($path) = @_;
        my @node = lstat($path);
        return unless @node;
        if (S_ISDIR($node[2])) {
            opendir my $dh, $path or fail("cleanup list $path: $!");
            my @names = grep { $_ ne '.' && $_ ne '..' } readdir($dh);
            closedir $dh or fail("cleanup close $path: $!");
            $remove->("$path/$_") for @names;
            rmdir $path or fail("cleanup directory $path: $!");
        } else {
            unlink $path or fail("cleanup file $path: $!");
        }
    };
    $remove->($root);
    undef $remove;
}

sub link_payload {
    my ($source, $dest) = @_;
    link($source, $dest) or fail("hardlink $dest: $! (copy fallback is forbidden)");
}

sub project {
    my ($source_input, $dest_input) = @_;
    my $snapshot = source_snapshot($source_input);
    my $plan = layout($snapshot); # All file/alias/manifest collisions precede mkdir.
    my $dest = destination_path($dest_input, $snapshot->{root});
    fail("DEST already exists: $dest") if lstat($dest);
    my ($created, @identity, $result);
    my $ok = eval {
        local $SIG{INT} = sub { die "interrupted\n" };
        local $SIG{TERM} = sub { die "terminated\n" };
        mkdir($dest, 0700) or fail("create fresh DEST $dest: $!");
        $created = 1;
        @identity = lstat($dest);
        fail('cannot record newly created DEST identity') unless @identity;
        for my $directory (sort keys %{$plan->{directories}}) {
            mkdir("$dest/$directory", 0700) or fail("create projection directory $directory: $!");
        }
        for my $link (@{$plan->{links}}) {
            my $source = tracked_node($snapshot->{root}, $link->{from});
            link_payload($source, "$dest/$link->{to}");
        }
        sysopen my $fh, "$dest/$MANIFEST", O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW, 0600
            or fail("create manifest: $!");
        binmode $fh;
        print {$fh} $plan->{json} or fail("write manifest: $!");
        close $fh or fail("close manifest: $!");
        $result = verify_projection($snapshot->{root}, $dest);
        1;
    };
    unless ($ok) {
        my $error = $@ || "projection failed\n";
        if ($created && @identity) {
            my $cleaned = eval { remove_owned_tree($dest, \@identity); 1 };
            $error .= "cleanup also failed: $@" unless $cleaned;
        }
        die $error;
    }
    $result->{operation} = 'create';
    delete $result->{read_only};
    return $result;
}

sub self_test {
    my $tmp = tempdir('lkj-project-XXXXXX', TMPDIR => 1, CLEANUP => 0);
    my @tmp_identity = lstat($tmp);
    my $source = "$tmp/source";
    my $checks = 0;
    my $check = sub { ++$checks; fail("self-test: $_[1]") unless $_[0] };
    my $write = sub {
        my ($path, $bytes) = @_;
        open my $fh, '>:raw', $path or fail("fixture write: $!");
        print {$fh} $bytes or fail("fixture write: $!");
        close $fh or fail("fixture close: $!");
    };
    my $rejected = sub {
        my ($code, $description, $reason) = @_;
        my $ok = eval { $code->(); 1 };
        my $error = $@;
        $check->(!$ok, $description);
        $check->($error =~ $reason, "$description: expected refusal reason") if defined $reason;
    };
    my $drop = sub {
        my ($path) = @_;
        my @identity = lstat($path);
        remove_owned_tree($path, \@identity) if @identity;
    };
    my $ok = eval {
        my $legacy = { root => '/fixture/source',
            source => { root => '/fixture/source', commit => 'a' x 40,
                origin => 'https://example.invalid/specimen.git', object_format => 'sha1' },
            files => [{ path => 'src/lib.rs', sha256 => sha256_hex(''), bytes => 0,
                git_blob => 'e69de29bb2d1d6434b8b29ae775ad8c2e48c5391', git_mode => '100644' }],
            nodes => { 'src/lib.rs' => { kind => 'regular' } } };
        # Measured from the delivered 7ebaa02c... regular-only producer before
        # adding link support; freezes its complete canonical manifest encoding.
        $check->(sha256_hex(layout($legacy)->{json}) eq
            '1f747dc9f11deb92d2ab2942bb19257601eddd9f4f2555ddd26c478767674ab2',
            'legacy regular-only manifest is byte-identical');
        mkdir($source, 0700) or fail("fixture mkdir: $!");
        mkdir("$source/src", 0700) or fail("fixture mkdir: $!");
        git_output($source, '-c', 'init.defaultBranch=main', 'init', '--quiet');
        git_output($source, 'config', 'user.name', 'Projection fixture');
        git_output($source, 'config', 'user.email', 'fixture@example.invalid');
        git_output($source, 'config', 'commit.gpgsign', 'false');
        git_output($source, 'config', 'core.autocrlf', 'false');
        git_output($source, 'config', 'remote.origin.url', 'https://example.invalid/specimen.git');
        my $rust = "#[test]\nfn original_test() { assert_eq!(1, 1); }\n";
        $write->("$source/src/lib.rs", $rust);
        $write->("$source/asset.bin", "\0\xffasset\n");
        git_output($source, 'add', '--', '.');
        git_output($source, 'commit', '--quiet', '-m', 'fixture');
        my $before = source_snapshot($source);
        my $first = project($source, "$tmp/first");
        $check->($first->{tracked_files} == 2 && $first->{aliases} == 1, 'complete tracked-file projection');
        my $verified = verify_projection($source, "$tmp/first");
        $check->($verified->{read_only}, 'read-only verification');
        my $after = source_snapshot($source);
        $check->(json($before->{files}) eq json($after->{files}), 'original bytes preserved');
        $check->($before->{nodes}{'src/lib.rs'}{mode} == $after->{nodes}{'src/lib.rs'}{mode}, 'original mode preserved');
        my $second = project($source, "$tmp/second");
        $check->($first->{manifest_sha256} eq $second->{manifest_sha256}, 'deterministic manifest');
        $rejected->(sub { project($source, "$tmp/first") }, 'existing DEST rejected');
        $check->(verify_projection($source, "$tmp/first")->{aliases} == 1, 'existing DEST was not cleaned');
        unlink "$tmp/first/src/lib.lkj" or fail("fixture unlink alias: $!");
        $write->("$tmp/first/src/lib.lkj", "changed alias\n");
        $rejected->(sub { verify_projection($source, "$tmp/first") }, 'changed alias rejected');
        $check->(-f "$tmp/first/src/lib.lkj", 'failed verify did not clean or write destination');
        $check->(source_snapshot($source)->{nodes}{'src/lib.rs'}{sha256} eq $after->{nodes}{'src/lib.rs'}{sha256}, 'alias replacement did not touch source');
        $write->("$tmp/first/src/lib.lkj", $rust);
        $rejected->(sub { verify_projection($source, "$tmp/first") }, 'identical copied bytes cannot substitute for a hardlink');
        $drop->("$tmp/first");
        $drop->("$tmp/second");
        my $base_mode = $after->{nodes}{'src/lib.rs'}{mode} & 0777;
        chmod($base_mode | 0011, "$source/src/lib.rs") or fail("fixture chmod: $!");
        $check->(project($source, "$tmp/nonowner-added")->{aliases} == 1,
            'non-owner execute bits do not make a Git 100644 file dirty');
        $check->((source_snapshot($source)->{nodes}{'src/lib.rs'}{mode} & 0777)
            == ($base_mode | 0011), 'non-owner execute bits preserved by projection');
        $drop->("$tmp/nonowner-added");
        chmod($base_mode | 0100, "$source/src/lib.rs") or fail("fixture chmod: $!");
        $rejected->(sub { project($source, "$tmp/owner-added") },
            'owner-execute addition to Git 100644 rejected', qr/dirty tracked executable mode/);
        $check->(!lstat("$tmp/owner-added"), 'owner-execute addition refused before destination creation');
        chmod($base_mode | 0111, "$source/src/lib.rs") or fail("fixture chmod: $!");
        git_output($source, 'add', '--', 'src/lib.rs');
        git_output($source, 'commit', '--quiet', '-m', 'owner-execute fixture');
        chmod($base_mode | 0011, "$source/src/lib.rs") or fail("fixture chmod: $!");
        $rejected->(sub { project($source, "$tmp/owner-removed") },
            'owner-execute removal from Git 100755 rejected despite other execute bits',
            qr/dirty tracked executable mode/);
        $check->(!lstat("$tmp/owner-removed"), 'owner-execute removal refused before destination creation');
        chmod($base_mode | 0100, "$source/src/lib.rs") or fail("fixture chmod: $!");
        $check->(project($source, "$tmp/nonowner-removed")->{aliases} == 1,
            'non-owner execute removal keeps a Git 100755 file clean');
        $check->((source_snapshot($source)->{nodes}{'src/lib.rs'}{mode} & 0777)
            == ($base_mode | 0100), 'owner-only execute mode preserved by projection');
        $drop->("$tmp/nonowner-removed");
        chmod($base_mode, "$source/src/lib.rs") or fail("fixture chmod: $!");
        git_output($source, 'add', '--', 'src/lib.rs');
        git_output($source, 'commit', '--quiet', '-m', 'restore original execute mode');
        $write->("$source/src/lib.rs", $rust . "// dirty\n");
        $rejected->(sub { project($source, "$tmp/dirty") }, 'dirty tracked bytes rejected');
        $check->(!lstat("$tmp/dirty"), 'dirty failure left no destination');
        $write->("$source/src/lib.rs", $rust);
        git_output($source, 'update-index', '--assume-unchanged', '--', 'src/lib.rs');
        $write->("$source/src/lib.rs", $rust . "// hidden dirty\n");
        $rejected->(sub { project($source, "$tmp/hidden-dirty") }, 'assume-unchanged does not hide changed bytes');
        $write->("$source/src/lib.rs", $rust);
        git_output($source, 'update-index', '--no-assume-unchanged', '--', 'src/lib.rs');
        $write->("$source/asset.bin", "staged asset\n");
        git_output($source, 'add', '--', 'asset.bin');
        $write->("$source/asset.bin", "\0\xffasset\n");
        $rejected->(sub { project($source, "$tmp/dirty-index") }, 'dirty index rejected even when working bytes equal HEAD');
        git_output($source, 'reset', '--quiet', 'HEAD', '--', 'asset.bin');
        $write->("$source/src/lib.lkj", "collision\n");
        git_output($source, 'add', '--', 'src/lib.lkj');
        git_output($source, 'commit', '--quiet', '-m', 'collision fixture');
        $rejected->(sub { project($source, "$tmp/collision") }, 'suffix collision rejected');
        $check->(!lstat("$tmp/collision"), 'collision failure left no destination');
        git_output($source, 'rm', '--quiet', '--', 'src/lib.lkj');
        git_output($source, 'commit', '--quiet', '-m', 'remove collision');
        symlink('lib.rs', "$source/src/link.rs") or fail("fixture symlink: $!");
        git_output($source, 'add', '--', 'src/link.rs');
        git_output($source, 'commit', '--quiet', '-m', 'symlink fixture');
        $check->(project($source, "$tmp/symlink")->{tracked_symlinks} == 1, 'safe file symlink preserved');
        my @original_link = lstat("$source/src/link.rs");
        my @alias_link = lstat("$tmp/symlink/src/link.lkj");
        $check->(S_ISLNK($alias_link[2]) && $original_link[0] == $alias_link[0]
            && $original_link[1] == $alias_link[1] && readlink("$tmp/symlink/src/link.lkj") eq 'lib.rs',
            'symlink alias hardlinks the link inode and preserves target text');
        $drop->("$tmp/symlink");
        mkdir("$source/bench", 0700) or fail("fixture mkdir: $!");
        mkdir("$source/test-pages", 0700) or fail("fixture mkdir: $!");
        my $html = "<p>fixture</p>\n";
        $write->("$source/test-pages/one.html", $html);
        symlink('../test-pages', "$source/bench/test-pages") or fail("fixture symlink: $!");
        symlink('test-pages/one.html', "$source/bench/page") or fail("fixture symlink: $!");
        git_output($source, 'add', '--', '.');
        git_output($source, 'commit', '--quiet', '-m', 'internal directory links');
        $check->(project($source, "$tmp/internal")->{tracked_symlinks} == 3, 'internal directory and intermediate links preserved');
        my $bound = JSON::PP->new->decode(read_node("$tmp/internal/$MANIFEST", undef, 1)->{content});
        my ($dir_link) = grep { $_->{path} eq 'bench/test-pages' } @{$bound->{files}};
        $check->($dir_link->{kind} eq 'symlink' && $dir_link->{target} eq '../test-pages'
            && $dir_link->{sha256} eq sha256_hex('../test-pages')
            && $dir_link->{resolved_target} eq 'test-pages' && $dir_link->{target_kind} eq 'directory',
            'manifest binds raw link bytes and original-graph resolution');
        $write->("$source/test-pages/one.html", $html . 'changed');
        $rejected->(sub { verify_projection($source, "$tmp/internal") }, 'source target payload mutation rejected', qr/dirty tracked bytes/);
        $write->("$source/test-pages/one.html", $html);
        unlink "$tmp/internal/bench/test-pages" or fail("fixture unlink: $!");
        symlink('../missing', "$tmp/internal/bench/test-pages") or fail("fixture symlink: $!");
        $rejected->(sub { verify_projection($source, "$tmp/internal") }, 'replaced projection symlink rejected', qr/not an original-source hardlink/);
        unlink "$tmp/internal/bench/test-pages" or fail("fixture unlink: $!");
        symlink('../test-pages', "$tmp/internal/bench/test-pages") or fail("fixture symlink: $!");
        $rejected->(sub { verify_projection($source, "$tmp/internal") }, 'same-target copied symlink rejected', qr/not an original-source hardlink/);
        unlink "$tmp/internal/bench/test-pages" or fail("fixture unlink: $!");
        link_payload("$source/bench/test-pages", "$tmp/internal/bench/test-pages");
        unlink "$source/bench/test-pages" or fail("fixture unlink: $!");
        symlink('../src', "$source/bench/test-pages") or fail("fixture symlink: $!");
        $rejected->(sub { verify_projection($source, "$tmp/internal") }, 'source link target-text mutation rejected', qr/dirty tracked bytes/);
        unlink "$source/bench/test-pages" or fail("fixture unlink: $!");
        link_payload("$tmp/internal/bench/test-pages", "$source/bench/test-pages");
        $check->(verify_projection($source, "$tmp/internal")->{tracked_symlinks} == 3, 'restored original links and targets verify');
        $drop->("$tmp/internal");
        for my $case (
            ['absolute', '/outside-fixture', qr/absolute/],
            ['escape', '../../outside-fixture', qr/escapes SOURCE/],
            ['dangling', 'missing', qr/dangling or untracked/],
            ['new-alias', '../src/lib.lkj', qr/dangling or untracked/],
            ['new-manifest', '../projection.json', qr/dangling or untracked/],
            ['directory-cycle', '..', qr/cyclic directory symlink/]) {
            my ($name, $target, $reason) = @$case;
            symlink($target, "$source/bench/bad") or fail("fixture symlink: $!");
            git_output($source, 'add', '--', 'bench/bad');
            git_output($source, 'commit', '--quiet', '-m', "$name link fixture");
            $rejected->(sub { project($source, "$tmp/$name") }, "$name link rejected", $reason);
            $check->(!lstat("$tmp/$name"), "$name refusal precedes destination creation");
            git_output($source, 'rm', '--quiet', '--', 'bench/bad');
            git_output($source, 'commit', '--quiet', '-m', "remove $name link");
        }
        symlink('b', "$source/bench/a") or fail("fixture symlink: $!");
        symlink('a', "$source/bench/b") or fail("fixture symlink: $!");
        git_output($source, 'add', '--', 'bench/a', 'bench/b');
        git_output($source, 'commit', '--quiet', '-m', 'link-chain cycle');
        $rejected->(sub { project($source, "$tmp/chain-cycle") }, 'link-chain cycle rejected', qr/cyclic symlink chain/);
        $check->(!lstat("$tmp/chain-cycle"), 'chain-cycle refusal precedes destination creation');
        git_output($source, 'rm', '--quiet', '--', 'bench/a', 'bench/b');
        git_output($source, 'commit', '--quiet', '-m', 'remove link-chain cycle');
        my $original_head = source_snapshot($source)->{source}{commit};
        $write->("$source/src/lib.rs", $rust . "// replacement tree\n");
        git_output($source, 'add', '--', 'src/lib.rs');
        git_output($source, 'commit', '--quiet', '-m', 'replacement fixture');
        my $replacement_head = source_snapshot($source)->{source}{commit};
        git_output($source, 'replace', $original_head, $replacement_head);
        git_output($source, 'update-ref', 'HEAD', $original_head);
        $rejected->(sub { project($source, "$tmp/replaced-head") }, 'replace refs cannot relabel a different tree with the original commit');
        $check->(!lstat("$tmp/replaced-head"), 'replace-ref failure left no destination');
        git_output($source, 'replace', '--delete', $original_head);
        git_output($source, 'update-ref', 'HEAD', $replacement_head);
        $write->("$source/src/lib.rs", $rust);
        git_output($source, 'add', '--', 'src/lib.rs');
        git_output($source, 'commit', '--quiet', '-m', 'remove replacement fixture');
        {
            no warnings 'redefine';
            my $real_link = \&link_payload;
            my $linked_symlink = 0;
            local *link_payload = sub {
                $real_link->(@_);
                my @st = lstat($_[0]);
                if (S_ISLNK($st[2])) {
                    $linked_symlink = 1;
                    fail('injected failure after linking a symlink object');
                }
            };
            $rejected->(sub { project($source, "$tmp/partial") }, 'mid-creation failure propagated');
            $check->($linked_symlink && !lstat("$tmp/partial"), 'own partial destination and symlink objects cleaned without traversal');
        }
        $check->(source_snapshot($source)->{nodes}{'src/lib.rs'}{sha256} eq sha256_hex($rust), 'source survived failure cleanup');
        $rejected->(sub { safe_path('../escape.rs') }, 'parent path escape rejected');
        $rejected->(sub { safe_path('/escape.rs') }, 'absolute path rejected');
        1;
    };
    my $error = $@;
    my $cleaned = eval { remove_owned_tree($tmp, \@tmp_identity); 1 };
    die($error || "self-test failed\n") unless $ok;
    die $@ unless $cleaned;
    return { passed => JSON::PP::true, checks => $checks, fixture_cleaned => JSON::PP::true };
}

sub run {
    my ($verify, $self_test, $help);
    Getopt::Long::Configure(qw(no_auto_abbrev no_ignore_case));
    GetOptions('verify' => \$verify, 'self-test' => \$self_test, 'help' => \$help)
        or fail('use --help for usage');
    if ($help) {
        print <<'HELP';
Usage: perl project.pl SOURCE DEST
       perl project.pl --verify SOURCE DEST
       perl project.pl --self-test
SOURCE must be a clean Git worktree root with one origin URL. DEST must be a
fresh directory outside SOURCE, with an existing parent on the same filesystem.
Every tracked regular file and safe internal relative symlink object is hardlinked
at its original path; every .rs path also gains a .lkj hardlink. Inventory never
follows symlinks. Targets must resolve within the original tracked-node/directory
graph; absolute, escaping, dangling, chain-cyclic and directory-cyclic links are
refused, as are gitlinks, dirty files, path escapes and all output collisions.
Assets and .rs paths remain present. No target payload copies, source edits, chmod,
or semantic translations are performed. projection.json records exact Git identity,
byte hashes, aliases and link target text. Regular-only manifests retain the old
encoding. Producer success does not imply the compiler loader accepts link paths.
--verify only reads; it checks the current clean SOURCE, exact manifest, every
projection node, byte hashes and source/projection inode identity. It does not
authenticate a publisher or protect hardlinks from later writes by other tools.
Keep SOURCE and DEST namespaces quiescent during all operations; cleanup checks
the created root identity but is not hardened against concurrent directory swaps.
HELP
        return;
    }
    if ($self_test) {
        fail('--self-test takes no SOURCE, DEST, or --verify') if @ARGV || $verify;
        print json({ self_test => self_test() });
        return;
    }
    fail('SOURCE and DEST are required; use --help') unless @ARGV == 2;
    print json($verify ? verify_projection(@ARGV) : project(@ARGV));
}

run();
