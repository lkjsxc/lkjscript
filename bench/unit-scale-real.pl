#!/usr/bin/env perl
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use File::Path qw(make_path);
my ($action, @arguments) = @ARGV;
my $json = JSON::PP->new->canonical->pretty;
sub read_file {
    my ($path) = @_;
    open my $file, '<:raw', $path or die "$path: $!";
    local $/; my $bytes = <$file>; close $file; return $bytes;
}
sub file_hash {
    my ($path) = @_;
    open my $file, '<:raw', $path or die "$path: $!";
    return Digest::SHA->new(256)->addfile($file)->hexdigest;
}
sub write_file {
    my ($path, $bytes) = @_;
    open my $file, '>:raw', $path or die "$path: $!";
    print {$file} $bytes; close $file or die "$path: $!";
}
if (defined($action) && $action eq 'verify-output') {
    @arguments == 4 or die "verify-output EXPECTED OBSERVED STDERR IMAGE\n";
    my ($expected, $observed, $stderr, $image) = @arguments;
    my $oracle = read_file($expected);
    my @rows = split /\n/, $oracle;
    @rows == 7239 && $oracle =~ /\n\z/ or die "wrong oracle row count\n";
    for my $index (0 .. $#rows) {
        $rows[$index] =~ /^\Q$index\E:[0-9]+$/ or die "invalid oracle row $index\n";
    }
    my $actual = read_file($observed);
    $actual eq $oracle or die "original unit specimen differs from the corpus oracle\n";
    read_file($stderr) eq '' or die "original unit specimen wrote to stderr\n";
    print $json->encode({
        schema=>1, specimen=>'levenshtein-original-units', corpus_rows=>scalar(@rows),
        output_bytes=>length($actual), expected_sha256=>sha256_hex($oracle),
        observed_sha256=>sha256_hex($actual), image_sha256=>file_hash($image),
        exact_output=>JSON::PP::true, empty_stderr=>JSON::PP::true,
    });
    exit 0;
}
defined($action) && $action eq 'prepare' && @arguments == 2
    or die "unit-scale-real.pl prepare REPOSITORY NEW_DIRECTORY | verify-output EXPECTED OBSERVED STDERR IMAGE\n";
my ($repository, $directory) = @arguments;
die "real specimen destination already exists\n" if -e $directory || -l $directory;
my $source = "$repository/examples/real_programs/levenshtein";
my $manifest = decode_json(read_file("$source/upstream.json"));
my $projection = read_file("$source/upstream.lkj");
my $boundary = "\n// UPSTREAM INTEGRATION TESTS: exact bodies, package header removed.\n";
my @parts = split /\Q$boundary\E/, $projection, -1;
@parts == 2 or die "invalid integration boundary\n";
my ($library, $tests) = @parts;
my $restored = $library =~ s/\Qstd::iter::range_from(1)\E/(1..)/g;
$restored == 1 or die "invalid inverse range count\n";
my $original_sha = sha256_hex($library);
$original_sha eq '6f295e027783399df21c081465b2ebc2d3a499895635a839c1c00aae4455f267'
    && $original_sha eq $manifest->{files}{'src/lib.rs'}
    or die "original library bytes differ\n";
my $library_lines = () = $library =~ /\n/g;
$library_lines == 65 && $library_lines == $manifest->{source_lines}
    or die "original library line count differs\n";
my $original_tests = "extern crate levenshtein;\nuse levenshtein::levenshtein;\n" . $tests;
my $tests_sha = sha256_hex($original_tests);
$tests_sha eq '88c8be4758d544673107a4b932e5c00cef6cd7972dc7f9293ccfaa4e5e672665'
    && $tests_sha eq $manifest->{files}{'tests/lib.rs'}
    or die "original test bytes differ\n";
my $license = read_file("$source/LICENSE");
my $license_sha = sha256_hex($license);
$license_sha eq 'ca4662cb5d1b738fbe5350c0d5485ba11773b4b7208974082ae6e129a52d631d'
    && $license_sha eq $manifest->{files}{license}
    or die "upstream license bytes differ\n";
$manifest->{name} eq 'levenshtein' && $manifest->{version} eq '1.0.5'
    && $manifest->{commit} eq '9c4730b1973d4e61e187f8fe6d5f299ad5c991fc'
    or die "unexpected upstream identity\n";
my $corpus = read_file("$source/corpus.lkj");
my $app = "pub fn main(mut output: Stdout) {\n"
    . "    let values = crate::corpus::output();\n"
    . "    output.write(values.as_bytes());\n}\n";
my $unit_manifest = <<'SOURCE';
const UNITS: &[&str] = &[
    "upstream=upstream.lkj",
    "corpus=corpus.lkj",
    "app=app.lkj",
];
const DEPENDS: &[&str] = &[
    "corpus=upstream",
    "app=corpus",
];
const ENTRY: &str = "app::main";
SOURCE
my @files;
my ($lines, $bytes, $largest, $functions) = (0,0,0,0);
make_path($directory);
for my $entry (['upstream.lkj',$library], ['corpus.lkj',$corpus], ['app.lkj',$app]) {
    my ($path, $contents) = @$entry;
    write_file("$directory/$path", $contents);
    my $count = () = $contents =~ /\n/g;
    push @files, {path=>$path, lines=>$count, bytes=>length($contents), sha256=>sha256_hex($contents)};
    $lines += $count; $bytes += length($contents);
    $largest = $count if $count > $largest;
    $functions += () = $contents =~ /^\s*(?:pub\s+)?fn\s+/mg;
}
write_file("$directory/units.lkj", $unit_manifest);
write_file("$directory/LICENSE", $license);
my $oracle_path = 'tests/fixtures/real_programs_levenshtein.output.gz';
my $metadata = {
    generator_version=>1, workload=>'original upstream algorithm with separate corpus and entry units',
    source_lines=>$lines, source_bytes=>$bytes, manifest_lines=>scalar(() = $unit_manifest =~ /\n/g),
    units=>3, leaf_units=>1, linking_units=>2, largest_unit_lines=>$largest,
    maximum_direct_dependencies=>1, dependency_depth=>2, source_function_definitions=>$functions,
    files=>\@files,
    upstream=>{
        name=>$manifest->{name}, version=>$manifest->{version}, origin=>$manifest->{origin},
        commit=>$manifest->{commit}, original_library_sha256=>$original_sha,
        original_library_lines=>$library_lines, original_tests_sha256=>$tests_sha,
        license_sha256=>$license_sha, algorithm_adaptations=>0,
        source_projection=>'exact src/lib.rs bytes in upstream.lkj; only extension and module boundary differ',
    },
    historical_projection=>{
        path=>'examples/real_programs/levenshtein/upstream.lkj', sha256=>sha256_hex($projection),
        receipt_sha256=>file_hash("$source/upstream.json"),
        normalized_range_expressions=>1, removed_integration_header_lines=>2,
        usage=>'read-only provenance input; not compiled in this unit cohort',
    },
    adapters=>{
        corpus=>'byte-for-byte existing corpus.lkj; same crate::upstream::levenshtein import',
        corpus_sha256=>sha256_hex($corpus),
        entry=>'four-line public Stdout entry calling crate::corpus::output',
        unit_manifest=>'three named units and two dependency edges',
        upstream_tests=>'complete upstream hash verified; executed by the separate real-program gate',
    },
    oracle=>{path=>$oracle_path, archive_sha256=>file_hash("$repository/$oracle_path"), rows=>7239},
};
write_file("$directory/workload.json", $json->encode($metadata));
print $json->encode($metadata);
