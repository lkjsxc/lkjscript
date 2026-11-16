use strict;
use warnings;
use JSON::PP;
use Digest::SHA qw(sha256_hex);
my $out='/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116/target/compiler-legacy-root-inventory-20261116';
my @names=qw(continuation-landing.log results-landing.log replay-pin-landing.log);
my @records;
for my $name (@names) {
    my $path="/home/coder/workspace/lkjscript/target/compiler-current50-root-audit/$name";
    open my $f,'<:raw',$path or die "$path: $!";
    local $/;
    my $bytes=<$f>;
    close $f or die $!;
    my %classes;
    my (@headers,@others);
    my $line_number=0;
    for my $line (split /\n/,$bytes) {
        $line_number++;
        if ($line =~ /^\[main [0-9a-f]{8}\] .+$/) {
            $classes{commit_header}++; push @headers,$line;
        } elsif ($line =~ /^ Date: [A-Za-z]{3} [A-Za-z]{3} [0-9]{1,2} [0-9:]{8} [0-9]{4} [+-][0-9]{4}$/) {
            $classes{date}++;
        } elsif ($line =~ /^ [0-9]+ files? changed, [0-9]+ insertions?\(\+\)(?:, [0-9]+ deletions?\(-\))?$/) {
            $classes{summary}++;
        } elsif ($line =~ /^ create mode [0-7]{6} (bench\/[A-Za-z0-9._\/-]+)$/) {
            $classes{created_path}++;
        } elsif ($line eq '') {
            $classes{blank}++;
        } else {
            push @others,{line=>$line_number,text=>$line};
        }
    }
    push @records,{
        path=>$path,bytes=>length($bytes),sha256=>sha256_hex($bytes),
        classes=>\%classes,headers=>\@headers,total_lines=>$line_number,
        unclassified=>\@others
    };
}
my $json=JSON::PP->new->canonical->pretty->encode(\@records);
open my $f,'>:raw',"$out/transcript-review-v2.json" or die $!;
print $f $json; close $f or die $!;
print $json;
