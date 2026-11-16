use strict;use warnings;use Digest::SHA qw(sha256_hex);
sub raw {open my $f,'<:raw',shift or die $!;local $/;return <$f>;}
my $core=raw('bench/compiler-query-v2.sh');my $supplemental=raw('bench/compiler-query-kinds.sh');
my($left)=$core=~/done <<'QUESTIONS'\n(.*?)\nQUESTIONS/s;
my($right)=$supplemental=~/done <<'QUESTIONS'\n(.*?)\nQUESTIONS/s;
die "historical lists absent\n" unless defined($left)&&defined($right);
my @core=split /\n/,$left;my @supp=split /\n/,$right;
die "historical populations differ\n" unless @core==20 && @supp==10;
my @expected;my $ordinal=0;
for my $row(@core){my($kind,$id)=split /\t/,$row;push @expected,join("\t",++$ordinal,'owner',$kind,$id,1,96,32768),join("\t",++$ordinal,'slice','slice',$id,1,96,32768);}
for my $row(@supp){my($kind,$id)=split /\t/,$row;push @expected,join("\t",++$ordinal,'supplemental',$kind,$id,2,96,32768);}
my $expected="ordinal\tgroup\tkind\tidentity\tdepth\tlimit\tbytes\n".join("\n",@expected)."\n";
my $actual=raw('bench/compiler-query-current-questions.tsv');die "question/order/bounds differ\n" unless $expected eq $actual;
print "50 rows equal historical 20 paired questions plus ten supplemental kinds\n";
print "questions_sha256=",sha256_hex($actual),"\n";
print "compiler_invocations=0 query_invocations=0\n";
