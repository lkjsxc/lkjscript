#!/usr/bin/env perl
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use Fcntl qw(O_RDONLY O_NOFOLLOW S_ISREG);
use File::Spec;
use Getopt::Long qw(GetOptions);
use JSON::PP qw(decode_json encode_json);

# This verifier reconstructs the upstream bytes. It does not trust the importer,
# the compiler's test discovery, or a count of successful test executions.
my $marker = "\n// UPSTREAM INTEGRATION TESTS: exact bodies, package header removed.\n";
my $header = "extern crate levenshtein;\nuse levenshtein::levenshtein;\n";
my $normalized = "std::iter::range_from(1)";
my $original = "(1..)";
my $self_test = 0;
GetOptions('self-test' => \$self_test) or usage();
@ARGV == 1 or usage();
my $directory = $ARGV[0];

sub usage {
    die "usage: perl verify-levenshtein.pl [--self-test] SPECIMEN_DIRECTORY\n";
}

sub read_regular {
    my ($path) = @_;
    sysopen my $handle, $path, O_RDONLY | O_NOFOLLOW
        or die "cannot open $path: $!\n";
    my @stat = stat $handle;
    @stat && S_ISREG($stat[2]) or die "not a regular file: $path\n";
    binmode $handle;
    local $/;
    my $bytes = <$handle>;
    defined $bytes or die "cannot read $path: $!\n";
    close $handle or die "cannot close $path: $!\n";
    return $bytes;
}

sub require_fact {
    my ($condition, $message) = @_;
    $condition or die "provenance refusal: $message\n";
}

sub occurrences {
    my ($text, $needle) = @_;
    my $count = () = $text =~ /\Q$needle\E/g;
    return $count;
}

sub line_count {
    my ($bytes) = @_;
    return scalar(() = $bytes =~ /\n/g);
}

sub verify {
    my ($manifest, $projection, $license) = @_;
    require_fact(ref($manifest) eq 'HASH', 'manifest must be an object');
    require_fact(($manifest->{schema} // '') eq '1', 'unsupported manifest schema');
    require_fact(($manifest->{name} // '') eq 'levenshtein', 'wrong package name');
    require_fact(($manifest->{version} // '') eq '1.0.5', 'wrong package version');
    require_fact(($manifest->{origin} // '') eq
        'https://github.com/wooorm/levenshtein-rs', 'wrong source origin');
    require_fact(($manifest->{commit} // '') eq
        '9c4730b1973d4e61e187f8fe6d5f299ad5c991fc', 'wrong pinned commit');
    require_fact(($manifest->{license} // '') eq 'MIT', 'wrong license identity');
    require_fact(ref($manifest->{files}) eq 'HASH', 'missing file hashes');
    require_fact(ref($manifest->{adaptation}) eq 'HASH', 'missing adaptation contract');
    require_fact(($manifest->{adaptation}{range_expression} // '') eq '1',
        'range adaptation count must be one');
    require_fact(($manifest->{adaptation}{integration_header_lines} // '') eq '2',
        'integration header count must be two');
    require_fact(occurrences($projection, $marker) == 1,
        'projection must contain exactly one integration boundary');

    my ($library, $tests) = split /\Q$marker\E/, $projection, -1;
    require_fact(defined($tests) && length($library) && length($tests),
        'empty projection section');
    require_fact(occurrences($library, $normalized) == 1,
        'library must contain exactly one normalized range expression');
    require_fact(occurrences($library, $original) == 0,
        'library contains an unexpected original range expression');
    $library =~ s/\Q$normalized\E/$original/;
    my $original_tests = $header . $tests;

    my %reconstructed = (
        'src/lib.rs' => $library,
        'tests/lib.rs' => $original_tests,
        'license' => $license,
    );
    my %verified;
    for my $name (sort keys %reconstructed) {
        my $expected = $manifest->{files}{$name} // '';
        require_fact($expected =~ /\A[0-9a-f]{64}\z/, "invalid hash for $name");
        my $actual = sha256_hex($reconstructed{$name});
        require_fact($actual eq $expected, "upstream bytes differ: $name");
        $verified{$name} = {
            sha256 => $actual,
            bytes => length($reconstructed{$name}),
            lines => line_count($reconstructed{$name}),
        };
    }
    require_fact($verified{'src/lib.rs'}{lines} ==
        ($manifest->{source_lines} // -1), 'source line count differs');
    require_fact($verified{'tests/lib.rs'}{lines} ==
        ($manifest->{test_lines} // -1), 'test line count differs');

    # Whole-file equality above already preserves every body, attribute, literal,
    # assertion and byte of whitespace. These counts are independently reported.
    my @names = $tests =~ /^#\[test\]\nfn ([A-Za-z_][A-Za-z_0-9]*)\(\) \{/mg;
    my $attributes = () = $tests =~ /^#\[test\]$/mg;
    my $expected_names = $manifest->{test_names};
    require_fact(ref($expected_names) eq 'ARRAY' && @$expected_names == 8,
        'manifest must name eight tests');
    require_fact(@names == 8 && $attributes == 8, 'projection must contain eight tests');
    require_fact(join("\0", @names) eq join("\0", @$expected_names),
        'ordered test names differ');
    my %unique;
    require_fact(!grep($unique{$_}++, @names), 'duplicate test name');
    my $assertions = () = $tests =~ /^[ \t]*assert(?:_eq|_ne)?!\s*\(/mg;
    require_fact($assertions > 0, 'no upstream assertions');

    return {
        schema => 1,
        kind => 'levenshtein-upstream-provenance',
        commit => $manifest->{commit},
        files => \%verified,
        projection_sha256 => sha256_hex($projection),
        test_names => \@names,
        test_count => scalar(@names),
        assertion_count => $assertions,
        inverse_range_replacements => 1,
        restored_header_lines => 2,
        exact_upstream_bytes => JSON::PP::true,
    };
}

sub replace_once {
    my ($probe, $old, $new) = @_;
    occurrences($probe->{projection}, $old) == 1
        or die "invalid mutation fixture: expected one $old\n";
    $probe->{projection} =~ s/\Q$old\E/$new/;
}

my $manifest_bytes = read_regular(File::Spec->catfile($directory, 'upstream.json'));
my $manifest = decode_json($manifest_bytes);
my $projection = read_regular(File::Spec->catfile($directory, 'upstream.lkj'));
my $license = read_regular(File::Spec->catfile($directory, 'LICENSE'));
my $report = verify($manifest, $projection, $license);

if ($self_test) {
    my $assertion = "    assert_eq!(levenshtein(\"\", \"a\"), 1);\n";
    my @mutants = (
        ['missing range normalization', sub {
            replace_once($_[0], $normalized, $original);
        }],
        ['extra normalized expression', sub {
            $_[0]{projection} = "// $normalized\n" . $_[0]{projection};
        }],
        ['algorithm change', sub {
            replace_once($_[0], 'cache[index_a] = result;',
                'cache[index_a] = result + 1;');
        }],
        ['deleted assertion', sub {
            replace_once($_[0], $assertion, '');
        }],
        ['changed expected value', sub {
            replace_once($_[0], $assertion,
                "    assert_eq!(levenshtein(\"\", \"a\"), 0);\n");
        }],
        ['renamed test', sub {
            replace_once($_[0], 'fn empty_left()', 'fn replaced_left()');
        }],
        ['excluded test', sub {
            replace_once($_[0], "#[test]\nfn empty_left()",
                "#[cfg(any())]\n#[test]\nfn empty_left()");
        }],
        ['injected test', sub {
            $_[0]{projection} .= "\n#[test]\nfn injected() {}\n";
        }],
        ['changed test literal bytes', sub {
            replace_once($_[0], '"DwAyNE"', '"DWAYNE"');
        }],
        ['changed boundary', sub {
            replace_once($_[0], $marker, "\n// changed boundary\n");
        }],
        ['duplicate boundary', sub {
            $_[0]{projection} .= $marker;
        }],
        ['unexpected integration header', sub {
            replace_once($_[0], $marker, $marker . $header);
        }],
        ['trailing unaccounted source', sub {
            $_[0]{projection} .= "// unrecorded source\n";
        }],
        ['changed license', sub {
            $_[0]{license} .= "\n";
        }],
        ['reordered manifest tests', sub {
            @{$_[0]{manifest}{test_names}} =
                reverse @{$_[0]{manifest}{test_names}};
        }],
        ['false source count', sub {
            $_[0]{manifest}{source_lines}++;
        }],
        ['wrong pinned commit', sub {
            $_[0]{manifest}{commit} = '0' x 40;
        }],
        ['false adaptation count', sub {
            $_[0]{manifest}{adaptation}{range_expression} = 0;
        }],
    );
    my @results;
    for my $mutant (@mutants) {
        my ($name, $mutate) = @$mutant;
        my $probe = {
            manifest => decode_json(encode_json($manifest)),
            projection => $projection,
            license => $license,
        };
        $mutate->($probe);
        my $accepted = eval {
            verify($probe->{manifest}, $probe->{projection}, $probe->{license});
            1;
        };
        die "mutation escaped verification: $name\n" if $accepted;
        my $reason = $@;
        $reason =~ s/\n\z//;
        require_fact($reason =~ /\Aprovenance refusal: /,
            "mutation did not reach a provenance refusal: $name");
        push @results, {name => $name, rejected => JSON::PP::true, reason => $reason};
    }
    $report->{mutation_controls} = \@results;
    $report->{mutants_rejected} = scalar(@results);
    $report->{temporary_files_created} = 0;
}

print JSON::PP->new->canonical->pretty->encode($report);
