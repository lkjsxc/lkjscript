#!/usr/bin/env perl
use strict;
use warnings;
use Cwd qw(abs_path);
use Fcntl qw(:DEFAULT :mode);
use File::Find;
use Digest::SHA qw(sha256_hex sha1_hex);
use JSON::PP;
use Archive::Tar;
use IO::Uncompress::Gunzip qw(gunzip $GunzipError);
my$repo='/home/coder/workspace/lkjscript';
my$base="$repo/target";
my$out="$base/compiler-reference-capture-final-recovery-1";
my$revision='d84cac6edcee59e2c2ad631fb6ca15d63340a1c8';
my$prefix='bench/compiler-reference-capture-data';
my$comparison='compiler-reference-capture-comparison-2';
my$outer='compiler-reference-capture-comparison-launch-2';
my$measurement='compiler-reference-capture-measurement-2';
my$trial='compiler-reference-capture-trial-preparation-2';
my$acceptance='compiler-reference-capture-comparison-acceptance-2';
my$recovery='compiler-reference-capture-comparison-recovery-2';
my$preparation='compiler-reference-capture-measurement-preparation-1';
my@scopes=($comparison,$outer,$measurement,$trial,$acceptance,$recovery,$preparation);
my%scope=map{$_=>1}@scopes;
my$config={compiler_input=>{revision=>'c887b7c5aca7bb80d70ca26d7dfdf1a44213a163'}};
my$json=JSON::PP->new->canonical->pretty;
die"no arguments accepted\n" if@ARGV;
sub read_regular {
 my($p)=@_;my@b=lstat$p;
 die"unsafe input $p\n" unless@b&&S_ISREG($b[2])&&$b[3]==1&&$b[4]==$>&&$b[7]<=134217728;
 sysopen my$f,$p,O_RDONLY|O_NOFOLLOW or die"open $p: $!\n";binmode$f;
 my@o=stat$f;die"input identity changed\n" unless$o[0]==$b[0]&&$o[1]==$b[1]&&$o[2]==$b[2]&&$o[7]==$b[7];
 local$/;my$x=<$f>;$x='' unless defined$x;my@a=stat$f;close$f or die$!;
 my@n=lstat$p;die"input changed/short\n" unless length($x)==$b[7]&&$a[0]==$b[0]&&$a[1]==$b[1]&&$a[2]==$b[2]&&$a[7]==$b[7]&&$a[9]==$b[9]&&$a[10]==$b[10]&&@n&&$n[0]==$b[0]&&$n[1]==$b[1]&&$n[2]==$b[2]&&$n[7]==$b[7];
 return$x;
}
sub write_json {
 my($n,$v)=@_;die"unsafe output name\n" unless$n=~/\A[a-z][a-z0-9.-]*\z/;
 sysopen my$f,"$out/$n",O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW,0600 or die"new $n: $!\n";print{$f}$json->encode($v)or die$!;close$f or die$!;
}
sub pin {return{bytes=>length($_[0]),sha256=>sha256_hex($_[0])};}
sub normalized_source {
 my($p)=@_;$p=~s{\A\Q$repo\E/}{};$p=~s{\Atarget/}{};my($root)=split m{/},$p;
 return undef unless$scope{$root};return$p;
}
my$metadata_bytes=read_regular("$out/metadata-inventory.json");
die"metadata pin\n" unless sha256_hex($metadata_bytes)eq'601d6cb24c4e77e26811e51128f466b4c50f3f63558e800d596d1178ade2064e';
my$initial=decode_json($metadata_bytes);
die"wrong scope list\n" unless$json->encode($initial->{scopes})eq$json->encode(\@scopes);
my%initial=map{$_->{path}=>$_}@{$initial->{entries}};
die"wrong inventory population\n" unless keys(%initial)==4468+141;
sub scan {
 my@rows;
 for my$s(@scopes){my$root="$base/$s";my@r=lstat$root;die"unsafe root\n" unless@r&&S_ISDIR($r[2])&&$r[4]==$>&&abs_path($root)eq$root;
 find({no_chdir=>1,follow=>0,wanted=>sub{
 my$p=$File::Find::name;my@v=lstat$p;die"missing/foreign member\n" unless@v&&$v[4]==$>;
 my$t=S_ISDIR($v[2])?'directory':S_ISREG($v[2])?'file':'special';die"special/hardlinked/privileged member\n" if$t eq'special'||($t eq'file'&&$v[3]!=1)||($v[2]&07000);
 my$r=substr($p,length($base)+1);die"unknown source path\n" unless exists$initial{$r};
 push@rows,{path=>$r,type=>$t,mode=>$v[2]&07777,mode_octal=>sprintf('%04o',$v[2]&07777),bytes=>$t eq'file'?$v[7]:0,uid=>$v[4],gid=>$v[5],nlink=>$v[3],device=>$v[0],inode=>$v[1],mtime=>$v[9],ctime=>$v[10]};
 }},$root);}
 return[sort{$a->{path}cmp$b->{path}}@rows];
}
die"source metadata drift before mapping\n" unless$json->encode(scan())eq$json->encode($initial->{entries});
my(@git_commands,%git_bytes,%git_coordinates);
sub git_blob {
 my($rev,$path)=@_;
 safe_name($path,0);
 die"unapproved recovery revision\n" unless$rev eq$revision||$rev eq'7d0731e99aeaf241c4334fc5ff065a341fb388c8'||$rev eq'e5294c339d9aefbbafc2e951c997f4fefabcc9ad';
 if($rev eq$revision){die"unapproved retained prefix\n" unless$path=~m{\A\Q$prefix\E/(?:comparison-(?:retention\.json|recovery/|acceptance/)|setup2(?:-artifacts-copy\.json|/)|trial-preparation(?:-copy\.json|-2/)|preparation-copy\.json|measurement-preparation/)};}
 else{die"unapproved seed path\n" unless$path eq'seed/lkjscript.gz';}
 my$key="$rev:$path";return($git_bytes{$key},$git_coordinates{$key})if exists$git_bytes{$key};
 my@a=('/usr/bin/git','-C',$repo,'cat-file','blob',$key);push@git_commands,\@a;
 open my$f,'-|',@a or die$!;binmode$f;local$/;my$b=<$f>;$b=''unless defined$b;close$f or die"Git read failed\n";die"large retained blob\n" if length$b>134217728;
 my$c={kind=>'git-blob',revision=>$rev,path=>$path,git_blob=>sha1_hex("blob ".length($b)."\0".$b),%{pin($b)}};
 $git_bytes{$key}=$b;$git_coordinates{$key}=$c;return($b,$c);
}
my(%providers,%provider_bytes,%provider_keys,%assignment,%direct_expectation);
my@provider_order;
sub provider {
 my($c,$bytes)=@_;my$key=$json->encode($c);return$provider_keys{$key}if exists$provider_keys{$key};
 my$id=sprintf('p%05d',scalar(@provider_order)+1);my$v={%$c,%{pin($bytes)}};$providers{$id}=$v;$provider_bytes{$id}=$bytes;$provider_keys{$key}=$id;push@provider_order,$id;return$id;
}
sub assign {
 my($path,$id)=@_;return unless defined$path;die"unknown assigned source\n" unless exists$initial{$path}&&$initial{$path}{type}eq'file';
 die"conflicting recovery assignment $path\n" if exists$assignment{$path}&&$assignment{$path}ne$id;
 $assignment{$path}=$id;
}
sub direct {
 my($source,$destination,$expected)=@_;my$p=normalized_source($source);return unless defined$p;
 $destination=~s{\A\Q$repo\E/}{};
 my($b,$c)=git_blob($revision,$destination);
 die"retention record digest mismatch\n" if defined($expected->{sha256})&&sha256_hex($b)ne$expected->{sha256};
 my$n=defined$expected->{bytes}?$expected->{bytes}:$expected->{size};
 die"retention record size mismatch\n" if defined($n)&&length($b)!=$n;
 my$id=provider($c,$b);assign($p,$id);$direct_expectation{$p}={recorded_sha256=>$expected->{sha256},retained_destination=>$destination};
}
sub decode_blob {my($path)=@_;my($b,$c)=git_blob($revision,"$prefix/$path");return(decode_json($b),$c);}
my($copies)=decode_blob('comparison-retention.json');
for my$g(@{$copies->{packages}}){for my$r(@{$g->{files}}){direct($r->{source},$r->{destination},$r);}}
my($setup)=decode_blob('setup2-artifacts-copy.json');
for my$r(@{$setup->{records}}){direct($r->{source},"$setup->{destination}/$r->{path}",$r);}
my($trial_copies)=decode_blob('trial-preparation-copy.json');
for my$g(@{$trial_copies->{packages}}){next unless defined normalized_source($g->{source});for my$r(@{$g->{records}}){direct("$g->{source}/$r->{path}","$g->{destination}/$r->{path}",$r);}}
my($prep_copies)=decode_blob('preparation-copy.json');
for my$g(@{$prep_copies->{groups}}){next unless defined normalized_source($g->{source});for my$r(@{$g->{rows}}){direct("$g->{source}/$r->{path}","$g->{destination}/$r->{path}",$r);}}
sub gunzip_blob {
 my($rev,$path)=@_;my($b,$c)=git_blob($rev,$path);my$x='';gunzip(\$b=>\$x)or die"gzip recovery: $GunzipError\n";die"large decoded object\n" if length$x>134217728;
 return($x,{kind=>'gunzip-git-blob',source=>$c,%{pin($x)}});
}
my%seeds;
for my$s(qw(A B)){
 my$rev=$s eq'A'?'7d0731e99aeaf241c4334fc5ff065a341fb388c8':'e5294c339d9aefbbafc2e951c997f4fefabcc9ad';
 my($b,$c)=git_blob($rev,'seed/lkjscript.gz');my($x,$xc)=gunzip_blob($rev,'seed/lkjscript.gz');
 my$compressed=$s eq'A'?'505be947525d53b4db1bff6c85cb77622cb2f5dc7e7d7fe92b772ee3b8ffc3e2':'c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae';
 my$expanded=$s eq'A'?'6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc':'b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db';
 die"seed identity mismatch\n" unless sha256_hex($b)eq$compressed&&sha256_hex($x)eq$expanded;
 $seeds{$s}={compressed=>provider($c,$b),expanded=>provider($xc,$x)};
}
assign("$measurement/producer-A",$seeds{A}{expanded});
assign("$measurement/artifacts/baseline-A.gz",$seeds{A}{compressed});
sub ordinary_archive {
 my($path,$compressed,$expected_sha)=@_;my($b,$c)=git_blob($revision,$path);die"archive pin\n" unless sha256_hex($b)eq$expected_sha;
 my$live=read_regular("$repo/$path");die"retained archive differs from commit\n" unless$live eq$b;
 my$t=Archive::Tar->new;$t->read("$repo/$path",$compressed?1:0)or die"retained archive read failed\n";
 my(%members,%dirs);
 for my$m($t->get_files){die"special retained member\n" unless$m->is_file||$m->is_dir;my$p=safe_name($m->full_path,$m->is_dir);next if$p eq''&&$m->is_dir;
 die"duplicate member\n" if exists$members{$p}||exists$dirs{$p};if($m->is_dir){$dirs{$p}=$m->mode;next;}
 my$d=$m->get_content;die"privileged archive mode\n" if$m->mode&07000;
 $members{$p}=provider({kind=>'archive-member',archive=>$c,compression=>$compressed?'gzip':'none',member=>$p,archive_mode=>$m->mode},$d);
 }
 die"retained archive drift\n" unless read_regular("$repo/$path")eq$b;
 return(\%members,\%dirs,$c);
}
sub safe_name {
    my ($name, $directory) = @_;
    $name =~ s{\A\./}{};
    $name =~ s{/\z}{} if $directory;
    return '' if $directory && ($name eq '' || $name eq '.');
    die "unsafe archive/tree path\n" if $name eq '' || $name =~ m{\A/|\\|[\x00-\x1f\x7f]};
    for my $part (split m{/}, $name, -1) { die "unsafe path component\n" if $part eq '' || $part eq '.' || $part eq '..'; }
    return $name;
}
sub tar_octal {
    my ($field) = @_;
    $field =~ s/[\0 ]+\z//;
    $field =~ s/\A +//;
    die "unsupported raw tar number\n" unless $field =~ /\A[0-7]+\z/;
    return oct($field);
}
sub tar_text {
    my ($field) = @_;
    $field =~ /\A([^\0]*)\0*\z/s or die "nonzero raw tar string padding\n";
    return $1;
}
sub raw_source_members {
    my ($source) = @_;
    my (@members, @pax, $pending, $global, $ended);
    my $offset = 0;
    while ($offset + 512 <= length($source)) {
        my $header = substr($source, $offset, 512);
        if ($header eq "\0" x 512) {
            die "missing tar terminator or nonzero trailer\n" unless length($source) - $offset >= 1024
                && substr($source, $offset) eq "\0" x (length($source) - $offset);
            die "unconsumed per-file PAX path\n" if defined $pending;
            $ended = 1;
            last;
        }
        die "unsupported source tar header\n" unless substr($header, 257, 8) eq "ustar\00000";
        my $checksum = tar_octal(substr($header, 148, 8));
        my $checked = $header;
        substr($checked, 148, 8) = ' ' x 8;
        die "raw tar checksum mismatch\n" unless unpack('%32C*', $checked) == $checksum;
        my $name = tar_text(substr($header, 0, 100));
        my $prefix = tar_text(substr($header, 345, 155));
        my $type = substr($header, 156, 1);
        my $size = tar_octal(substr($header, 124, 12));
        my $mode_bits = tar_octal(substr($header, 100, 8));
        die "unexpected source link name\n" unless tar_text(substr($header, 157, 100)) eq '';
        my $end = $offset + 512 + 512 * int(($size + 511) / 512);
        die "raw tar extent exceeds source\n" if $end > length($source);
        my $content = substr($source, $offset + 512, $size);
        die "nonzero raw member padding\n" unless substr($source, $offset + 512 + $size, $end - $offset - 512 - $size)
            eq "\0" x ($end - $offset - 512 - $size);
        my $raw_path = safe_name(length($prefix) ? "$prefix/$name" : $name, $type eq '5');
        die "PAX path must immediately precede a regular member\n" if defined($pending) && $type ne '0';
        if ($type eq 'g') {
            die "unexpected global PAX metadata\n" if $global++ || @members || @pax || $raw_path ne 'pax_global_header'
                || $content ne ("52 comment=" . $config->{compiler_input}{revision} . "\n");
        } elsif ($type eq 'x') {
            $content =~ /\A([1-9][0-9]{0,5}) path=([^\n]*)\n\z/s or die "unsupported per-file PAX record\n";
            my ($record_length, $path) = ($1, $2);
            die "incorrect PAX byte length\n" unless $record_length == length($content);
            $path = safe_name($path, 0);
            $pending = { header_offset => $offset, raw_header_path => $raw_path, path => $path,
                payload => $content, record_bytes => $end - $offset,
                record_sha256 => sha256_hex(substr($source, $offset, $end - $offset)) };
        } elsif ($type eq '0' || $type eq '5') {
            die "directory has a payload\n" if $type eq '5' && $size != 0;
            my $path = defined($pending) ? $pending->{path} : $raw_path;
            if (defined $pending) {
                push @pax, { %$pending, following_header_offset => $offset, following_raw_path => $raw_path,
                    following_header_sha256 => sha256_hex($header) };
                undef $pending;
            }
            push @members, { path => $path, raw_path => $raw_path, type => $type, mode => $mode_bits,
                bytes => $size, sha256 => sha256_hex($content), header_offset => $offset };
        } else {
            die "unsupported source tar member type\n";
        }
        $offset = $end;
    }
    die "incomplete source tar framing\n" unless $ended && $global == 1;
    return { members => \@members, pax_paths => \@pax };
}

my($result_members)=ordinary_archive("$prefix/comparison-recovery/recovery.tar.gz",1,'2451a0a876fc2ddf839e5774280a39c500500b17d65a494028c1a18002af1833');
die"result archive population\n" unless keys(%$result_members)==490;
my($result_inventory)=decode_blob('comparison-recovery/inventory.json');
for my$e(@{$result_inventory->{entries}}){
 next unless$e->{type}eq'file';my$p=$e->{path};die"unexpected result scope\n" unless$p=~m{\A(?:\Q$comparison\E|\Q$outer\E)/};
 die"result metadata mismatch\n" unless$initial{$p}{mode}==$e->{mode}&&$initial{$p}{bytes}==$e->{bytes};
 my$s=$e->{storage};my$id;
 if($s->{kind}eq'archive'||$s->{kind}eq'archive-alias'){$id=$result_members->{$s->{member}};die"missing result member\n"unless$id;}
 elsif($s->{kind}eq'git-blob'){$id=$seeds{$s->{recipe}}{compressed};}
 elsif($s->{kind}eq'gunzip-git-blob'){$id=$seeds{$s->{recipe}}{expanded};}
 else{die"unknown result recipe\n";}
 die"result inventory/provider mismatch\n" unless$providers{$id}{sha256}eq$e->{sha256}&&$providers{$id}{bytes}==$e->{bytes};assign($p,$id);
}
my($source_archive,$source_coordinate)=git_blob($revision,"$prefix/setup2/artifacts/compiler-source.tar");
die"compiler archive pin\n" unless sha256_hex($source_archive)eq'79aa24a9eb6f8e7940b4424fc20839759a79dd8d480b1b746f7e29b17171d806';
my$raw=raw_source_members($source_archive);my%source_members;
for my$m(@{$raw->{members}}){next if$m->{type}eq'5';die"duplicate source member\n"if exists$source_members{$m->{path}};
 my$b=substr($source_archive,$m->{header_offset}+512,$m->{bytes});die"source member hash\n"unless sha256_hex($b)eq$m->{sha256};
 $source_members{$m->{path}}=provider({kind=>'archive-member',archive=>$source_coordinate,compression=>'none',member=>$m->{path},archive_mode=>$m->{mode},raw_header_offset=>$m->{header_offset},raw_header_path=>$m->{raw_path},pax_path_applied=>$m->{path}ne$m->{raw_path}?JSON::PP::true:JSON::PP::false},$b);
}
die"source archive shape\n"unless keys(%source_members)==1742&&@{$raw->{pax_paths}}==2;
my($generator_source,$generator_coordinate)=git_blob($revision,"$prefix/setup2/artifacts/scale-generator-source.lkj.txt");
die"generator source pin\n"unless sha256_hex($generator_source)eq'557d6b124fe739f17c678c77b92e5f6259d87ebca69c2a56aca411071c5ff1e9';
my$generator_source_id=provider($generator_coordinate,$generator_source);
for my$p(sort keys%source_members){assign("$measurement/compiler-input/$p",$source_members{$p});assign("$measurement/factory/$p",$p eq'src/main.lkj'?$generator_source_id:$source_members{$p});}
my($generated)=ordinary_archive("$prefix/setup2/artifacts/generated-100k.tar.gz",1,'57ead08d82ea53ba4d833b2689cbf1158550931b8cf288aac3363644d2dab07c');
die"generated population\n"unless keys(%$generated)==170;for my$p(sort keys%$generated){assign("$measurement/generated-100k/$p",$generated->{$p});}
my($generated_tar,$generated_tar_coordinate)=gunzip_blob($revision,"$prefix/setup2/artifacts/generated-100k.tar.gz");
die"generated tar pin\n"unless sha256_hex($generated_tar)eq'ae6b1704d0796e9c2437c7e4ebefe25995c493fedca7308fe0a2fc6c42857cb4';
assign("$measurement/artifacts/generated-100k.tar",provider($generated_tar_coordinate,$generated_tar));
my($generator,$generator_c)=gunzip_blob($revision,"$prefix/setup2/artifacts/generator.gz");
die"generator binary pin\n"unless length($generator)==1642167&&sha256_hex($generator)eq'be23723deaf08b25756a497a674a3455a75415851ebf5ae4a746dc16024e882c';
assign("$measurement/generator",provider($generator_c,$generator));
my(@rows,@unresolved,@content_aliases,%totals);
for my$e(@{$initial->{entries}}){
 my%row=%$e;my($s)=split m{/},$e->{path};$totals{$s}{directories}++ if$e->{type}eq'directory';
 if($e->{type}eq'directory'){$row{recovery}={kind=>'directory',restore_mode=>$e->{mode}};push@rows,\%row;next;}
 $totals{$s}{files}++;$totals{$s}{bytes}+=$e->{bytes};
 my$id=$assignment{$e->{path}};
 if(!defined$id){
  if($e->{path}=~m{\A\Q$trial\E/[^/]*release[^/]*\.json\z}){
   my$b=read_regular("$base/$e->{path}");my@matches=sort grep{$provider_bytes{$_}eq$b}values%$result_members;
   if(@matches){$id=$matches[0];$assignment{$e->{path}}=$id;push@content_aliases,{path=>$e->{path},provider=>$id,method=>'Exact full bytes against an already admitted result-archive member; no additional scope searched.'};}
  }
 }
 if(!defined$id){$row{recovery}={kind=>'unresolved',reason=>'No exact recovery coordinate in the fixed retained sets; content was not read unless the explicit trial-release alias rule applied.'};push@unresolved,{path=>$e->{path},bytes=>$e->{bytes}};push@rows,\%row;next;}
 my$b=read_regular("$base/$e->{path}");
 die"original/provider full bytes differ: $e->{path}\n"unless$b eq$provider_bytes{$id}&&length($b)==$e->{bytes};
 $row{sha256}=sha256_hex($b);$row{recovery}={kind=>'provider',id=>$id,full_byte_equal=>JSON::PP::true,restore_mode=>$e->{mode}};
 $totals{$s}{verified_files}++;$totals{$s}{verified_bytes}+=length$b;push@rows,\%row;
}
die"source metadata drift after mapping\n"unless$json->encode(scan())eq$json->encode($initial->{entries});
my$argv_path="$recovery/capture/argv.json";my$argv_bytes=read_regular("$base/$argv_path");my$argv=decode_json($argv_bytes);
die"unexpected packaging interpreter\n"unless@$argv==14&&$argv->[12]eq'/usr/bin/perl'&&$argv->[13]eq"$base/$recovery/package.pl";
my$interpreter={source=>$argv_path,%{pin($argv_bytes)},argv=>$argv,interpreter=>'/usr/bin/perl',confirmation=>'Read from completed existing capture; no helper was re-executed.',workload_rerun=>JSON::PP::false};
my$map={schema=>'compiler-reference-capture-final-recovery-map-1',retained_revision=>$revision,source_base=>$base,scopes=>\@scopes,source_entries=>\@rows,providers=>\%providers,content_aliases=>\@content_aliases,unresolved=>\@unresolved,
 contract=>'Exact regular bytes and file modes plus directory paths/modes. Observed UID/GID/inode/time fields are audit metadata, not a promise to recreate incidental filesystem identities or timestamps. No deletion is authorized.'};
write_json('recovery-map.json',$map);
write_json('git-read-commands.json',\@git_commands);
write_json('source-archive-pax.json',$raw->{pax_paths});
write_json('interpreter-confirmation.json',$interpreter);
my($tf,$td,$tb)=(0,0,0);for my$s(@scopes){$tf+=$totals{$s}{files};$td+=$totals{$s}{directories};$tb+=$totals{$s}{bytes};}
die"final totals differ\n"unless$tf==4468&&$td==141&&$tb==342778212;
my$summary={schema=>'compiler-reference-capture-final-recovery-summary-1',status=>@unresolved?'unresolved':'complete',retained_revision=>$revision,scopes=>[map{{scope=>$_,%{$totals{$_}}}}@scopes],files=>4468,directories=>141,bytes=>342778212,unresolved=>\@unresolved,providers=>scalar(keys%providers),source_metadata_unchanged=>JSON::PP::true,regular_byte_comparisons=>4468-scalar@unresolved,archive_member_source_files=>1742,generated_member_files=>170,source_pax_records=>scalar@{$raw->{pax_paths}},packaging_interpreter=>'/usr/bin/perl',new_compression=>JSON::PP::false,deletion=>JSON::PP::false,compiler_or_native_execution=>JSON::PP::false};
write_json('summary.json',$summary);
print$json->encode($summary);
