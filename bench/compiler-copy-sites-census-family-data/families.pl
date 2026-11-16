#!/usr/bin/env perl
use strict;use warnings;
use Digest::SHA qw(sha256_hex);
use Encode qw(decode FB_CROAK);
use Fcntl qw(O_RDONLY O_WRONLY O_CREAT O_EXCL O_NOFOLLOW :mode);
use JSON::PP;
my @ORDER=(4545,4546,4452,3101,5729,7492,4631,4445,7710,752,5719,2234,2615,2652,6179,6244,7372,690,1417,9782);
my %PIN=(extraction=>'63d59716e15c302d241d9e19ba792a0d715b0a0e25aeda75827b24be0799d480',
 join=>'b69c1d21e9ac89fde114dda7a0abee592bc16698c94cb902a4de669193ddff31',
 companion=>'32c0f43dbde125e43f2ca5a0c6d8548ba0f1774ce3b51bc62d94a6488af0fdca');
my %args;while(@ARGV){my $key=shift @ARGV;die "arguments\n" unless $key=~/\A--(extraction|join|output)\z/&&@ARGV;
 my $name=$1;die "duplicate argument\n" if exists $args{$name};$args{$name}=shift @ARGV;}
die "need extraction, join, output\n" unless keys(%args)==3;
die "output exists\n" if lstat($args{output});mkdir($args{output},0700) or die "claim output\n";
my $OUT=$args{output};my %written;my $J=JSON::PP->new->canonical->utf8;
sub dec{return join('',$_[0]);}
sub json{return JSON::PP->new->canonical->utf8->pretty->encode($_[0]);}
sub uint{my($s,$where)=@_;die "bad index $where\n" unless defined($s)&&$s=~/\A(?:0|[1-9][0-9]*)\z/&&length($s)<16;return 0+$s;}
sub read_file{my($path,$cap)=@_;sysopen(my $f,$path,O_RDONLY|O_NOFOLLOW) or die "open $path\n";binmode $f;my @a=stat($f);
 die "nonregular/large input\n" unless @a&&S_ISREG($a[2])&&$a[7]<=$cap;local $/;my $b=<$f>;$b//='';my @z=stat($f);
 for my $i(0,1,2,7,9,10){die "input changed\n" unless $a[$i]==$z[$i];}close $f or die "close";die "size changed\n" unless length($b)==$a[7];return $b;}
sub put{my($name,$b)=@_;die "unsafe output name\n" unless $name=~/\A[A-Za-z0-9_.\/-]+\z/&&$name!~m{(?:\A|/)\.\.?(?:/|\z)};
 sysopen(my $f,"$OUT/$name",O_WRONLY|O_CREAT|O_EXCL,0600)or die "write $name";binmode $f;print $f $b or die "write";close $f or die "close";$written{$name}=sha256_hex($b);}
sub exact_count{my($w,$n,$where)=@_;die "operand count $where\n" unless @$w==$n;}
sub pairs{my($w,$i)=@_;die "fields\n" if (@$w-$i)%2;my @f;while($i<@$w){push @f,{name=>$w->[$i],type=>uint($w->[$i+1],'field')};$i+=2;}return \@f;}
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

my(%FLAT,%DTOR);
sub type_screen{
 my($id)=@_;return $FLAT{$id} if exists $FLAT{$id};my $t=$TYPE{$id}//die "missing type\n";my @why;my $extent=0;my $align=1;my @offset;my $nonzero=0;
 push @why,'not-struct' unless $t->{kind} eq 'struct';
 if($t->{kind} eq 'struct'){
  push @why,'move-struct' unless $t->{copy} eq 'copy';
  for my $i(0..$#{$t->{fields}}){my $f=$t->{fields}[$i];my $ft=$TYPE{$f->{type}}//die "missing field type\n";my $kind=$ft->{kind};my($size,$a);
   if($kind eq 'unit'){($size,$a)=(0,1);}elsif($kind eq 'bool'){($size,$a)=(1,1);}elsif($kind eq 'char'){($size,$a)=(4,4);}
   elsif($kind eq 'f64'){($size,$a)=(8,8);}elsif($kind eq 'int'&&($ft->{bits}==8||$ft->{bits}==16||$ft->{bits}==32||$ft->{bits}==64)){($size,$a)=($ft->{bits}/8,$ft->{bits}/8);}
   else{push @why,"field-$i-not-flat-scalar:$kind";next;}
   push @why,"field-$i-custom-destructor" if exists $DTOR{$f->{type}};
   $extent=int(($extent+$a-1)/$a)*$a;push @offset,dec($extent);$extent+=$size;$align=$a if $a>$align;$nonzero=1 if $size;
  }
  push @why,'no-nonzero-field' unless $nonzero;
 }
 push @why,'custom-destructor' if exists $DTOR{$id};
 $extent=1 if !$extent;$extent=int(($extent+$align-1)/$align)*$align;
 $FLAT{$id}={type=>dec($id),name=>$t->{name},shape_pass=>@why?'no':'yes',reasons=>\@why,
  nominal_origin=>'unestablished-by-ir-shape',layout_bytes=>@why?'not-derived-for-blocked-shape':dec($extent),alignment=>@why?'NA':dec($align),
  scalar_field_offsets=>@why?[]:\@offset};return $FLAT{$id};
}
sub verified_inputs{
 my($root,$expected)=@_;my $manifest=read_file("$root/manifest.sha256",1048576);
 die "manifest pin\n" unless sha256_hex($manifest) eq $expected;my %files;
 for my $line(split /\n/,$manifest){my($sha,$name)=$line=~/\A([0-9a-f]{64})  ([A-Za-z0-9_.\/-]+)\z/;
  die "manifest path\n" unless defined($name)&&$name!~m{(?:\A|/)\.\.?(?:/|\z)}&&!exists $files{$name};
  my $b=read_file("$root/$name",134217728);die "hash mismatch $name\n" unless sha256_hex($b) eq $sha;$files{$name}=$b;}
 return \%files;
}
sub local_id{my($f,$s)=@_;my $i=uint($s,'local');die "local out of range\n" unless $i<@{$f->{locals}};return $i;}
sub add_event{
 my($f,$pc,$local,$kind,$role,$extra)=@_;die "event local\n" unless $local<@{$f->{locals}};
 my $e={instruction=>$pc,local=>$local,kind=>$kind,role=>$role,%{$extra//{}}};
 push @{$f->{events}},$e;push @{$f->{by_local}{$local}},$e;
 push @{$f->{definitions}{$local}},$e if $kind eq 'whole-definition';
 return $e;
}
sub operand{
 my($f,$pc,$value,$role,$consume)=@_;my $local=local_id($f,$value);
 add_event($f,$pc,$local,'use',$role,{consumption=>$consume});return $local;
}
sub place{
 my($f,$pc,$w,$at,$role,$consume)=@_;my $root=local_id($f,$w->[$at++]);my @path;
 while($at<@$w){my $p=$w->[$at++];my @args;
  if($p eq 'deref'){}
  elsif($p eq 'field'){push @args,uint($w->[$at++],'field');}
  elsif($p eq 'variant'){push @args,uint($w->[$at++],'variant'),uint($w->[$at++],'field');}
  elsif($p eq 'index'){my $i=operand($f,$pc,$w->[$at++],'projection-index','not-classified');push @args,$i;}
  else{die "place projection\n";}push @path,[$p,@args];}
 my $e=add_event($f,$pc,$root,'use',$role,{consumption=>$consume,projections=>\@path});return($root,\@path,$e);
}
sub destination{my($f,$pc,$w)=@_;my $dst=local_id($f,$w->[2]);add_event($f,$pc,$dst,'whole-definition',$w->[1],{});return $dst;}
sub inventory{
 my($f)=@_;my @scope;my @barrier;
 for my $i(0..$f->{params}-1){add_event($f,-1,$i,'whole-definition','parameter',{});}
 for my $pc(0..$#{$f->{body}}){
  my $w=$f->{body}[$pc]{words};my $op=$w->[1];$f->{body}[$pc]{scope}=[@scope];
  if($op=~/\A(?:text|int|float|unit)\z/){destination($f,$pc,$w);}
  elsif($op eq 'read'){
   my $dst=destination($f,$pc,$w);my($src,$path)=place($f,$pc,$w,4,$w->[3] eq 'move'?'read-move':'read-copy',$w->[3] eq 'move'?'consumes':'preserves');
   if(!@$path&&$f->{locals}[$src]{type}==$f->{locals}[$dst]{type}){my $edge={instruction=>$pc,source=>$src,destination=>$dst,mode=>$w->[3],type=>$f->{locals}[$src]{type}};
    push @{$f->{edges}},$edge;push @{$f->{edges_from}{$src}},$edge;}
  }elsif($op eq 'borrow'||$op eq 'slice-borrow'){
   destination($f,$pc,$w);place($f,$pc,$w,4,$op,'borrow');
  }elsif($op eq 'view-range'){
   destination($f,$pc,$w);operand($f,$pc,$w->[5],'range-start','not-classified');operand($f,$pc,$w->[6],'range-end','not-classified');place($f,$pc,$w,7,'view-range','borrow');
  }elsif($op eq 'array'||$op eq 'struct'||$op eq 'variant'){
   my $dst=destination($f,$pc,$w);my $start=$op eq 'variant'?4:3;my @inputs;
   for my $n($start..$#$w){push @inputs,operand($f,$pc,$w->[$n],"$op-input","consumes");}
   push @{$f->{roots}},{instruction=>$pc,local=>$dst,type=>$f->{locals}[$dst]{type},inputs=>\@inputs} if $op eq 'struct';
  }elsif($op eq 'assign'){
   operand($f,$pc,$w->[2],'assignment-source','consumes');my($dst,$path,$event)=place($f,$pc,$w,3,'assignment-place','writes');
   if(!@$path){add_event($f,$pc,$dst,'whole-definition','assign',{});}else{$event->{kind}='partial-write';}
  }elsif($op eq 'drop'||$op eq 'drop-if'){place($f,$pc,$w,2,$op,$op eq 'drop'?'consumes':'conditionally-consumes');}
  elsif($op eq 'unary'){destination($f,$pc,$w);operand($f,$pc,$w->[4],'unary-operand','not-classified');}
  elsif($op eq 'binary'){destination($f,$pc,$w);operand($f,$pc,$w->[4],'binary-left','not-classified');operand($f,$pc,$w->[5],'binary-right','not-classified');}
  elsif($op eq 'call'||$op eq 'intrinsic'){
   destination($f,$pc,$w);for my $n(4..$#$w){operand($f,$pc,$w->[$n],$op eq 'call'?'call-argument':'intrinsic-argument',$op eq 'call'?'consumes':'intrinsic-specific');}
  }elsif($op eq 'tag'){destination($f,$pc,$w);operand($f,$pc,$w->[3],'discriminant-source','not-classified');}
  elsif($op eq 'tag-test'){destination($f,$pc,$w);operand($f,$pc,$w->[3],'tag-test-source','not-classified');}
  elsif($op eq 'tag-test-place'){destination($f,$pc,$w);place($f,$pc,$w,4,'tag-test-place','not-classified');}
  elsif($op eq 'if'){operand($f,$pc,$w->[2],'if-condition','not-classified');push @scope,"if:$pc:then";}
  elsif($op eq 'else'){die "else scope\n" unless @scope&&$scope[-1]=~/\Aif:([0-9]+):then\z/;$scope[-1]="if:$1:else";}
  elsif($op eq 'loop'){push @scope,"loop:$pc";}
  elsif($op eq 'end'){die "end scope\n" unless @scope;pop @scope;}
  elsif($op eq 'return'){operand($f,$pc,$w->[2],'return-source','consumes');}
  elsif($op=~/\A(?:break|continue|break-to|continue-to|trap)\z/){}
  else{die "unhandled body operation $op\n";}
  if($op=~/\A(?:if|else|loop|end|break|continue|break-to|continue-to|return|trap|call|intrinsic)\z/){
   push @barrier,{instruction=>$pc,operation=>$op,scope_before=>$f->{body}[$pc]{scope},scope_after=>[@scope]};}
 }
 die "unfinished body scope\n" if @scope;$f->{barriers}=\@barrier;
}
sub event_text{
 my($f,$e)=@_;return $e->{instruction}<0?$f->{locals}[$e->{local}]{raw}:$f->{body}[$e->{instruction}]{raw};
}
my $result={version=>'1',purpose=>'complete-bounded-family-evidence',whole_program_conclusion=>'none',
 nominal_origin=>'unestablished-by-ir-shape',compiler_executed=>'no',profile_executed=>'no',join_or_extractor_rerun=>'no',extra_page=>'no',pins=>\%PIN};
my $ok=eval{
 my $x=verified_inputs($args{extraction},$PIN{extraction});my $j=verified_inputs($args{join},$PIN{join});
 die "companion pin\n" unless sha256_hex($x->{'inputs/companion.lkjir'}) eq $PIN{companion};
 my $top=$J->decode($x->{'extraction.json'});die "incomplete extraction\n" unless $top->{status} eq 'complete'&&@{$top->{functions}}==20;
 my @joined=split /\n/,$j->{'selected-copy-rows.tsv'};my @keys=split /\t/,shift(@joined),-1;my(%rows_by_function,%row_by_id);
 for my $line(@joined){my @v=split /\t/,$line,-1;die "joined row width\n" unless @v==@keys;my %r;@r{@keys}=@v;
  die "duplicate joined raw row\n" if exists $row_by_id{$r{raw_row}};$row_by_id{$r{raw_row}}=\%r;push @{$rows_by_function{$r{function}}},\%r;}
 my(%F,%local_types);
 for my $ordinal(0..$#ORDER){my $id=$ORDER[$ordinal];my $meta=$top->{functions}[$ordinal];die "population change\n" unless $meta->{function} eq dec($id)&&$meta->{ordinal} eq dec($ordinal)&&$meta->{status} eq 'complete';
  my $dir=sprintf('functions/%02u-function-%u',$ordinal,$id);my $raw=$x->{"$dir/body.lkjir"}//die "body absent\n";
  die "body hash\n" unless sha256_hex($raw) eq $meta->{body_sha256};my @lines=split /(?<=\n)/,$raw;my $h=words(shift(@lines),1);
  my $f={id=>$id,ordinal=>$ordinal,name=>$h->[1],params=>uint($h->[2],'params'),result=>uint($h->[3],'result'),meta=>$meta,locals=>[],body=>[],events=>[],roots=>[],edges=>[],edges_from=>{},by_local=>{},definitions=>{}};
  die "body framing\n" unless $h->[0] eq 'fn';my $ended=0;
  for my $line(@lines){my $w=words($line,1);next unless @$w;die "after endfn\n" if $ended;
   if($w->[0] eq 'local'){push @{$f->{locals}},{name=>$w->[1],type=>uint($w->[2],'local type'),mutable=>$w->[3],raw=>$line};}
   elsif($w->[0] eq 'return-leaf'){}
   elsif($w->[0] eq 'endfn'){$ended=1;}
   else{die "instruction framing\n" unless $w->[0]=~/\A\@/;push @{$f->{body}},{words=>$w,raw=>$line};}}
  die "body counts\n" unless $ended&&@{$f->{locals}}==$meta->{local_count}&&@{$f->{body}}==$meta->{instructions};
  my @types=split /\n/,$x->{"$dir/types.tsv"};shift @types;
  for my $line(@types){my($ty,$at,$len,$sha,$kind,$name)=split /\t/,$line,-1;($ty,$at,$len)=map{uint($_,'type index')}($ty,$at,$len);
   my $tr=substr($x->{'inputs/companion.lkjir'},$at,$len);die "type hash\n" unless sha256_hex($tr) eq $sha;
   if(exists $TYPE{$ty}){die "type conflict\n" unless $TYPE{$ty}{raw} eq $tr;}else{$TYPE{$ty}=parse_type($ty,$tr,{sha256=>$sha,offset=>$at,length=>$len});}}
  $F{$id}=$f;
 }
 for my $line(split /\n/,$x->{'shared/destructors.lkjir'}){my $w=words($line,1);next unless @$w;die "destructor record\n" unless @$w==3&&$w->[0] eq 'destructor';push @{$DTOR{$w->[1]}},uint($w->[2],'destructor');}
 my $locals_tsv="function\tlocal\ttype\tlocal_name_json\tparameter\twhole_definition_count\tuse_count\tshape_json\n";
 my $events_tsv="function\tlocal\ttype\tinstruction\tkind\trole\tconsumption\tprojections_json\tscope_json\trecord_json\n";
 my $edges_tsv="function\tinstruction\tsource\tdestination\ttype\tmode\n";
 my $roots_tsv="function\tinstruction\tlocal\ttype\tshape_status\tmembers\tknown_blockers_json\tbody_sha256\tassociated_raw_rows\tassociated_raw_samples\n";
 my $controls_tsv="function\tinstruction\toperation\tscope_before_json\tscope_after_json\n";
 my $functions_tsv="ordinal\tfunction\tname_json\tinstructions\tflat_shape_locals\tflat_shape_types\tall_struct_roots\tflat_shape_struct_roots\n";
 my(@families,@flat_types,@function_results);my %global_flat;my($total_flat_locals,$total_roots,$total_flat_roots,$instruction_count)=(0,0,0,0);
 mkdir("$OUT/families",0700) or die "families directory\n";
 for my $id(@ORDER){my $f=$F{$id};inventory($f);my @flat_locals;my %flat_types_here;
  for my $local(0..$#{$f->{locals}}){my $ty=$f->{locals}[$local]{type};my $s=type_screen($ty);next unless $s->{shape_pass} eq 'yes';
   push @flat_locals,$local;$flat_types_here{$ty}=1;$global_flat{$ty}=1;
   my @defs=@{$f->{definitions}{$local}//[]};my @uses=grep {$_->{kind} ne 'whole-definition'}@{$f->{by_local}{$local}//[]};
   $locals_tsv.=join("\t",dec($id),dec($local),dec($ty),$J->encode($f->{locals}[$local]{name}),$local<$f->{params}?'yes':'no',scalar(@defs),scalar(@uses),$J->encode($s))."\n";
   for my $e(@{$f->{by_local}{$local}//[]}){my $raw=event_text($f,$e);my $text=decode('UTF-8',$raw,FB_CROAK);
    $events_tsv.=join("\t",dec($id),dec($local),dec($ty),dec($e->{instruction}),$e->{kind},$e->{role},$e->{consumption}//'not-applicable',
     $J->encode($e->{projections}//[]),$J->encode($e->{instruction}<0?[]:$f->{body}[$e->{instruction}]{scope}),$J->encode($text))."\n";}
  }
  for my $edge(@{$f->{edges}}){next unless type_screen($edge->{type})->{shape_pass} eq 'yes';$edges_tsv.=join("\t",dec($id),map{dec($edge->{$_})}qw(instruction source destination type mode))."\n";}
  for my $barrier(@{$f->{barriers}}){$controls_tsv.=join("\t",dec($id),dec($barrier->{instruction}),$barrier->{operation},$J->encode($barrier->{scope_before}),$J->encode($barrier->{scope_after}))."\n";}
  my $flat_root_count=0;
  for my $root(@{$f->{roots}}){my($pc,$local,$ty)=@$root{qw(instruction local type)};my $shape=type_screen($ty);
   my @members;my @family_edges;my @events;my %block;my @uncertain;
   my %member;
   if($shape->{shape_pass} eq 'yes'){
    $flat_root_count++;my @queue=($local);my $at=0;
    while($at<@queue){my $node=$queue[$at++];next if $member{$node}++;push @members,$node;
     for my $edge(@{$f->{edges_from}{$node}//[]}){next unless $edge->{type}==$ty;push @family_edges,$edge;push @queue,$edge->{destination};}}
    @members=sort {$a<=>$b}@members;
    for my $node(@members){
     my @defs=@{$f->{definitions}{$node}//[]};$block{"member-$node-is-parameter"}=1 if $node<$f->{params};
     $block{"member-$node-not-single-whole-definition"}=1 unless @defs==1;
     my $def_at=@defs==1?$defs[0]{instruction}:undef;
     if(defined($def_at)){for my $e(@{$f->{by_local}{$node}//[]}){$block{"member-$node-used-before-definition"}=1 if $e->{kind} ne 'whole-definition'&&$e->{instruction}<$def_at;}}
     for my $e(@{$f->{by_local}{$node}//[]}){push @events,$e;next if $e->{kind} eq 'whole-definition';
      my $role=$e->{role};my $p=$e->{projections}//[];
      if($role eq 'read-copy'||$role eq 'read-move'){
       if(@$p){if(@$p!=1||$p->[0][0] ne 'field'){$block{"non-direct-field-read-at-$e->{instruction}"}=1;}
        elsif($role eq 'read-move'){$block{"partial-field-move-at-$e->{instruction}"}=1;}}
      }elsif($role eq 'drop'||$role eq 'drop-if'){$block{"partial-drop-at-$e->{instruction}"}=1 if @$p;}
      elsif($role eq 'assignment-place'){$block{"write-or-reinitialization-at-$e->{instruction}"}=1;}
      elsif($role eq 'assignment-source'){$block{"assignment-transfer-outside-read-family-at-$e->{instruction}"}=1;}
      elsif($role eq 'call-argument'){$block{"aggregate-call-argument-at-$e->{instruction}"}=1;}
      elsif($role eq 'return-source'){$block{"aggregate-return-at-$e->{instruction}"}=1;}
      elsif($role=~/\A(?:array|struct|variant)-input\z/){$block{"embedding-at-$e->{instruction}"}=1;}
      elsif($role eq 'borrow'||$role eq 'slice-borrow'||$role eq 'view-range'){$block{"borrow-or-view-at-$e->{instruction}"}=1;}
      elsif($role eq 'intrinsic-argument'){$block{"aggregate-intrinsic-use-at-$e->{instruction}"}=1;}
      else{$block{"outside-initial-use-subset:$role:at-$e->{instruction}"}=1;}
     }
    }
    push @uncertain,'nominal-source-origin-unestablished';
    push @uncertain,'initialization-and-consumption-states-not-automatically-proved';
    push @uncertain,'control/lifetime-not-established-by-last-textual-use';
   }else{$block{"type:$_"}=1 for @{$shape->{reasons}};}
   my %event_pc=map {$_->{instruction}=>1}grep {$_->{instruction}>=0}@events;$event_pc{$pc}=1;
   my @associated=grep {$_->{instruction} ne 'parameter'&&$event_pc{$_->{instruction}}}@{$rows_by_function{$id}//[]};
   my %row;@associated=grep {!$row{$_->{raw_row}}++}@associated;
   my $samples=0;$samples+=$_->{samples} for @associated;
   my $last=$pc;for my $e(@events){$last=$e->{instruction} if $e->{instruction}>$last;}
   my @barriers=grep {$_->{instruction}>=$pc&&$_->{instruction}<=$last}@{$f->{barriers}};
   my $record={function=>dec($id),name=>$f->{name},body_sha256=>$f->{meta}{body_sha256},constructor_instruction=>dec($pc),root_local=>dec($local),type=>dec($ty),
    shape=>$shape,constructor_inputs=>[map{{local=>dec($_),type=>dec($f->{locals}[$_]{type}),name=>$f->{locals}[$_]{name}}}@{$root->{inputs}}],
    members=>[map{dec($_)}@members],whole_read_edges=>\@family_edges,all_member_definitions_and_uses=>\@events,
    known_blockers=>[sort keys %block],uncertainties=>\@uncertain,
    verdict=>keys(%block)?'blocked-by-known-rule':'unassessed',
    textual_envelope=>{from=>dec($pc),through_last_member_event=>dec($last),is_lifetime_proof=>'no'},
    enclosing_scope=>$f->{body}[$pc]{scope},control_and_call_barriers_in_textual_envelope=>\@barriers,
    raw_row_ids=>[map{$_->{raw_row}}@associated],associated_raw_samples=>dec($samples),
    sample_association=>'all joined intervals at constructor/member-event instructions; may overlap another family; not removable-time attribution'};
   put("families/function-$id-root-$pc.json",json($record));push @families,$record;
   $roots_tsv.=join("\t",dec($id),dec($pc),dec($local),dec($ty),$shape->{shape_pass},join(',',@members),$J->encode([sort keys %block]),$f->{meta}{body_sha256},
    join(',',map{$_->{raw_row}}@associated),dec($samples))."\n";
  }
  my $fr={ordinal=>dec($f->{ordinal}),function=>dec($id),name=>$f->{name},instructions=>dec(scalar @{$f->{body}}),
   flat_shape_locals=>[map{dec($_)}@flat_locals],flat_shape_types=>[map{dec($_)}sort{$a<=>$b}keys%flat_types_here],
   all_struct_roots=>dec(scalar @{$f->{roots}}),flat_shape_struct_roots=>dec($flat_root_count)};
  push @function_results,$fr;$total_flat_locals+=@flat_locals;$total_roots+=@{$f->{roots}};$total_flat_roots+=$flat_root_count;$instruction_count+=@{$f->{body}};
  $functions_tsv.=join("\t",dec($f->{ordinal}),dec($id),$J->encode($f->{name}),dec(scalar @{$f->{body}}),dec(scalar @flat_locals),join(',',sort{$a<=>$b}keys%flat_types_here),
   dec(scalar @{$f->{roots}}),dec($flat_root_count))."\n";
 }
 put('functions.tsv',$functions_tsv);put('flat-shape-locals.tsv',$locals_tsv);put('flat-shape-definitions-uses.tsv',$events_tsv);
 put('whole-local-read-edges.tsv',$edges_tsv);put('struct-roots.tsv',$roots_tsv);put('body-control-call-sites.tsv',$controls_tsv);
 put('flat-shape-types.json',json([map{type_screen($_)}sort{$a<=>$b}keys%global_flat]));
 $result->{selection}=[map{dec($_)}@ORDER];$result->{functions}=\@function_results;
 $result->{instructions}=dec($instruction_count);$result->{flat_shape_locals}=dec($total_flat_locals);
 $result->{flat_shape_types}=[map{dec($_)}sort{$a<=>$b}keys%global_flat];
 $result->{all_struct_constructor_roots}=dec($total_roots);$result->{flat_shape_struct_constructor_roots}=dec($total_flat_roots);
 $result->{family_root_verdicts}={blocked=>dec(scalar grep{$_->{verdict} eq 'blocked-by-known-rule'}@families),
  unassessed=>dec(scalar grep{$_->{verdict} eq 'unassessed'}@families),eligible=>'not-issued'};
 $result->{read_closure_rule}='same-type whole-local Read edges; multi-definition local ambiguity is a blocker, not a proven version-sensitive family';
 $result->{initialization_policy}='explicit consumption roles are evidence; no serialized loan state or complete lifetime proof is invented';
 $result->{status}='complete-inventory';1;
};
my $error=$@;if(!$ok){$result->{status}='refused';$result->{error}="$error";}$result->{exit_status}=$ok?'0':'1';
put('result.json',json($result));my $m=join('',map{"$written{$_}  $_\n"}sort keys%written);
sysopen(my $mf,"$OUT/manifest.sha256",O_WRONLY|O_CREAT|O_EXCL,0600) or die "manifest";print $mf $m;close $mf or die "manifest close";
print json($result);exit($ok?0:1);
