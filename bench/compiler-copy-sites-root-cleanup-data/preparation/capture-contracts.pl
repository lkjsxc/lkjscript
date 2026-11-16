use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use Fcntl qw(O_WRONLY O_CREAT O_EXCL);
my$out='/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116/target/compiler-copy-sites-root-inventory-20261116';
my$repo='/home/coder/workspace/lkjscript';
my$pin='416da03c86b6073832c9b2cdbb319ff8a3b8c991';
sub read_json {open my$f,'<',$_[0]or die$!;local$/;return decode_json(<$f>);}
sub blob {my($p)=@_;open my$f,'-|','git','-C',$repo,'show',"$pin:$p"or die$!;binmode$f;local$/;my$v=<$f>//'';close$f or die"Git read $p";return$v;}
my@records;
for my$p(qw(tools/check.sh tools/http-test.sh bench/compiler-copy-sites.md bench/compiler-copy-sites-data/gate/logs/http.log bench/compiler-copy-sites-data/gate/logs/details-http.log bench/compiler-origin-container-integration-data/local/suite-logs/http.log bench/compiler-origin-container-integration-data/local/suite-logs/details-http.log)){
 my$v=blob($p);my@n=split/\n/,$v;my@excerpt;
 for my$i(0..$#n){
  my$line=$n[$i];
  my$select=$p eq'tools/check.sh'?$line=~/target\/gate|profile|generated/:
   $p eq'tools/http-test.sh'?($i+1>=199&&$i+1<=224):
   $p eq'bench/compiler-copy-sites.md'?(($i+1>=140&&$i+1<=154)||($i+1>=182&&$i+1<=188)||($i+1>=198&&$i+1<=215)):
   $line=~/HTTP (?:memory|profile)|ok http|warmup/;
  push@excerpt,{line=>$i+1,text=>$line}if$select;
 }
 push@records,{reference=>"$pin:$p",sha256=>(sha256_hex($v)),bytes=>length($v),excerpt=>\@excerpt};
}
my$recovery=read_json("$out/proposed-cleanup/recovery.json");
my@logs=grep{$_->{relative}=~m{^gate/[^/]+\.log$}}@$recovery;
die"current gate top-level log retention"unless@logs==114;
my$disposable=read_json("$out/proposed-cleanup/disposable.json");
my@http=grep{$_->{kind}eq'disposable_http_test_product'}@$disposable;
my(%tokens,@matched);
for my$r(@records){
 next unless$r->{reference}=~/details-http\.log$/;
 for my$e(@{$r->{excerpt}}){
  $tokens{$1}=$r->{reference}while$e->{text}=~m{target/gate/(http-profiles/http\.(?:memory|samples)\.[0-9]+)}g;
 }
}
for my$r(@http){my($tail)=$r->{relative}=~m{/(http-profiles/[^/]+)$};next unless$tokens{$tail};push@matched,{%$r,outcome_reference=>$tokens{$tail}};}
my$report={
 inventory_pin=>$pin,contracts=>\@records,current_gate_top_level_logs=>\@logs,
 current_gate_top_level_log_count=>scalar(@logs),http_product_count=>scalar(@http),
 http_products_named_by_retained_outcome=>\@matched,
 accumulated_products_without_individual_outcome_binding=>scalar(@http)-scalar(@matched),
 claim=>'The current and preceding retained HTTP gate logs name their output pairs and record test outcomes. Remaining older accumulated HTTP files are disposable test products under the same named output contract; no separate historical success or recoverable raw bytes is claimed for each older PID.',
 source_or_artifact_executed=>JSON::PP::false,deleted=>JSON::PP::false,
};
sysopen(my$f,"$out/proposed-cleanup/test-product-contracts.json",O_WRONLY|O_CREAT|O_EXCL,0600)or die$!;
print$f JSON::PP->new->canonical->pretty->encode($report);close$f or die$!;
print JSON::PP->new->canonical->pretty->encode({retained_top_level_logs=>scalar(@logs),http_products=>scalar(@http),named_by_retained_outcome=>scalar(@matched),unbound_older_products=>scalar(@http)-scalar(@matched),contract_files=>scalar(@records)});
