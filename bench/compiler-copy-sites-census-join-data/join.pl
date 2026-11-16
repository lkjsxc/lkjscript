#!/usr/bin/env perl
use strict;
use warnings;
use Config;
use Digest::SHA qw(sha256_hex);
use Encode qw(decode FB_CROAK);
use Fcntl qw(O_RDONLY O_WRONLY O_CREAT O_EXCL O_NOFOLLOW :mode);
use JSON::PP;
use Math::BigInt;

# Independent offline join. Exact accepted cohort only; no executable inputs.
my %PIN=(
 profile=>'7849d4dd69906491d07664f493eb0f4fd8340cfde3760f6e4f168dbbde95f302',
 companion=>'32c0f43dbde125e43f2ca5a0c6d8548ba0f1774ce3b51bc62d94a6488af0fdca',
 detail=>'06093752531ea752f7081548166f33cc95c71aad381e72d2713e35f5bcaab25d',
 extraction_manifest=>'63d59716e15c302d241d9e19ba792a0d715b0a0e25aeda75827b24be0799d480',
 selection=>'3dd145c8920a3169b322bf6fbee0e637ceb4e8989980229260b217c04e38d80c',
 acceptance=>'adfb9a9837ec044cc1a03cec186a7ada643300bab7efe79b324df6e597bddcbe',
 layout=>'f88d1ff9a50ce796d2fc10bfa04cd4e8d73a30d8284d5a9b3bcd3488f724daf0',
 plan=>'8a1f3e3d5354b3eca7fd9adb68bfee892f94360130a19b265f3f9ffb73c4dc15',
);
my @FROZEN=(4545,4546,4452,3101,5729,7492,4631,4445,7710,752,5719,2234,2615,2652,6179,6244,7372,690,1417,9782);
my @ROLE=qw(invalid local typed-place constructor-field array-element abi-incoming parameter-local abi-result vector-element result-variant-field repeated-array-element box-payload referent view-header);
my %args;
while(@ARGV) {
 my $key=shift @ARGV;
 die "unknown/missing argument\n" unless $key=~/\A--(profile|extraction|acceptance|layout|output)\z/ && @ARGV;
 my $name=$1; die "duplicate argument\n" if exists $args{$name}; $args{$name}=shift @ARGV;
}
die "need --profile --extraction --acceptance --layout --output\n" unless keys(%args)==5;
die "64-bit integer Perl required\n" unless $Config{ivsize}>=8;
die "output must be absent; no retries in place\n" if lstat($args{output});
mkdir($args{output},0700) or die "claim output: $!\n";
my $OUT=$args{output}; my %written;
my $J=JSON::PP->new->canonical->utf8;
my $MAX_TEXT='18446744073709551615';
my $MAX=unpack('Q<',"\xff"x8);
my $MAX_BIG=Math::BigInt->new($MAX_TEXT);
sub dec {return join('',$_[0]);}
sub big {return Math::BigInt->new(dec($_[0]));}
sub add64 {
 my($acc,$n,$where)=@_; $acc->badd(dec($n));
 die "u64 sum overflow: $where\n" if $acc->bcmp($MAX_BIG)>0;
}
sub as_json {return JSON::PP->new->canonical->utf8->pretty->encode($_[0]);}
sub uint {
 my($s,$where)=@_;
 die "bad unsigned value $where\n" unless defined($s) && $s=~/\A(?:0|[1-9][0-9]*)\z/
   && (length($s)<20 || (length($s)==20 && $s le $MAX_TEXT));
 return 0+$s;
}
sub read_regular {
 my($path,$cap)=@_;
 sysopen(my $f,$path,O_RDONLY|O_NOFOLLOW) or die "open $path: $!\n"; binmode $f;
 my @before=stat($f);
 die "not regular/bounded $path\n" unless @before && S_ISREG($before[2]) && $before[7]<=$cap;
 my $bytes=''; while(1) {my $n=read($f,my $part,65536); die "read $path\n" unless defined($n);
  last if !$n; $bytes.=$part; die "input grew past bound\n" if length($bytes)>$cap;}
 my @after=stat($f);for my $i(0,1,2,7,9,10){die "input changed while reading\n" unless $before[$i]==$after[$i];}
 close $f or die "input close\n"; die "input size changed\n" unless length($bytes)==$before[7];
 return $bytes;
}
sub write_bytes {
 my($name,$bytes)=@_;die "unsafe output name\n" unless $name=~/\A[A-Za-z0-9_.\/-]+\z/ && $name!~m{(?:\A|/)\.\.?(?:/|\z)};
 sysopen(my $f,"$OUT/$name",O_WRONLY|O_CREAT|O_EXCL,0600) or die "write $name: $!\n";
 binmode $f; print $f $bytes or die "output write\n";close $f or die "output close\n";
 $written{$name}=sha256_hex($bytes);
}
sub exact_count {my($w,$n,$where)=@_;die "wrong operand count $where\n" unless @$w==$n;}
sub pairs {
 my($w,$at)=@_;die "field pairing\n" if (@$w-$at)%2;
 my @out;while($at<@$w){push @out,{name=>$w->[$at],type=>uint($w->[$at+1],'field type')};$at+=2;}
 return \@out;
}
sub words {
    my ($raw, $line) = @_;
    my $text = decode('UTF-8', $raw, FB_CROAK);
    $text =~ s/\n\z//; $text =~ s/\r\z//;
    my @c = split //, $text;
    my (@result, $at);
    $at = 0;
    while ($at < @c) {
        if ($c[$at] =~ /\p{White_Space}/) { $at++; next; }
        last if $c[$at] eq '#';
        my $word = '';
        if ($c[$at] eq '"') {
            $at++; my $closed = 0;
            while ($at < @c) {
                my $ch = $c[$at++];
                if ($ch eq '"') { $closed = 1; last; }
                if ($ch ne '\\') {
                    die "line $line: control character in quoted string\n" if $ch =~ /\p{Cc}/;
                    $word .= $ch; next;
                }
                die "line $line: unfinished string escape\n" if $at >= @c;
                my $esc = $c[$at++];
                if ($esc eq '"' || $esc eq '\\' || $esc eq '/') { $word .= $esc; next; }
                my %simple = (n=>"\n", r=>"\r", t=>"\t", b=>"\x08", f=>"\x0c");
                if (exists $simple{$esc}) { $word .= $simple{$esc}; next; }
                die "line $line: unknown string escape\n" unless $esc eq 'u';
                die "line $line: expected four hexadecimal digits\n" if $at+4 > @c;
                my $hex = join('', @c[$at .. $at+3]);
                die "line $line: expected four hexadecimal digits\n" unless $hex =~ /\A[0-9A-Fa-f]{4}\z/;
                $at += 4; my $value = hex($hex);
                if ($value >= 0xd800 && $value <= 0xdbff) {
                    die "line $line: high surrogate needs a low surrogate\n"
                        unless $at+6 <= @c && $c[$at] eq '\\' && $c[$at+1] eq 'u';
                    my $low_hex = join('', @c[$at+2 .. $at+5]);
                    die "line $line: invalid low surrogate\n" unless $low_hex =~ /\A[0-9A-Fa-f]{4}\z/;
                    my $low = hex($low_hex);
                    die "line $line: invalid low surrogate\n" unless $low >= 0xdc00 && $low <= 0xdfff;
                    $at += 6; $value = 0x10000 + ($value-0xd800)*1024 + $low-0xdc00;
                } elsif ($value >= 0xdc00 && $value <= 0xdfff) {
                    die "line $line: invalid Unicode scalar\n";
                }
                $word .= chr($value);
            }
            die "line $line: unterminated quoted string\n" unless $closed;
            die "line $line: quoted word needs a separator\n"
                if $at < @c && $c[$at] !~ /\p{White_Space}/ && $c[$at] ne '#';
        } else {
            while ($at < @c && $c[$at] !~ /\p{White_Space}/ && $c[$at] ne '#') {
                die "line $line: quote inside an unquoted word\n" if $c[$at] eq '"';
                $word .= $c[$at++];
            }
        }
        push @result, $word;
    }
    return \@result;
}

my %TYPE;
sub parse_type {
 my($id,$raw,$meta)=@_;
 my @lines=split /(?<=\n)/,$raw;my $w=words(shift(@lines),1);
 die "type declaration\n" unless @$w>=3 && $w->[0] eq 'type';
 my $k=$w->[2];my $t={id=>$id,name=>$w->[1],kind=>$k,raw=>$raw,%$meta};
 if($k=~/\A(?:unit|bool|char|f64|str|world|stdout|stdin|stderr|directory|read_directory|read_file|image|channel|worker|read_region|foreign_command|foreign_session|null_device|file|process|clock|child|listener|connector|readiness|entropy|stop|arguments|environment|exit_status)\z/){exact_count($w,3,'type');}
 elsif($k eq 'int'){exact_count($w,5,'int');$t->{bits}=uint($w->[3],'bits');die "int flag\n" unless $w->[4]=~/\A(?:signed|unsigned)\z/;$t->{signed}=$w->[4];}
 elsif($k eq 'ref'||$k eq 'slice'){exact_count($w,5,'view');die "view flag\n" unless $w->[3]=~/\A(?:mut|shared)\z/;$t->{inner}=uint($w->[4],'inner');}
 elsif($k eq 'array'){exact_count($w,5,'array');$t->{inner}=uint($w->[3],'inner');$t->{length}=uint($w->[4],'length');}
 elsif($k eq 'vec'||$k eq 'box'){exact_count($w,4,'owner');$t->{inner}=uint($w->[3],'inner');}
 elsif($k eq 'struct'){die "struct flag\n" unless ($w->[3]//'')=~/\A(?:copy|move)\z/;$t->{copy}=$w->[3];$t->{fields}=pairs($w,4);}
 elsif($k eq 'enum'){
  exact_count($w,4,'enum');die "enum flag\n" unless $w->[3]=~/\A(?:copy|move)\z/;$t->{copy}=$w->[3];my @variants;my $ended=0;
  for my $line(@lines){my $v=words($line,1);next unless @$v;
   die "data after endtype\n" if $ended;
   if($v->[0] eq 'endtype'){exact_count($v,1,'endtype');$ended=1;}
   elsif($v->[0] eq 'variant' && @$v>=2){push @variants,{name=>$v->[1],fields=>pairs($v,2)};}
   else{die "enum framing\n";}}
  die "missing endtype\n" unless $ended;$t->{variants}=\@variants;
 }else{die "unsupported type shape $k\n";}
 if($k ne 'enum'){for my $line(@lines){die "trailing type material\n" if @{words($line,1)};}}
 return $t;
}
sub type_at {my($id)=@_;return $TYPE{$id}//die "missing complete type $id\n";}
my(%LAYOUT,%VISIT,%DTOR);
sub add_extent {
 my($a,$b)=@_;my $v=big($a)->badd(dec($b));die "layout overflow\n" if $v->bcmp($MAX_BIG)>0;return $v;
}
sub aligned {
 my($n,$alignment)=@_;die "bad alignment\n" unless $alignment==1||$alignment==2||$alignment==4||$alignment==8;
 my $v=add_extent($n,$alignment-1);$v->bdiv($alignment)->bmul($alignment);return $v;
}
sub field_layout {
 my($fields,$start)=@_;my $extent=big($start);my $alignment=$start==0?1:8;my @offsets;
 for my $field(@$fields){my $l=layout($field->{type});$extent=aligned($extent,$l->{alignment});
  push @offsets,$extent->copy;$alignment=$l->{alignment} if $l->{alignment}>$alignment;$extent=add_extent($extent,$l->{bytes});}
 $extent=big(1) if $extent->is_zero;return(aligned($extent,$alignment),$alignment,\@offsets);
}
sub layout {
 my($id)=@_;return $LAYOUT{$id} if exists $LAYOUT{$id};
 die "inline layout cycle\n" if $VISIT{$id}++;my $t=type_at($id);my $k=$t->{kind};
 my $l={bytes=>big(8),alignment=>8,fields=>[],variants=>[]};
 if($k eq 'unit'){$l->{bytes}=big(0);$l->{alignment}=1;}
 elsif($k eq 'bool'){$l->{bytes}=big(1);$l->{alignment}=1;}
 elsif($k eq 'char'){$l->{bytes}=big(4);$l->{alignment}=4;}
 elsif($k eq 'int'){die "unsupported checked integer width\n" unless $t->{bits}==8||$t->{bits}==16||$t->{bits}==32||$t->{bits}==64;
  $l->{bytes}=big($t->{bits}/8);$l->{alignment}=$t->{bits}/8;}
 elsif($k eq 'struct'){($l->{bytes},$l->{alignment},$l->{fields})=field_layout($t->{fields},0);}
 elsif($k eq 'enum'){
  for my $variant(@{$t->{variants}}){my($bytes,$alignment,$offsets)=field_layout($variant->{fields},8);
   $l->{bytes}=$bytes if $bytes->bcmp($l->{bytes})>0;$l->{alignment}=$alignment if $alignment>$l->{alignment};push @{$l->{variants}},$offsets;}
  $l->{bytes}=aligned($l->{bytes},$l->{alignment});
 }elsif($k eq 'array'){my $inner=layout($t->{inner});$l->{bytes}=$inner->{bytes}->copy->bmul(dec($t->{length}));
  die "array layout overflow\n" if $l->{bytes}->bcmp($MAX_BIG)>0;$l->{alignment}=$inner->{alignment};}
 elsif($k eq 'vec'){$l->{bytes}=big(24);}
 elsif($k eq 'stop'||$k eq 'slice'||$k eq 'str'){$l->{bytes}=big(16);}
 elsif($k eq 'ref'){my $inner=type_at($t->{inner});$l->{bytes}=big(16) if $inner->{kind} eq 'slice'||$inner->{kind} eq 'str';}
 $LAYOUT{$id}=$l;$VISIT{$id}=0;return $l;
}
my(%CLEANUP,%CLEAN_SEEN);
sub cleanup {
 my($id)=@_;return $CLEANUP{$id} if exists $CLEANUP{$id};
 die "inline cleanup cycle\n" if $CLEAN_SEEN{$id}++;
 my $t=type_at($id);my $k=$t->{kind};
 my $yes=exists($DTOR{$id}) || $k=~/\A(?:vec|box|file|directory|read_directory|read_file|image|channel|worker|read_region|foreign_command|foreign_session|null_device|child|listener|readiness|stop)\z/;
 if(!$yes&&$k eq 'struct'){for my $field(@{$t->{fields}}){$yes=1 if cleanup($field->{type});}}
 if(!$yes&&$k eq 'enum'){for my $variant(@{$t->{variants}}){for my $field(@{$variant->{fields}}){$yes=1 if cleanup($field->{type});}}}
 if(!$yes&&$k eq 'array'){$yes=cleanup($t->{inner});}
 $CLEAN_SEEN{$id}=0;$CLEANUP{$id}=$yes?1:0;return $CLEANUP{$id};
}
sub local_type {
 my($f,$i)=@_;die "local coordinate outside body\n" unless $i<@{$f->{locals}};return $f->{locals}[$i];
}
sub place_type {
 my($f,$w,$at)=@_;my $ty=local_type($f,uint($w->[$at++],'place local'));my $flags=0;
 while($at<@$w){my $p=$w->[$at++];my $t=type_at($ty);
  if($p eq 'deref'){die "deref type mismatch\n" unless $t->{kind} eq 'ref'||$t->{kind} eq 'box';$ty=$t->{inner};$flags|=2;}
  elsif($p eq 'field'){my $i=uint($w->[$at++],'field');die "field type/index\n" unless $t->{kind} eq 'struct'&&$i<@{$t->{fields}};$ty=$t->{fields}[$i]{type};$flags|=1;}
  elsif($p eq 'variant'){my $v=uint($w->[$at++],'variant');my $i=uint($w->[$at++],'variant field');
   die "variant type/index\n" unless $t->{kind} eq 'enum'&&$v<@{$t->{variants}}&&$i<@{$t->{variants}[$v]{fields}};
   $ty=$t->{variants}[$v]{fields}[$i]{type};$flags|=1;}
  elsif($p eq 'index'){my $i=uint($w->[$at++],'index local');die "index type\n" unless local_type($f,$i)==4&&$t->{kind}=~/\A(?:vec|slice|array)\z/;$ty=$t->{inner};$flags|=4;}
  else{die "unknown place projection\n";}}
 return($ty,$flags);
}
sub operand_contract {
 my($f,$instruction,$subsite)=@_;my($ty,$origin,$source,$dest,$sf,$df,$op)=(undef,1,1,1,0,0,'');
 if($instruction==$MAX){die "incoming parameter subsite\n" unless $subsite<$f->{params};
  ($ty,$origin,$source,$dest,$op)=(local_type($f,$subsite),2,5,6,'incoming-parameter');
 }else{
  die "instruction index outside body\n" unless $instruction<@{$f->{body}};
  my $w=$f->{body}[$instruction]{words};$op=$w->[1];
  if($op eq 'read'&&$subsite==0){($ty,$sf)=place_type($f,$w,4);$source=2 if @$w>5;}
  elsif($op eq 'assign'&&$subsite==0){my($place,$flags)=place_type($f,$w,3);$ty=local_type($f,uint($w->[2],'assignment source'));if(@$w>4){$dest=2;$df=$flags;}}
  elsif($op eq 'struct'||$op eq 'variant'||$op eq 'array'){
   my $start=$op eq 'variant'?4:3;die "constructor subsite outside original operands\n" unless $subsite<@$w-$start;
   $ty=local_type($f,uint($w->[$start+$subsite],'constructor operand'));$dest=$op eq 'array'?4:3;
  }elsif($op eq 'return'&&$subsite==0){$ty=local_type($f,uint($w->[2],'return source'));$dest=7;}
  elsif($op eq 'intrinsic'&&$subsite==0){
   my $name=$w->[3];my @a=map {uint($_,'intrinsic argument')} @$w[4..$#$w];
   if($name eq 'vec_push'&&@a==2){$ty=local_type($f,$a[1]);$dest=8;}
   elsif($name eq 'vec_pop'&&@a==1){my $ref=type_at(local_type($f,$a[0]));die "vec_pop reference\n" unless $ref->{kind} eq 'ref';
    my $vec=type_at($ref->{inner});die "vec_pop vector\n" unless $vec->{kind} eq 'vec';$ty=$vec->{inner};($source,$dest)=(8,9);}
   elsif(($name eq 'array_repeat'||$name eq 'box_new'||$name eq 'str_as_bytes')&&@a==1){$ty=local_type($f,$a[0]);
    $dest=$name eq 'array_repeat'?10:$name eq 'box_new'?11:13;$source=13 if $name eq 'str_as_bytes';}
   elsif($name eq 'clone'&&@a==1){my $ref=type_at(local_type($f,$a[0]));die "clone reference\n" unless $ref->{kind} eq 'ref';$ty=$ref->{inner};$source=12;}
   else{die "unsupported copy-producing intrinsic/subsite\n";}
  }else{die "instruction/subsite is not a copy contract\n";}
 }
 type_at($ty);my $tags=$origin|($source<<8)|($dest<<16)|($sf<<24)|($df<<32);
 return($ty,$tags,$op);
}
my $P;
sub get64 {my($at)=@_;die "profile u64 out of bounds\n" if $at>length($P)||8>length($P)-$at;return unpack('Q<',substr($P,$at,8));}
sub extent {my($start,$count,$width)=@_;die "profile table bounds\n" if $start>length($P)||$count>int((length($P)-$start)/$width);return $start+$count*$width;}
sub validate_tags {
 my($tags)=@_;die "reserved tags\n" if ($tags & hex('0000000707ffffff'))!=$tags;
 my($o,$s,$d,$sf,$df)=($tags&255,($tags>>8)&255,($tags>>16)&255,($tags>>24)&7,($tags>>32)&7);
 die "role flags\n" if ($s!=2&&$sf)||($d!=2&&$df);
 if($o==2){die "incoming roles\n" unless $s==5&&$d==6;}
 elsif($o==1){die "body roles\n" unless ($s==1&&($d==1||$d==2||$d==3||$d==4||$d==7||$d==8||$d==10||$d==11))
  ||($s==2&&$d==1)||($s==8&&$d==9)||($s==12&&$d==1)||($s==13&&$d==13);}
 else{die "copy origin\n";}return($o,$s,$d,$sf,$df);
}
my $result={version=>'1',purpose=>'all-raw-copy-row-join-and-type-only-screen',whole_body_eligibility=>'not-assessed',
 gain=>'not-inferred',sample_times_width=>'not-computed',profile_executed=>'no',compiler_executed=>'no',
 official_reader_executed=>'no',extra_page_requested=>'no',source_mutation=>'none',pins=>\%PIN};
my $ok=eval {
 my $accept=read_regular($args{acceptance},1048576);
 die "acceptance proof pin\n" unless sha256_hex($accept) eq $PIN{acceptance};
 my $layout_source=read_regular($args{layout},1048576);
 die "native layout source pin\n" unless sha256_hex($layout_source) eq $PIN{layout};
 write_bytes('layout-source.lkj',$layout_source);write_bytes('accepted-measurement.json',$accept);
 my $m=read_regular("$args{extraction}/manifest.sha256",1048576);
 die "extraction manifest pin\n" unless sha256_hex($m) eq $PIN{extraction_manifest};
 my(%blob,%verified);my $inventory="sha256\tbytes\tpath\n";
 for my $line(split /\n/,$m){my($sha,$name)=$line=~/\A([0-9a-f]{64})  ([A-Za-z0-9_.\/-]+)\z/;
  die "extraction manifest framing\n" unless defined($name)&&$name!~m{(?:\A|/)\.\.?(?:/|\z)}&&!exists($verified{$name});
  my $bytes=read_regular("$args{extraction}/$name",134217728);die "extraction hash mismatch $name\n" unless sha256_hex($bytes) eq $sha;
  $verified{$name}=$sha;$blob{$name}=$bytes;$inventory.=join("\t",$sha,length($bytes),$name)."\n";}
 for my $required(qw(extraction.json selection.tsv inputs/companion.lkjir inputs/detail.txt shared/destructors.lkjir)){
  die "missing extraction input $required\n" unless exists $blob{$required};}
 $result->{extraction}={path=>$args{extraction},manifest_sha256=>$PIN{extraction_manifest},selection_sha256=>$PIN{selection}};
 my $all=$J->decode($blob{'extraction.json'});die "extraction incomplete\n" unless $all->{status} eq 'complete'&&$all->{exit_status} eq '0'&&$all->{selected_functions} eq '20';
 die "selection pin\n" unless sha256_hex($blob{'selection.tsv'}) eq $PIN{selection};
 my $IR=$blob{'inputs/companion.lkjir'};my $DETAIL=$blob{'inputs/detail.txt'};
 die "companion/detail pins\n" unless sha256_hex($IR) eq $PIN{companion}&&sha256_hex($DETAIL) eq $PIN{detail};
 write_bytes('extraction-inventory-check.tsv',$inventory);write_bytes('selection.tsv',$blob{'selection.tsv'});
 write_bytes('displayed-detail.txt',$DETAIL);write_bytes('shared-destructors.lkjir',$blob{'shared/destructors.lkjir'});
 my(%FUNCTION,%selected,%type_in_function);
 for my $ordinal(0..$#FROZEN){
  my $id=$FROZEN[$ordinal];my $meta=$all->{functions}[$ordinal];
  die "frozen selection mismatch\n" unless $meta->{function} eq dec($id)&&$meta->{ordinal} eq dec($ordinal)&&$meta->{status} eq 'complete';
  $selected{$id}=$ordinal;my $dir=sprintf('functions/%02u-function-%u',$ordinal,$id);
  my $body=$blob{"$dir/body.lkjir"}//die "missing complete body\n";
  die "body correspondence\n" unless length($body)==$meta->{body_bytes}&&sha256_hex($body) eq $meta->{body_sha256}
   && substr($IR,$meta->{byte_offset},$meta->{body_bytes}) eq $body;
  my @body_lines=split /(?<=\n)/,$body;my $head=words(shift(@body_lines),1);
  die "body header\n" unless @$head>=4&&$head->[0] eq 'fn'&&$head->[1] eq $meta->{name};
  my $f={id=>$id,ordinal=>$ordinal,name=>$head->[1],params=>uint($head->[2],'parameters'),locals=>[],local_records=>[],body=>[],meta=>$meta,dir=>$dir};
  my $ended=0;
  for my $line(@body_lines){my $w=words($line,1);next unless @$w;die "material after endfn\n" if $ended;
   if($w->[0] eq 'local'){exact_count($w,4,'local');push @{$f->{locals}},uint($w->[2],'local type');push @{$f->{local_records}},$line;}
   elsif($w->[0] eq 'return-leaf'){next;}
   elsif($w->[0] eq 'endfn'){exact_count($w,1,'endfn');$ended=1;}
   else{die "instruction framing\n" unless $w->[0]=~/\A\@/;push @{$f->{body}},{words=>$w,raw=>$line};}}
  die "complete body counts\n" unless $ended&&@{$f->{body}}==$meta->{instructions}&&@{$f->{locals}}==$meta->{local_count}&&$f->{params}==$meta->{parameter_count};
  $FUNCTION{$id}=$f;
  my @table=split /\n/,$blob{"$dir/types.tsv"}//die "missing type index\n";
  die "type index header\n" unless shift(@table) eq "type\tbyte_offset\tbytes\tsha256\tkind\tname_json";
  for my $row(@table){my($ty,$start,$length,$sha,$kind,$name)=split /\t/,$row,-1;
   ($ty,$start,$length)=map {uint($_,'type range')}($ty,$start,$length);
   die "type range/hash\n" unless $start<=length($IR)&&$length<=length($IR)-$start&&$sha=~/\A[0-9a-f]{64}\z/;
   my $raw=substr($IR,$start,$length);die "type range hash\n" unless sha256_hex($raw) eq $sha;
   my $decoded_name=$J->decode($name);
   if(exists $TYPE{$ty}){die "conflicting complete type\n" unless $TYPE{$ty}{raw} eq $raw;}
   else{$TYPE{$ty}=parse_type($ty,$raw,{offset=>$start,length=>$length,sha256=>$sha});}
   die "type index identity\n" unless $TYPE{$ty}{kind} eq $kind&&$TYPE{$ty}{name} eq $decoded_name;
   $type_in_function{$id}{$ty}=1;
  }
 }
 for my $line(split /\n/,$blob{'shared/destructors.lkjir'}){my $w=words($line,1);next unless @$w;
  exact_count($w,3,'destructor');die "destructor record\n" unless $w->[0] eq 'destructor';
  push @{$DTOR{uint($w->[1],'destructor type')}},uint($w->[2],'destructor function');}
 $P=read_regular($args{profile},268435456);
 die "raw profile pin\n" unless sha256_hex($P) eq $PIN{profile};
 die "profile header\n" unless length($P)>=160&&substr($P,0,8) eq 'LKJPROF4'&&get64(16)==1000000&&get64(72)==0&&get64(152)==0;
 die "profile companion digest\n" unless unpack('H*',substr($P,80,32)) eq $PIN{companion};
 my $N=get64(8);my $name_at=extent(160,$N,64);my($code_start,$code_end)=(get64(56),get64(64));
 die "code extent\n" unless $code_start<$code_end;
 my $S=get64(24);my(@starts,@ends);my $prior=$code_start;my $self=big(0);
 for my $i(0..$N-1){my $at=160+$i*64;my($start,$end,$own,$inclusive,$sample_epoch,$name,$length,$frame)=map {get64($at+$_*8)}0..7;
  die "symbol extent/accounting\n" unless $start>=$code_start&&$start<$end&&$end<=$code_end&&(!$i||$start==$prior)
   &&($frame==0||($frame>=$start&&$frame<=$end))&&$name==$name_at&&$own<=$S&&$inclusive<=$S&&$sample_epoch<=$S;
  $name_at=extent($name,$length,1);my $name_text=substr($P,$name,$length);decode('UTF-8',$name_text,FB_CROAK);
  push @starts,$start;push @ends,$end;add64($self,$own,'symbol self');$prior=$end;}
 die "symbol terminal extent\n" if $N&&$prior!=$code_end;
 my($ko,$kn,$counter)=(get64(32),get64(40),get64(48));die "kind framing\n" unless $ko==$name_at&&$kn&&extent($ko,$kn,16)==$counter;
 my $counter_end=extent($counter,16,8);$prior=0;
 for my $i(0..$kn-1){my($start,$kind)=(get64($ko+16*$i),get64($ko+16*$i+8));
  die "kind table\n" unless $kind<16&&$start>=$code_start&&$start<$code_end&&($i?$start>$prior:$start==$code_start);$prior=$start;}
 my $T=big(0);my @kind_counts;for my $i(0..15){my $n=get64($counter+8*$i);push @kind_counts,$n;add64($T,$n,'kind total');}
 my $serial_gap=big($S)->bsub($T);die "serial/function accounting\n" unless $serial_gap->bcmp(0)>=0&&$serial_gap->bcmp(1)<=0&&$self->bcmp($T)<=0;
 my($co,$C,$mo,$F)=(get64(112),get64(120),get64(128),get64(136));
 die "copy/map framing\n" unless $co==$counter_end&&extent($co,$C,72)==$mo&&extent($mo,$F,16)==length($P)&&$F==$all->{companion_functions};
 my(@map,%mapped_symbol);for my $i(0..$F-1){die "function map coordinate\n" unless get64($mo+16*$i)==$i;my $sym=get64($mo+16*$i+8);
  die "function map symbol\n" unless $sym==$MAX||($sym<$N&&!$mapped_symbol{$sym}++);push @map,$sym;}
 my(@rows,%interval);my $M=big(0);my $ki=0;$prior=$code_start;
 for my $i(0..$C-1){my $at=$co+72*$i;my @r=map {get64($at+$_*8)}0..8;my($start,$end,$samples,$fn,$instruction,$subsite,$ty,$width,$tags)=@r;
  die "copy interval/owner/width\n" unless $start>=$prior&&$start<$end&&$end<=$code_end&&$fn<$F&&$width>0;
  my($origin)=validate_tags($tags);die "copy origin coordinate\n" unless ($origin==2)==($instruction==$MAX);
  my $sym=$map[$fn];die "copy symbol interval\n" unless $sym!=$MAX&&$start>=$starts[$sym]&&$end<=$ends[$sym];
  $ki++ while $ki+1<$kn&&get64($ko+16*($ki+1))<=$start;
  die "copy kind interval\n" unless get64($ko+16*$ki+8)==1&&($ki+1==$kn||get64($ko+16*($ki+1))>=$end);
  die "duplicate interval\n" if $interval{"$start:$end"}++;$prior=$end;add64($M,$samples,'mapped copies');push @rows,\@r;
 }
 my $U=get64(144);my $D=$M->copy;add64($D,$U,'copy detail');my $K=$kind_counts[1];my $detail_gap=$D->copy->bsub(dec($K));
 die "copy interrupted accounting\n" unless $detail_gap->bcmp(0)>=0&&$detail_gap->bcmp(1)<=0&&($detail_gap->is_zero||$serial_gap->bcmp(1)==0);
 my @detail_lines=split /\n/,$DETAIL;
 my $head=shift @detail_lines;my $scope=shift @detail_lines;my $tail=pop @detail_lines;
 my $expected_header="copy-sites version=4 profile=$PIN{profile} companion=$PIN{companion} companion-check=checked-ir serial=$S total-samples=".$T->bstr." copy-samples=$K mapped=".$M->bstr." unmapped=$U in-flight=".$detail_gap->bstr." sites=$C order=samples-desc/function/instruction/subsite/start/end";
 die "official detail header/accounting\n" unless $head eq $expected_header;
 die "official detail page contract\n" unless @detail_lines==200&&$scope eq 'scope=existing-kind-copy runtime=excluded variable-width-copy=excluded replace=excluded scalar-frame-address-work=excluded counters=sampled-native-observation'
  &&$tail eq "continuation cursor=4:$PIN{profile}:$PIN{companion}:200 rows=200 next=200 total-sites=$C";
 my @order=sort {$rows[$b][2]<=>$rows[$a][2] || $rows[$a][3]<=>$rows[$b][3] || $rows[$a][4]<=>$rows[$b][4]
  ||$rows[$a][5]<=>$rows[$b][5] ||$rows[$a][0]<=>$rows[$b][0] ||$rows[$a][1]<=>$rows[$b][1] ||$a<=>$b}0..$C-1;
 my(%displayed,%seen_display);my @first;my $display_samples=big(0);
 for my $rank(0..199){
  my $line=$detail_lines[$rank];my($fn,$instruction,$subsite,$symbol,$samples,$ty,$width,$source,$dest,$sf,$df,$start,$end,$name,$type_name)=
   $line=~/\Acopy function=([0-9]+) instruction=(parameter|[0-9]+) subsite=([0-9]+) symbol=([0-9]+) samples=([0-9]+) type=([0-9]+) width=([0-9]+) source=([a-z-]+) destination=([a-z-]+) source-flags=([0-9]+) destination-flags=([0-9]+) start=([0-9]+) end=([0-9]+) name=("(?:[^"\\]|\\.)*") type-name=("(?:[^"\\]|\\.)*")\z/;
  die "detail row framing\n" unless defined $type_name;
  my $row_id=$order[$rank];my $r=$rows[$row_id];my($o,$rs,$rd,$rsf,$rdf)=validate_tags($r->[8]);
  my @want=($r->[3],$r->[4]==$MAX?'parameter':dec($r->[4]),$r->[5],$map[$r->[3]],$r->[2],$r->[6],$r->[7],$ROLE[$rs],$ROLE[$rd],$rsf,$rdf,$r->[0],$r->[1]);
  my @actual=($fn,$instruction,$subsite,$symbol,$samples,$ty,$width,$source,$dest,$sf,$df,$start,$end);
  for my $i(0..$#want){die "detail does not match complete raw sorted prefix\n" unless dec($want[$i]) eq $actual[$i];}
  if(exists $selected{$r->[3]}){die "selected detail names\n" unless $J->decode($name) eq $FUNCTION{$r->[3]}{name}&&$J->decode($type_name) eq type_at($r->[6])->{name};}
  $displayed{$row_id}=$rank;add64($display_samples,$r->[2],'displayed page');
  push @first,$r->[3] if !$seen_display{$r->[3]}++&&@first<20;
 }
 die "first-appearance population differs\n" unless join(',',@first) eq join(',',@FROZEN);
 my(%function_sum,%payload);my $selected_samples=big(0);my $selected_display=big(0);my $selected_rows=0;
 my $joined="raw_row\trow_sha256\tselection_ordinal\tfunction\tinstruction_u64\tinstruction\tsubsite\ttype\twidth\ttags\torigin\tsource\tdestination\tsource_flags\tdestination_flags\tstart\tend\tsymbol\tsamples\tdisplayed_rank\toperation\tfunction_name_json\ttype_name_json\tbody_sha256\tcoordinate_record_kind\tcoordinate_record_json\n";
 for my $id(@FROZEN){$function_sum{$id}={total_rows=>0,displayed_rows=>0,unshown_rows=>0,zero_rows=>0,total=>big(0),displayed=>big(0),unshown=>big(0)};}
 for my $i(0..$C-1){my $r=$rows[$i];my($start,$end,$samples,$id,$instruction,$subsite,$ty,$width,$tags)=@$r;next unless exists $selected{$id};
  my $f=$FUNCTION{$id};die "payload type absent from complete function closure\n" unless $type_in_function{$id}{$ty};
  my($expected_type,$expected_tags,$op)=operand_contract($f,$instruction,$subsite);
  die "copy coordinate/type/role mismatch at row $i\n" unless $expected_type==$ty&&$expected_tags==$tags;
  my $l=layout($ty);die "copy width differs from pinned native layout at row $i\n" unless $l->{bytes}->bcmp(dec($width))==0;
  my($origin,$source,$dest,$sf,$df)=validate_tags($tags);my $is_displayed=exists $displayed{$i};
  my $coordinate_record=$instruction==$MAX?$f->{local_records}[$subsite]:$f->{body}[$instruction]{raw};
  my $coordinate_text=decode('UTF-8',$coordinate_record,FB_CROAK);
  $joined.=join("\t",dec($i),sha256_hex(substr($P,$co+72*$i,72)),dec($selected{$id}),dec($id),dec($instruction),
   $instruction==$MAX?'parameter':dec($instruction),dec($subsite),dec($ty),dec($width),dec($tags),
   $origin==1?'body':'incoming-parameter',$ROLE[$source],$ROLE[$dest],dec($sf),dec($df),dec($start),dec($end),dec($map[$id]),dec($samples),
   $is_displayed?dec($displayed{$i}):'unshown',$op,$J->encode($f->{name}),$J->encode(type_at($ty)->{name}),$f->{meta}{body_sha256},
   $instruction==$MAX?'parameter-local':'instruction',
   $J->encode($coordinate_text))."\n";
  my $sum=$function_sum{$id};$sum->{total_rows}++;$sum->{zero_rows}++ if !$samples;add64($sum->{total},$samples,'selected function');
  if($is_displayed){$sum->{displayed_rows}++;add64($sum->{displayed},$samples,'selected displayed');add64($selected_display,$samples,'population displayed');}
  else{$sum->{unshown_rows}++;add64($sum->{unshown},$samples,'selected unshown');}
  $payload{$ty}//={rows=>0,zero_rows=>0,samples=>big(0),functions=>{}};$payload{$ty}{rows}++;$payload{$ty}{zero_rows}++ if !$samples;
  add64($payload{$ty}{samples},$samples,'payload type');$payload{$ty}{functions}{$id}=1;
  $selected_rows++;add64($selected_samples,$samples,'population');
 }
 write_bytes('selected-copy-rows.tsv',$joined);
 my $function_table="selection_ordinal\tfunction\tname_json\tdisplayed_rows\tunshown_rows\ttotal_rows\tzero_sample_rows\tdisplayed_samples\tunshown_samples\ttotal_samples\n";
 for my $id(@FROZEN){my $s=$function_sum{$id};
  die "function partition discrepancy\n" unless $s->{displayed_rows}+$s->{unshown_rows}==$s->{total_rows}&&$s->{displayed}->copy->badd($s->{unshown})->bcmp($s->{total})==0;
  $function_table.=join("\t",dec($selected{$id}),dec($id),$J->encode($FUNCTION{$id}{name}),map {dec($s->{$_})}qw(displayed_rows unshown_rows total_rows zero_rows))
   ."\t".join("\t",map {dec($s->{$_}->bstr)}qw(displayed unshown total))."\n";}
 write_bytes('function-totals.tsv',$function_table);
 mkdir("$OUT/payload-types",0700) or die "payload type directory\n";
 my $type_table="type\tname_json\tkind\tcopy_flag\tlayout_bytes\talignment\tcleanup\tcustom_destructors\ttype_only_status\treasons_json\trows\tzero_sample_rows\tsamples\tfunctions\n";
 my $fields_table="parent_type\tfield_index\tfield_name_json\tfield_type\tfield_kind\tfield_bytes\tfield_alignment\tfield_offset\tfield_custom_destructors\n";
 my($flat_types,$flat_rows,$flat_sampled_types)=(0,0,0);my $flat_samples=big(0);my @screens;
 for my $ty(sort {$a<=>$b}keys %payload){my $t=type_at($ty);my $l=layout($ty);my $clean=cleanup($ty);my @reasons;my $nonzero=0;
  push @reasons,'not-a-struct' if $t->{kind} ne 'struct';
  if($t->{kind} eq 'struct'){
   push @reasons,'struct-is-move' unless $t->{copy} eq 'copy';
   for my $i(0..$#{$t->{fields}}){my $field=$t->{fields}[$i];my $ft=type_at($field->{type});my $fl=layout($field->{type});
    my $allowed=$ft->{kind}=~/\A(?:unit|bool|char|f64)\z/||($ft->{kind} eq 'int'&&($ft->{bits}==8||$ft->{bits}==16||$ft->{bits}==32||$ft->{bits}==64));
    push @reasons,"field-$i-outside-flat-scalar-subset:$ft->{kind}" unless $allowed;
    push @reasons,"field-$i-custom-destructor" if exists $DTOR{$field->{type}};
    $nonzero=1 unless $fl->{bytes}->is_zero;
    $fields_table.=join("\t",dec($ty),dec($i),$J->encode($field->{name}),dec($field->{type}),$ft->{kind},dec($fl->{bytes}->bstr),dec($fl->{alignment}),
      dec($l->{fields}[$i]->bstr),exists($DTOR{$field->{type}})?join(',',@{$DTOR{$field->{type}}}):'none')."\n";
   }
   push @reasons,'no-nonzero-field' unless $nonzero;
  }
  push @reasons,'custom-destructor' if exists $DTOR{$ty};push @reasons,'native-cleanup' if $clean;
  my $status=@reasons?'blocked-type-only':'flat-copy-ir-struct-shape-only';my $p=$payload{$ty};
  if(!@reasons){$flat_types++;$flat_rows+=$p->{rows};$flat_sampled_types++ unless $p->{samples}->is_zero;add64($flat_samples,$p->{samples}->bstr,'flat type samples');}
  my $meta={type=>dec($ty),name=>$t->{name},kind=>$t->{kind},copy_flag=>$t->{copy}//'not-applicable',
   layout_bytes=>dec($l->{bytes}->bstr),alignment=>dec($l->{alignment}),cleanup=>$clean?'true':'false',
   custom_destructors=>exists($DTOR{$ty})?[map {dec($_)}@{$DTOR{$ty}}]:[],
   type_only_status=>$status,reasons=>\@reasons,nominal_origin=>'not-established-by-ir-shape',whole_body_eligibility=>'not-assessed',
   rows=>dec($p->{rows}),zero_sample_rows=>dec($p->{zero_rows}),samples=>dec($p->{samples}->bstr),
   functions=>[map {dec($_)}sort {$a<=>$b}keys %{$p->{functions}}],
   declaration_offset=>dec($t->{offset}),declaration_bytes=>dec($t->{length}),declaration_sha256=>$t->{sha256}};
  write_bytes("payload-types/type-$ty.lkjir",$t->{raw});push @screens,$meta;
  $type_table.=join("\t",dec($ty),$J->encode($t->{name}),$t->{kind},$t->{copy}//'not-applicable',dec($l->{bytes}->bstr),dec($l->{alignment}),
   $clean?'true':'false',exists($DTOR{$ty})?join(',',@{$DTOR{$ty}}):'none',$status,$J->encode(\@reasons),dec($p->{rows}),dec($p->{zero_rows}),
   dec($p->{samples}->bstr),join(',',sort {$a<=>$b}keys %{$p->{functions}}))."\n";
 }
 write_bytes('payload-types.tsv',$type_table);write_bytes('payload-type-fields.tsv',$fields_table);write_bytes('payload-types.json',as_json(\@screens));
 my $outside=$M->copy->bsub($selected_samples);my $selected_unshown=$selected_samples->copy->bsub($selected_display);
 my $outside_display=$display_samples->copy->bsub($selected_display);my $outside_unshown=$outside->copy->bsub($outside_display);
 die "negative population partition\n" if grep {$_->is_neg}($outside,$selected_unshown,$outside_display,$outside_unshown);
 $result->{profile}={path=>$args{profile},bytes=>dec(length($P)),sha256=>$PIN{profile},symbol_rows=>dec($N),function_rows=>dec($F),copy_rows=>dec($C),
  serial=>dec($S),kind_total=>dec($T->bstr),kind_copy_samples=>dec($K),mapped_copy_samples=>dec($M->bstr),unmapped_copy_samples=>dec($U),
  serial_minus_kind_total=>dec($serial_gap->bstr),detail_minus_kind_copy=>dec($detail_gap->bstr),
  interrupted_sample_location=>'unallocated; no row adjusted'};
 $result->{population}={functions=>[map{dec($_)}@FROZEN],rows=>dec($selected_rows),displayed_samples=>dec($selected_display->bstr),
  unshown_samples=>dec($selected_unshown->bstr),total_samples=>dec($selected_samples->bstr)};
 $result->{outside_population}={rows=>dec($C-$selected_rows),displayed_samples=>dec($outside_display->bstr),
  unshown_samples=>dec($outside_unshown->bstr),total_mapped_samples=>dec($outside->bstr)};
 $result->{sole_detail_page}={rows=>'200',samples=>dec($display_samples->bstr),complete_raw_census=>'no'};
 $result->{type_only_screen}={payload_types=>dec(scalar keys %payload),flat_copy_ir_struct_shape_types=>dec($flat_types),
  flat_copy_ir_struct_shape_types_with_nonzero_samples=>dec($flat_sampled_types),rows=>dec($flat_rows),raw_samples=>dec($flat_samples->bstr),
  interpretation=>'payload-type-only shape screen; nominal origin is not established because tuples also lower to IR Struct; this is not a containing-family screen or verdict; constructor scalar payloads may belong to a flat Copy aggregate; fresh roots, complete families, uses, control flow and escape are not assessed'};
 $result->{deduplication}='one output per original table row and unique nonoverlapping interval; duplicates refuse';
 $result->{verified_extraction_files}=dec(scalar keys %verified);
 # Re-read the accepted primary records only as an unchanged-input check.
 die "profile changed after join\n" unless sha256_hex(read_regular($args{profile},268435456)) eq $PIN{profile};
 die "companion changed after join\n" unless sha256_hex(read_regular("$args{extraction}/inputs/companion.lkjir",134217728)) eq $PIN{companion};
 die "detail changed after join\n" unless sha256_hex(read_regular("$args{extraction}/inputs/detail.txt",1048576)) eq $PIN{detail};
 die "extraction manifest changed\n" unless sha256_hex(read_regular("$args{extraction}/manifest.sha256",1048576)) eq $PIN{extraction_manifest};
 $result->{primary_inputs_after}='unchanged';$result->{status}='complete';1;
};
my $error=$@;
if(!$ok){$result->{status}='refused';$result->{error}="$error";}
$result->{exit_status}=$ok?'0':'1';write_bytes('result.json',as_json($result));
my $manifest=join('',map{"$written{$_}  $_\n"}sort keys %written);
sysopen(my $mf,"$OUT/manifest.sha256",O_WRONLY|O_CREAT|O_EXCL,0600) or die "manifest create\n";
print $mf $manifest or die "manifest write\n";close $mf or die "manifest close\n";
print as_json($result);
exit($ok?0:1);
