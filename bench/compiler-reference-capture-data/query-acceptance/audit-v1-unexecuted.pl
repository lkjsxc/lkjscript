use strict;
use warnings;
use Fcntl qw(:DEFAULT :mode);
use Digest::SHA qw(sha256_hex);
use JSON::PP;
my$ROOT='/home/coder/workspace/lkjscript';
my$INPUT="$ROOT/target/compiler-reference-capture-query-1";
my$DATA="$INPUT/evidence";
my$OUT="$ROOT/target/compiler-reference-capture-query-acceptance-1";
my$PRODUCER='b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db';
my$HARNESS='b36bf7c4eed7906d573bd769d26de99a572d6f8f78f0a14bf54c3cec3c930162';
my@BUILDS=qw(capture-ordinary capture-from-ordinary capture-cold-before capture-unchanged capture-body-ordinary capture-from-body capture-cold-after);
my@BEFORE=qw(capture-from-ordinary capture-cold-before capture-unchanged);
my@AFTER=qw(capture-from-body capture-cold-after);
my@FAMILIES=(@BEFORE,@AFTER);
my@SUFFIXES=qw(shadowed shadow-type generic outline held-type);
my@CONTROLS=qw(inputs-before.sha256 inputs-after.sha256 argv cwd environment status time start.ns end.ns start.utc end.utc load-before load-after stdout stderr);
my$J=JSON::PP->new->canonical->pretty->utf8;
my(%input_pins,%input_bytes);
$|=1;
sub read_raw {
 my($p)=@_;my@s=lstat$p;die"not regular $p\n"unless@s&&S_ISREG($s[2]);die"input cap $p\n"if$s[7]>16777216;
 sysopen my$f,$p,O_RDONLY|O_NOFOLLOW or die"open $p: $!\n";binmode$f;local$/;my$b=<$f>;close$f or die"close $p: $!\n";$b=''unless defined$b;
 die"input size changed $p\n"unless length($b)==$s[7];$input_pins{$p}={path=>$p,bytes=>length($b),sha256=>sha256_hex($b)};
 $input_bytes{$p}=$b;return$b;
}
sub save {
 my($name,$b)=@_;sysopen my$f,"$OUT/$name",O_WRONLY|O_CREAT|O_EXCL,0600 or die"new output $name: $!\n";
 binmode$f;print{$f}$b or die$!;close$f or die$!;
}
sub jsave{save($_[0],$J->encode($_[1]));}
sub fields {
 my($text)=@_;my%v;pos($text)=0;
 while(pos($text)<length($text)){
  die"field framing: $text\n"unless$text=~/\G([A-Za-z0-9_-]+)=("(?:[^"\\]|\\.)*"|[^ \t]+)(?: |\z)/gc;
  my($k,$raw)=($1,$2);die"duplicate field $k\n"if exists$v{$k};$v{$k}=$raw=~/\A"/?decode_json($raw):$raw;
 }
 return\%v;
}
sub same {my($left,$right,$label)=@_;die"$label differs\n"unless$left eq$right;}
my%control;
for my$n(@CONTROLS){$control{$n}=read_raw("$INPUT/$n");save("raw-controls/$n",$control{$n});}
same($control{'inputs-before.sha256'},$control{'inputs-after.sha256'},'producer/harness before-after');
same($control{'inputs-before.sha256'},"$PRODUCER  target/lkjscript\n$HARNESS  tools/semantic-query-test.sh\n",'captured producer/harness');
same($control{status},"0\n",'suite status');
same($control{cwd},"$ROOT\n",'suite cwd');
die"suite time exit\n"unless$control{time}=~/\bexit_status=0(?:\s|\z)/;
my@argv=split/\0/,$control{argv};die"suite command contract\n"unless@argv>=4&&join("\0",@argv[-3..-1])eq join("\0",'bash','tools/semantic-query-test.sh','target/lkjscript');
opendir my$dir,$DATA or die"evidence directory: $!\n";my@names=readdir$dir;closedir$dir;
my@logs=sort grep{/\Acapture-.*\.log\z/}@names;
same(join("\0",@logs),join("\0",sort map{"$_.log"}@BUILDS),'seven build-log population');
my@reply_names=sort grep{/\Acapture-(?:from-ordinary|cold-before|unchanged|from-body|cold-after)-/}@names;
same(join("\0",@reply_names),join("\0",sort map{my$f=$_;map{"$f-$_"}@SUFFIXES}@FAMILIES),'25 reply population');

my%build;
for my$name(@BUILDS){
 my$raw=read_raw("$DATA/$name.log");my(@stats,@snapshots);
 for my$l(split/\n/,$raw){
  next unless length$l;
  if($l=~/\ALKJSCRIPT_STATS (.+)\z/){push@stats,decode_json($1);}
  elsif($l=~/\ALKJSCRIPT_SNAPSHOT ([0-9a-f]{64}) source=([0-9a-f]{64}) checked-ir=([0-9a-f]{64})\z/){push@snapshots,{snapshot=>$1,source=>$2,checked_ir=>$3};}
  elsif($l=~/\ALKJSCRIPT_STORE_IO (.+)\z/){decode_json($1);}
  else{die"unexpected build log record: $name: $l\n";}
 }
 die"stats row count $name\n"unless@stats==1;
 my$s=$stats[0];die"build terminal state $name\n"unless$s->{route}eq'compiler'&&$s->{stopped_stage}eq'done';
 for my$k(qw(front_definitions front_reused front_functions build_reused checked_reused store_writes)){die"missing integer counter $name $k\n"unless exists$s->{$k}&&$s->{$k}=~/\A\d+\z/;}
 my$is_index=grep{$_ eq$name}@FAMILIES;
 die"snapshot count $name\n"unless@snapshots==($is_index?1:0);
 if($name eq'capture-body-ordinary'){
  die"ordinary changed-body resume\n"unless$s->{front_reused}==3&&$s->{front_definitions}==1&&$s->{front_functions}==1&&$s->{build_reused}==0;
 }elsif($name eq'capture-unchanged'){
  die"unchanged full image reuse\n"unless$s->{build_reused}==1&&$s->{front_definitions}==0&&$s->{front_reused}==0&&$s->{front_functions}==0&&$s->{store_writes}==0;
 }else{
  die"fresh definition inference $name\n"unless$s->{front_definitions}==4&&$s->{front_reused}==0&&$s->{build_reused}==0&&$s->{front_functions}==3;
 }
 if($is_index&&$name ne'capture-unchanged'){
  die"full index phases $name\n"unless exists$s->{semantic_catalog_ms}&&exists$s->{semantic_index_ms};
 }elsif(!$is_index){
  die"ordinary build emitted an index phase $name\n"if exists$s->{semantic_catalog_ms}||exists$s->{semantic_index_ms};
 }
 $build{$name}={name=>$name,stats_rows=>1,stats=>$s,snapshot=>@snapshots?$snapshots[0]:undef,
  compiler_terminal_stage=>'done',independent_build_exit_status=>undef,
  status_boundary=>'No separate per-build exit file was captured. Terminal compiler record plus successful fail-fast suite is retained; no individual exit status is invented.'};
}
my$before_snap=$build{$BEFORE[0]}{snapshot};my$after_snap=$build{$AFTER[0]}{snapshot};
for my$f(@BEFORE){same($J->encode($build{$f}{snapshot}),$J->encode($before_snap),"before build snapshot $f");}
for my$f(@AFTER){same($J->encode($build{$f}{snapshot}),$J->encode($after_snap),"after build snapshot $f");}
for my$k(qw(snapshot source checked_ir)){die"before/after identity unchanged $k\n"if$before_snap->{$k}eq$after_snap->{$k};}
print"seven unique terminal build stats rows and five snapshot records verified\n";

my$before_source=read_raw("$DATA/capture-before.lkj");my$after_source=read_raw("$DATA/capture-after.lkj");
my$expected_after=$before_source;my$replacements=($expected_after=~s/scalar\(1\)/scalar(2)/g);
die"source edit boundary\n"unless$replacements==1&&$expected_after eq$after_source&&length($before_source)==length($after_source);
sub fixture {
 my($source,$literal)=@_;my@l=split/\n/,$source;die"source fixture line count\n"unless@l==4&&$source=~/\n\z/;
 same($l[0],'pub fn main() -> i64 { shadowed() }','main fixture');
 same($l[1],'pub fn scalar(value: i64) -> i64 { value + 1 }','scalar fixture');
 die"shadowing fixture\n"unless$l[2]=~/\Apub fn shadowed\(\) -> i64 \{ let before = scalar\(\Q$literal\E\); \{ let scalar = before; let _ = scalar; \} scalar\(before\) \}\z/;
 same($l[3],'pub fn local_generic<T>(value: T) -> T { let held = value; held }','generic fixture');
 my%p;
 $p{global_first}=index($source,"scalar($literal)");
 $p{global_last}=index($source,'scalar(before)');
 $p{before_inner}=index($source,'let scalar = before;')+length('let scalar = ');
 $p{before_last}=$p{global_last}+length('scalar(');
 $p{shadow_local}=index($source,'let _ = scalar;')+length('let _ = ');
 $p{generic_value}=index($source,'let held = value;')+length('let held = ');
 $p{generic_held}=index($source,'; held }')+2;
 for my$k(keys%p){die"fixture span not found\n"if$p{$k}<0;}
 return\%p;
}
my$before_positions=fixture($before_source,'1');my$after_positions=fixture($after_source,'2');
same($J->encode($before_positions),$J->encode($after_positions),'unchanged token offsets');
my(%reply,%binding_ids);my$total_facts=0;my$total_bytes=0;my$max_bytes=0;my$max_facts=0;
for my$f(@FAMILIES){
 my$is_after=grep{$_ eq$f}@AFTER;my$source=$is_after?$after_source:$before_source;my$positions=$is_after?$after_positions:$before_positions;
 my$snapshot=$build{$f}{snapshot};my%parsed;
 for my$suffix(@SUFFIXES){
  my$name="$f-$suffix";my$raw=read_raw("$DATA/$name");die"reply byte bound $name\n"if length($raw)>32768;
  die"reply trailing newline $name\n"unless$raw=~/\n\z/;my@lines=split/\n/,$raw;my@rows;
  for my$l(@lines){
   die"reply row framing $name\n"unless$l=~/\A("(?:[^"\\]|\\.)*")\tlevel=([a-z-]+)\tfact=([a-z-]+)\t(.*)\z/;
   push@rows,{id=>decode_json($1),level=>$2,fact=>$3,fields=>fields($4)};
  }
  die"reply too short $name\n"unless@rows>=3;my$header=shift@rows;my$footer=pop@rows;
  die"protocol endpoints $name\n"unless$header->{level}eq'protocol'&&$header->{fact}eq'query'&&$footer->{level}eq'protocol'&&$footer->{fact}eq'continuation'&&$footer->{id}eq$header->{id};
  my$h=$header->{fields};my$c=$footer->{fields};my$kind=($suffix eq'shadowed'||$suffix eq'generic')?'references':$suffix eq'outline'?'outline':'type';
  my%expect=(version=>'3',producer=>$PRODUCER,validation=>'authenticated-producing-compiler','current-check'=>'not-performed',
   snapshot=>$snapshot->{snapshot},source=>$snapshot->{source},'checked-ir'=>$snapshot->{checked_ir},kind=>$kind,depth=>'1',
   freshness=>'not-asserted','source-reference-coverage'=>'recorded-sites','instance-coverage'=>'demanded',selection=>'explicit');
  die"query metadata field population $name\n"unless join("\0",sort keys%$h)eq join("\0",sort keys%expect);
  for my$k(keys%expect){die"query metadata $name $k\n"unless$h->{$k}eq$expect{$k};}
  die"continuation metadata fields $name\n"unless join("\0",sort keys%$c)eq join("\0",sort qw(cursor facts pages-read traversal-steps));
  die"query bounds/footer $name\n"unless$c->{cursor}eq'none'&&$c->{facts}=~/\A\d+\z/&&$c->{facts}==@rows&&@rows<=80
   &&$c->{'pages-read'}=~/\A\d+\z/&&$c->{'traversal-steps'}=~/\A\d+\z/;
  for my$r(@rows){die"unexpected fact level $name\n"unless$r->{level}eq'source-claim';}
  my$fact_count=scalar@rows;my$length=length($raw);$total_facts+=$fact_count;$total_bytes+=$length;
  $max_bytes=$length if$length>$max_bytes;$max_facts=$fact_count if$fact_count>$max_facts;
  $parsed{$suffix}={owner=>$header->{id},kind=>$kind,rows=>\@rows,footer=>$c};
  $reply{$name}={name=>$name,family=>$f,suffix=>$suffix,sha256=>sha256_hex($raw),bytes=>$length,facts=>$fact_count,
   owner=>$header->{id},kind=>$kind,snapshot=>$h->{snapshot},source=>$h->{source},checked_ir=>$h->{'checked-ir'},cursor=>$c->{cursor},depth=>1};
 }
 same($parsed{shadowed}{owner},'shadowed',"$f shadow owner");same($parsed{generic}{owner},'local_generic',"$f generic owner");same($parsed{outline}{owner},'local_generic',"$f outline owner");
 my%shadow;my($shadow_id,$before_id,$body_id);
 for my$r(@{$parsed{shadowed}{rows}}){
  die"shadow reference kind\n"unless$r->{fact}eq'references';my$v=$r->{fields};die"shadow reference owner\n"unless$v->{from}eq'shadowed';
  die"reference file spelling\n"unless$v->{file}=~m{\Atarget/semantic-query\.[A-Za-z0-9]+/capture-source/main\.lkj\z};
  die"reference span\n"unless$v->{start}=~/\A\d+\z/&&$v->{end}=~/\A\d+\z/&&$v->{end}>$v->{start}&&$v->{end}<=length($source);
  my$token=substr($source,$v->{start},$v->{end}-$v->{start});my$identity=$r->{id};
  if($identity eq'scalar'){die"global scalar fact\n"unless$v->{kind}eq'value'&&$token eq'scalar';$shadow{"global:$v->{start}"}++;}
  elsif($identity=~/\Ashadowed::body:([0-9a-f]{64})::binding:(before|scalar):0\z/){
   my($digest,$name)=($1,$2);die"shadow local fact\n"unless$v->{kind}eq'local-value'&&$token eq$name;
   $body_id=$digest unless defined$body_id;die"inconsistent shadow body ids\n"unless$body_id eq$digest;
   if($name eq'scalar'){$shadow_id=$identity;}else{$before_id=$identity;}
   $shadow{"$name:$v->{start}"}++;
  }else{die"unexpected shadow target\n";}
 }
 my%shadow_expected=map{$_=>1}("global:$positions->{global_first}","global:$positions->{global_last}","before:$positions->{before_inner}","before:$positions->{before_last}","scalar:$positions->{shadow_local}");
 same($J->encode(\%shadow),$J->encode(\%shadow_expected),"$f exact five shadow references");
 die"shadow type owner\n"unless defined$shadow_id&&$parsed{'shadow-type'}{owner}eq$shadow_id;
 my@st=@{$parsed{'shadow-type'}{rows}};die"shadow i64 type\n"unless@st==1&&$st[0]{id}eq$shadow_id&&$st[0]{fact}eq'inferred-type'&&$st[0]{fields}{value}eq'i64';

 my%generic;my$held_id;
 for my$r(@{$parsed{generic}{rows}}){
  die"generic reference fact\n"unless$r->{fact}eq'references';my$v=$r->{fields};die"generic owner/kind\n"unless$v->{from}eq'local_generic'&&$v->{kind}eq'local-value';
  die"generic reference file\n"unless$v->{file}=~m{\Atarget/semantic-query\.[A-Za-z0-9]+/capture-source/main\.lkj\z};
  die"generic span\n"unless$v->{start}=~/\A\d+\z/&&$v->{end}=~/\A\d+\z/&&$v->{end}>$v->{start}&&$v->{end}<=length($source);
  my$token=substr($source,$v->{start},$v->{end}-$v->{start});
  if($r->{id}eq'local_generic::param:value'){die"generic parameter token\n"unless$token eq'value';$generic{"value:$v->{start}"}++;}
  elsif($r->{id}=~/\Alocal_generic::body:[0-9a-f]{64}::binding:held:0\z/){die"generic held token\n"unless$token eq'held';$held_id=$r->{id};$generic{"held:$v->{start}"}++;}
  else{die"unexpected generic reference identity\n";}
 }
 my%generic_expected=("value:$positions->{generic_value}"=>1,"held:$positions->{generic_held}"=>1);
 same($J->encode(\%generic),$J->encode(\%generic_expected),"$f exact generic references");
 die"held type owner\n"unless defined$held_id&&$parsed{'held-type'}{owner}eq$held_id;
 my@ht=@{$parsed{'held-type'}{rows}};die"held T type\n"unless@ht==1&&$ht[0]{id}eq$held_id&&$ht[0]{fact}eq'inferred-type'&&$ht[0]{fields}{value}eq'T';
 my%outline;
 for my$r(@{$parsed{outline}{rows}}){
  die"outline fact\n"unless$r->{fact}eq'signature';my$v=$r->{fields};die"outline complete/byte count\n"unless$v->{complete}eq'true'&&$v->{bytes}==length($v->{text});
  die"duplicate outline identity\n"if exists$outline{$r->{id}};$outline{$r->{id}}={kind=>$v->{kind},text=>$v->{text},bytes=>0+$v->{bytes}};
 }
 my%outline_expected=($held_id=>{kind=>'binding',text=>'held',bytes=>4},'local_generic::generic:T'=>{kind=>'generic-parameter',text=>'T',bytes=>1},'local_generic::param:value'=>{kind=>'parameter',text=>'value: T',bytes=>8});
 same($J->encode(\%outline),$J->encode(\%outline_expected),"$f exact complete outline facts");
 $binding_ids{$f}={shadowed_scalar=>$shadow_id,shadowed_before=>$before_id,generic_held=>$held_id,shadow_body=>$body_id};
}
my@parity;
for my$suffix(@SUFFIXES){
 for my$i(1..$#BEFORE){my($a,$b)=("$BEFORE[0]-$suffix","$BEFORE[$i]-$suffix");same($input_bytes{"$DATA/$a"},$input_bytes{"$DATA/$b"},"complete before reply $suffix");push@parity,{left=>$a,right=>$b,complete_bytes_equal=>JSON::PP::true};}
 my($a,$b)=("$AFTER[0]-$suffix","$AFTER[1]-$suffix");same($input_bytes{"$DATA/$a"},$input_bytes{"$DATA/$b"},"complete after reply $suffix");push@parity,{left=>$a,right=>$b,complete_bytes_equal=>JSON::PP::true};
}
die"changed shadow identity not reflected\n"if$binding_ids{$BEFORE[0]}{shadow_body}eq$binding_ids{$AFTER[0]}{shadow_body};
same($binding_ids{$BEFORE[0]}{generic_held},$binding_ids{$AFTER[0]}{generic_held},'unchanged generic binding identity');
die"reply/fact counts\n"unless keys(%reply)==25&&$total_facts==60&&@parity==15;
# Recheck only the already selected raw records, with no fixture/store/executable access.
for my$p(sort keys%input_pins){my$b=read_raw($p);die"raw record changed during audit $p\n"unless$b eq$input_bytes{$p};}
my$result={
 schema=>1,status=>'passed bounded raw-record lifecycle audit',producer=>$PRODUCER,harness=>$HARNESS,
 suite_exit_status=>0,suite_time=>$control{time},suite_load_before=>$control{'load-before'},suite_load_after=>$control{'load-after'},
 per_build_status_qualification=>'Seven logs each contain one compiler stats row with stopped_stage=done; separate build exit-status files do not exist. Successful suite status plus its declared fail-fast build helper is retained, not invented per-build exits.',
 build_count=>7,builds=>[map{$build{$_}}@BUILDS],
 source_fixtures=>{before_sha256=>sha256_hex($before_source),after_sha256=>sha256_hex($after_source),before_bytes=>length($before_source),after_bytes=>length($after_source),change=>'Exactly one scalar(1) call becomes scalar(2); other bytes and reference token offsets unchanged.'},
 before_snapshot=>$before_snap,after_snapshot=>$after_snap,
 replies=>[map{$reply{$_}}sort keys%reply],reply_count=>25,total_fact_rows=>$total_facts,total_reply_bytes=>$total_bytes,
 observed_max_facts=>$max_facts,observed_max_reply_bytes=>$max_bytes,
 bounds=>{fact_limit=>80,byte_limit=>32768,depth=>1,all_cursors=>'none',qualification=>'Observed output bounds and explicit header selection are verified. Individual query argv/exit files are not part of these captures; requested limits are the declared fixture contract.'},
 complete_reply_comparisons=>\@parity,complete_reply_comparisons_count=>15,binding_identities=>\%binding_ids,
 asserted_facts=>{global_scalar_references_per_family=>2,shadowed_local_references_per_family=>3,shadowed_binding_type=>'i64',generic_local_references_per_family=>2,generic_held_type=>'T',complete_generic_outline_signatures_per_family=>3},
 claim_boundary=>'Captured v3 replies declare authenticated-producing-compiler, current-check=not-performed, freshness=not-asserted, recorded-sites and demanded instances. This audit checks records/bytes only, not HMACs or current filesystem freshness.',
 no_compiler_query_native=>JSON::PP::true,no_fixture_reconstruction=>JSON::PP::true,no_key_or_store_read=>JSON::PP::true,no_compression=>JSON::PP::true};
jsave('acceptance.json',$result);
jsave('input-pins.json',[map{$input_pins{$_}}sort keys%input_pins]);
print"PASS builds=7 stats_rows=7 snapshots=5 replies=25 parity_comparisons=15 fact_rows=$total_facts reply_bytes=$total_bytes\n";
print"max_facts=$max_facts max_reply_bytes=$max_bytes all_cursors=none source_edit=one_literal\n";
print"before_snapshot=$before_snap->{snapshot}\nafter_snapshot=$after_snap->{snapshot}\n";
