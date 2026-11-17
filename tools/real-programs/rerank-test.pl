#!/usr/bin/env perl
# Exact-site evidence controls; no compiler or external specimen is required.
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use File::Temp qw(tempdir);
use FindBin;
use JSON::PP;

my $temporary = tempdir('lkjscript-rerank-XXXXXX', TMPDIR => 1, CLEANUP => 1);
my $tool = "$FindBin::Bin/rerank.pl";
my ($checks, $sequence) = (0, 0);
sub check { ++$checks; die "rerank-test: $_[1]\n" unless $_[0] }
sub bytes {
    open my $fh, '<:raw', $_[0] or die $!;
    local $/; my $raw = <$fh>; return defined($raw) ? $raw : '';
}
sub write_file {
    open my $fh, '>:raw', $_[0] or die $!;
    print {$fh} $_[1]; close $fh or die $!;
}
sub write_json { write_file($_[0], JSON::PP->new->canonical->ascii->encode($_[1])) }
sub clone { decode_json(JSON::PP->new->canonical->ascii->encode($_[0])) }
sub captured { return { text => $_[0], bytes => length($_[0]), sha256 => sha256_hex($_[0]) } }
sub invoke {
    my ($after, @extra) = @_;
    my $output = "$temporary/report-" . $sequence++ . '.json';
    write_json("$temporary/after.json", $after);
    my @command = ($^X, $tool, '--baseline', "$temporary/baseline.json", '--before', "$temporary/before.json",
        '--after', "$temporary/after.json", '--projection', "$temporary/projection.json",
        '--source', "$temporary/source", '--output', $output, @extra);
    my $pid = fork(); die $! unless defined $pid;
    if (!$pid) {
        open STDOUT, '>', "$temporary/stdout" or die $!;
        open STDERR, '>', "$temporary/stderr" or die $!;
        exec {$command[0]} @command; die $!;
    }
    waitpid($pid, 0) == $pid or die $!;
    return ($? >> 8, -f $output ? decode_json(bytes($output)) : undef, bytes("$temporary/stderr"));
}

mkdir "$temporary/source" or die $!;
mkdir "$temporary/source/src" or die $!;
my $source = "fn a() -> impl Value {}\nfn b(x: Qualified) {}\nfn c() {}\n";
write_file("$temporary/source/src/lib.rs", $source);
my $manifest = { schema => 1, kind => 'git_tracked_suffix_projection',
    source => { root => "$temporary/source", commit => 'a' x 40 },
    files => [{ path => 'src/lib.rs', bytes => length($source), sha256 => sha256_hex($source) }],
    aliases => [{ from => 'src/lib.rs', to => 'src/lib.lkj' }] };
write_json("$temporary/projection.json", $manifest);
my $manifest_sha = sha256_hex(bytes("$temporary/projection.json"));
my @diagnostics = (
    'src/lib.lkj:1:11: E_UNSUPPORTED unsupported construct: impl',
    'src/lib.lkj:2:9: E_PARSE expected an identifier',
    'src/lib.lkj:1:16: E_UNSUPPORTED where predicate on a nonparameter',
    'src/lib.lkj:1:1: E_BLOCKED remaining semantic analysis: source errors',
);
my $receipt = { schema => 1, kind => 'unmodified_real_program_root_observations',
    package => { commit => 'a' x 40 }, compiler => { path => '/compiler', sha256 => 'b' x 64 },
    projection_manifest_sha256 => $manifest_sha,
    verification_before => { manifest_sha256 => $manifest_sha },
    verification_after => { manifest_sha256 => $manifest_sha }, observations => [] };
for my $route ('original_suffix', 'suffix_projection') {
    push @{$receipt->{observations}}, { route => $route, upstream_entry => 'src/lib.rs',
        entry_sha256 => sha256_hex($source), entry_bytes => length($source), exit_status => 1,
        launcher_signal => 0, command => ['/compiler', 'check', $route eq 'original_suffix' ? 'src/lib.rs' : 'src/lib.lkj'],
        stdout => captured(''), stderr => captured($route eq 'original_suffix'
            ? "src/lib.rs: E_SOURCE_EXTENSION source files use .lkj\n" : join("\n", @diagnostics) . "\n"),
        time_report => captured("0\n") };
}
write_json("$temporary/before.json", $receipt);
my $table = { accounting => { unique_primary_sites => 2 }, current_verification => {
        receipt_sha256 => sha256_hex(bytes("$temporary/before.json")) },
    causes => [
        { id => 'impl_trait', title => 'impl Trait', owner => 'Language', rank => 1,
          sites => [['src/lib.rs', 1, 11, undef, JSON::PP::false]] },
        { id => 'qualified', title => 'Qualified path', owner => 'Language', rank => 2,
          sites => [['src/lib.rs', 2, 9, undef, JSON::PP::false]] },
    ], recovery_exclusions => [{ id => 'known_recovery', diagnostic_indices => [2], parent_diagnostic_indices => [0] }] };
write_json("$temporary/baseline.json", $table);

my ($status, $report, $error) = invoke($receipt);
check($status == 0, "unchanged baseline failed: $error");
check(@{$report->{baseline_site_comparison}} == 2 && $report->{unclassified_unique_diagnostics} == 0,
    'unchanged baseline exactly accounts for primary sites');
check($report->{inherited_recovery_unique_diagnostics} == 1 && $report->{raw_by_code}{E_BLOCKED} == 1,
    'recovery and E_BLOCKED stay separate');
check(!grep({ $_->{state} ne 'exact_diagnostic_survives' } @{$report->{baseline_site_comparison}}),
    'both old sites survive unchanged');

my $after = clone($receipt);
$after->{observations}[1]{stderr} = captured("$diagnostics[1]\n$diagnostics[3]\n");
($status, $report, $error) = invoke($after);
check($status == 0, "removed diagnostic case failed: $error");
my ($impl) = grep { $_->{id} eq 'impl_trait' } @{$report->{baseline_site_counts}};
check($impl->{not_observed_sites} == 1 && $impl->{exact_surviving_sites} == 0,
    'absent old site is not observed rather than accepted');

$after = clone($receipt);
$after->{observations}[1]{stderr} = captured("$diagnostics[1]\n$diagnostics[2]\n$diagnostics[3]\n");
($status, $report, $error) = invoke($after);
check($status == 0 && $report->{inherited_recovery_unique_diagnostics} == 0,
    'recovery exclusion cannot survive removal of its exact parent');
check($report->{unclassified_unique_diagnostics} == 1,
    'formerly recovering diagnostic without parent needs fresh review');

$after = clone($receipt);
$after->{observations}[1]{stderr} = captured("src/lib.lkj:1:11: E_TYPE different frontier\n$diagnostics[1]\n$diagnostics[3]\n");
($status, $report, $error) = invoke($after);
check($status == 0 && $report->{unclassified_unique_diagnostics} == 1, 'changed diagnostic is unclassified');
($impl) = grep { $_->{id} eq 'impl_trait' } @{$report->{baseline_site_counts}};
check($impl->{changed_diagnostic_sites} == 1 && !$impl->{not_observed_sites}, 'changed coordinate stays distinguished');

$after = clone($receipt);
$after->{observations}[1]{stderr} = captured(join("\n", @diagnostics,
    'src/lib.lkj:3:1: E_UNSUPPORTED unsupported construct: impl') . "\n");
($status, $report, $error) = invoke($after);
check($status == 0 && $report->{unclassified_unique_diagnostics} == 1,
    'identical words at a new source site never inherit a guessed cause');
my $review = { schema => 1, sites => [{ path => 'src/lib.rs', line => 3, column => 1,
    code => 'E_UNSUPPORTED', message => 'unsupported construct: impl', file_sha256 => sha256_hex($source),
    cause => 'new_reviewed_form', title => 'Reviewed form', owner => 'Language', reason => 'Fixture source review' }] };
write_json("$temporary/review.json", $review);
($status, $report, $error) = invoke($after, '--review', "$temporary/review.json");
check($status == 0 && !$report->{unclassified_unique_diagnostics}, 'exact reviewed new site is classified');
$review->{sites}[0]{disposition} = 'recovery';
$review->{sites}[0]{parent} = { path => 'src/lib.rs', line => 1, column => 11,
    code => 'E_UNSUPPORTED', message => 'unsupported construct: impl' };
write_json("$temporary/review.json", $review);
($status, $report, $error) = invoke($after, '--review', "$temporary/review.json");
check($status == 0 && $report->{inherited_recovery_unique_diagnostics} == 2
    && !$report->{unclassified_unique_diagnostics}, 'new recovery requires an exact surviving parent');
$review->{sites}[0]{parent}{message} = 'unobserved parent';
write_json("$temporary/review.json", $review);
($status, $report, $error) = invoke($after, '--review', "$temporary/review.json");
check($status != 0 && $error =~ /parent does not match/, 'invented recovery parent refuses');
$review->{sites}[0]{parent}{message} = 'unsupported construct: impl';
$review->{sites}[0]{parent}{line} = 3;
$review->{sites}[0]{parent}{column} = 1;
write_json("$temporary/review.json", $review);
($status, $report, $error) = invoke($after, '--review', "$temporary/review.json");
check($status != 0 && $error =~ /parent does not match/, 'self-referential recovery parent refuses');
$review->{sites}[0]{parent}{line} = 1;
$review->{sites}[0]{parent}{column} = 11;
write_json("$temporary/review.json", $review);
my $reordered = clone($after);
$reordered->{observations}[1]{stderr} = captured('src/lib.lkj:3:1: E_UNSUPPORTED unsupported construct: impl' . "\n"
    . join("\n", @diagnostics) . "\n");
($status, $report, $error) = invoke($reordered, '--review', "$temporary/review.json");
check($status != 0 && $error =~ /parent is not earlier/, 'later recovery parent refuses');
delete $review->{sites}[0]{disposition};
delete $review->{sites}[0]{parent};
$review->{sites}[0]{file_sha256} = 'f' x 64;
write_json("$temporary/review.json", $review);
($status, $report, $error) = invoke($after, '--review', "$temporary/review.json");
check($status != 0 && $error =~ /review source hash mismatch/, 'review cannot name unrelated source bytes');

for my $mutation (
    ['stream hash', sub { $_[0]{observations}[1]{stderr}{sha256} = 'f' x 64 }, qr/stream byte\/hash mismatch/],
    ['signal', sub { $_[0]{observations}[1]{launcher_signal} = 9 }, qr/ended abnormally/],
    ['missing route', sub { pop @{$_[0]{observations}} }, qr/incomplete original\/projection/],
    ['changed command', sub { push @{$_[0]{observations}[1]{command}}, '--changed' }, qr/command changed/],
    ['unparsed output', sub { $_[0]{observations}[1]{stderr} = captured("unknown error\n") }, qr/unparsed diagnostic/],
    ['bad coordinate', sub { $_[0]{observations}[1]{stderr} = captured("src/lib.lkj:1:999: E_PARSE wrong\n") }, qr/outside original source coordinates/],
    ['unknown source', sub { $_[0]{observations}[1]{stderr} = captured("other.lkj:1:1: E_PARSE wrong\n") }, qr/outside the tracked original source/],
) {
    $after = clone($receipt); $mutation->[1]->($after);
    ($status, $report, $error) = invoke($after);
    check($status != 0 && !defined($report) && $error =~ $mutation->[2], "$mutation->[0] must refuse: $error");
}
write_file("$temporary/source/src/lib.rs", $source . '// changed');
($status, $report, $error) = invoke($receipt);
check($status != 0 && $error =~ /original source bytes changed/, 'source hashes are checked before counting');
write_file("$temporary/source/src/lib.rs", $source);

# DOM retains one primary site when parser recovery emits a second message at
# the same coordinate. Its different table schema must preserve this distinction.
my $dom = clone($receipt);
my $duplicate = 'src/lib.lkj:1:11: E_UNSUPPORTED unsupported construct: impl item';
$dom->{observations}[1]{stderr} = captured("$diagnostics[0]\n$diagnostics[1]\n$duplicate\n$diagnostics[3]\n");
write_json("$temporary/before.json", $dom);
my $dom_table = { counts => { unique_primary_sites => 2 },
    current_verification => { receipt_sha256 => sha256_hex(bytes("$temporary/before.json")) },
    causes => [ { id => 'impl_trait', cause => 'impl Trait', owner => 'Language', rank => 1 },
        { id => 'qualified', cause => 'Qualified path', owner => 'Language', rank => 2 } ],
    primary_sites => [ ['src/lib.rs:1:11', 'impl_trait', 'E_UNSUPPORTED', 'observed_cause'],
        ['src/lib.rs:2:9', 'qualified', 'E_PARSE', 'observed_cause'] ],
    recovery_duplicate_messages => [{ coordinate => 'src/lib.rs:1:11',
        primary_code => 'E_UNSUPPORTED', primary_message => 'unsupported construct: impl',
        secondary_code => 'E_UNSUPPORTED', secondary_message => 'unsupported construct: impl item' }] };
write_json("$temporary/baseline.json", $dom_table);
($status, $report, $error) = invoke($dom);
check($status == 0 && $report->{observed_classified_sites} == 2
    && $report->{inherited_recovery_unique_diagnostics} == 1, "DOM schema retains primary/recovery split: $error");
$dom->{observations}[1]{stderr} = captured("$diagnostics[1]\n$duplicate\n$diagnostics[3]\n");
($status, $report, $error) = invoke($dom);
check($status == 0 && $report->{unclassified_unique_diagnostics} == 1,
    'same-coordinate duplicate without its former primary is not hidden');

# Sōzu de-duplicates complete located diagnostics across workspace roots while
# retaining each root that reported the site and its pre-existing unknown rows.
write_file("$temporary/source/src/other.rs", $source);
push @{$manifest->{files}}, { path => 'src/other.rs', bytes => length($source), sha256 => sha256_hex($source) };
push @{$manifest->{aliases}}, { from => 'src/other.rs', to => 'src/other.lkj' };
write_json("$temporary/projection.json", $manifest);
my $sozu = clone($receipt);
$sozu->{projection_manifest_sha256} = sha256_hex(bytes("$temporary/projection.json"));
$sozu->{verification_before}{manifest_sha256} = $sozu->{projection_manifest_sha256};
$sozu->{verification_after}{manifest_sha256} = $sozu->{projection_manifest_sha256};
$sozu->{observations}[1]{stderr} = captured("$diagnostics[0]\n$diagnostics[1]\n");
for my $observation (@{clone($sozu->{observations})}) {
    $observation->{upstream_entry} = 'src/other.rs';
    $observation->{command}[2] = $observation->{route} eq 'original_suffix' ? 'src/other.rs' : 'src/other.lkj';
    push @{$sozu->{observations}}, $observation;
}
write_json("$temporary/before.json", $sozu);
my $sozu_table = { current_verification => { receipt_sha256 => sha256_hex(bytes("$temporary/before.json")) },
    groups => {
        impl_trait => { owner => 'Language', rank => 1, diagnosed_sites => 1, independent_gap_sites => 1,
            diagnostic_kinds => ['E_UNSUPPORTED: unsupported construct: impl'],
            sites => [['src/lib.rs', 1, 11, 0, 'src/lib.rs']] },
        parse_recovery_or_unclassified => { owner => 'Unknown', rank => 2, diagnosed_sites => 1, independent_gap_sites => 0,
            diagnostic_kinds => ['E_PARSE: expected an identifier'],
            sites => [['src/lib.rs', 2, 9, 0, 'src/lib.rs']] },
    } };
write_json("$temporary/baseline.json", $sozu_table);
($status, $report, $error) = invoke($sozu);
check($status == 0, "Sozu schema and repeated-root diagnostics: $error");
check($report->{raw_projected_diagnostic_count} == 4 && $report->{observed_classified_sites} == 1
    && $report->{observed_prior_unclassified_sites} == 1, 'Sozu raw rows, classified and prior unknown counts stay distinct');
check(@{$report->{ranked_observed_causes}[0]{sites}[0]{observed_roots}} == 2,
    'deduplicated site retains both observing roots');
print JSON::PP->new->canonical->encode({ passed => JSON::PP::true, checks => $checks }), "\n";
