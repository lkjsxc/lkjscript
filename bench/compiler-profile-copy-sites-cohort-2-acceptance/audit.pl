#!/usr/bin/env perl
use strict;
use warnings;
use Config;
use Cwd qw(abs_path);
use Digest::SHA qw(sha256_hex sha1_hex);
use Encode qw(decode FB_CROAK LEAVE_SRC);
use Fcntl qw(O_RDONLY O_WRONLY O_CREAT O_EXCL O_NOFOLLOW S_ISREG);
use File::Find ();
use File::Path qw(make_path);
use IO::Uncompress::Gunzip qw($GunzipError);
use JSON::PP ();
use Math::BigInt;

# Independent, data-only acceptance audit. No compiler/image/official reader,
# frozen accounting helper, extractor, new detail request, or cleanup invocation.
my ($out)=@ARGV;
my $repo='/home/coder/workspace/lkjscript';
my $e="$repo/target/compiler-copy-sites-evidence.20261116-2";
my $r="$repo/target/compiler-copy-sites-run.20261116-2";
my $release_commit='88789cac357a455f8e752e6d820f59ce3cc38fb7';
my $release_sha='cdf8f17528800d6d152e8f7986e389f37da9712bbcd740547b59452e639f692b';
my $j=JSON::PP->new->canonical->ascii;
my $pretty=JSON::PP->new->canonical->ascii->pretty;
$ENV{GIT_OPTIONAL_LOCKS}=0;
sub require_ok ($$) { die "AUDIT DISCREPANCY: $_[1]\n" unless $_[0]; }
sub str { return join('',$_[0]); }
sub read_bytes {
    my ($p)=@_;
    sysopen(my $f,$p,O_RDONLY|O_NOFOLLOW) or die "open $p: $!\n";
    binmode $f; my @a=stat($f);
    require_ok(@a && S_ISREG($a[2]),"not regular: $p");
    local $/;my $v=<$f>;my @b=stat($f);close$f or die "close $p: $!\n";
    require_ok($a[0]==$b[0] && $a[1]==$b[1] && $a[7]==$b[7] && length($v//"")==$a[7],"read changed: $p");
    return $v//"";
}
sub write_new {
    my ($p,$v)=@_;
    sysopen(my $f,"$out/$p",O_WRONLY|O_CREAT|O_EXCL,0600) or die "create audit $p: $!\n";
    binmode$f;print $f $v or die "write audit $p: $!\n";close$f or die "close audit $p: $!\n";
}
sub json_file { return $j->decode(read_bytes($_[0])); }
sub trim_file { my $v=read_bytes($_[0]);$v=~s/\n\z//;return $v; }
sub metadata {
    my ($p)=@_;my %m;
    for(split /\n/,read_bytes($p)){ require_ok(/\A([^=]+)=(.*)\z/,"metadata syntax: $p");require_ok(!exists$m{$1},"duplicate metadata: $p");$m{$1}=$2; }
    return \%m;
}
sub hash_file {
    my ($p)=@_;
    sysopen(my $f,$p,O_RDONLY|O_NOFOLLOW) or die "hash open $p: $!\n";binmode$f;
    my @a=stat($f);require_ok(@a && S_ISREG($a[2]),"not regular: $p");
    my $s=Digest::SHA->new(256);$s->addfile($f);my @b=stat($f);close$f or die "hash close: $!\n";
    require_ok($a[0]==$b[0] && $a[1]==$b[1] && $a[7]==$b[7] && $a[9]==$b[9] && $a[10]==$b[10],"hash changed: $p");
    return {sha256=>$s->hexdigest,bytes=>str($a[7]),links=>str($a[3])};
}
sub files_under {
    my ($root)=@_;my @paths;
    File::Find::find({no_chdir=>1,wanted=>sub {
        my $p=$File::Find::name;my @s=lstat($p);require_ok(@s && !-l _ && (-d _ || -f _),"link/special: $p");
        if(-f _){require_ok($s[3]==1,"hardlinked evidence: $p");push @paths,substr($p,length($root)+1);}
    }},$root);
    return [sort @paths];
}
sub inventory {
    my ($root,$paths)=@_;return join('',map{my $h=hash_file("$root/$_");"$h->{sha256}  $_\n"}@$paths);
}
sub git {
    open my $g,'-|','/usr/bin/git','-C',$repo,@_ or die "git pipe: $!\n";
    binmode$g;local$/;my $v=<$g>;close$g or die "git failed: @_\n";return$v//"";
}
sub same_hash {
    my ($p,$h,$n)=@_;my $a=hash_file($p);
    require_ok($a->{sha256} eq $h && (!defined($n)||$a->{bytes} eq "$n"),"digest/length: $p");return $a;
}
sub json_equal { return $j->encode($_[0]) eq $j->encode($_[1]); }
sub sum_exact { my $x=Math::BigInt->new(0);$x->badd(str($_)) for @_;return str($x->bstr); }
require_ok(defined($out) && -d$out && $Config{ivsize}==8,"audit output and exact 64-bit Perl required");
make_path("$out/observed","$out/reconstructed");
my $ep=files_under($e);my $rp=files_under($r);
my $ei=inventory($e,$ep);my $ri=inventory($r,$rp);
write_new('evidence-before.sha256',$ei);write_new('held-before.sha256',$ri);
for my $f (@$ep) {
    next if $f=~/\.gz\z/;
    my $parent=$f;$parent=~s{/[^/]+\z}{};
    make_path("$out/observed/$parent") if $parent ne $f;
    write_new("observed/$f",read_bytes("$e/$f"));
}
my $release=json_file("$e/release.json");
require_ok(sha256_hex(read_bytes("$e/release.json")) eq $release_sha,"released JSON");
my $relpath=$release->{release_path};
require_ok(git('show',"$release_commit:$relpath") eq read_bytes("$e/release.json"),"release commit bytes");
require_ok(read_bytes("$repo/$relpath") eq read_bytes("$e/release.json"),"tracked release bytes");
my $pre=json_file("$e/preflight.json");
require_ok($pre->{status} eq 'passed' && $pre->{head} eq $release_commit && $pre->{release_sha256} eq $release_sha,"preflight pins");
my $completion=metadata("$e/completion");
require_ok($completion->{supervisor_exit} eq '0' && $completion->{completed} eq 'true' &&
 $completion->{last_phase} eq 'preservation' && $completion->{reason} eq 'success' && $completion->{run} eq $r,"completion");
require_ok(read_bytes("$e/not-run.tsv") eq '' && read_bytes("$e/preflight.stderr") eq '',"unexpected preflight/not-run material");
for my $name(sort keys %{$release->{files}}){
 my $pin=$release->{files}{$name};same_hash("$repo/$pin->{path}",$pin->{sha256});
 require_ok(sha256_hex(git('show',"$release_commit:$pin->{path}")) eq $pin->{sha256},"release $name commit bytes");
}
for my $name(sort keys %{$release->{tools}}){my $pin=$release->{tools}{$name};same_hash($pin->{path},$pin->{sha256});}
my $p=$release->{producer};
same_hash($p->{path},$p->{sha256},$p->{bytes});
same_hash("$r/producer",$p->{sha256},$p->{bytes});
same_hash("$r/accounting.pl",$release->{files}{accounting}{sha256});
require_ok(trim_file("$e/fixture.path") eq $r && trim_file("$e/cwd") eq $repo,"fixture/cwd ownership");
for my $root(qw(src library)){
 my $want=$root eq 'src' ? $p->{source_tree}:$p->{library_tree};
 my $at=git('rev-parse',"$p->{source_commit}:$root");chomp$at;require_ok($at eq $want,"producer historical $root tree");
 my $now=git('rev-parse',"HEAD:$root");chomp$now;require_ok($now eq $want,"current committed $root tree");
}
my $main_records=git('ls-files','-s','-z','--','src','library');
my $main_count=0;
for my $row(split /\0/,$main_records){
 require_ok($row=~/\A[0-7]+ ([0-9a-f]{40}) 0\t(.+)\z/s,"main index entry");
 my($oid,$path)=($1,$2);my $b=read_bytes("$repo/$path");
 require_ok(sha1_hex('blob '.length($b)."\0".$b) eq $oid,"current source differs from index: $path");$main_count++;
}
my $w=$release->{witness};
my $before=read_bytes("$e/source-before.sha256");
require_ok($before eq read_bytes("$e/source-after.sha256"),"before/after source inventories");
my %source_hash;
for(split/\n/,$before){require_ok(/\A([0-9a-f]{64})  (.+)\z/,"source inventory line");require_ok(!exists$source_hash{$2},"duplicate source");$source_hash{$2}=$1;}
my $source_paths=files_under("$r/source");
require_ok(@$source_paths==$w->{input_files} && keys(%source_hash)==@$source_paths,"exact source count");
my $tree=git('ls-tree','-r','-z',$w->{revision},'--','src','library','seed/lkjscript.gz');
my %tree;
for(split/\0/,$tree){require_ok(/\A([0-7]+) blob ([0-9a-f]{40})\t(.+)\z/s,"witness Git tree entry");$tree{$3}=[$1,$2];}
require_ok(keys(%tree)==@$source_paths,"witness Git population");
for my $path(@$source_paths){
 require_ok(exists$source_hash{$path} && exists$tree{$path},"unexpected witness path: $path");
 my $b=read_bytes("$r/source/$path");
 require_ok(sha256_hex($b) eq $source_hash{$path} && sha1_hex('blob '.length($b)."\0".$b) eq $tree{$path}[1],"witness content: $path");
}
for my $root(qw(src library)){
 my $v=git('rev-parse',"$w->{revision}:$root");chomp$v;
 require_ok($v eq ($root eq 'src'?$w->{source_tree}:$w->{library_tree}),"witness tree $root");
}
same_hash("$r/source/library/runtime.lkj",$w->{runtime_sha256});
same_hash("$r/source/src/optimizer.lkj",$w->{optimizer_sha256});
same_hash("$r/source/seed/lkjscript.gz",$w->{seed_archive_sha256},$w->{seed_archive_bytes});
my $producer_seed=git('show',"$p->{seed_commit}:seed/lkjscript.gz");
require_ok(sha256_hex($producer_seed) eq $p->{seed_archive_sha256} && length($producer_seed)==$p->{seed_archive_bytes},"producer seed archive");
my $producer_raw='';IO::Uncompress::Gunzip::gunzip(\$producer_seed=>\$producer_raw) or die "producer seed gunzip: $GunzipError\n";
require_ok(sha256_hex($producer_raw) eq $p->{sha256} && length($producer_raw)==$p->{bytes},"producer seed decoded identity");
undef$producer_raw;undef$producer_seed;
my $expected=read_bytes("$r/expected");
require_ok(sha256_hex($expected) eq $w->{expected_sha256} && length($expected)==$w->{expected_bytes},"oracle identity");
my $witness_raw='';IO::Uncompress::Gunzip::gunzip("$r/source/seed/lkjscript.gz"=>\$witness_raw) or die "witness seed gunzip: $GunzipError\n";
require_ok($witness_raw eq $expected,"witness seed and oracle exact bytes");undef$witness_raw;
for my $name(qw(output-ordinary output-profile)){require_ok(read_bytes("$r/$name") eq $expected,"oracle exact output: $name");}
undef$expected;
my %artifacts;
for my $label(qw(ordinary64 profile64 selected-ir raw-profile 03-execute-ordinary-output 04-execute-profile-output)){
 my $m=metadata("$e/$label.artifact");require_ok(index($m->{path},"$r/")==0,"artifact outside cohort");
 same_hash($m->{path},$m->{sha256},$m->{bytes});$artifacts{$label}=$m;
}
my @samples=grep{m{\Aproducer-profile64\.samples\.[0-9]+\z}}@$rp;
require_ok(@samples==1 && "$r/$samples[0]" eq $artifacts{'raw-profile'}{path},"exactly one sample artifact");

my @phase_names=qw(01-construct-ordinary 02-construct-profile 03-execute-ordinary 04-execute-profile 05-summary 06-copy-detail 07-accounting);
my @selectors=('build','src/main.lkj','--source-dir',"$r/source",'--library-dir',"$r/source/library",'--runtime-source',"$r/source/library/runtime.lkj");
my $sample=$artifacts{'raw-profile'}{path};my $companion="$r/selected-native.lkjir";
my @expected_argv=(
 ["$r/producer",@selectors,'--no-store','--inline-limit','64','--stats','-o',"$r/producer-ordinary64"],
 ["$r/producer",@selectors,'--no-store','--inline-limit','64','--profile','--profile-ir',$companion,'--stats','-o',"$r/producer-profile64"],
 ["$r/producer-ordinary64",@selectors,'--no-store','--inline-limit','0','-o',"$r/output-ordinary"],
 ["$r/producer-profile64",@selectors,'--no-store','--inline-limit','0','-o',"$r/output-profile"],
 ["$r/producer",'profile',$sample],
 ["$r/producer",'profile',$sample,'--copy-sites','--profile-ir',$companion,'--limit','200','--bytes','1048576'],
 ['/usr/bin/perl',"$r/accounting.pl",'--profile',$sample,'--companion',$companion,'--summary',"$e/05-summary/stdout",'--detail',"$e/06-copy-detail/stdout",'--output',"$e/accounting"]
);
my @phases;my $prior_end=Math::BigInt->new(0);
for my $i(0..$#phase_names){
 my $name=$phase_names[$i];my $d="$e/$name";my $argv=json_file("$d/argv.json");
 require_ok(json_equal($argv,$expected_argv[$i]),"argv $name");
 my $wrapper=['/usr/bin/time','-f','%e\t%U\t%S\t%M\t%x','-o',"$d/time",'/usr/bin/prlimit','--as=4294967296','--','/usr/bin/timeout','--kill-after=10s','900s',@$argv];
 require_ok(json_equal(json_file("$d/wrapper-argv.json"),$wrapper),"wrapper $name");
 require_ok(trim_file("$d/cwd") eq $repo && trim_file("$d/status") eq '0' && trim_file("$d/capture.status") eq '0' && read_bytes("$d/capture.stderr") eq '',"cwd/status $name");
 my $cap=json_file("$d/capture.json");require_ok($cap->{capture} eq 'complete' && $cap->{outcome} eq 'ok' && $cap->{status} eq '0' && $cap->{gnu_exit} eq '0' && !@{$cap->{errors}},"capture $name");
 my $time=trim_file("$d/time");require_ok($time=~/\A(\d+(?:\.\d+)?)\t(\d+(?:\.\d+)?)\t(\d+(?:\.\d+)?)\t(\d+)\t0\z/,"GNU time $name");
 my @time=($1,$2,$3,$4);for my $pair([wall_s=>0],[user_s=>1],[system_s=>2],[rss_kib=>3]){require_ok($cap->{$pair->[0]} eq $time[$pair->[1]],"raw/capture metrics $name");}
 for my $pair([start_ns=>'start.ns'],[end_ns=>'end.ns'],[start_utc=>'start.utc'],[end_utc=>'end.utc'],[load_before=>'load.before'],[load_after=>'load.after']){
  require_ok($cap->{$pair->[0]} eq trim_file("$d/$pair->[1]"),"raw/capture $name $pair->[0]");
 }
 for(qw(load_before load_after)){require_ok($cap->{$_}=~/\A\d+\.\d+ \d+\.\d+ \d+\.\d+\z/,"load $name");}
 my $s=Math::BigInt->new($cap->{start_ns});my $end=Math::BigInt->new($cap->{end_ns});
 require_ok($s->bcmp($prior_end)>=0 && $end->bcmp($s)>=0,"sequential timestamps $name");$prior_end=$end;
 require_ok($cap->{wall_s}<900 && $cap->{rss_kib}<4194304,"resource bounds $name");
 require_ok(read_bytes("$d/stderr") eq '',"unexpected stderr $name") if $i>=2;
 require_ok(read_bytes("$d/stdout") eq '',"unexpected workload stdout $name") if $i<4;
 push @phases,{name=>$name,%$cap,argv=>$argv};
}
my @stats;
for my $name(@phase_names[0,1]){
 my $raw=read_bytes("$e/$name/stderr");require_ok($raw=~/\ALKJSCRIPT_STATS (\{[^\n]+\})\n\z/,"stats raw $name");
 my $stats=$j->decode($1);require_ok(json_equal($stats,json_file("$e/$name/stats.json")),"stats JSON $name");
 require_ok($stats->{route} eq 'compiler' && $stats->{stopped_stage} eq 'done' && $stats->{inline_accepted}==1 && $stats->{inline_rejected}==0 && $stats->{inline_attempts}==1,"selected inline $name");
 for(qw(build_reused front_reused checked_reused emitted_reused)){require_ok($stats->{$_}==0,"store reuse $name $_");}
 require_ok(read_bytes("$e/$name/validation.stdout") eq '' && read_bytes("$e/$name/validation.stderr") eq '',"construction validator output $name");
 push @stats,$stats;
}
for(qw(front_definitions front_functions ir_functions ir_instructions inline_calls inline_functions inline_accepted inline_rejected)){require_ok($stats[0]{$_}==$stats[1]{$_},"construction identity metric $_");}
require_ok($stats[0]{emitted_bytes}==$artifacts{ordinary64}{bytes} && $stats[1]{emitted_bytes}==$artifacts{profile64}{bytes} && $stats[1]{profile_ir_bytes}==$artifacts{'selected-ir'}{bytes},"construction emitted sizes");

my @roundtrips;
for my $base(qw(profile.lkjprof selected.lkjir)){
 my $m=metadata("$e/artifacts/$base.meta");
 require_ok(index($m->{compressed_path},"$e/artifacts/")==0 && index($m->{original_path},"$r/")==0,"preservation paths");
 same_hash($m->{compressed_path},$m->{compressed_sha256},$m->{compressed_bytes});
 same_hash($m->{original_path},$m->{original_sha256},$m->{original_bytes});
 require_ok(trim_file("$e/artifacts/$base.gzip.status") eq '0' && read_bytes("$e/artifacts/$base.roundtrip.status") eq "0\n0\n","recorded roundtrip status");
 for(qw(gzip gunzip sha256)){require_ok(read_bytes("$e/artifacts/$base.$_.stderr") eq '',"preservation stderr");}
 require_ok(read_bytes("$e/artifacts/$base.roundtrip.sha256") eq "$m->{original_sha256}  -\n","recorded roundtrip digest");
 my $z=IO::Uncompress::Gunzip->new($m->{compressed_path},MultiStream=>1) or die "artifact gunzip: $GunzipError\n";
 open my $raw,'<:raw',$m->{original_path} or die "raw artifact: $!\n";my $sha=Digest::SHA->new(256);my $nbytes=0;
 while(1){my $n=$z->read(my $piece,1048576);die "artifact decode: $GunzipError\n" if $n<0;last if !$n;
  my $other='';while(length($other)<$n){my $got=read($raw,my $part,$n-length($other));require_ok(defined($got)&&$got>0,"short held original");$other.=$part;}
  require_ok($piece eq $other,"compressed/held byte mismatch $base");$sha->add($piece);$nbytes+=$n;
 }
 my $extra=read($raw,my $tail,1);require_ok(defined($extra)&&$extra==0,"held original tail");
 close$raw or die "raw close";$z->close or die "decode close";
 require_ok($nbytes==$m->{original_bytes} && $sha->hexdigest eq $m->{original_sha256},"independent roundtrip length/digest");
 push @roundtrips,{%$m,independent_exact_byte_comparison=>JSON::PP::true};
}

my $raw=read_bytes($sample);my $raw_sha=sha256_hex($raw);my $length=length($raw);
my $MAX=unpack('Q<',"\xff"x8);
sub u64 { my($at)=@_;require_ok($at>=0 && $at+8<=$length,"binary word extent");return unpack('Q<',substr($raw,$at,8)); }
sub block_end { my($at,$count,$width)=@_;require_ok($at<=$length && $count<=int(($length-$at)/$width),"binary section extent");return $at+$count*$width; }
require_ok(substr($raw,0,8) eq 'LKJPROF4' && u64(16)==1000000 && u64(72)==0 && u64(152)==0,"profile header");
my ($N,$S,$KO,$KN,$CO,$lo,$hi,$XO,$C,$MO,$F,$U)=map{u64($_)}(8,24,32,40,48,56,64,112,120,128,136,144);
require_ok(unpack('H*',substr($raw,80,32)) eq $artifacts{'selected-ir'}{sha256},"profile companion digest");
require_ok($lo<$hi,"code range");my $name_at=block_end(160,$N,64);my $names_begin=$name_at;
my(@symbols,@counts,@map,@copy);my($self_total,$incl_total)=(Math::BigInt->new(0),Math::BigInt->new(0));my $last_end=$lo;
for my $i(0..$N-1){
 my @v=unpack('Q<8',substr($raw,160+$i*64,64));my($a,$b,$self,$inc,$seen,$off,$len,$frame)=@v;
 require_ok($a>=$lo && $a<$b && $b<=$hi && (!$i || $a==$last_end) && $off==$name_at &&
  (!$frame || ($frame>=$a && $frame<=$b)) && $self<=$S && $inc<=$S && $seen<=$S,"symbol $i");
 $name_at=block_end($off,$len,1);my $name=decode('UTF-8',substr($raw,$off,$len),FB_CROAK|LEAVE_SRC);
 push @symbols,{start=>$a,end=>$b,self=>$self,inclusive=>$inc,seen=>$seen,frame=>$frame,name=>$name};
 $self_total->badd(str($self));$incl_total->badd(str($inc));$last_end=$b;
}
require_ok((!$N || $last_end==$hi) && $KO==$name_at && $KN>0 && $CO==block_end($KO,$KN,16),"symbol/kind framing");
my @intervals=(0)x16;my $previous=0;
for(my $i=0;$i<$KN;$i++){
 my($start,$kind)=unpack('Q<2',substr($raw,$KO+16*$i,16));
 require_ok($start>=$lo && $start<$hi && ($i?$start>$previous:$start==$lo) && $kind<16,"kind interval $i");
 $intervals[$kind]++;$previous=$start;
}
@counts=unpack('Q<16',substr($raw,$CO,128));my $T=sum_exact(@counts);my $gap=Math::BigInt->new(str($S))->bsub($T);
require_ok(($gap->is_zero || $gap->is_one) && $self_total->bcmp($T)<=0,"serial/self/completed invariant");
require_ok($XO==block_end($CO,16,8) && $MO==block_end($XO,$C,72) && block_end($MO,$F,16)==$length,"copy/map framing");
my %selected_symbol;
for my $i(0..$F-1){
 my($index,$symbol)=unpack('Q<2',substr($raw,$MO+16*$i,16));require_ok($index==$i,"map index");
 if($symbol!=$MAX){require_ok($symbol<$N && !exists$selected_symbol{$symbol},"map symbol");$selected_symbol{$symbol}=$i;}
 push @map,$symbol;
}
my @roles=('invalid','local','typed-place','constructor-field','array-element','abi-incoming','parameter-local','abi-result','vector-element','result-variant-field','repeated-array-element','box-payload','referent','view-header');
my @kind_names=('unknown','inline copy','checks','call instruction','frame load/store','other memory','arithmetic / comparison / conversion','branch','allocator','runtime','register-to-register move','address formation (LEA)','immediate constant','argument setup / result capture','prologue / epilogue','return');
my %allowed=map{$_=>1}qw(1:1:1 1:1:2 1:1:3 1:1:4 1:1:7 1:1:8 1:1:10 1:1:11 1:2:1 1:8:9 1:12:1 1:13:13 2:5:6);
my $mask=255|(255<<8)|(255<<16)|(7<<24)|(7<<32);
my(%width,%role,%joint);my $mapped=Math::BigInt->new(0);my($nz,$ki,$last_copy_end)=(0,0,$lo);
sub bucket_add {
 my($h,$key,$samples)=@_;$h->{$key}//=[0,0,Math::BigInt->new(0)];$h->{$key}[0]++;$h->{$key}[1]++ if$samples;$h->{$key}[2]->badd(str($samples));
}
for my $i(0..$C-1){
 my @v=unpack('Q<9',substr($raw,$XO+72*$i,72));my($a,$b,$n,$f,$ins,$sub,$ty,$bytes,$tag)=@v;
 require_ok($a>=$last_copy_end && $a<$b && $b<=$hi && $f<$F && $bytes>0,"copy row $i");
 my($origin,$src,$dst,$sf,$df)=($tag&255,($tag>>8)&255,($tag>>16)&255,($tag>>24)&7,($tag>>32)&7);
 require_ok(($tag&~$mask)==0 && $allowed{"$origin:$src:$dst"} && ($src==2||!$sf) && ($dst==2||!$df) &&
  ($origin==2 ? $ins==$MAX : $ins!=$MAX),"copy tags/sentinel $i");
 my $symbol=$map[$f];require_ok($symbol!=$MAX && $a>=$symbols[$symbol]{start} && $b<=$symbols[$symbol]{end},"copy owner $i");
 $ki++ while $ki+1<$KN && u64($KO+16*($ki+1))<=$a;
 require_ok(u64($KO+16*$ki+8)==1 && ($ki+1==$KN || u64($KO+16*($ki+1))>=$b),"copy kind coverage $i");
 my $rk=join("\t",$origin==1?'body':'incoming-parameter',$roles[$src],$roles[$dst],$sf,$df);
 bucket_add(\%width,str($bytes),$n);bucket_add(\%role,$rk,$n);bucket_add(\%joint,join("\t",$bytes,$rk),$n);
 $mapped->badd(str($n));$nz++ if$n;
 push @copy,{start=>$a,end=>$b,samples=>$n,function=>$f,instruction=>$ins,subsite=>$sub,type=>$ty,width=>$bytes,source=>$roles[$src],destination=>$roles[$dst],source_flags=>$sf,destination_flags=>$df,symbol=>$symbol};
 $last_copy_end=$b;
}
my $D=$mapped->copy->badd(str($U));my $inflight=$D->copy->bsub(str($counts[1]));
require_ok(($inflight->is_zero||$inflight->is_one) && ($inflight->is_zero||$gap->is_one),"copy/K interrupted-handler invariant");
for my $h(\%width,\%role,\%joint){
 my($sites,$nonzero,$sum)=(0,0,Math::BigInt->new(0));for(values%$h){$sites+=$_->[0];$nonzero+=$_->[1];$sum->badd($_->[2]);}
 require_ok($sites==$C && $nonzero==$nz && $sum->bcmp($mapped)==0,"joint complete coverage");
}
sub row_tsv { return join("\t",@_)."\n"; }
my %rebuilt;
$rebuilt{'kinds.tsv'}="kind\tname_json\tsamples\tintervals\n";
for(0..15){$rebuilt{'kinds.tsv'}.=row_tsv($_,$j->encode($kind_names[$_]),$counts[$_],$intervals[$_]);}
$rebuilt{'functions.tsv'}="symbol\tstart\tend\tself_samples\tinclusive_samples\tlast_seen_serial\tframe_start\tselected_function\tname_json\n";
for my $i(0..$N-1){my $s=$symbols[$i];$rebuilt{'functions.tsv'}.=row_tsv($i,(map{$s->{$_}}qw(start end self inclusive seen frame)),exists$selected_symbol{$i}?$selected_symbol{$i}:'none',$j->encode($s->{name}));}
$rebuilt{'function-map.tsv'}="selected_function\tsymbol\tencoded_symbol_u64\temitted\n";
for(0..$F-1){$rebuilt{'function-map.tsv'}.=row_tsv($_,$map[$_]==$MAX?'none':$map[$_],$map[$_],$map[$_]==$MAX?'no':'yes');}
$rebuilt{'widths.tsv'}="width_bytes\tsites\tnonzero_sites\tsamples\n";
for(sort{$a<=>$b}keys%width){$rebuilt{'widths.tsv'}.=row_tsv($_,$width{$_}[0],$width{$_}[1],$width{$_}[2]->bstr);}
$rebuilt{'widths.tsv'}.=row_tsv('unmapped','NA','NA',$U);
$rebuilt{'roles.tsv'}="origin\tsource\tdestination\tsource_flags\tdestination_flags\tsites\tnonzero_sites\tsamples\n";
for(sort keys%role){$rebuilt{'roles.tsv'}.=row_tsv($_,$role{$_}[0],$role{$_}[1],$role{$_}[2]->bstr);}
$rebuilt{'roles.tsv'}.=row_tsv('unmapped','unknown','unknown','NA','NA','NA','NA',$U);
$rebuilt{'width-roles.tsv'}="width_bytes\torigin\tsource\tdestination\tsource_flags\tdestination_flags\tsites\tnonzero_sites\tsamples\n";
for(sort{my($aw,$ar)=split/\t/,$a,2;my($bw,$br)=split/\t/,$b,2;$aw<=>$bw||$ar cmp $br}keys%joint){$rebuilt{'width-roles.tsv'}.=row_tsv($_,$joint{$_}[0],$joint{$_}[1],$joint{$_}[2]->bstr);}
$rebuilt{'width-roles.tsv'}.=row_tsv('unmapped','unknown','unknown','unknown','NA','NA','NA','NA',$U);
my %rebuilt_hash;
for(sort keys%rebuilt){require_ok(read_bytes("$e/accounting/$_") eq $rebuilt{$_},"independent table bytes: $_");write_new("reconstructed/$_",$rebuilt{$_});$rebuilt_hash{$_}=sha256_hex($rebuilt{$_});}

# Independently collect canonical declaration names only, never admit/execute IR.
my(@fn_names,@type_names);
open my $ir,'<:raw',$companion or die "companion: $!\n";
while(my $line=<$ir>){
 if($line=~/\A(fn|type) ("(?:[^"\\]|\\.)*") /){my($k,$q)=($1,$2);my$name=JSON::PP->new->utf8->decode($q);push @{$k eq 'fn'?\@fn_names:\@type_names},$name;}
}
close$ir or die "companion close";
require_ok(@fn_names==$F,"selected function declaration count");
my $detail=read_bytes("$e/06-copy-detail/stdout");require_ok(length($detail)<=1048576,"detail byte bound");
my @dl=split/\n/,decode('UTF-8',$detail,FB_CROAK|LEAVE_SRC);
my $header=shift@dl;my$scope=shift@dl;my$tail=pop@dl;
my $comp_sha=$artifacts{'selected-ir'}{sha256};
my $want_header="copy-sites version=4 profile=$raw_sha companion=$comp_sha companion-check=checked-ir serial=$S total-samples=$T copy-samples=$counts[1] mapped=".$mapped->bstr." unmapped=$U in-flight=".$inflight->bstr." sites=$C order=samples-desc/function/instruction/subsite/start/end";
require_ok($header eq $want_header,"detail header exact counters");
require_ok($scope eq 'scope=existing-kind-copy runtime=excluded variable-width-copy=excluded replace=excluded scalar-frame-address-work=excluded counters=sampled-native-observation',"detail exclusions");
require_ok(@dl==200 && $tail eq "continuation cursor=4:$raw_sha:$comp_sha:200 rows=200 next=200 total-sites=$C","sole detail page and continuation");
my @order=sort{$copy[$b]{samples}<=>$copy[$a]{samples} ||
 $copy[$a]{function}<=>$copy[$b]{function} || $copy[$a]{instruction}<=>$copy[$b]{instruction} ||
 $copy[$a]{subsite}<=>$copy[$b]{subsite} || $copy[$a]{start}<=>$copy[$b]{start} || $copy[$a]{end}<=>$copy[$b]{end}}0..$#copy;
my $detail_samples=0;
for my $i(0..199){
 my $v=$copy[$order[$i]];my $instruction=$v->{instruction}==$MAX?'parameter':$v->{instruction};
 my $prefix="copy function=$v->{function} instruction=$instruction subsite=$v->{subsite} symbol=$v->{symbol} samples=$v->{samples} type=$v->{type} width=$v->{width} source=$v->{source} destination=$v->{destination} source-flags=$v->{source_flags} destination-flags=$v->{destination_flags} start=$v->{start} end=$v->{end} name=";
 require_ok(index($dl[$i],$prefix)==0,"detail row raw coordinates/order $i");
 my $rest=substr($dl[$i],length($prefix));require_ok($rest=~/\A("(?:[^"\\]|\\.)*") type-name=("(?:[^"\\]|\\.)*")\z/,"detail names $i");
 my($fn,$ty)=($j->decode($1),$j->decode($2));
 require_ok($v->{type}<@type_names && $fn eq $fn_names[$v->{function}] && $ty eq $type_names[$v->{type}],"detail names/companion $i");
 $detail_samples+=$v->{samples};
}
my @summary=split/\n/,decode('UTF-8',read_bytes("$e/05-summary/stdout"),FB_CROAK|LEAVE_SRC);
require_ok(shift(@summary) eq "CPU samples: $T; requested period 1000 us; inclusive counts each function once per sample","summary total");
for my $mode(qw(self inclusive)){
 require_ok(shift(@summary) eq ($mode eq 'self'?'Top self time':'Top inclusive time') && shift(@summary) eq 'self% incl% samples function',"summary framing");
 my @top=sort{$symbols[$b]{$mode}<=>$symbols[$a]{$mode}||$a<=>$b}grep{$symbols[$_]{$mode}>0}0..$#symbols;splice@top,7 if@top>7;
 for my $index(@top){
  my $line=shift@summary;require_ok($line=~/\A(\d+\.\d) (\d+\.\d) (\d+) (.*)\z/,"summary row");
  my($number,$name)=($3,$4);my$want=$symbols[$index]{name};$want=~s/([\x00-\x1f\x7f\x{2028}\x{2029}])/sprintf('\\u{%x}',ord($1))/ge;
  require_ok($number==$symbols[$index]{$mode} && $name eq $want,"summary ordered integer/name");
 }
}
require_ok(shift(@summary) eq 'Emitted instruction kinds (self samples)' && shift(@summary) eq 'kind self% samples class',"kind summary framing");
for my $i(0..15){my$l=shift@summary;require_ok($l=~/\Akind \d+\.\d (\d+) (.*)\z/,"kind summary row");require_ok($1==$counts[$i] && $2 eq $kind_names[$i],"kind summary integer/name");}
require_ok(!@summary,"summary tail");
my $a=json_file("$e/accounting/accounting.json");
my %counters=(serial_S=>str($S),completed_T=>str($T),copy_kind_K=>str($counts[1]),mapped=>str($mapped->bstr),unmapped=>str($U),detail_D=>str($D->bstr),
 serial_minus_completed=>str($gap->bstr),detail_minus_copy_in_flight=>str($inflight->bstr),function_self_total=>str($self_total->bstr),
 function_unattributed=>str(Math::BigInt->new($T)->bsub($self_total)->bstr),overlapping_inclusive_sum_not_total=>str($incl_total->bstr));
for(keys%counters){require_ok($a->{counters}{$_} eq $counters{$_},"accounting JSON counter $_");}
my %tables=(file_bytes=>$length,header_bytes=>160,symbol_count=>$N,symbol_row_bytes=>64,symbols_offset=>160,symbols_bytes=>64*$N,names_offset=>$names_begin,names_bytes=>$KO-$names_begin,
 kind_offset=>$KO,kind_count=>$KN,kind_row_bytes=>16,kind_bytes=>16*$KN,counters_offset=>$CO,counters_count=>16,counters_bytes=>128,
 copy_offset=>$XO,C=>$C,nonzero_copy_sites=>$nz,copy_row_bytes=>72,copy_bytes=>72*$C,map_offset=>$MO,F=>$F,map_row_bytes=>16,map_bytes=>16*$F,
 encoded_increment_over_otherwise_equal_v3=>80+72*$C+16*$F,alignment_bytes=>0,code_start=>$lo,code_end=>$hi);
for(keys%tables){require_ok($a->{tables}{$_} eq str($tables{$_}),"accounting JSON table $_");}
my $account_manifest=read_bytes("$e/accounting/manifest.sha256");my%listed;
for(split/\n/,$account_manifest){require_ok(/\A([0-9a-f]{64})  ([A-Za-z0-9_.-]+)\z/,"account manifest");my($hash,$file)=($1,$2);require_ok(!exists$listed{$file},"duplicate account artifact");$listed{$file}=1;same_hash("$e/accounting/$file",$hash);}
require_ok(keys(%listed)==7 && $listed{'accounting.json'},"account manifest population");
my $eafter=inventory($e,files_under($e));my $rafter=inventory($r,files_under($r));
require_ok($eafter eq $ei && $rafter eq $ri,"cohort artifact changed during audit");
write_new('evidence-after.sha256',$eafter);write_new('held-after.sha256',$rafter);
my @costs;
for my $pair([0,1,'construction'],[2,3,'execution']){
 my($o,$q,$name)=@$pair;push@costs,{kind=>$name,ordinary=>$phases[$o],profile=>$phases[$q],
 wall_ratio=>sprintf('%.9f',$phases[$q]{wall_s}/$phases[$o]{wall_s}),
 cpu_ratio=>sprintf('%.9f',($phases[$q]{user_s}+$phases[$q]{system_s})/($phases[$o]{user_s}+$phases[$o]{system_s})),
 rss_ratio=>sprintf('%.9f',$phases[$q]{rss_kib}/$phases[$o]{rss_kib})};
}
my $result={status=>'accepted',scope=>'independent-offline-cohort-2-audit',release_commit=>$release_commit,release_sha256=>$release_sha,
 evidence_files=>str(scalar@$ep),held_files=>str(scalar@$rp),witness_files=>str(scalar@$source_paths),current_source_files=>str($main_count),
 producer=>$p,source_before_sha256=>sha256_hex($before),artifacts=>\%artifacts,roundtrips=>\@roundtrips,phases=>\@phases,cost_pairs=>\@costs,
 construction_stats=>\@stats,profile_counters=>\%counters,profile_tables=>{map{$_=>str($tables{$_})}sort keys%tables},
 reconstructed_table_sha256=>\%rebuilt_hash,detail=>{rows=>'200',bytes=>str(length($detail)),samples_in_visible_rows=>str($detail_samples),sha256=>sha256_hex($detail),continuation=>$tail,
 exact_global_sorted_prefix=>JSON::PP::true,declared_type_names_compared=>JSON::PP::true},
 copy_sample_percent=>sprintf('%.9f',100*$counts[1]/$T),limitations=>[
 'One fixed-order ordinary/profile pair; no confidence interval, causal optimization gain, fresh Rust ratio or old-sample reconstruction.',
 'Profile mode includes existing sampler, joint-table collection and selected-IR publication; this pair does not isolate incremental copy-table overhead.',
 'Only the sole captured 200-row detail page was read; full raw tables were accounted without requesting more detail.',
 'No semantic eligibility, ownership/loan recovery, SROA opportunity, sample-times-width traffic or removable-time claim.',
 'Official IR/operand/layout admission rests on the retained successful official command; this audit checked binary structure, exact accounting and declaration names without rerunning admission.',
 'Hashes and before/after inventories observe retained bytes, not freshness or a kernel sealing guarantee.'
]};
write_new('acceptance.json',$pretty->encode($result));
print $pretty->encode({status=>'accepted',evidence_files=>$result->{evidence_files},held_files=>$result->{held_files},
 counters=>\%counters,tables=>$result->{profile_tables},detail_samples=>str($detail_samples),rebuilt_tables=>\%rebuilt_hash});
