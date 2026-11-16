use strict;use warnings;use File::Find;use Fcntl qw(:DEFAULT :mode);use Digest::SHA qw(sha256_hex);use JSON::PP;
my$p='/home/coder/workspace/lkjscript/target/compiler-scalar-cohort-package-1';
sub raw{my$f=shift;open my$h,'<:raw',$f or die"$f: $!";local$/;my$b=<$h>;close$h;return defined($b)?$b:'';}
die"packaging/correction status\n"unless raw("$p/capture/status")eq"0\n"&&raw("$p/capture/correction.status")eq"0\n";
my$cb=raw("$p/verification-corrected.json");die"corrected report pin\n"unless sha256_hex($cb)eq'1df0de7ff739413788067a4c131a7d74889f3a7a2b56784baeb349052e7fd8da';
my$v=decode_json($cb);die"verification count\n"unless$v->{raw}{members}==154&&$v->{source_archive}{members}==172&&$v->{proposal_copy_files}==18&&@{$v->{metadata_correction}{corrected_fields}}==7;
my@rows;my$dirs=0;
find({no_chdir=>1,follow=>0,wanted=>sub{my$f=$File::Find::name;my@s=lstat($f);die"lstat $f\n"unless@s;if(S_ISDIR($s[2])){$dirs++;return;}die"nonregular $f\n"unless S_ISREG($s[2]);my$r=substr($f,length($p)+1);die"seal already exists\n"if$r eq'package-inventory.json'||$r eq'manifest.sha256';open my$h,'<:raw',$f or die"$f: $!";my$d=Digest::SHA->new(256);$d->addfile($h);close$h;push@rows,{path=>$r,bytes=>$s[7],sha256=>$d->hexdigest,mode=>sprintf('%04o',$s[2]&07777)};}},$p);
@rows=sort{$a->{path}cmp$b->{path}}@rows;my$total=0;$total+=$_->{bytes}for@rows;
my$j=JSON::PP->new->canonical->pretty->encode({schema=>1,kind=>'frozen scalar cohort recovery package',files=>\@rows,described_files=>scalar(@rows),described_bytes=>$total,final_files=>scalar(@rows)+2,directories_including_root=>$dirs,self_entries=>['package-inventory.json','manifest.sha256'],note=>'manifest covers every regular package file except itself, including this inventory; external manifest hash is the package pin'});
sub save{my($f,$b)=@_;sysopen my$h,$f,O_WRONLY|O_CREAT|O_EXCL,0600 or die"$f: $!";print{$h}$b or die"$f: $!";close$h or die"$f close: $!";}
save("$p/package-inventory.json",$j);my$jh=sha256_hex($j);my$m=join('',map{"$_->{sha256}  $_->{path}\n"}@rows)."$jh  package-inventory.json\n";save("$p/manifest.sha256",$m);
print"package=$p\nfiles=".(scalar(@rows)+2)." bytes=".($total+length($j)+length($m))." directories=$dirs\nmanifest_records=".(scalar(@rows)+1)." manifest_sha256=".sha256_hex($m)."\ninventory_sha256=$jh\n";
