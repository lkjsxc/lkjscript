use strict;
use warnings;
use Fcntl qw(:DEFAULT :mode);
use IPC::Open2;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
my $ROOT='/home/coder/workspace/lkjscript';
my $OUT="$ROOT/target/compiler-reference-capture-commit-acceptance-1";
my $PRIOR="$ROOT/target/compiler-reference-capture-adoption-acceptance-1";
my $BASE='0b0b27803d27b932a5b91cde2d766bd3c8bb82b9';
my $PARENT='d03ad66bf672b3338ec7dd073560cf599e182303';
my $COMMIT='bd8a9050a3786de9f227c6c4037f5acd43205f71';
my $SRC='626c9c1f234f4d003a08820d9b69840535eff8b8';
my $LIB='25bdb8ff8567b941790d3951c78e896b23e26e8e';
my $PROGRAMS='235aa6c283d334688289ab73e472cae920d25b1e8cbad76cb4461f30d18cc9f4';
my $J=JSON::PP->new->canonical->pretty->utf8;
my (@commands,@input_pins);
$|=1;
sub save {
 my($rel,$bytes)=@_;
 sysopen my$f,"$OUT/$rel",O_WRONLY|O_CREAT|O_EXCL,0600 or die "create $rel: $!\n";
 binmode$f;print{$f}$bytes or die$!;close$f or die$!;
}
sub raw {
 my($path)=@_;my@s=lstat$path;die"not regular $path\n"unless@s&&S_ISREG($s[2]);
 die"oversized input $path\n"if$s[7]>16777216;
 sysopen my$f,$path,O_RDONLY|O_NOFOLLOW or die"$path: $!\n";binmode$f;local$/;my$b=<$f>;close$f or die$!;
 $b=''unless defined$b;die"size changed $path\n"unless length($b)==$s[7];return$b;
}
sub pinned {
 my($path,$expected)=@_;my$b=raw($path);die"input pin $path\n"unless sha256_hex($b)eq$expected;
 push@input_pins,{path=>$path,bytes=>length($b),sha256=>$expected};return$b;
}
sub git {
 my(@args)=@_;my@cmd=('/usr/bin/git','-C',$ROOT,@args);
 open my$f,'-|',@cmd or die"git open: $!\n";binmode$f;local$/;my$b=<$f>;close$f or die"git command failed (@args): $?\n";
 $b=''unless defined$b;push@commands,{argv=>\@cmd,exit=>0,stdout_sha256=>sha256_hex($b),stdout_bytes=>length($b)};return$b;
}
sub same {my($a,$b,$label)=@_;die"$label mismatch\n"unless$a eq$b;}
sub manifest {
 my($b)=@_;my%h;for my$l(split/\n/,$b){die"manifest framing\n"unless$l=~/\A([0-9a-f]{64})  ((?:src|library|tests)\/[A-Za-z0-9_.\/-]+)\z/;die"duplicate manifest path\n"if exists$h{$2};$h{$2}=$1;}return\%h;
}
my$adoption_bytes=pinned("$PRIOR/acceptance.json",'e0d850b5f8bfea5b14573e694cd2ee64fe5682b1548138daa3a158f32e93729c');
my$final_bytes=pinned("$PRIOR/live-actual.sha256",'3bcdcb955f65b037311343b6fedf02384a39546c6a2711a6e7a926fdda43a45f');
pinned("$PRIOR/manifest.sha256",'60b8d81c0c5d1bca452c65a65689f5a0a16d76b0383b9e0868feab648ed2954a');
my$adoption=decode_json($adoption_bytes);my$final=manifest($final_bytes);
die"prior population\n"unless keys(%$final)==1743&&$adoption->{final_population}==1743;
same(git('show',"$COMMIT:bench/compiler-reference-capture-data/adoption/acceptance.json"),$adoption_bytes,'committed original adoption acceptance');
same(git('show',"$COMMIT:bench/compiler-reference-capture-data/adoption/live-actual.sha256"),$final_bytes,'committed adopted source inventory');
same(git('show',"$COMMIT:bench/compiler-reference-capture-data/adoption/manifest.sha256"),raw("$PRIOR/manifest.sha256"),'committed original adoption manifest');
same(git('rev-parse',"$COMMIT^"),"$PARENT\n",'exact commit parent');
same(git('rev-parse',"$COMMIT:src"),"$SRC\n",'committed src tree');
same(git('rev-parse',"$COMMIT:library"),"$LIB\n",'committed library tree');
my%trees;
for my$area(qw(src library tests)){
 my$b=git('rev-parse',"$BASE:$area");my$p=git('rev-parse',"$PARENT:$area");my$c=git('rev-parse',"$COMMIT:$area");
 same($b,$p,"baseline/immediate-parent $area tree");
 same($b,$c,"unchanged $area tree")if$area ne'src';
 chomp($b,$p,$c);$trees{$area}={baseline=>$b,parent=>$p,committed=>$c};
}
my$tree=git('ls-tree','-r','-z','--full-tree',$COMMIT,'--','src','library','tests');
save('git-source-tree.raw',$tree);
my%blobs;my%groups;
for my$r(split/\0/,$tree){
 die"Git tree framing\n"unless$r=~/\A(100644|100755) blob ([0-9a-f]{40})\t((?:src|library|tests)\/[A-Za-z0-9_.\/-]+)\z/;
 die"duplicate Git source path\n"if exists$blobs{$3};$blobs{$3}=$2;
}
same(join("\0",sort keys%blobs),join("\0",sort keys%$final),'all committed source paths');
my($reader,$writer);my$pid=open2($reader,$writer,'/usr/bin/git','-C',$ROOT,'cat-file','--batch');
binmode$reader;binmode$writer;my$previous=select$writer;$|=1;select$previous;
sub take {my($f,$n)=@_;my$b='';while(length($b)<$n){my$r=read($f,my$v,$n-length($b));die"batch read\n"unless defined$r&&$r;$b.=$v;}return$b;}
my$git_manifest='';my$total_source_bytes=0;
for my$p(sort keys%blobs){
 print{$writer}"$blobs{$p}\n"or die"batch input\n";my$h=<$reader>;
 die"batch header\n"unless defined$h&&$h=~/\A([0-9a-f]{40}) blob ([0-9]+)\n\z/&&$1 eq$blobs{$p};
 my$n=$2;die"blob cap $p\n"if$n>16777216;my$b=take($reader,$n);same(take($reader,1),"\n",'batch framing');
 my$sha=sha256_hex($b);same($sha,$final->{$p},"committed source $p");
 $git_manifest.="$sha  $p\n";$total_source_bytes+=$n;my($area)=$p=~/\A([^\/]+)/;$groups{$area}++;
}
close$writer or die"batch input close\n";close$reader or die"batch output close\n";waitpid($pid,0);die"batch status $?\n"if$?;
push@commands,{argv=>['/usr/bin/git','-C',$ROOT,'cat-file','--batch'],exit=>0,objects=>scalar(keys%blobs)};
same($git_manifest,$final_bytes,'complete committed source SHA-256 inventory');
save('committed-source.sha256',$git_manifest);
print"1,743 adopted source/library/test records match exact committed Git blobs\n";
my$line=git('diff','--numstat','--no-renames','-z',$PARENT,$COMMIT,'--','*.lkj');
save('committed-language-numstat.raw',$line);
my$base_line=git('diff','--numstat','--no-renames','-z',$BASE,$COMMIT,'--','*.lkj');
save('baseline-language-numstat.raw',$base_line);same($line,$base_line,'parent/base retained language delta');
my%expected=map{$_->{path}=>$_}@{$adoption->{retained_line_accounting}{files}};
my@changes;my($adds,$deletes)=(0,0);
for my$r(split/\0/,$line){
 die"numstat framing\n"unless$r=~/\A([0-9]+)\t([0-9]+)\t([A-Za-z0-9_.\/-]+\.lkj)\z/;
 my($a,$d,$p)=($1,$2,$3);die"extra language path $p\n"unless exists$expected{$p};
 die"retained line mismatch $p\n"unless$a==$expected{$p}{added}&&$d==$expected{$p}{deleted};
 push@changes,{path=>$p,added=>0+$a,deleted=>0+$d};$adds+=$a;$deletes+=$d;delete$expected{$p};
}
die"missing retained path\n"if keys%expected;
die"retained totals\n"unless@changes==10&&$adds==564&&$deletes==21&&$adoption->{retained_line_accounting}{operation_backed}==585&&$adoption->{requested_operations}==24;
my$all_changed=git('diff','--name-only','--no-renames','-z',$PARENT,$COMMIT,'--','src','library','tests');
same(join("\0",sort split/\0/,$all_changed),join("\0",sort map{$_->{path}}@changes),'no additional source/library/test changes');
my$all_added=git('diff','--name-only','--diff-filter=A','--no-renames','-z',$PARENT,$COMMIT);
my@added_lkj=sort grep{/\.lkj\z/}split/\0/,$all_added;
same(join("\0",@added_lkj),'src/semantic_tests/reference_capture.lkj','sole added language file');
save('changed-language.json',$J->encode(\@changes));
my@programs;my$common;
for my$c($BASE,$PARENT,$COMMIT){
 my$b=git('show',"$c:docs/status.md");my($section)=$b=~/^(## Programs\n.*?)(?=^## |\z)/ms;
 die"Programs section missing $c\n"unless defined$section;same(sha256_hex($section),$PROGRAMS,"Programs section $c");
 $common=$section unless defined$common;same($section,$common,'Programs bytes unchanged');
 push@programs,{commit=>$c,bytes=>length($section),sha256=>$PROGRAMS};
}
die"Programs size\n"unless length($common)==4146;save('Programs-preserved.md',$common);
my$pub_note=git('show',"$COMMIT:bench/compiler-reference-capture-data/source-publication/initial-whitespace-check.md");
my$pub_raw=git('show',"$COMMIT:bench/compiler-reference-capture-data/source-publication/initial-whitespace-check.stdout");
save('initial-whitespace-check.md',$pub_note);
my$canonical=git('show',"$COMMIT:bench/compiler-reference-capture-data/bootstrap/canonical.sha256");
die"captured candidate pin\n"unless$canonical=~/\Ab01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db  [^\n]+\n\z/;
my$result={
 schema=>1,status=>'passed committed-source closure audit',commit=>$COMMIT,parent=>$PARENT,baseline=>$BASE,
 trees=>\%trees,source_records=>scalar(keys%blobs),source_bytes=>$total_source_bytes,source_groups=>\%groups,
 all_adopted_hashes_match_git=>JSON::PP::true,committed_original_adoption_records_match=>JSON::PP::true,
 retained_lines=>{added=>$adds,deleted=>$deletes,total=>$adds+$deletes,operation_backed=>585,requested_operations=>24,paths=>scalar@changes,all_changed_lkj_paths_are_audited=>JSON::PP::true,additional_lkj_evidence_files=>0},
 added_lkj_paths=>\@added_lkj,programs=>\@programs,
 candidate_pin_from_committed_bootstrap_record=>'b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db',
 producer_scope=>'Committed capture metadata only; no current executable or seed read. Seed refresh remains separate.',
 source_publication=>{qualification=>'The preserved first whitespace-check stop concerns immutable raw captures. This audit checks committed blobs and does not rewrite them or reinterpret that stop as a source/compiler failure.',note_sha256=>sha256_hex($pub_note),raw_stdout_sha256=>sha256_hex($pub_raw),raw_stdout_bytes=>length($pub_raw)},
 proof_boundary=>'Closes the prior working-source adoption proof against this exact Git commit. No new compiler/native run, admission or performance claim.',
 no_source_mutation=>JSON::PP::true,no_compiler_or_native=>JSON::PP::true,no_key_or_store_read=>JSON::PP::true,no_compression=>JSON::PP::true
};
save('acceptance.json',$J->encode($result));save('input-pins.json',$J->encode(\@input_pins));save('commands.json',$J->encode(\@commands));
print"PASS commit=$COMMIT retained=585/585 additions=564 deletions=21 paths=10 extra_lkj_evidence=0\n";
print"Programs bytes=4146 sha256=$PROGRAMS unchanged across baseline,parent,commit\n";
