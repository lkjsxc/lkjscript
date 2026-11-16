use strict;use warnings;
use File::Find;use Fcntl qw(:DEFAULT :mode);use Digest::SHA qw(sha256_hex);use JSON::PP;
my $p='/home/coder/workspace/lkjscript-compiler-scalar-20261116/target/compiler-scalar-validation-package-1';
sub read_bytes{my($f)=@_;open my $h,'<:raw',$f or die "$f: $!";local $/;my $b=<$h>;close $h;return defined($b)?$b:'';}
die "package not successful\n" unless read_bytes("$p/capture/status") eq "0\n";
my $v=decode_json(read_bytes("$p/verification.json"));die "verification counts\n" unless $v->{archived_records}==386&&$v->{copied_proposal_records}==14&&$v->{exact_gateway_exports}==2&&$v->{source_inputs_unchanged};
my @rows;my $dirs=0;
find({no_chdir=>1,follow=>0,wanted=>sub{my$f=$File::Find::name;my@s=lstat($f);die"lstat $f\n"unless@s;if(S_ISDIR($s[2])){$dirs++;return;}die"nonregular $f\n"unless S_ISREG($s[2]);my$r=substr($f,length($p)+1);die"seal destinations already present\n"if$r eq 'package-inventory.json'||$r eq 'manifest.sha256';open my$h,'<:raw',$f or die"$f: $!";my$d=Digest::SHA->new(256);$d->addfile($h);close$h;push@rows,{path=>$r,bytes=>$s[7],sha256=>$d->hexdigest,mode=>sprintf('%04o',$s[2]&07777)};}},$p);
@rows=sort{$a->{path}cmp$b->{path}}@rows;
my$total=0;$total+=$_->{bytes}for@rows;
my$j=JSON::PP->new->canonical->pretty->encode({schema=>1,kind=>'frozen scalar validation package inventory',files=>\@rows,described_files=>scalar(@rows),described_bytes=>$total,final_files=>scalar(@rows)+2,directories_including_root=>$dirs,self_entries=>['package-inventory.json','manifest.sha256'],note=>'manifest.sha256 covers every regular package file except itself, including this inventory; its own hash is the external package pin'});
sub save{my($f,$b)=@_;sysopen my$h,$f,O_WRONLY|O_CREAT|O_EXCL,0600 or die"$f: $!";print{$h}$b or die"$f: $!";close$h or die"$f close: $!";}
save("$p/package-inventory.json",$j);my$jh=sha256_hex($j);
my$m=join('',map{"$_->{sha256}  $_->{path}\n"}@rows)."$jh  package-inventory.json\n";
save("$p/manifest.sha256",$m);
print"package=$p\nfiles=".(scalar(@rows)+2)." bytes=".($total+length($j)+length($m))." directories=$dirs\n";
print"manifest_records=".(scalar(@rows)+1)." manifest_sha256=".sha256_hex($m)."\ninventory_sha256=$jh\n";
