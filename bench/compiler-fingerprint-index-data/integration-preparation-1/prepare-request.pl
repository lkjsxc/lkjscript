#!/usr/bin/perl
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use Encode qw(encode decode FB_CROAK LEAVE_SRC);
use IPC::Open2;
use File::Find;
use Fcntl qw(:DEFAULT :mode);
my $repo='/home/coder/workspace/lkjscript';
my $root="$repo/target/compiler-fingerprint-index-integration-1";
my $prep="$repo/target/compiler-fingerprint-index-preparation-1";
my $gold="$repo/target/compiler-fingerprint-index-golden-1";
my $json=JSON::PP->new->canonical->pretty->utf8;
sub raw { my($p)=@_; my @s=lstat($p); die "not regular single-link: $p\n" unless @s&&S_ISREG($s[2])&&$s[3]==1; open my $f,'<:raw',$p or die "$p: $!"; local $/; my $b=<$f>; close $f or die "close"; return $b; }
sub put { my($p,$b)=@_; sysopen my $f,$p,O_WRONLY|O_CREAT|O_EXCL,0600 or die "$p: $!"; binmode $f; print {$f} $b or die "write"; close $f or die "close"; }
sub save { put($_[0],$json->encode($_[1])); }
sub text { decode('UTF-8',$_[0],FB_CROAK|LEAVE_SRC); }
sub command { open my $f,'-|',@_ or die "command: $!"; binmode $f; local $/; my $b=<$f>; close $f or die "command failed: @_\n"; return $b; }
sub pinned { my($p,$sha)=@_; my $b=raw($p); die "pin mismatch: $p\n" unless sha256_hex($b) eq $sha; return $b; }
sub once_index { my($whole,$part)=@_; my $at=index($whole,$part); die "missing or duplicate exact source item\n" if $at<0 || index($whole,$part,$at+1)>=0; return $at; }
sub record { my($file,$before,$after,$span)=@_; return {file=>$file,before_sha256=>sha256_hex($before),after_payload_sha256=>sha256_hex($after),after_payload_bytes=>length($after),source_span=>$span}; }
my $commit=command('git','-C',$repo,'rev-parse','HEAD'); chomp $commit;
my @trees=split /\n/,command('git','-C',$repo,'rev-parse',"$commit:src","$commit:library","$commit:tests");
die "src/library baseline mismatch\n" unless $trees[0] eq '626c9c1f234f4d003a08820d9b69840535eff8b8' && $trees[1] eq '25bdb8ff8567b941790d3951c78e896b23e26e8e';
my $listing=command('git','-C',$repo,'ls-tree','-r','-l','-z',$commit);
my @selected;
for my $line(split /\0/,$listing) {
 die "bad Git row\n" unless $line=~/\A([0-7]{6}) (blob|commit) ([0-9a-f]{40})\s+([0-9-]+)\t([^\0]+)\z/;
 my($mode,$kind,$oid,$size,$path)=($1,$2,$3,$4,$5);
 next unless $path=~/\.lkj\z/ || $path=~m{\A(?:src|library|tests)/};
 die "selected Git entry not regular\n" unless $kind eq 'blob' && $mode eq '100644' && $size=~/\A[0-9]+\z/;
 die "unsafe/non-ASCII selected path\n" if $path=~/[^\x20-\x7e]/ || $path=~m{(?:\A|/)\.\.?(?:/|\z)};
 push @selected,{path=>$path,git_oid=>$oid,bytes=>0+$size,mode=>$mode};
}
my @index=grep{/\.lkj\z/ || m{\A(?:src|library|tests)/}}split /\0/,command('git','-C',$repo,'ls-files','-z');
die "Git index/HEAD population differs\n" unless join("\n",sort @index) eq join("\n",sort map{$_->{path}}@selected);
my($out,$in); my $pid=open2($out,$in,'git','-C',$repo,'cat-file','--batch'); binmode $out; binmode $in;
my $old=select $in; $|=1; select $old;
sub exact_read { my($f,$n)=@_; my $b=''; while(length($b)<$n){my $r=read($f,my $p,$n-length($b)); die "short Git read\n" unless defined($r)&&$r>0; $b.=$p;} return $b; }
my %files;
for my $e(@selected) {
 print {$in} "$e->{git_oid}\n" or die "Git request";
 my $h=<$out>; die "Git header\n" unless defined($h)&&$h eq "$e->{git_oid} blob $e->{bytes}\n";
 my $blob=exact_read($out,$e->{bytes}); die "Git separator\n" unless exact_read($out,1) eq "\n";
 my $live=raw("$repo/$e->{path}"); die "working bytes differ from Git: $e->{path}\n" unless $live eq $blob;
 $e->{sha256}=sha256_hex($live); $files{$e->{path}}=$e;
}
close $in or die "Git input close"; close $out or die "Git output close"; waitpid($pid,0)==$pid && $?==0 or die "Git batch exit";
my @lang=sort{$a->{path}cmp$b->{path}}grep{$_->{path}=~/\.lkj\z/}@selected;
my @inputs=sort{$a->{path}cmp$b->{path}}grep{$_->{path}=~m{\A(?:src|library|tests)/}}@selected;
die "input population\n" unless @inputs==1743;
my %physical;
for my $sub(qw(src library tests)) {
 find({no_chdir=>1,wanted=>sub{
  my $p=$File::Find::name; my @s=lstat($p); die "lstat" unless @s; return if S_ISDIR($s[2]);
  die "special or linked source\n" unless S_ISREG($s[2])&&$s[3]==1;
  (my $rel=$p)=~s/^\Q$repo\E\///; $physical{$rel}=1;
 }}, "$repo/$sub");
}
die "untracked/missing physical input\n" unless join("\n",sort keys %physical) eq join("\n",map{$_->{path}}@inputs);
sub manifest { join('',map{"$_->{sha256}  $_->{path}\n"}@_); }
put("$root/before/tracked-language.sha256",manifest(@lang));
put("$root/before/inputs.sha256",manifest(@inputs));
save("$root/before/git-correspondence.json",{schema=>'fingerprint-integration-before-git-1',commit=>$commit,trees=>{src=>$trees[0],library=>$trees[1],tests=>$trees[2]},tracked_language_files=>scalar(@lang),input_files=>scalar(@inputs),entries=>\@selected});
my $baseline=pinned("$prep/baseline-fingerprint.txt",'b85c36fc780c40f80894581e059e6eb00e32617a06176c322033fd838e01c5d5');
my $replacement=pinned("$prep/replacement-body.txt",'abb7f4dc76121a6d9f6a0e885da16ac22f50b35ef1f0a606cf07c9e7d3139271');
my $frozen=pinned("$prep/golden-preparation-1/incremental-fingerprint-module.txt",'5b5a7c739cb6c45c5fd88bf366b46a04c196c1ea88d58997484623a6dee1c7b5');
my $dump=pinned("$gold/records/execution/dump/stdout",'346340880cf8b9bd247c0caf30f75ed534a702c8b0257dc81fcea2cb5849e4cd');
my $gold_bytes=pinned("$gold/records/execution/dump/goldens.json",'ab08cc552e7e979a60c158000ba5266f920df6d3d34bc14c984e200fc4215fe1');
my $goldens=$json->decode($gold_bytes);
die "wrong captured population/pins\n" unless @{$goldens->{cases}}==17 && $goldens->{some}==11 && $goldens->{none}==6
 && $goldens->{producer_sha256} eq 'b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db'
 && $goldens->{incremental_module_sha256} eq '760485a5a12d97ce320ba9113797370df584c154084ee4a1f9689da28c7d1127';
my $joined="lkjscript-fingerprint-goldens 1\n"; my $expected="fn expected(case: usize) -> String {\n    match case {\n"; my @record_pins;
for my $i(0..16) {
 my $case=$goldens->{cases}->[$i]; die "case ordinal\n" unless $case->{ordinal}==$i;
 my $r=encode('UTF-8',$case->{exact_record},FB_CROAK|LEAVE_SRC);
 die "unexpected record bytes\n" if $r=~/[^\x20-\x7e\n]/;
 $joined.=$r;
 my $lit=JSON::PP->new->allow_nonref->ascii->encode($case->{exact_record});
 die "literal roundtrip\n" unless encode('UTF-8',JSON::PP->new->decode($lit),FB_CROAK|LEAVE_SRC) eq $r;
 $expected.="        ".$i."usize => $lit.to_string(),\n";
 push @record_pins,{ordinal=>$i,id=>$case->{id},record_bytes=>length($r),record_sha256=>sha256_hex($r)};
}
$joined.="complete 17\n"; die "derived goldens/raw dump mismatch\n" unless $joined eq $dump;
$expected.="        _ => panic!(\"unknown fingerprint golden {case}\"),\n    }\n}";
my $old_expected="fn expected(case: usize) -> String {\n    panic!(\"fingerprint golden {case} is pending the root baseline capture\")\n}";
my $expected_at=once_index($frozen,$old_expected);
my $module=substr($frozen,0,$expected_at).$expected.substr($frozen,$expected_at+length($old_expected));
die "outside expected item changed\n" unless substr($module,0,$expected_at) eq substr($frozen,0,$expected_at)
 && substr($module,$expected_at+length($expected)) eq substr($frozen,$expected_at+length($old_expected));
my $rolled=$module; substr($rolled,$expected_at,length($expected),$old_expected);
die "module rollback mismatch\n" unless $rolled eq $frozen;
my $inc=raw("$repo/src/elaborate/incremental.lkj");
die "fingerprint original module drift\n" unless sha256_hex($inc) eq '760485a5a12d97ce320ba9113797370df584c154084ee4a1f9689da28c7d1127';
my $fp_at=once_index($inc,$baseline);
my $body_at=index($baseline,'{'); die "fingerprint signature boundary\n" unless $body_at>0;
my $old_body=substr($baseline,$body_at); die "baseline item final LF\n" unless $old_body=~s/\n\z//;
my $body=$replacement; die "replacement exact final LF\n" unless $body=~s/\n\z// && $body=~/\A\{/ && $body=~/\}\z/;
my $parent=raw("$repo/src/semantic_tests.lkj");
my @core_matches=$parent=~/^pub fn core\(\) -> usize (\{.*?^\})/msg; die "core extraction\n" unless @core_matches==1;
my $old_core=$core_matches[0]; my $core=$old_core;
my $anchor="    count += reference_capture::run();\n";
once_index($old_core,$anchor);
$core=~s/\Q$anchor\E/$anchor    count += incremental_fingerprint::run();\n/ or die "registration insertion";
my $semantic=raw("$repo/src/semantic_main.lkj");
my $old_closed='{ assert_eq!(semantic_tests::core(), 192usize); }';
my $closed='{ assert_eq!(semantic_tests::core(), 196usize); }';
my $closed_item="pub fn closed_core() $old_closed";
once_index($semantic,$closed_item);
my $destination='src/semantic_tests/incremental_fingerprint.lkj';
my $alternate='src/semantic_tests/incremental_fingerprint/mod.lkj';
for($destination,$alternate) { die "new module candidate already exists: $_\n" if lstat("$repo/$_"); die "unexpected absence error\n" unless $!{ENOENT}; }
my @parent_stat=lstat("$repo/src/semantic_tests"); die "physical parent missing/special\n" unless @parent_stat&&S_ISDIR($parent_stat[2]);
my $declaration='mod incremental_fingerprint;';
my $request={version=>1,operations=>[
 {op=>'replace-body',id=>'elaborate::incremental::fingerprint',source=>text($body)},
 {op=>'add-item',id=>'semantic_tests',source=>$declaration,projection=>{path=>$destination,source=>text($module)}},
 {op=>'replace-body',id=>'semantic_tests::core',source=>text($core)},
 {op=>'replace-body',id=>'closed_core',source=>$closed}
]};
my $request_bytes=$json->encode($request);
my $decoded=$json->decode($request_bytes);
die "projected module JSON roundtrip\n" unless encode('UTF-8',$decoded->{operations}->[1]->{projection}->{source},FB_CROAK|LEAVE_SRC) eq $module;
die "body JSON roundtrip\n" unless encode('UTF-8',$decoded->{operations}->[0]->{source},FB_CROAK|LEAVE_SRC) eq $body;
put("$root/request.json",$request_bytes);
put("$root/proposals/incremental-fingerprint-module.txt",$module);
put("$root/proposals/expected-item.txt",$expected."\n");
put("$root/proposals/fingerprint-body.txt",$body);
put("$root/proposals/core-body.txt",$core);
put("$root/proposals/closed-core-body.txt",$closed);
put("$root/before/fingerprint-item.txt",$baseline);
put("$root/before/core-body.txt",$old_core);
put("$root/before/closed-core-body.txt",$old_closed);
put("$root/before/incremental.lkj.txt",$inc);
put("$root/before/semantic_tests.lkj.txt",$parent);
put("$root/before/semantic_main.lkj.txt",$semantic);
put("$root/before/baseline-dump.txt",$dump);
put("$root/before/goldens.json",$gold_bytes);
my $tool=raw("$repo/tools/closed-semantics.sh"); my $tool_after=$tool;
my $shell_anchor='if [[ "$entry" == main ]]; then count=192; fi';
once_index($tool,$shell_anchor);
$tool_after=~s/\Q$shell_anchor\E/if [[ "\$entry" == main ]]; then count=196; fi/ or die "runner count replacement";
put("$root/before/closed-semantics.sh.txt",$tool);
put("$root/proposals/closed-semantics.sh.txt",$tool_after);
my $canonical=raw("$repo/target/lkjscript");
my $producer_sha='b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db';
die "canonical changed\n" unless length($canonical)==32588975&&sha256_hex($canonical) eq $producer_sha;
my $correspondence={
 schema=>'fingerprint-index-four-operation-correspondence-1',status=>'PREPARED_NOT_APPLIED',root=>'src/semantic_main.lkj',operations=>[
 {operation=>0,id=>'elaborate::incremental::fingerprint',op=>'replace-body',%{record('src/elaborate/incremental.lkj',$inc,$body,{start=>$fp_at+$body_at,end=>$fp_at+$body_at+length($old_body)})},before_body_sha256=>sha256_hex($old_body),frozen_body_sha256=>sha256_hex($replacement),payload_normalization=>'Remove exactly one terminal LF from the frozen brace-delimited replacement to fit the body span; all other bytes identical.'},
 {operation=>1,id=>'semantic_tests',op=>'add-item',parent_file=>'src/semantic_tests.lkj',parent_before_sha256=>sha256_hex($parent),declaration=>$declaration,projection=>{identity=>'semantic_tests::incremental_fingerprint',path=>$destination,alternate=>$alternate,before=>'Absent',sha256=>sha256_hex($module),bytes=>length($module)},placement=>'No before override: compiler selects the parent module insertion boundary.'},
 {operation=>2,id=>'semantic_tests::core',op=>'replace-body',%{record('src/semantic_tests.lkj',$parent,$core,{start=>once_index($parent,$old_core),end=>once_index($parent,$old_core)+length($old_core)})},before_body_sha256=>sha256_hex($old_core),only_change=>'Add count += incremental_fingerprint::run(); immediately after reference_capture::run().'},
 {operation=>3,id=>'closed_core',op=>'replace-body',%{record('src/semantic_main.lkj',$semantic,$closed,{start=>once_index($semantic,$old_closed),end=>once_index($semantic,$old_closed)+length($old_closed)})},before_body_sha256=>sha256_hex($old_closed),only_change=>'192usize -> 196usize'}
 ],
 fixture=>{frozen_sha256=>sha256_hex($frozen),filled_sha256=>sha256_hex($module),filled_bytes=>length($module),only_changed_item=>'expected(case)',old_item_start=>$expected_at,old_item_end=>$expected_at+length($old_expected),new_item_end=>$expected_at+length($expected),unchanged_prefix_sha256=>sha256_hex(substr($frozen,0,$expected_at)),unchanged_suffix_sha256=>sha256_hex(substr($frozen,$expected_at+length($old_expected))),rollback_equals_frozen=>JSON::PP::true,raw_dump_sha256=>sha256_hex($dump),records=>\@record_pins},
 host_runner=>{path=>'tools/closed-semantics.sh',before_sha256=>sha256_hex($tool),after_sha256=>sha256_hex($tool_after),exact_before=>$shell_anchor,exact_after=>'if [[ "$entry" == main ]]; then count=196; fi',when=>'Root applies this separate guarded shell edit only after the four-op language transaction succeeds; no language-operation credit.'},
 hash_scope=>'Source-coordinate body hashes are audit facts, not guessed semantic item expect digests. Request omits expect and snapshot. Full source manifests are external prelaunch guards; compiler planning, input-staleness checks and independent candidate admission remain required.'
};
save("$root/correspondence.json",$correspondence);
save("$root/commands.json",{
 schema=>'fingerprint-index-integration-commands-1',status=>'PREPARED_NO_EXECUTION_RELEASE',cwd=>$repo,
 preflight=>['/usr/bin/perl',"$root/guard.pl"],
 argv=>["$repo/target/lkjscript",'edit','src/semantic_main.lkj','--source-dir',$repo,'--library-dir',"$repo/library",'--request',"$root/request.json",'--no-store','--store-key','/home/coder/.local/state/lkjscript/store-key'],
 limits=>{address_space_bytes=>4294967296,timeout_seconds=>900,grace_seconds=>10,compiler_workers=>1},
 key=>{path=>'/home/coder/.local/state/lkjscript/store-key',source=>'Previously committed reference-capture operation argv; path only. No key bytes or metadata read by this preparation.'},
 policy=>'One combined transaction, no replay/against and no additional index build. Any dry-run or apply requires root sequencing/release. Do not mutate original golden source, controls or dumps.'
});
my $state={schema=>'fingerprint-index-prelaunch-pins-1',commit=>$commit,baseline_source_commit=>'9326dbaab1bf6832b9f35dc156e12d0d62eccd9e',trees=>{src=>$trees[0],library=>$trees[1],tests=>$trees[2]},tracked_language_files=>scalar(@lang),input_files=>scalar(@inputs),
 language_manifest_sha256=>sha256_hex(raw("$root/before/tracked-language.sha256")),input_manifest_sha256=>sha256_hex(raw("$root/before/inputs.sha256")),
 request_sha256=>sha256_hex($request_bytes),module_sha256=>sha256_hex($module),module_bytes=>length($module),producer_sha256=>$producer_sha,producer_bytes=>length($canonical),
 runner_before_sha256=>sha256_hex($tool),runner_after_sha256=>sha256_hex($tool_after),new_module=>$destination,alternate_module=>$alternate,parent_identity=>{path=>'src/semantic_tests',dev=>$parent_stat[0],inode=>$parent_stat[1],uid=>$parent_stat[4],gid=>$parent_stat[5],mode=>$parent_stat[2]&07777},
 operation_schema_sources=>['docs/structure.md','src/structure_edit.lkj','bench/compiler-reference-capture-data/integration-preparation/tests-request.json']};
save("$root/pins.json",$state);
for my $e(@selected){die "source changed during preparation: $e->{path}\n" unless sha256_hex(raw("$repo/$e->{path}")) eq $e->{sha256};}
put("$root/records/tracked-language-after.sha256",manifest(@lang));
put("$root/records/inputs-after.sha256",manifest(@inputs));
save("$root/records/preparation-result.json",{status=>'prepared_only_no_compiler_native_or_semantic_edit',operation_count=>4,touched_existing_files=>3,new_module_files=>1,tracked_language_files=>scalar(@lang),input_files=>scalar(@inputs),source_before_after_equal=>JSON::PP::true,request_sha256=>$state->{request_sha256},module_sha256=>$state->{module_sha256},pins_sha256=>sha256_hex(raw("$root/pins.json"))});
print $json->encode({status=>'prepared_only',request_sha256=>$state->{request_sha256},module_sha256=>$state->{module_sha256},module_bytes=>length($module),tracked_language_files=>scalar(@lang),input_files=>scalar(@inputs),source_before_after_equal=>JSON::PP::true});
