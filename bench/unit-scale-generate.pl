#!/usr/bin/env perl
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use File::Spec;
use File::Path qw(make_path);
my ($action, $directory, $requested, $maximum) = @ARGV;
die "usage: unit-scale-generate.pl generate NEW_DIRECTORY LINES [MAXIMUM_UNIT_LINES] | edit DIRECTORY\n"
    unless defined($action) && defined($directory);
my $json = JSON::PP->new->canonical->pretty;
sub read_file {
    my ($path) = @_;
    open my $file, '<:raw', $path or die "$path: $!";
    local $/; my $value = <$file>; close $file; return $value;
}
sub write_file {
    my ($path, $value) = @_;
    open my $file, '>:raw', $path or die "$path: $!";
    print {$file} $value; close $file or die "$path: $!";
}
if ($action eq 'edit') {
    my $metadata = $json->decode(read_file("$directory/workload.json"));
    die "unexpected edit path\n" unless $metadata->{edit_file} eq 'units/leaf00000.lkj';
    my $path = "$directory/$metadata->{edit_file}";
    die "symlink edit refused\n" if -l $path;
    my $bytes = read_file($path);
    die "body edit input is stale\n" unless sha256_hex($bytes) eq $metadata->{edit_before_sha256};
    my $count = ($bytes =~ s/let bias = 7i64;/let bias = 8i64;/);
    die "edit literal absent\n" unless $count == 1;
    write_file($path, $bytes);
    print $json->encode({ before_sha256 => $metadata->{edit_before_sha256},
        after_sha256 => sha256_hex($bytes), expected_stdout => "$metadata->{expected_edited}\n",
        changed_files => 1, changed_bodies => 1, changed_lines => 1 });
    exit 0;
}
die "unknown action\n" unless $action eq 'generate';
die "positive integer line count of at least 100 required\n"
    unless defined($requested) && $requested =~ /^[1-9][0-9]*$/ && $requested >= 100;
die "generation destination already exists\n" if -e $directory || -l $directory;
$maximum = 10_000 unless defined $maximum;
die "maximum unit line count of at least 100 required\n"
    unless $maximum =~ /^[1-9][0-9]*$/ && $maximum >= 100;
make_path("$directory/units");
my $leaves = int(($requested + $maximum - 1) / $maximum);
my $joins = $leaves - 1;
my $entry_lines = 5;
my $leaf_total = $requested - $joins * 3 - $entry_lines;
my $base = int($leaf_total / $leaves);
my $surplus = $leaf_total % $leaves;
my @units;
my @edges;
my @files;
my @current;
my $source_bytes = 0;
my $source_lines = 0;
my $largest = 0;
my $function_count = 0;
my $expected = 0;
my $edited = 0;
my $edit_hash;
my @prefix = (
    'pub enum Token {',
    '    Empty,',
    '    Number(i64),',
    '    Pair(i64, i64),',
    '}',
    '#[derive(Clone, Copy)]',
    'pub struct Pair<T: Copy> {',
    '    pub left: T,',
    '    pub right: T,',
    '}',
    'pub enum Chain {',
    '    End,',
    '    Link(i64, Box<Chain>),',
    '}',
    'pub fn copy_value<T: Copy>(value: T) -> T {',
    '    value',
    '}',
    'fn decode(token: Token) -> i64 {',
    '    match token {',
    '        Token::Empty => 0i64,',
    '        Token::Number(value) => value,',
    '        Token::Pair(left, right) => left + right,',
    '    }',
    '}',
    'fn sum(chain: &Chain) -> i64 {',
    '    match chain {',
    '        Chain::End => 0i64,',
    '        Chain::Link(value, next) => *value + sum(&**next),',
    '    }',
    '}',
);
sub save_unit {
    my ($name, $lines) = @_;
    my $path = "units/$name.lkj";
    my $bytes = join("\n", @$lines) . "\n";
    write_file("$directory/$path", $bytes);
    push @units, "$name=$path";
    push @files, {path=>$path, lines=>scalar(@$lines), bytes=>length($bytes), sha256=>sha256_hex($bytes)};
    $source_lines += scalar(@$lines);
    $source_bytes += length($bytes);
    $largest = scalar(@$lines) if scalar(@$lines) > $largest;
    return sha256_hex($bytes);
}
sub value {
    my ($seed, $count, $extra, $changed) = @_;
    my $number = $seed;
    for my $index (0 .. $count - 1) {
        $number %= 1009;
        $number += 3 if $number % 2;
        my $bias = ($changed && $index == 0) ? 8 : 7;
        for my $step (0 .. 2) { $number = ($number + $step + $bias) % 1009; }
    }
    $number = ($number + 13 * $extra) % 1009;
    return $number;
}
for my $leaf (0 .. $leaves - 1) {
    my $name = sprintf('leaf%05d', $leaf);
    my $budget = $base + ($leaf < $surplus ? 1 : 0);
    my $count = int(($budget - scalar(@prefix) - 5) / 14);
    die "unit too small\n" if $count < 1;
    my $extra = $budget - scalar(@prefix) - 5 - 14 * $count;
    my @lines = @prefix;
    for my $function (0 .. $count - 1) {
        my $fn = sprintf('f%05d', $function);
        my $prior = sprintf('f%05d', $function - 1);
        my @body = (
            "fn $fn(input: i64) -> i64 {",
            $function ? "    let input = $prior(input);" : '    let input = input % 1009i64;',
            '    let pair = Pair { left: input, right: 3i64 };',
            '    let token = if input % 2i64 == 0i64 { Token::Number(pair.left) } else { Token::Pair(pair.left, pair.right) };',
            '    let seed = decode(token);',
            '    let mut value = copy_value(seed);',
            '    let bias = 7i64;',
            '    let mut step = 0i64;',
            '    while step < 3i64 {',
            '        value = (value + step + bias) % 1009i64;',
            '        step += 1i64;',
            '    }',
        );
        if ($function == $count - 1) {
            push @body, ('    value = (value + 13i64) % 1009i64;') x $extra;
        }
        push @body, '    value', '}';
        push @lines, @body;
    }
    my $last = sprintf('f%05d', $count - 1);
    my $seed = $leaf % 97;
    push @lines, 'pub fn value() -> i64 {',
        "    let chain = Chain::Link(${seed}i64, Box::new(Chain::End));",
        '    let start = sum(&chain);', "    $last(start)", '}';
    die "leaf line accounting differs\n" unless @lines == $budget;
    my $hash = save_unit($name, \@lines);
    $edit_hash = $hash if $leaf == 0;
    push @current, $name;
    $function_count += $count + 4;
    my $baseline = value($seed, $count, $extra, 0);
    $expected += $baseline;
    $edited += $leaf == 0 ? value($seed, $count, $extra, 1) : $baseline;
}
my $join_index = 0;
my $depth = 0;
while (@current > 1) {
    my @next;
    while (@current) {
        my $left = shift @current;
        if (!@current) { push @next, $left; last; }
        my $right = shift @current;
        my $name = sprintf('join%05d', $join_index++);
        my @lines = ('pub fn value() -> i64 {',
            "    crate::$left\::value() + crate::$right\::value()", '}');
        save_unit($name, \@lines);
        push @edges, "$name=$left", "$name=$right";
        push @next, $name;
        ++$function_count;
    }
    @current = @next;
    ++$depth;
}
my $root = $current[0];
my @entry = ('pub fn main(mut world: World) {',
    "    let result = crate::$root\::value();",
    '    let message = format!("{result}\\n");',
    '    world.stdout().write(message.as_bytes());', '}');
save_unit('entry', \@entry);
push @edges, "entry=$root";
++$function_count;
die "total source lines differ\n" unless $source_lines == $requested;
my @manifest = ('const UNITS: &[&str] = &[',
    (map {qq{    "$_",}} @units), '];', 'const DEPENDS: &[&str] = &[',
    (map {qq{    "$_",}} @edges), '];', 'const ENTRY: &str = "entry::main";');
write_file("$directory/units.lkj", join("\n", @manifest) . "\n");
my $metadata = {
    generator_version=>2, maximum_unit_lines=>$maximum, workload=>'connected balanced source-unit DAG; explicit package-boundary workload',
    source_lines=>$source_lines, source_bytes=>$source_bytes, manifest_lines=>scalar(@manifest),
    units=>scalar(@units), leaf_units=>$leaves, linking_units=>$join_index+1,
    largest_unit_lines=>$largest, maximum_direct_dependencies=>($leaves > 1 ? 2 : 1), dependency_depth=>$depth+1,
    source_function_definitions=>$function_count,
    forms=>['ordinary cross-unit calls','generic Copy functions and aggregates','enums and matches',
        'Box-indirected recursive nominal types','reference recursion','bounded arithmetic loops'],
    expected_baseline=>$expected, expected_edited=>$edited, edit_file=>'units/leaf00000.lkj',
    edit_before_sha256=>$edit_hash, files=>\@files,
};
write_file("$directory/workload.json", $json->encode($metadata));
print $json->encode({ map {$_=>$metadata->{$_}} grep {$_ ne 'files'} keys %$metadata });
