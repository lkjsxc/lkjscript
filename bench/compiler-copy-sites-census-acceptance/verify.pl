#!/usr/bin/env perl
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use Encode qw(decode FB_CROAK LEAVE_SRC);
use Fcntl qw(O_RDONLY O_WRONLY O_CREAT O_EXCL O_NOFOLLOW S_ISREG);
use File::Find ();
use File::Path qw(make_path);
use JSON::PP ();

# Data-only independent byte-range and closure verification. Does not import or
# execute the frozen extractor, IR checker, compiler, profile decoder or helper.
my($out)=@ARGV;
my $repo='/home/coder/workspace/lkjscript';
my $root="$repo/target/compiler-copy-sites-census-output.20261116-1";
my $cap="$repo/target/compiler-copy-sites-census-evidence.20261116-1";
my $manifest_pin='63d59716e15c302d241d9e19ba792a0d715b0a0e25aeda75827b24be0799d480';
my $companion_pin='32c0f43dbde125e43f2ca5a0c6d8548ba0f1774ce3b51bc62d94a6488af0fdca';
my $detail_pin='06093752531ea752f7081548166f33cc95c71aad381e72d2713e35f5bcaab25d';
my $j=JSON::PP->new->canonical->utf8;
my $jp=JSON::PP->new->canonical->utf8->pretty;
sub ok ($$) { die "CENSUS AUDIT DISCREPANCY: $_[1]\n" unless $_[0]; }
sub sdec { return join('',$_[0]); }
sub rb {
 my($p)=@_;sysopen(my$f,$p,O_RDONLY|O_NOFOLLOW) or die "open $p: $!\n";binmode$f;my@a=stat($f);
 ok(@a&&S_ISREG($a[2]),"not regular $p");local$/;my$v=<$f>;my@b=stat($f);close$f or die "close $p\n";
 ok($a[0]==$b[0]&&$a[1]==$b[1]&&$a[7]==$b[7]&&length($v//"")==$a[7],"read changed $p");return$v//"";
}
sub wr {
 my($p,$v)=@_;sysopen(my$f,"$out/$p",O_WRONLY|O_CREAT|O_EXCL,0600) or die "create proof $p: $!\n";
 binmode$f;print {$f} $v or die "write proof $p\n";close$f or die "close proof $p\n";
}
sub jf { return $j->decode(rb($_[0])); }
sub hf {
 my($p)=@_;sysopen(my$f,$p,O_RDONLY|O_NOFOLLOW) or die "hash $p: $!\n";binmode$f;
 my@s=stat($f);ok(@s&&S_ISREG($s[2]),"hash nonregular $p");my$sha=Digest::SHA->new(256)->addfile($f)->hexdigest;
 my@a=stat($f);close$f or die "hash close\n";ok($s[0]==$a[0]&&$s[1]==$a[1]&&$s[7]==$a[7]&&$s[9]==$a[9]&&$s[10]==$a[10],"hash changed $p");return$sha;
}
sub tree {
 my($p)=@_;my@paths;
 File::Find::find({no_chdir=>1,wanted=>sub{my$f=$File::Find::name;my@s=lstat($f);ok(@s&&!-l _&&(-d _||-f _),"special/link $f");if(-f _){ok($s[3]==1,"hardlink $f");push@paths,substr($f,length($p)+1);}}},$p);
 return[sort@paths];
}
sub inventory { my($p,$files)=@_;return join('',map{hf("$p/$_")."  $_\n"}@$files); }
sub lines { my$v=rb($_[0]);ok($v=~/\n\z/,"missing final LF $_[0]");return[split/\n/,$v]; }
sub tsv {return join("\t",@_)."\n";}
ok(defined($out)&&-d$out,"owned proof directory");
make_path("$out/captures");
my$paths=tree($root);my$before=inventory($root,$paths);wr('output-before.sha256',$before);
my$manifest=rb("$root/manifest.sha256");ok(sha256_hex($manifest)eq$manifest_pin,"output manifest pin");
my%listed;
for(split/\n/,$manifest){ok(/\A([0-9a-f]{64})  ([A-Za-z0-9_.\/-]+)\z/,"manifest syntax");my($sha,$p)=($1,$2);
 ok($p!~m{(?:\A|/)\.\.?(?:/|\z)}&&!exists$listed{$p},"manifest path");$listed{$p}=$sha;ok(hf("$root/$p")eq$sha,"manifest $p");
}
ok(@$paths==keys(%listed)+1&&!exists$listed{'manifest.sha256'},"manifest exact population");
for(@$paths){ok($_ eq 'manifest.sha256'||exists$listed{$_},"unlisted output $_");}
wr('extraction.manifest.sha256',$manifest);
for my$p(@{tree($cap)}){ok($p!~m{/},"unexpected capture subdirectory");wr("captures/$p",rb("$cap/$p"));}
ok(rb("$cap/status")eq"0\n"&&rb("$cap/stderr")eq''&&rb("$cap/cwd")eq"$repo\n","capture completion");
my$captured=jf("$cap/stdout");
ok($captured->{status}eq'complete'&&$captured->{exit_status}eq'0'&&$captured->{selected_functions}eq'20'&&
 $captured->{eligibility}eq'not-assessed'&&$captured->{manifest_sha256}eq$manifest_pin&&$captured->{output}eq$root,"stdout receipt");
my$expected_command="/usr/bin/prlimit --as=4294967296 -- /usr/bin/timeout --kill-after=10s 300s /usr/bin/perl $repo/bench/compiler-copy-sites-extract.pl --companion $repo/target/compiler-copy-sites-run.20261116-2/selected-native.lkjir --detail $repo/bench/compiler-profile-copy-sites-cohort-2-data/06-copy-detail/stdout --output $root \n";
ok(rb("$cap/command")eq$expected_command,"exact captured command");
my$time=rb("$cap/time");ok($time=~/\Awall=(\d+\.\d+) user=(\d+\.\d+) sys=(\d+\.\d+) rss_kib=(\d+) exit=0\n\z/,"raw time");
my$timing={wall_s=>$1,user_s=>$2,system_s=>$3,rss_kib=>$4};
for my$k(qw(before after)){my$l=rb("$cap/load.$k");ok($l=~/\A(\d+\.\d+ \d+\.\d+ \d+\.\d+) \d+\/\d+ \d+\n\z/,"load $k");$timing->{"load_$k"}=$1;}
my%pins=(
 'bench/compiler-copy-sites-extract.pl'=>'9ca679c6f24cfe1ad9fcfd14a54f27be0edef56a1c9a0faeb7c455ab76536517',
 'bench/compiler-copy-sites-extract-capture.sh'=>'85dcbc04be568238d8b7f079292b5e4475567428fc7d1c8be326ed9e10eb5109',
 'bench/compiler-copy-sites-census-plan.md'=>'8a1f3e3d5354b3eca7fd9adb68bfee892f94360130a19b265f3f9ffb73c4dc15',
 'bench/compiler-copy-sites-census-extract-release-20261116.md'=>'615597132d59db7a2f4203e34edd31ca77e28bbcf6b4a966174dcee288be1af8',
 'bench/compiler-copy-sites-census-extract-release-20261116.sh'=>'4f4975c645ce3e4ab3e2c0f6f186490e593bc1e08b89bc616430973ccec049bb',
 'src/ir_text.lkj'=>'af4c78b51063bce8a24e2a35bd364a48cbb3a888d01c1fd6219faa3d5983d4f8'
);
for(sort keys%pins){ok(hf("$repo/$_")eq$pins{$_},"contract pin $_");}
my$result=jf("$root/extraction.json");
ok($result->{status}eq'complete'&&$result->{exit_status}eq'0'&&$result->{selected_functions}eq'20'&&
 $result->{eligibility}eq'not-assessed'&&$result->{semantic_admission}eq'not-performed',"top extraction scope");
ok($result->{plan_sha256}eq$pins{'bench/compiler-copy-sites-census-plan.md'}&&$result->{ir_text_sha256}eq$pins{'src/ir_text.lkj'},"reported grammar/plan pins");
my$ir=rb("$root/inputs/companion.lkjir");my$detail=rb("$root/inputs/detail.txt");
ok(length($ir)==57649044&&sha256_hex($ir)eq$companion_pin,"owned exact companion");
ok(length($detail)==53808&&sha256_hex($detail)eq$detail_pin,"owned exact detail");
ok(hf("$repo/target/compiler-copy-sites-run.20261116-2/selected-native.lkjir")eq$companion_pin&&
 hf("$repo/bench/compiler-profile-copy-sites-cohort-2-data/06-copy-detail/stdout")eq$detail_pin,"original accepted inputs");
ok($result->{companion}{sha256}eq$companion_pin&&$result->{detail}{sha256}eq$detail_pin,"reported input digests");
my(@selected,%seen);my$row=0;
for(split/\n/,$detail){next unless/\Acopy function=(\d+) /;push@selected,{id=>0+$1,first=>$row} if!$seen{$1}++&&@selected<20;$row++;}
ok($row==200&&@selected==20,"first twenty from sole page");
my%selected=map{$_->{id}=>1}@selected;

# The input is the exact canonical writer output already admitted officially.
# Independently tokenize structural records; no IR validity or loan inference.
sub words {
 my($raw)=@_;my$text=decode('UTF-8',$raw,FB_CROAK|LEAVE_SRC);my@w;pos($text)=0;
 while(pos($text)<length($text)){
  if($text=~/\G\p{White_Space}+/gc){next;}
  last if$text=~/\G\#/gc;
  if($text=~/\G("(?:[^"\\\p{Cc}]|\\(?:["\\\/bfnrt]|u[0-9a-fA-F]{4}))*")/gc){
   push@w,JSON::PP->new->decode($1);my$at=pos($text);ok($at==length($text)||substr($text,$at,1)=~/[\p{White_Space}#]/,"quoted separator");
  }elsif($text=~/\G([^\p{White_Space}"#]+)/gc){push@w,$1;}
  else{die"unsupported canonical word\n";}
 }
 return\@w;
}
sub uint {ok(defined($_[0])&&$_[0]=~/\A\d+\z/,"canonical unsigned integer");return 0+$_[0];}
sub field_refs {my($w,$at)=@_;ok((@$w-$at)%2==0,"field pairing");my@r;for(my$i=$at+1;$i<@$w;$i+=2){push@r,uint($w->[$i]);}return\@r;}
my(@types,@funcs,@sources,@dtors);my($fn,$en);my$offset=0;my$line_number=0;
while($ir=~/([^\n]*\n|[^\n]+\z)/g){
 my$raw=$1;my$start=$-[0];my$end=$+[0];$line_number++;ok($start==$offset,"contiguous input scan");$offset=$end;
 if($line_number==1){ok($raw eq"lkjscript-ir 1\n","canonical IR header");next;}
 next if$raw=~/\A[\t\r ]*(?:\#.*)?\n\z/;
 my$span=[$start,length($raw)];
 if(defined$fn && $raw=~/\A\@/){
  ok($raw=~/\A\@(?:(\d+)\/)?(\d+)(?::(\d+))?[ \t]+([a-z-]+)(?:[ \t]|\r?\n|\z)/,"canonical instruction");
  my($file,$line,$column,$op)=($1,$2,$3,$4);
  if($selected{$fn->{id}}){push@{$fn->{ins}},[$fn->{count},@$span,$op,defined($file)?$file:'default',$line,defined($column)?$column:'1'];
   if($op eq'call'){my$w=words($raw);$fn->{calls}{uint($w->[3])}=1;}
  }
  $fn->{count}++;next;
 }
 my$w=words($raw);next unless@$w;
 if(defined$en){
  if($w->[0]eq'variant'){push@{$en->{refs}},@{field_refs($w,2)};}
  elsif($w->[0]eq'endtype'){ok(@$w==1,"enum end");$en->{length}=$end-$en->{start};push@types,$en;undef$en;}
  else{die"unexpected enum record\n";}next;
 }
 if(defined$fn){
  if($w->[0]eq'local'){
   ok(@$w==4,"local framing");my$ty=uint($w->[2]);
   if($fn->{locals}<$fn->{params}){push@{$fn->{param_types}},$ty;push@{$fn->{sig}},$span;}
   push@{$fn->{local_types}},$ty if$selected{$fn->{id}};$fn->{locals}++;
  }elsif($w->[0]eq'return-leaf'){push@{$fn->{sig}},$span;}
  elsif($w->[0]eq'endfn'){ok(@$w==1&&$fn->{params}<=$fn->{locals},"function end");$fn->{length}=$end-$fn->{start};push@{$fn->{sig}},$span;push@funcs,$fn;undef$fn;}
  else{die"unexpected function record\n";}next;
 }
 if($w->[0]eq'type'){
  my$kind=$w->[2];my@refs;
  if($kind eq'struct'){@refs=@{field_refs($w,4)};}
  elsif($kind eq'ref'||$kind eq'slice'){push@refs,uint($w->[4]);}
  elsif($kind eq'array'||$kind eq'vec'||$kind eq'box'){push@refs,uint($w->[3]);}
  elsif($kind eq'enum'){}
  else{ok($kind=~/\A(?:unit|bool|char|f64|str|world|stdout|stdin|stderr|directory|read_directory|read_file|image|channel|worker|read_region|foreign_command|foreign_session|null_device|file|process|clock|child|listener|connector|readiness|entropy|stop|arguments|environment|exit_status|int)\z/,"known scalar/capability type");}
  my$t={id=>scalar@types,name=>$w->[1],kind=>$kind,refs=>\@refs,start=>$start,length=>length($raw)};
  if($kind eq'enum'){$en=$t;}else{push@types,$t;}
 }elsif($w->[0]eq'fn'){
  $fn={id=>scalar@funcs,name=>$w->[1],params=>uint($w->[2]),result=>uint($w->[3]),start=>$start,locals=>0,count=>0,sig=>[$span],param_types=>[],local_types=>[],ins=>[],calls=>{}};
 }elsif($w->[0]eq'source'){push@sources,$span;}
 elsif($w->[0]eq'destructor'){push@dtors,{type=>uint($w->[1]),function=>uint($w->[2]),span=>$span};}
 else{die"unexpected top-level record\n";}
}
ok($offset==length($ir)&&!defined($fn)&&!defined($en),"complete source framing");
ok(@types==$result->{companion_types}&&@funcs==$result->{companion_functions}&&@sources==$result->{source_records}&&@dtors==$result->{destructor_records},"global declaration counts");
my%dtors;for(@dtors){ok($_->{type}<@types&&$_->{function}<@funcs,"destructor range");push@{$dtors{$_->{type}}},$_->{function};}
sub bytes_at {my($at,$len)=@_;ok($at<=length($ir)&&$len<=length($ir)-$at,"span extent");return substr($ir,$at,$len);}
my$shared_index="kind\tindex\tbyte_offset\tbytes\tsha256\n";my($source_bytes,$dtor_bytes)=('','');
for my$i(0..$#sources){my$span=$sources[$i];my$b=bytes_at(@$span);$source_bytes.=$b;$shared_index.=tsv('source',$i,@$span,sha256_hex($b));}
for my$i(0..$#dtors){my$span=$dtors[$i]{span};my$b=bytes_at(@$span);$dtor_bytes.=$b;$shared_index.=tsv('destructor',$i,@$span,sha256_hex($b));}
ok(rb("$root/shared/sources.lkjir")eq$source_bytes&&rb("$root/shared/destructors.lkjir")eq$dtor_bytes&&
 rb("$root/shared/index.tsv")eq$shared_index,"all shared bytes/ranges/hashes");
my$shared=length($source_bytes)+length($dtor_bytes);ok($shared==$result->{shared_context_bytes},"shared context length");
my($type_instances,$signature_instances,$signature_parts,$instruction_total,$type_bytes_total,$signature_bytes_total)=(0,0,0,0,0,0);
my(@report,@bodies,@contexts,%unique_type,%unique_sig);
my$selection="ordinal\tfunction\tfirst_display_row\tname_json\tstatus\tbody_bytes\tinstructions\ttypes\tsignatures\tcontext_bytes\treasons_json\n";
for my$ordinal(0..$#selected){
 my$pick=$selected[$ordinal];my$id=$pick->{id};ok($id<@funcs,"selected function exists");my$f=$funcs[$id];
 my$folder=sprintf('functions/%02u-function-%u',$ordinal,$id);my$m=jf("$root/$folder/extraction.json");
 ok($m->{status}eq'complete'&&$m->{eligibility}eq'not-assessed'&&!@{$m->{reasons}},"per-function scope");
 ok($j->encode($m)eq$j->encode($result->{functions}[$ordinal]),"top/per-function metadata");
 my%expected=(ordinal=>$ordinal,function=>$id,first_display_row=>$pick->{first},byte_offset=>$f->{start},body_bytes=>$f->{length},instructions=>$f->{count},local_count=>$f->{locals},parameter_count=>$f->{params},result_type=>$f->{result});
 for(keys%expected){ok($m->{$_}eq sdec($expected{$_}),"body metadata $id $_");}
 ok($m->{name}eq$f->{name},"function name");my$body=bytes_at($f->{start},$f->{length});
 ok(rb("$root/$folder/body.lkjir")eq$body&&sha256_hex($body)eq$m->{body_sha256},"complete body $id");
 my(%closure,%sig,%why);my@queue=map{['type',$_]}($f->{result},@{$f->{local_types}});
 for(sort{$a<=>$b}keys%{$f->{calls}}){push@queue,['signature',$_];$why{$_}{direct_call}=1;}
 for(my$qi=0;$qi<@queue;$qi++){
  my($kind,$n)=@{$queue[$qi]};
  if($kind eq'type'){next if$closure{$n}++;ok($n<@types,"closure type exists");
   push@queue,map{['type',$_]}@{$types[$n]{refs}};
   for my$df(@{$dtors{$n}//[]}){$why{$df}{destructor}=1;push@queue,['signature',$df];}
  }else{next if$sig{$n}++;ok($n<@funcs,"closure signature exists");push@queue,map{['type',$_]}($funcs[$n]{result},@{$funcs[$n]{param_types}});}
 }
 my@ti=sort{$a<=>$b}keys%closure;my@si=sort{$a<=>$b}keys%sig;
 ok(join(',',@ti)eq join(',',@{$m->{type_indices}})&&join(',',@si)eq join(',',@{$m->{signature_indices}}),"full fixed-point closure $id");
 my$type_text='';my$type_index="type\tbyte_offset\tbytes\tsha256\tkind\tname_json\n";
 for my$t(@ti){my$v=$types[$t];my$b=bytes_at($v->{start},$v->{length});$type_text.=$b;
  $type_index.=tsv($t,$v->{start},$v->{length},sha256_hex($b),$v->{kind},$j->encode($v->{name}));$unique_type{$t}=1;}
 ok(rb("$root/$folder/types.lkjir")eq$type_text&&rb("$root/$folder/types.tsv")eq$type_index,"all complete type ranges $id");
 my$sig_index="function\treasons\tbytes\tsha256\tname_json\n";my$part_index="function\tpart\tbyte_offset\tbytes\tsha256\n";my$sig_bytes=0;my$parts=0;
 for my$c(@si){my$text='';my$part=0;for my$span(@{$funcs[$c]{sig}}){my$b=bytes_at(@$span);$text.=$b;$part_index.=tsv($c,$part++,@$span,sha256_hex($b));}
  ok(rb("$root/$folder/signatures/function-$c.lkjir")eq$text,"signature bytes $id/$c");
  $sig_index.=tsv($c,join(',',sort keys%{$why{$c}}),length($text),sha256_hex($text),$j->encode($funcs[$c]{name}));
  $sig_bytes+=length($text);$parts+=$part;$unique_sig{$c}=1;
 }
 ok(rb("$root/$folder/signatures.tsv")eq$sig_index&&rb("$root/$folder/signature-parts.tsv")eq$part_index,"all signature parts/reasons $id");
 my$ins_index="instruction\tbyte_offset\tbytes\top\texplicit_source_file\tsource_line\tsource_column\tsha256\n";
 for my$in(@{$f->{ins}}){$ins_index.=tsv(@$in,sha256_hex(bytes_at($in->[1],$in->[2])));}
 ok(@{$f->{ins}}==$f->{count}&&rb("$root/$folder/instructions.tsv")eq$ins_index,"all instruction indices/coordinates/ranges $id");
 my$context=$f->{length}+$shared+length($type_text)+$sig_bytes;
 ok($m->{types}==@ti&&$m->{signatures}==@si&&$m->{context_bytes}==$context,"context counts $id");
 ok($f->{count}<=65536&&$f->{length}<=16777216&&@ti<=16384&&@si<=2048&&$context<=33554432,"declared export bounds $id");
 $selection.=tsv($ordinal,$id,$pick->{first},$j->encode($f->{name}),'complete',$f->{length},$f->{count},scalar@ti,scalar@si,$context,'[]');
 push@bodies,$f->{length};push@contexts,$context;
 $type_instances+=@ti;$signature_instances+=@si;$signature_parts+=$parts;$instruction_total+=$f->{count};$type_bytes_total+=length($type_text);$signature_bytes_total+=$sig_bytes;
 push@report,{ordinal=>sdec($ordinal),function=>sdec($id),first_display_row=>sdec($pick->{first}),body_bytes=>sdec($f->{length}),context_bytes=>sdec($context),
 instructions=>sdec($f->{count}),types=>sdec(scalar@ti),signatures=>sdec(scalar@si),signature_parts=>sdec($parts),
 body_sha256=>sha256_hex($body),type_index_sha256=>sha256_hex($type_index),signature_parts_sha256=>sha256_hex($part_index),instruction_index_sha256=>sha256_hex($ins_index),
 type_closure_sha256=>sha256_hex(join(',',@ti)),signature_closure_sha256=>sha256_hex(join(',',@si)),eligibility=>'not-assessed'};
}
ok(rb("$root/selection.tsv")eq$selection,"first-twenty order and complete selection TSV");
sub distribution {my@v=sort{$a<=>$b}@_;my$sum=0;$sum+=$_ for@v;return{count=>sdec(scalar@v),total=>sdec($sum),median=>sdec(($v[(@v-1)/2]+$v[@v/2])/2),max=>sdec($v[-1]),min=>sdec($v[0])};}
my$after=inventory($root,tree($root));ok($after eq$before,"extraction files changed during audit");wr('output-after.sha256',$after);
my$proof={status=>'verified-complete-extraction',eligibility=>'not-assessed',manifest_sha256=>$manifest_pin,
 input_sha256=>{companion=>$companion_pin,detail=>$detail_pin},contract_pins=>\%pins,original_extraction_timing=>$timing,
 original_output_files=>sdec(scalar@$paths),manifest_records=>sdec(scalar keys%listed),selected_functions=>'20',
 companion_functions=>sdec(scalar@funcs),companion_types=>sdec(scalar@types),source_records=>sdec(scalar@sources),destructor_records=>sdec(scalar@dtors),
 instruction_records_verified=>sdec($instruction_total),type_blocks_verified_including_repetition=>sdec($type_instances),
 unique_type_blocks=>sdec(scalar keys%unique_type),signature_excerpts_verified_including_repetition=>sdec($signature_instances),
 unique_signatures=>sdec(scalar keys%unique_sig),signature_parts_verified=>sdec($signature_parts),shared_context_bytes=>sdec($shared),
 type_context_bytes_including_repetition=>sdec($type_bytes_total),signature_context_bytes_including_repetition=>sdec($signature_bytes_total),
 body_bytes=>distribution(@bodies),context_bytes=>distribution(@contexts),functions=>\@report,
 limits=>['Exact retained canonical input checked structurally; no new IR admission or ownership/loan/layout facts.',
 'No type-only or whole-family eligibility conclusion, optimizer execution or sampled-site coverage inference.',
 'Type/signature/context bounds are export bounds; full scanning/traversal is subject to the external resource envelope.',
 'Only the already accepted detail page selected functions; no additional page or later join output was opened.']};
wr('proof.json',$jp->encode($proof));wr('verified-selection.tsv',$selection);
print$jp->encode({status=>$proof->{status},eligibility=>'not-assessed',manifest_records=>$proof->{manifest_records},
 instruction_records_verified=>$proof->{instruction_records_verified},type_blocks_verified_including_repetition=>$proof->{type_blocks_verified_including_repetition},
 signature_excerpts_verified_including_repetition=>$proof->{signature_excerpts_verified_including_repetition},signature_parts_verified=>$proof->{signature_parts_verified},
 body_bytes=>$proof->{body_bytes},context_bytes=>$proof->{context_bytes}});
