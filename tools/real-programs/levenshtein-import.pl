#!/usr/bin/env perl
use strict;
use warnings;
use JSON::PP;
use Digest::SHA qw(sha256_hex);
sub read_bytes { my ($path)=@_; open my $f, '<:raw', $path or die "$path: $!\n"; local $/; return <$f>; }
@ARGV == 3 or die "usage: levenshtein-import.pl UPSTREAM MANIFEST REQUEST\n";
my ($upstream,$manifest_path,$request_path)=@ARGV;
my $manifest=decode_json(read_bytes($manifest_path));
for my $name (sort keys %{$manifest->{files}}) {
    my $bytes=read_bytes("$upstream/$name");
    sha256_hex($bytes) eq $manifest->{files}{$name} or die "upstream digest mismatch: $name\n";
}
my $algorithm=read_bytes("$upstream/src/lib.rs");
my $ranges=($algorithm =~ s/\(1\.\.\)/std::iter::range_from(1)/g);
$ranges == 1 or die "expected one range-value producer gap\n";
my $tests=read_bytes("$upstream/tests/lib.rs");
$tests =~ s/\Aextern crate levenshtein;\nuse levenshtein::levenshtein;\n// or die "unrecognized integration header\n";
my @names=($tests =~ /#\[test\]\nfn ([a-z_]+)\(\)/g);
join("\0",@names) eq join("\0",@{$manifest->{test_names}}) or die "test inventory changed\n";
my $projection=$algorithm."\n// UPSTREAM INTEGRATION TESTS: exact bodies, package header removed.\n".$tests;
my $request={version=>1,operations=>[{op=>"add-item",id=>"crate",source=>"mod upstream;",projection=>{path=>"examples/real_programs/levenshtein/upstream.lkj",source=>$projection}}]};
open my $f, '>:raw', $request_path or die "$request_path: $!\n";
print {$f} JSON::PP->new->canonical->pretty->encode($request);
close $f or die "$request_path: $!\n";
print "verified upstream: 65 algorithm lines, 8 unchanged test bodies; range-expression=1, integration-header-lines=2\n";
