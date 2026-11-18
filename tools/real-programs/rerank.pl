#!/usr/bin/env perl
# Recount a complete source-root probe using reviewed, exact diagnostic sites.
# This does not infer causes from source keywords or equate silence with admission.
use strict;
use warnings;
use Cwd qw(abs_path);
use Digest::SHA qw(sha256_hex);
use Encode qw(decode encode FB_CROAK LEAVE_SRC);
use Fcntl qw(O_WRONLY O_CREAT O_EXCL);
use Getopt::Long qw(GetOptions);
use JSON::PP;
use MIME::Base64 qw(decode_base64);

sub fail { die "real-programs-rerank: $_[0]\n" }
sub json { JSON::PP->new->canonical->ascii->encode($_[0]) }
sub read_bytes {
    open my $fh, '<:raw', $_[0] or fail("read $_[0]: $!");
    my $raw = do { local $/; <$fh> };
    close $fh or fail("close $_[0]: $!");
    return defined($raw) ? $raw : '';
}
sub text {
    my ($raw, $label) = @_;
    my $result = eval { decode('UTF-8', $raw, FB_CROAK | LEAVE_SRC) };
    fail("invalid UTF-8 $label") if $@;
    return $result;
}
sub load_json {
    my ($path) = @_;
    my $raw = read_bytes($path);
    my $value = eval { decode_json($raw) };
    fail("invalid JSON $path: $@") if $@;
    return ($value, sha256_hex($raw));
}
sub stream_text {
    my ($stream) = @_;
    fail('missing captured stream') unless ref($stream) eq 'HASH';
    my $raw = exists($stream->{text})
        ? encode('UTF-8', $stream->{text}, FB_CROAK | LEAVE_SRC)
        : exists($stream->{base64}) ? decode_base64($stream->{base64})
        : fail('captured stream has neither text nor base64');
    fail('captured stream byte/hash mismatch') unless length($raw) == $stream->{bytes}
        && sha256_hex($raw) eq $stream->{sha256};
    return text($raw, 'diagnostic stream');
}
sub coordinate { join(':', @{$_[0]}{qw(path line column)}) }
sub diagnostic_key { json([@{$_[0]}{qw(path line column code message)}]) }
sub observation_key { json([@{$_[0]}{qw(route upstream_entry)}]) }
sub unique { my %seen; return grep { !$seen{$_}++ } @_ }

# Absolute macro-origin locations are meaningful only within the source-dir
# captured by the authenticated baseline command. Offline source restoration
# must not consult or invent a new projection root.
sub projection_prefixes {
    my ($receipt) = @_;
    my %prefixes;
    for my $observation (@{$receipt->{observations}}) {
        next unless $observation->{route} eq 'suffix_projection';
        my $command = $observation->{command};
        fail('missing captured probe command') unless ref($command) eq 'ARRAY';
        my @roots;
        for (my $index = 0; $index < @$command; ++$index) {
            next unless $command->[$index] eq '--source-dir';
            fail('captured source-dir is missing its value') if ++$index >= @$command;
            push @roots, $command->[$index];
        }
        fail('ambiguous captured source-dir') if @roots > 1;
        next unless @roots;
        my $root = $roots[0];
        next unless $root =~ m{\A/} && $root !~ m{(?:\A|/)\.\.?(?:/|\z)|\x00|//};
        $root =~ s{/\z}{} unless $root eq '/';
        $prefixes{$root eq '/' ? '/' : "$root/"} = 1;
    }
    return [sort { length($b) <=> length($a) || $a cmp $b } keys %prefixes];
}

sub source_reader {
    my ($manifest, $root, $projection_prefixes) = @_;
    my %files = map { $_->{path} => $_ } @{$manifest->{files}};
    my %aliases = map { $_->{to} => $_->{from} } @{$manifest->{aliases}};
    my %cache;
    return sub {
        my ($path, $line, $column) = @_;
        if ($path =~ m{\A/}) {
            for my $prefix (@$projection_prefixes) {
                next unless index($path, $prefix) == 0;
                $path = substr($path, length($prefix));
                last;
            }
        }
        fail("diagnostic is outside the tracked original source: $path")
            if $path =~ m{\A/|(?:\A|/)\.\.?(?:/|\z)|\x00};
        $path = $aliases{$path} if exists($aliases{$path});
        fail("diagnostic is outside the tracked original source: $path")
            unless exists($files{$path}) && $path !~ m{\A/|(?:\A|/)\.\.?(?:/|\z)|\x00};
        if (!exists($cache{$path})) {
            my $raw = read_bytes("$root/$path");
            fail("original source bytes changed: $path")
                unless length($raw) == $files{$path}{bytes}
                    && sha256_hex($raw) eq $files{$path}{sha256};
            my @lines = split /\n/, $raw, -1;
            my @starts = (0);
            push @starts, pos($raw) while $raw =~ /\n/g;
            $cache{$path} = { raw => $raw, lines => \@lines, starts => \@starts,
                physical_lines => length($raw) ? @lines - ($raw =~ /\n\z/ ? 1 : 0) : 0 };
        }
        return { path => $path, line => 0, column => 0, source => undef } if !$line && !$column;
        my $file = $cache{$path};
        fail("diagnostic is outside original source coordinates: $path:$line:$column")
            unless $line > 0 && $line <= @{$file->{lines}} && $column > 0
                && $column <= length($file->{lines}[$line - 1]) + 1;
        my $raw_line = $file->{lines}[$line - 1];
        return { path => $path, line => 0 + $line, column => 0 + $column,
            source => { file_sha256 => $files{$path}{sha256}, file_bytes => $files{$path}{bytes},
                file_physical_lines => $file->{physical_lines},
                byte_offset => $file->{starts}[$line - 1] + $column - 1,
                line_sha256 => sha256_hex($raw_line), line_text => text($raw_line, "$path:$line") } };
    };
}

sub read_observations {
    my ($receipt, $manifest, $manifest_sha, $source) = @_;
    fail('not an unmodified root-probe receipt') unless $receipt->{schema} == 1
        && $receipt->{kind} eq 'unmodified_real_program_root_observations';
    fail('receipt/projection identity mismatch')
        unless $receipt->{projection_manifest_sha256} eq $manifest_sha
            && $receipt->{verification_before}{manifest_sha256} eq $manifest_sha
            && $receipt->{verification_after}{manifest_sha256} eq $manifest_sha
            && $receipt->{package}{commit} eq $manifest->{source}{commit};
    my %files = map { $_->{path} => $_ } @{$manifest->{files}};
    my (@rows, @annotations, %observations, %routes);
    for my $observation (@{$receipt->{observations}}) {
        my $key = observation_key($observation);
        fail('duplicate route/entry in receipt') if exists($observations{$key});
        $observations{$key} = $observation;
        $routes{$observation->{upstream_entry}}{$observation->{route}} = 1;
        my $entry = $files{$observation->{upstream_entry}};
        fail('probe entry does not match a tracked original file') unless $entry
            && $entry->{sha256} eq $observation->{entry_sha256}
            && $entry->{bytes} == $observation->{entry_bytes};
        my $stdout = stream_text($observation->{stdout});
        my $stderr = stream_text($observation->{stderr});
        stream_text($observation->{time_report});
        fail('root observation ended abnormally') if $observation->{launcher_signal}
            || ($observation->{exit_status} != 0 && $observation->{exit_status} != 1);
        next if $observation->{route} eq 'original_suffix';
        fail('unknown probe route') unless $observation->{route} eq 'suffix_projection';
        my $index = 0;
        for my $line (split /\n/, $stderr) {
            if ($line =~ /\A(.+):(\d+):(\d+): (E_[A-Z0-9_]+) (.*)\z/) {
                my ($path, $number, $column, $code, $message) = ($1, $2, $3, $4, $5);
                fail('unlocated primary diagnostic') if (!$number || !$column)
                    && !($code eq 'E_BLOCKED' && !$number && !$column);
                my $site = $source->($path, $number, $column);
                push @rows, { %$site, code => $code, message => $message,
                    root => $observation->{upstream_entry}, diagnostic_index => $index++,
                    raw => $line };
            } elsif ($line =~ /\ALKJSCRIPT_EDITS (.*)\z/) {
                my $edits = eval { decode_json(encode('UTF-8', $1)) };
                fail('malformed diagnostic edit annotation') if $@ || ref($edits) ne 'HASH';
                push @annotations, { root => $observation->{upstream_entry}, raw => $line };
            } else {
                fail("unparsed diagnostic line (must be reviewed): $line");
            }
        }
        fail('successful root check emitted diagnostics') if !$observation->{exit_status} && length($stderr);
        fail('refused root check emitted no diagnostics') if $observation->{exit_status} && !length($stderr);
    }
    for my $entry (keys %routes) {
        fail("incomplete original/projection observation pair: $entry")
            unless keys(%{$routes{$entry}}) == 2 && $routes{$entry}{original_suffix}
                && $routes{$entry}{suffix_projection};
    }
    fail('empty observation cohort') unless keys %routes;
    return { rows => \@rows, annotations => \@annotations, observations => \%observations };
}

sub baseline_classes {
    my ($table, $rows) = @_;
    my (%classes, %labels, %primary, %excluded, %by_coordinate);
    for my $row (@$rows) { push @{$by_coordinate{coordinate($row)}}, $row }
    my $add = sub {
        my ($row, $cause, $site_id, $partition) = @_;
        my $key = diagnostic_key($row);
        fail('conflicting reviewed cause at exact diagnostic')
            if exists($classes{$key}) && $classes{$key}{cause} ne $cause;
        $classes{$key} = { cause => $cause, site_id => $site_id, partition => $partition };
        push @{$primary{$site_id}{keys}}, $key unless grep { $_ eq $key } @{$primary{$site_id}{keys} // []};
        $primary{$site_id}{cause} = $cause;
        $primary{$site_id}{coordinate} = coordinate($row);
        $primary{$site_id}{source} = $row->{source};
    };
    if (exists($table->{accounting})) {
        fail('nom baseline expects exactly one projected root')
            unless scalar(unique(map { $_->{root} } @$rows)) == 1;
        my %indices = map { $_->{diagnostic_index} => $_ } @$rows;
        for my $exclusion (@{$table->{recovery_exclusions}}) {
            fail('reviewed recovery requires a parent diagnostic for each row')
                unless @{$exclusion->{diagnostic_indices}} == @{$exclusion->{parent_diagnostic_indices}};
            for my $position (0 .. $#{$exclusion->{diagnostic_indices}}) {
                my $index = $exclusion->{diagnostic_indices}[$position];
                my $parent = $exclusion->{parent_diagnostic_indices}[$position];
                fail("missing reviewed recovery diagnostic index $index") unless exists $indices{$index};
                fail("missing reviewed recovery parent index $parent") unless exists $indices{$parent};
                $excluded{diagnostic_key($indices{$index})} = {
                    reason => $exclusion->{id}, parent_key => diagnostic_key($indices{$parent}) };
            }
        }
        for my $cause (@{$table->{causes}}) {
            $labels{$cause->{id}} = { title => $cause->{title}, owner => $cause->{owner}, baseline_rank => $cause->{rank} };
            for my $site (@{$cause->{sites}}) {
                my $coordinate = join(':', @$site[0..2]);
                my @matched = grep { $_->{code} ne 'E_BLOCKED' && !exists($excluded{diagnostic_key($_)}) }
                    @{$by_coordinate{$coordinate} // []};
                fail("reviewed nom primary coordinate has no exact diagnostic: $coordinate") unless @matched;
                $add->($_, $cause->{id}, "$cause->{id}:$coordinate", $site->[4] ? 'inactive_in_upstream_default' : 'default_candidate') for @matched;
            }
        }
        fail('nom baseline primary count mismatch') unless keys(%primary) == $table->{accounting}{unique_primary_sites};
    } elsif (exists($table->{primary_sites})) {
        for my $cause (@{$table->{causes}}) {
            $labels{$cause->{id}} = { title => $cause->{cause}, owner => $cause->{owner}, baseline_rank => $cause->{rank} };
        }
        for my $duplicate (@{$table->{recovery_duplicate_messages}}) {
            my @parents = grep { $_->{code} eq $duplicate->{primary_code}
                    && $_->{message} eq $duplicate->{primary_message} }
                @{$by_coordinate{$duplicate->{coordinate}} // []};
            fail('reviewed duplicate has no primary diagnostic') unless @parents;
            for my $row (@{$by_coordinate{$duplicate->{coordinate}} // []}) {
                $excluded{diagnostic_key($row)} = { reason => 'same_coordinate_recovery_duplicate',
                    parent_key => diagnostic_key($parents[0]) }
                    if $row->{code} eq $duplicate->{secondary_code} && $row->{message} eq $duplicate->{secondary_message};
            }
        }
        for my $site (@{$table->{primary_sites}}) {
            my ($coordinate, $cause, $code, $partition) = @$site;
            my @matched = grep { $_->{code} eq $code && !exists($excluded{diagnostic_key($_)}) }
                @{$by_coordinate{$coordinate} // []};
            fail("reviewed DOM primary coordinate has no exact diagnostic: $coordinate") unless @matched;
            $add->($_, $cause, "$cause:$coordinate", $partition) for @matched;
        }
        fail('DOM baseline primary count mismatch') unless keys(%primary) == $table->{counts}{unique_primary_sites};
    } elsif (exists($table->{groups})) {
        for my $cause (sort keys %{$table->{groups}}) {
            my $group = $table->{groups}{$cause};
            $labels{$cause} = { title => $cause, owner => $group->{owner}, baseline_rank => $group->{rank} };
            for my $site (@{$group->{sites}}) {
                my $coordinate = join(':', @$site[0..2]);
                my $kind = $group->{diagnostic_kinds}[$site->[3]];
                my @matched = grep { "$_->{code}: $_->{message}" eq $kind }
                    @{$by_coordinate{$coordinate} // []};
                fail("reviewed Sozu site has no exact diagnostic: $coordinate $kind") unless @matched;
                $add->($_, $cause, "$cause:$coordinate:$kind", $group->{independent_gap_sites} ? 'observed_cause' : 'unclassified_recovery') for @matched;
            }
        }
        my $expected = 0; $expected += $_->{diagnosed_sites} for values %{$table->{groups}};
        fail('Sozu baseline primary count mismatch') unless keys(%primary) == $expected;
    } else { fail('unsupported baseline classification schema') }
    for my $row (@$rows) {
        my $key = diagnostic_key($row);
        next if $row->{code} eq 'E_BLOCKED';
        fail("unaccounted baseline diagnostic: $row->{raw}")
            unless exists($classes{$key}) || exists($excluded{$key});
        fail('baseline primary is also excluded') if exists($classes{$key}) && exists($excluded{$key});
    }
    return (\%classes, \%labels, \%primary, \%excluded);
}

sub main {
    my ($baseline_path, $before_path, $after_path, $projection_path, $source_root, $output, $review_path, $help);
    Getopt::Long::Configure(qw(no_auto_abbrev no_ignore_case));
    GetOptions('baseline=s' => \$baseline_path, 'before=s' => \$before_path, 'after=s' => \$after_path,
        'projection=s' => \$projection_path, 'source=s' => \$source_root, 'output=s' => \$output,
        'review=s' => \$review_path, 'help' => \$help) or fail('use --help');
    if ($help) {
        print <<'HELP';
Usage: perl rerank.pl --baseline docs/real-programs-nom.json
       --before OLD_PROBE.json --after NEW_PROBE.json
       --projection PROJECTION/projection.json [--source ORIGINAL_SOURCE]
       [--review REVIEWED_NEW_SITES.json] --output NEW_REPORT.json

The original per-site classifications are joined to complete baseline diagnostics,
then inherited only by byte-identical code/message at the same original coordinate.
Source line/column anchors are verified against the projection's complete file hash.
Absolute projected locations require the baseline command's exact source-dir prefix;
new roots, traversal and untracked paths are refused.
Unknown and changed diagnostics remain unclassified. No keyword classifier runs.
Pass the same before/after receipt first to reproduce the published baseline.

Optional review format: {"schema":1,"sites":[{"path":"src/file.rs","line":1,
"column":1,"code":"E_TYPE","message":"exact full message","file_sha256":"...",
"cause":"reviewed_cause","title":"Title","owner":"Language","reason":"Source and diagnostic review"}]}.
Every review must match an actual new diagnostic and its complete source-file hash.
For a reviewed recovery row add "disposition":"recovery" and "parent" with
the complete path/line/column/code/message of its earlier diagnostic in each root.
Missing old diagnostics mean not observed at this frontier, never accepted code.
The output must not already exist. All input files are read-only.
HELP
        return;
    }
    fail('missing required arguments; use --help') if @ARGV
        || grep { !defined($_) || !length($_) } ($baseline_path, $before_path, $after_path, $projection_path, $output);
    fail('output already exists') if lstat($output);
    my ($table, $table_sha) = load_json($baseline_path);
    my ($before, $before_sha) = load_json($before_path);
    my ($after, $after_sha) = load_json($after_path);
    my ($manifest, $manifest_sha) = load_json($projection_path);
    fail('not a tracked suffix-projection manifest') unless $manifest->{schema} == 1
        && $manifest->{kind} eq 'git_tracked_suffix_projection';
    fail('before receipt is not the baseline table current verification')
        unless $table->{current_verification}{receipt_sha256} eq $before_sha;
    $source_root //= $manifest->{source}{root};
    my $source = source_reader($manifest, $source_root, projection_prefixes($before));
    my $old = read_observations($before, $manifest, $manifest_sha, $source);
    my $new = read_observations($after, $manifest, $manifest_sha, $source);
    fail('probe root/route scope changed') unless json([sort keys %{$old->{observations}}])
        eq json([sort keys %{$new->{observations}}]);
    my $original_routes_equal = 1;
    for my $key (keys %{$old->{observations}}) {
        my ($left, $right) = ($old->{observations}{$key}, $new->{observations}{$key});
        fail('probe source-entry identity changed') unless $left->{entry_sha256} eq $right->{entry_sha256}
            && $left->{entry_bytes} == $right->{entry_bytes};
        fail('probe command changed beyond compiler path') unless json([@{$left->{command}}[1..$#{$left->{command}}]])
            eq json([@{$right->{command}}[1..$#{$right->{command}}]]);
        next unless $left->{route} eq 'original_suffix';
        $original_routes_equal &&= json([@{$left}{qw(exit_status launcher_signal stdout stderr)}])
            eq json([@{$right}{qw(exit_status launcher_signal stdout stderr)}]);
    }
    my ($classes, $labels, $primary, $excluded) = baseline_classes($table, $old->{rows});
    my (%new_by_key, %new_at_coordinate);
    for my $row (@{$new->{rows}}) {
        next if $row->{code} eq 'E_BLOCKED';
        my $key = diagnostic_key($row);
        $new_by_key{$key} //= { %$row, observed_roots => [] };
        push @{$new_by_key{$key}{observed_roots}}, $row->{root}
            unless grep { $_ eq $row->{root} } @{$new_by_key{$key}{observed_roots}};
        push @{$new_at_coordinate{coordinate($row)}}, diagnostic_key($row);
    }
    my ($review, $review_sha);
    if (defined $review_path) {
        ($review, $review_sha) = load_json($review_path);
        fail('unsupported explicit-review schema') unless $review->{schema} == 1 && ref($review->{sites}) eq 'ARRAY';
        for my $entry (@{$review->{sites}}) {
            my $key = diagnostic_key($entry);
            fail('review does not match an observed diagnostic') unless exists($new_by_key{$key});
            fail('review source hash mismatch') unless $entry->{file_sha256} eq $new_by_key{$key}{source}{file_sha256};
            fail('review needs cause/title/owner/reason') if grep { !defined($entry->{$_}) || !length($entry->{$_}) } qw(cause title owner reason);
            fail('explicit review cannot overwrite surviving baseline classification') if exists($classes->{$key})
                || (exists($excluded->{$key}) && exists($new_by_key{$excluded->{$key}{parent_key}}));
            my $disposition = $entry->{disposition} // 'primary';
            if ($disposition eq 'recovery') {
                fail('reviewed recovery requires a parent diagnostic') unless ref($entry->{parent}) eq 'HASH';
                my $parent = diagnostic_key($entry->{parent});
                fail('reviewed recovery parent does not match another observed diagnostic')
                    unless $parent ne $key && exists($new_by_key{$parent});
                for my $row (grep { diagnostic_key($_) eq $key } @{$new->{rows}}) {
                    fail('reviewed recovery parent is not earlier in the same root') unless grep {
                        diagnostic_key($_) eq $parent && $_->{root} eq $row->{root}
                            && $_->{diagnostic_index} < $row->{diagnostic_index}
                    } @{$new->{rows}};
                }
                $excluded->{$key} = { reason => $entry->{cause}, parent_key => $parent,
                    review_reason => $entry->{reason} };
                next;
            }
            fail('unknown reviewed disposition') unless $disposition eq 'primary';
            delete $excluded->{$key};
            $classes->{$key} = { cause => $entry->{cause}, site_id => "review:$key",
                partition => 'explicitly_reviewed', review_reason => $entry->{reason} };
            $labels->{$entry->{cause}} //= { title => $entry->{title}, owner => $entry->{owner} };
        }
    }
    my (@comparison, %before_counts, %survival_counts);
    for my $id (sort keys %$primary) {
        my $site = $primary->{$id};
        my @surviving = grep { exists($new_by_key{$_}) } @{$site->{keys}};
        my @at_coordinate = unique(@{$new_at_coordinate{$site->{coordinate}} // []});
        my $state = @surviving ? 'exact_diagnostic_survives'
            : @at_coordinate ? 'different_diagnostic_at_same_coordinate' : 'not_observed';
        ++$before_counts{$site->{cause}};
        ++$survival_counts{$site->{cause}}{$state};
        push @comparison, { site_id => $id, %$site, state => $state,
            surviving_diagnostic_keys => \@surviving, current_diagnostic_keys_at_coordinate => \@at_coordinate };
    }
    my (%groups, %raw_by_code, %seen_site, @unknown, @recovery, $classified_count, $prior_unclassified_count);
    ($classified_count, $prior_unclassified_count) = (0, 0);
    for my $row (@{$new->{rows}}) { ++$raw_by_code{$row->{code}} }
    for my $key (sort keys %new_by_key) {
        my $row = $new_by_key{$key};
        if (exists($excluded->{$key}) && exists($new_by_key{$excluded->{$key}{parent_key}})) {
            push @recovery, { %$row, exclusion => $excluded->{$key} }; next;
        }
        my $classification = $classes->{$key};
        if (!$classification) { push @unknown, $row; next }
        my $cause = $classification->{cause};
        next if $seen_site{$classification->{site_id}}++;
        if ($classification->{partition} eq 'unclassified_recovery') { ++$prior_unclassified_count }
        else { ++$classified_count }
        push @{$groups{$cause}{sites}}, { %$row, %$classification };
        $groups{$cause}{files}{$row->{path}} = $row->{source}{file_physical_lines};
    }
    my @ranked;
    for my $cause (sort keys %groups) {
        my $group = $groups{$cause};
        my $coverage = 0; $coverage += $_ for values %{$group->{files}};
        push @ranked, { id => $cause, %{$labels->{$cause}}, sites => $group->{sites},
            observed_sites => scalar @{$group->{sites}}, affected_files => [sort keys %{$group->{files}}],
            overlapping_file_physical_lines => $coverage,
            baseline_sites => $before_counts{$cause} // 0,
            old_site_survival => $survival_counts{$cause} // {} };
    }
    @ranked = sort { $b->{observed_sites} <=> $a->{observed_sites} || $a->{id} cmp $b->{id} } @ranked;
    $ranked[$_]{count_rank} = $_ + 1 for 0..$#ranked;
    my @counts = map { { id => $_, %{$labels->{$_}}, baseline_sites => $before_counts{$_},
        exact_surviving_sites => $survival_counts{$_}{exact_diagnostic_survives} // 0,
        changed_diagnostic_sites => $survival_counts{$_}{different_diagnostic_at_same_coordinate} // 0,
        not_observed_sites => $survival_counts{$_}{not_observed} // 0 } } sort keys %before_counts;
    my $report = { schema => 1, kind => 'exact_foreign_refusal_comparison',
        inputs => { baseline => { path => $baseline_path, sha256 => $table_sha },
            before => { path => $before_path, sha256 => $before_sha, compiler => $before->{compiler} },
            after => { path => $after_path, sha256 => $after_sha, compiler => $after->{compiler} },
            projection => { path => $projection_path, sha256 => $manifest_sha, source => $manifest->{source} },
            defined($review_path) ? (review => { path => $review_path, sha256 => $review_sha }) : () },
        observation_count => scalar keys %{$new->{observations}},
        original_suffix_observations_unchanged => $original_routes_equal ? JSON::PP::true : JSON::PP::false,
        raw_projected_diagnostic_count => scalar @{$new->{rows}}, raw_by_code => \%raw_by_code,
        observed_classified_sites => $classified_count,
        observed_prior_unclassified_sites => $prior_unclassified_count,
        inherited_recovery_unique_diagnostics => scalar @recovery,
        unclassified_unique_diagnostics => scalar @unknown,
        baseline_site_counts => \@counts, baseline_site_comparison => \@comparison,
        ranked_observed_causes => \@ranked, unclassified_diagnostics => \@unknown,
        inherited_recovery_diagnostics => \@recovery, diagnostic_edit_annotations => $new->{annotations},
        interpretation => [
            'Causes are inherited only from an explicitly classified, identical diagnostic at the same unchanged source coordinate; reviewed new sites name the complete code/message and file hash.',
            'Missing old diagnostics are not observed at this frontier. This is not proof of admission, successful execution, or equivalent Cargo feature/test configuration.',
            'New or changed diagnostics stay unclassified until explicitly reviewed. Known recovery exclusions remain exclusions only while both their complete diagnostic identity and their reviewed parent diagnostic survive.',
            'Source columns are one-based UTF-8 byte columns, matching the compiler lexer, not the Unicode-scalar columns of census.pl.',
            'Count ranking is descriptive. Ownership and implementation priority remain explicit decisions. File coverage overlaps and is not a causal blocked-line estimate.' ] };
    sysopen my $fh, $output, O_WRONLY | O_CREAT | O_EXCL, 0600 or fail("create output: $!");
    binmode $fh;
    print {$fh} JSON::PP->new->canonical->pretty->ascii->encode($report) or fail("write output: $!");
    close $fh or fail("close output: $!");
    print json({ report => $output, observations => $report->{observation_count},
        old_sites => scalar(@comparison), unclassified_diagnostics => scalar(@unknown),
        ranked_causes => scalar(@ranked) }), "\n";
}

main();
