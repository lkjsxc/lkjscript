#!/usr/bin/env perl
use strict;
use warnings;
use Config;
use Cwd qw(abs_path);
use Digest::SHA qw(sha256_hex);
use Encode qw(decode encode FB_CROAK);
use Fcntl qw(O_RDONLY O_WRONLY O_CREAT O_EXCL O_NOFOLLOW SEEK_SET :mode);
use File::Basename qw(dirname basename);
use JSON::PP;

# Offline extraction only. No compiler, profile execution, admission, layout,
# loan-state reconstruction, eligibility decision, or unseen-site join.
my %limit = (
    functions => 20, detail_rows => 200, detail_bytes => 1048576,
    instructions => 65536, body_bytes => 16777216, types => 16384,
    signatures => 2048, context_bytes => 33554432,
    scan_line_bytes => 33554432, free_space_bytes => 3221225472,
);
my $plan_sha = '8a1f3e3d5354b3eca7fd9adb68bfee892f94360130a19b265f3f9ffb73c4dc15';
my $grammar_sha = 'af4c78b51063bce8a24e2a35bd364a48cbb3a888d01c1fd6219faa3d5983d4f8';
die "requires a 64-bit Perl integer build\n" unless $Config{ivsize} >= 8;
my %arg;
while (@ARGV) {
    my $name = shift @ARGV;
    die "expected --companion, --detail, or --output\n"
        unless $name =~ /\A--(companion|detail|output)\z/ && @ARGV;
    my $key = $1;
    die "duplicate $name\n" if exists $arg{$key};
    $arg{$key} = shift @ARGV;
}
die "usage: --companion IR --detail FIRST_PAGE --output ABSENT_DIR\n"
    unless keys(%arg) == 3;
my $parent = abs_path(dirname($arg{output}));
die "output parent must exist\n" unless defined $parent && -d $parent;
my $base = basename($arg{output});
die "unsafe output basename\n" unless $base =~ /\A[A-Za-z0-9][A-Za-z0-9_.-]*\z/;
my $out = "$parent/$base";
die "output already exists; preserve it and use a separately released path\n" if lstat($out);
open(my $df, '-|', 'df', '-Pk', '--', $parent) or die "free-space observation: $!\n";
my @df = <$df>; close $df or die "free-space observation failed\n";
my @df_fields = split /\s+/, $df[-1] // '';
die "unrecognized free-space observation\n"
    unless @df_fields >= 6 && $df_fields[3] =~ /\A[0-9]+\z/;
my $free_kib = 0 + $df_fields[3];
die "need at least 3 GiB available before extraction\n"
    if $free_kib < $limit{free_space_bytes} / 1024;
mkdir($out, 0700) or die "claim output: $!\n";
for my $d ('inputs', 'shared', 'functions') {
    mkdir("$out/$d", 0700) or die "create output subdirectory: $!\n";
}
my %written;
my $j = JSON::PP->new->canonical->utf8;
sub dec { return sprintf('%u', $_[0]); }
sub json { return JSON::PP->new->canonical->utf8->pretty->encode($_[0]); }
sub qjson { return $j->encode($_[0]); }
sub bytes_write {
    my ($relative, $bytes) = @_;
    die "unsafe output path\n" unless $relative =~ /\A[A-Za-z0-9_.\/-]+\z/
        && $relative !~ m{(?:\A|/)\.\.?(?:/|\z)};
    sysopen(my $f, "$out/$relative", O_WRONLY|O_CREAT|O_EXCL, 0600)
        or die "write $relative: $!\n";
    binmode $f;
    print $f $bytes or die "write bytes $relative: $!\n";
    close $f or die "close $relative: $!\n";
    $written{$relative} = sha256_hex($bytes);
}
sub uint {
    my ($text, $context) = @_;
    die "$context: invalid usize\n" unless defined($text) && $text =~ /\A\+?[0-9]+\z/;
    $text =~ s/\A\+//; $text =~ s/\A0+(?=[0-9])//;
    die "$context: usize overflow\n" if length($text) > 20
        || (length($text) == 20 && $text gt '18446744073709551615');
    return 0 + $text;
}
sub count_exact {
    my ($w, $n, $line) = @_;
    die "line $line: wrong operand count\n" unless @$w == $n;
}
sub need_flag {
    my ($text, $a, $b, $line) = @_;
    die "line $line: invalid flag\n" unless defined($text) && ($text eq $a || $text eq $b);
}
# Mirrors ir_text::words, including Unicode White_Space, Cc rejection, JSON
# escapes, paired surrogates, quoted separators, and '#' outside quoted words.
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
sub place {
    my ($w, $at, $end, $line) = @_;
    uint($w->[$at++], "line $line place");
    while ($at < $end) {
        my $kind = $w->[$at++];
        my $n = $kind eq 'deref' ? 0 : ($kind eq 'field' || $kind eq 'index') ? 1
            : $kind eq 'variant' ? 2 : -1;
        die "line $line: unknown place projection\n" if $n < 0;
        for (1..$n) { die "line $line: missing place operand\n" if $at >= $end;
            uint($w->[$at++], "line $line projection"); }
    }
    die "line $line: incomplete place\n" if $at != $end;
}
sub fields {
    my ($w, $start, $line) = @_;
    die "line $line: field name/type pairing\n" if (@$w-$start) % 2;
    my @refs;
    for (my $i=$start+1; $i<@$w; $i+=2) { push @refs, uint($w->[$i], "line $line field"); }
    return \@refs;
}
sub signature_header {
    my ($w, $line) = @_;
    die "line $line: short fn header\n" unless @$w >= 4;
    my ($params,$result) = (uint($w->[2],"line $line params"), uint($w->[3],"line $line result"));
    my ($file,$from,$test) = (undef,0,0);
    for (my $i=4; $i<@$w;) {
        my $flag=$w->[$i];
        if ($flag eq 'test' && !$test) { $test=1; $i++; }
        elsif ($flag eq 'file' && !defined($file)) { $file=uint($w->[$i+1],"line $line file"); $i+=2; }
        elsif ($flag eq 'from' && !$from) {
            $from=1; $i++;
            if (($w->[$i]//'') eq 'static') { $i++; }
            else { my $end=$i+1; $end++ while $end<@$w && $w->[$end] !~ /\A(?:test|file|from)\z/;
                place($w,$i,$end,$line); $i=$end; }
        } else { die "line $line: unknown/repeated fn flag\n"; }
    }
    return ($params,$result,$file//0,$from);
}
sub return_leaf {
    my ($w,$line)=@_;
    my $pivot;
    for (0..$#$w) { if ($w->[$_] =~ /\A(?:exact|bound|static)\z/) { $pivot=$_; last; } }
    die "line $line: missing return origin\n" unless defined($pivot);
    place($w,1,$pivot,$line);
    die "line $line: nonzero return leaf root\n" unless uint($w->[1],"return root")==0;
    if ($w->[$pivot] eq 'static') { count_exact($w,$pivot+1,$line); }
    else {
        my $at=$pivot+1;
        while ($at<@$w) { my $end=$at; $end++ while $end<@$w && $w->[$end] ne 'or';
            place($w,$at,$end,$line); $at=$end+1; }
        die "line $line: empty returned input\n" if $pivot+1>=@$w || $w->[-1] eq 'or';
    }
}
sub instruction {
    my ($w,$line)=@_;
    die "line $line: instruction source location required\n"
        unless defined($w->[0]) && $w->[0] =~ /\A\@(?:(\+?[0-9]+)\/)?(\+?[0-9]+)(?::(\+?[0-9]+))?\z/;
    my ($file,$source_line,$column)=($1,$2,$3);
    uint($file,"line $line source file") if defined $file;
    uint($source_line,"line $line source line"); uint($column,"line $line column") if defined $column;
    my $op=$w->[1]//die "line $line: missing operation\n";
    my %fixed=(text=>4,int=>4,float=>4,unit=>3,unary=>5,binary=>6,tag=>4,'tag-test'=>5,
        if=>3,'break-to'=>3,'continue-to'=>3,return=>3,trap=>3,else=>2,end=>2,loop=>2,break=>2,continue=>2);
    my @numbers;
    if (exists $fixed{$op}) {
        count_exact($w,$fixed{$op},$line);
        if ($op eq 'text') { push @numbers,2; die "line $line: bad text bytes\n" unless $w->[3] =~ /\A(?:[0-9a-fA-F]{2})*\z/; }
        elsif ($op eq 'int') { push @numbers,2; die "line $line: invalid signed integer spelling\n" unless $w->[3] =~ /\A[+-]?[0-9]+\z/; }
        elsif ($op eq 'float') { push @numbers,2,3; }
        elsif ($op eq 'unary') { push @numbers,2,4; }
        elsif ($op eq 'binary') { push @numbers,2,4,5; }
        elsif ($op eq 'tag') { push @numbers,2,3; }
        elsif ($op eq 'tag-test') { push @numbers,2,3,4; }
        elsif ($op eq 'unit' || $op eq 'if' || $op eq 'return' || $op =~ /\A(?:break|continue)-to\z/) { push @numbers,2; }
    } elsif ($op eq 'read' || $op eq 'borrow' || $op eq 'slice-borrow') {
        push @numbers,2;
        need_flag($w->[3],$op eq 'read'?'move':'mut',$op eq 'read'?'copy':'shared',$line);
        place($w,4,scalar(@$w),$line);
    } elsif ($op eq 'view-range') {
        push @numbers,2,5,6; need_flag($w->[3],'mut','shared',$line);
        need_flag($w->[4],'inclusive','exclusive',$line); place($w,7,scalar(@$w),$line);
    } elsif ($op eq 'assign') { push @numbers,2; place($w,3,scalar(@$w),$line); }
    elsif ($op eq 'drop' || $op eq 'drop-if') { place($w,2,scalar(@$w),$line); }
    elsif ($op eq 'tag-test-place') { push @numbers,2,3; place($w,4,scalar(@$w),$line); }
    elsif ($op =~ /\A(?:array|struct|variant|call|intrinsic)\z/) {
        my $minimum=$op =~ /\A(?:variant|call|intrinsic)\z/ ? 4 : 3;
        die "line $line: missing variadic operand\n" unless @$w >= $minimum;
        @numbers=grep { !($op eq 'intrinsic' && $_==3) } (2..$#$w);
    } else { die "line $line: unsupported operation $op\n"; }
    uint($w->[$_],"line $line operand") for @numbers;
    return ($op, $op eq 'call' ? uint($w->[3],"line $line callee") : undef,
        defined($file)?dec(uint($file,'file')):'default',dec(uint($source_line,'source line')),defined($column)?dec(uint($column,'column')):'1');
}
sub open_regular {
    my ($path)=@_;
    sysopen(my $f,$path,O_RDONLY|O_NOFOLLOW) or die "open input $path: $!\n";
    binmode $f; my @st=stat($f);
    die "input is not regular: $path\n" unless @st && S_ISREG($st[2]);
    return ($f,\@st);
}
sub snapshot {
    my ($path,$relative,$cap)=@_;
    my ($in,$before)=open_regular($path);
    die "input exceeds explicit bound: $path\n" if defined($cap) && $before->[7]>$cap;
    sysopen(my $dest,"$out/$relative",O_WRONLY|O_CREAT|O_EXCL,0600) or die "snapshot destination: $!\n";
    binmode $dest; my $sha=Digest::SHA->new(256); my $length=0;
    while (1) {
        my $n=read($in,my $chunk,65536); die "snapshot read: $!\n" unless defined $n;
        last if !$n; $length+=$n;
        die "input grew past explicit bound\n" if defined($cap) && $length>$cap;
        print $dest $chunk or die "snapshot write: $!\n"; $sha->add($chunk);
    }
    my @after=stat($in);
    for my $i (0,1,2,7,9,10) { die "input metadata changed during snapshot\n" unless $before->[$i]==$after[$i]; }
    die "input length changed during snapshot\n" unless $length==$before->[7];
    close $in or die "input close\n"; close $dest or die "snapshot close\n";
    my $hash=$sha->hexdigest; $written{$relative}=$hash;
    return {path=>$path,bytes=>dec($length),sha256=>$hash,snapshot=>$relative,
        device=>dec($before->[0]),inode=>dec($before->[1])};
}
sub read_small {
    my ($path)=@_; my ($f,$st)=open_regular($path);
    local $/; my $bytes=<$f>; close $f or die "read close\n"; return $bytes//'';
}
my $result = {
    version=>'1',purpose=>'complete-extraction-only',plan_sha256=>$plan_sha,ir_text_sha256=>$grammar_sha,
    eligibility=>'not-assessed',semantic_admission=>'not-performed',
    layout=>'not-computed',loan_states=>'not-serialized',native_offsets=>'not-in-companion',
    unseen_copy_rows=>'not-joined',source_paths_opened=>'no',
    limits=>{map { $_=>dec($limit{$_}) } sort keys %limit},
    free_space_kib_before=>dec($free_kib),
};
my $exit=0;
my $ok=eval {
    my $detail_pin=snapshot($arg{detail},'inputs/detail.txt',$limit{detail_bytes});
    my $detail=read_small("$out/inputs/detail.txt");
    die "detail must end in LF\n" unless $detail =~ /\n\z/;
    my @rows=split /\n/,$detail;
    my $header=shift @rows;
    my ($profile_id,$companion_id,$sites)=$header =~ /\Acopy-sites version=4 profile=([0-9a-f]{64}) companion=([0-9a-f]{64}) companion-check=checked-ir serial=[0-9]+ total-samples=[0-9]+ copy-samples=[0-9]+ mapped=[0-9]+ unmapped=[0-9]+ in-flight=[0-9]+ sites=([0-9]+) order=samples-desc\/function\/instruction\/subsite\/start\/end\z/;
    die "unsupported detail header\n" unless defined $sites;
    $sites=uint($sites,'sites');
    my $scope=shift @rows;
    die "unsupported detail scope\n" unless $scope eq 'scope=existing-kind-copy runtime=excluded variable-width-copy=excluded replace=excluded scalar-frame-address-work=excluded counters=sampled-native-observation';
    my $tail=pop @rows;
    my ($cursor,$nrows,$next,$total)=$tail =~ /\Acontinuation cursor=(none|4:[0-9a-f]{64}:[0-9a-f]{64}:[0-9]+) rows=([0-9]+) next=([0-9]+) total-sites=([0-9]+)\z/;
    die "unsupported detail continuation\n" unless defined $total;
    ($nrows,$next,$total)=map { uint($_,'continuation') } ($nrows,$next,$total);
    die "detail is not the declared first page\n" unless $nrows==@rows && $next==$nrows && $total==$sites
        && $nrows<=$limit{detail_rows} && $next<=$sites;
    die "detail continuation pin mismatch\n" unless $cursor eq ($next==$sites ? 'none' : "4:$profile_id:$companion_id:$next");
    my (@displayed,@selected,%seen);
    for my $ordinal (0..$#rows) {
        my $line=$rows[$ordinal];
        my ($fn,$inst,$sub,$symbol,$samples,$ty,$width,$source,$dest,$sf,$df,$start,$end,$name,$type_name)=
            $line =~ /\Acopy function=([0-9]+) instruction=(parameter|[0-9]+) subsite=([0-9]+) symbol=([0-9]+) samples=([0-9]+) type=([0-9]+) width=([0-9]+) source=([a-z-]+) destination=([a-z-]+) source-flags=([0-9]+) destination-flags=([0-9]+) start=([0-9]+) end=([0-9]+) name=("(?:[^"\\]|\\.)*") type-name=("(?:[^"\\]|\\.)*")\z/;
        die "unsupported detail row $ordinal\n" unless defined $type_name;
        my $parsed_name=$j->decode($name); my $parsed_type=$j->decode($type_name);
        die "detail name framing\n" if ref($parsed_name) || ref($parsed_type);
        ($fn,$sub,$symbol,$samples,$ty,$width,$sf,$df,$start,$end)=map {uint($_,'detail row')} ($fn,$sub,$symbol,$samples,$ty,$width,$sf,$df,$start,$end);
        $inst=uint($inst,'instruction') unless $inst eq 'parameter';
        die "detail interval/flags\n" unless $start<$end && $sf<=7 && $df<=7;
        push @displayed,{function=>$fn,instruction=>$inst,subsite=>$sub,type=>$ty,name=>$parsed_name,
            type_name=>$parsed_type,row=>$ordinal};
        if (!$seen{$fn}++ && @selected<$limit{functions}) { push @selected,{function=>$fn,first_row=>$ordinal}; }
    }
    my %selected=map { $_->{function}=>1 } @selected;
    $result->{detail}=$detail_pin; $result->{profile_sha256_declared}=$profile_id;
    $result->{displayed_rows}=dec(scalar @displayed); $result->{total_sites_declared}=dec($sites);
    $result->{selection_rule}='first-20-distinct-functions-in-first-appearance-order';
    $result->{companion}=snapshot($arg{companion},'inputs/companion.lkjir',undef);
    die "companion differs from exact official detail digest\n"
        unless $result->{companion}{sha256} eq $companion_id;
    my ($ir,$ir_stat)=open_regular("$out/inputs/companion.lkjir");
    my (@types,@functions,@sources,@destructors);
    my ($function,$enumeration,$line_no,$offset)=(undef,undef,0,0);
    my $buffer='';
    my $parse_line=sub {
        my ($raw)=@_; $line_no++;
        my $length=length($raw); my $span=[$offset,$length]; $offset+=$length;
        if ($line_no==1) { my $v=$raw; $v =~ s/\r?\n\z//;
            die "unsupported IR header\n" unless $v eq 'lkjscript-ir 1'; return; }
        my $w=words($raw,$line_no); return unless @$w;
        if ($enumeration) {
            if ($w->[0] eq 'variant') {
                die "line $line_no: variant name missing\n" unless @$w>=2;
                push @{$enumeration->{refs}},@{fields($w,2,$line_no)};
            } elsif ($w->[0] eq 'endtype') {
                count_exact($w,1,$line_no); $enumeration->{length}=$offset-$enumeration->{start};
                push @types,$enumeration; $enumeration=undef;
            } else { die "line $line_no: expected variant/endtype\n"; }
            return;
        }
        if ($function) {
            if ($w->[0] eq 'local') {
                count_exact($w,4,$line_no); need_flag($w->[3],'mut','let',$line_no);
                my $ty=uint($w->[2],"line $line_no local type");
                push @{$function->{local_types}},$ty if $selected{$function->{index}};
                if ($function->{locals}<$function->{params}) {
                    push @{$function->{parameter_types}},$ty; push @{$function->{signature_parts}},$span;
                }
                $function->{locals}++;
            } elsif ($w->[0] eq 'return-leaf') {
                die "line $line_no: mixed returned-view contract\n" if $function->{aggregate_from};
                return_leaf($w,$line_no); push @{$function->{signature_parts}},$span;
            } elsif ($w->[0] eq 'endfn') {
                count_exact($w,1,$line_no); $function->{length}=$offset-$function->{start};
                push @{$function->{signature_parts}},$span;
                die "function parameter count exceeds locals\n" if $function->{params}>$function->{locals};
                push @functions,$function; $function=undef;
            } else {
                my @op; my $valid=eval { @op=instruction($w,$line_no); 1; };
                if (!$valid) { push @{$function->{problems}},"$@"; }
                if ($selected{$function->{index}}) {
                    push @{$function->{instruction_spans}},[$function->{instructions},@$span,
                        $valid?$op[0]:'unsupported',@op[2..4]] if $function->{instructions}<$limit{instructions};
                    $function->{calls}{$op[1]}=1 if $valid && defined $op[1];
                }
                $function->{instructions}++;
            }
            return;
        }
        if ($w->[0] eq 'source') { count_exact($w,2,$line_no); push @sources,{span=>$span,name=>$w->[1]}; }
        elsif ($w->[0] eq 'destructor') {
            count_exact($w,3,$line_no);
            push @destructors,{span=>$span,type=>uint($w->[1],'destructor type'),function=>uint($w->[2],'destructor function')};
        } elsif ($w->[0] eq 'fn') {
            my ($params,$type,$file,$from)=signature_header($w,$line_no);
            $function={index=>scalar(@functions),name=>$w->[1],params=>$params,result=>$type,file=>$file,
                aggregate_from=>$from,start=>$span->[0],locals=>0,instructions=>0,signature_parts=>[$span],
                parameter_types=>[],local_types=>[],instruction_spans=>[],calls=>{},problems=>[]};
        } elsif ($w->[0] eq 'type') {
            die "line $line_no: missing type shape\n" unless @$w>=3;
            my $kind=$w->[2]; my @refs;
            if ($kind =~ /\A(?:unit|bool|char|f64|str|world|stdout|stdin|stderr|directory|read_directory|read_file|image|channel|worker|read_region|foreign_command|foreign_session|null_device|file|process|clock|child|listener|connector|readiness|entropy|stop|arguments|environment|exit_status)\z/) { count_exact($w,3,$line_no); }
            elsif ($kind eq 'int') { count_exact($w,5,$line_no); uint($w->[3],'integer bits'); need_flag($w->[4],'signed','unsigned',$line_no); }
            elsif ($kind eq 'ref' || $kind eq 'slice') { count_exact($w,5,$line_no); need_flag($w->[3],'mut','shared',$line_no); push @refs,uint($w->[4],'inner type'); }
            elsif ($kind eq 'array') { count_exact($w,5,$line_no); push @refs,uint($w->[3],'inner type'); uint($w->[4],'array length'); }
            elsif ($kind eq 'vec' || $kind eq 'box') { count_exact($w,4,$line_no); push @refs,uint($w->[3],'inner type'); }
            elsif ($kind eq 'struct') { need_flag($w->[3],'copy','move',$line_no); @refs=@{fields($w,4,$line_no)}; }
            elsif ($kind eq 'enum') { count_exact($w,4,$line_no); need_flag($w->[3],'copy','move',$line_no); }
            else { die "line $line_no: unsupported type shape $kind\n"; }
            my $type={index=>scalar(@types),name=>$w->[1],kind=>$kind,refs=>\@refs,start=>$span->[0],length=>$span->[1]};
            if ($kind eq 'enum') { $enumeration=$type; } else { push @types,$type; }
        } else { die "line $line_no: unsupported top-level record\n"; }
    };
    while (1) {
        my $n=sysread($ir,my $chunk,65536); die "IR scan read\n" unless defined $n;
        last if !$n; $buffer.=$chunk;
        while ((my $at=index($buffer,"\n"))>=0) {
            die "structural line exceeds explicit 32 MiB scan bound; unassessed\n" if $at+1>$limit{scan_line_bytes};
            my $raw=substr($buffer,0,$at+1,''); $parse_line->($raw);
        }
        die "structural line exceeds explicit 32 MiB scan bound; unassessed\n" if length($buffer)>$limit{scan_line_bytes};
    }
    $parse_line->($buffer) if length($buffer);
    die "empty or unfinished IR framing\n" unless $line_no && !$function && !$enumeration;
    die "IR snapshot size changed\n" unless $offset==$ir_stat->[7];
    my $range=sub {
        my ($start,$length)=@_;
        die "range outside snapshot\n" if $start>$offset || $length>$offset-$start;
        my $position=sysseek($ir,$start,SEEK_SET);
        die "range seek\n" unless defined($position) && $position==$start;
        my $bytes='';
        while (length($bytes)<$length) { my $n=sysread($ir,my $part,$length-length($bytes));
            die "short snapshot range\n" unless defined($n)&&$n; $bytes.=$part; }
        return $bytes;
    };
    for my $row (@displayed) {
        die "displayed function missing from companion\n" unless $row->{function}<@functions;
        die "displayed type missing from companion\n" unless $row->{type}<@types;
        my $f=$functions[$row->{function}];
        die "displayed names differ from companion\n" unless $row->{name} eq $f->{name}
            && $row->{type_name} eq $types[$row->{type}]{name};
        die "displayed instruction coordinate outside complete body\n"
            unless $row->{instruction} eq 'parameter' ? $row->{subsite}<$f->{params}
                : $row->{instruction}<$f->{instructions};
    }
    my %dtors;
    for my $d (@destructors) {
        die "destructor outside declaration tables\n" unless $d->{type}<@types && $d->{function}<@functions;
        push @{$dtors{$d->{type}}},$d->{function};
    }
    my ($source_bytes,$destructor_bytes)=('','');
    my $shared_index="kind\tindex\tbyte_offset\tbytes\tsha256\n";
    for my $i (0..$#sources) {
        my $span=$sources[$i]{span}; my $raw=$range->(@$span); $source_bytes.=$raw;
        $shared_index.=join("\t",'source',dec($i),map {dec($_)} @$span)."\t".sha256_hex($raw)."\n";
    }
    for my $i (0..$#destructors) {
        my $span=$destructors[$i]{span}; my $raw=$range->(@$span); $destructor_bytes.=$raw;
        $shared_index.=join("\t",'destructor',dec($i),map {dec($_)} @$span)."\t".sha256_hex($raw)."\n";
    }
    bytes_write('shared/sources.lkjir',$source_bytes);
    bytes_write('shared/destructors.lkjir',$destructor_bytes);
    bytes_write('shared/index.tsv',$shared_index);
    my $shared_bytes=length($source_bytes)+length($destructor_bytes);
    my $selection="ordinal\tfunction\tfirst_display_row\tname_json\tstatus\tbody_bytes\tinstructions\ttypes\tsignatures\tcontext_bytes\treasons_json\n";
    my @verdicts;
    for my $ordinal (0..$#selected) {
        my $pick=$selected[$ordinal]; my $id=$pick->{function}; my $f=$functions[$id];
        my @problems=@{$f->{problems}};
        push @problems,"body instruction bound exceeded" if $f->{instructions}>$limit{instructions};
        push @problems,"exact fn/endfn byte bound exceeded" if $f->{length}>$limit{body_bytes};
        my (%closure,%signature,%why,@tq,@fq,%missing);
        @tq=($f->{result},@{$f->{local_types}}); @fq=sort {$a<=>$b} keys %{$f->{calls}};
        $why{$_}{direct_call}=1 for @fq;
        my ($ti,$fi)=(0,0);
        while ($ti<@tq || $fi<@fq) {
            while ($ti<@tq) {
                my $ty=$tq[$ti++]; next if $closure{$ty}++;
                if ($ty>=@types) { $missing{"type $ty"}=1; next; }
                push @tq,@{$types[$ty]{refs}};
                for my $callee (@{$dtors{$ty}//[]}) { $why{$callee}{destructor}=1; push @fq,$callee; }
            }
            while ($fi<@fq) {
                my $callee=$fq[$fi++]; next if $signature{$callee}++;
                if ($callee>=@functions) { $missing{"function $callee"}=1; next; }
                my $sig=$functions[$callee];
                push @tq,$sig->{result},@{$sig->{parameter_types}};
            }
        }
        push @problems,map {"missing dependency: $_"} sort keys %missing;
        my @type_ids=sort {$a<=>$b} keys %closure;
        my @signature_ids=sort {$a<=>$b} keys %signature;
        push @problems,"referenced type bound exceeded" if @type_ids>$limit{types};
        push @problems,"direct/destructor signature bound exceeded" if @signature_ids>$limit{signatures};
        my $context_bytes=$f->{length}+$shared_bytes;
        $context_bytes+=$types[$_]{length} for grep {$_<@types} @type_ids;
        for my $callee (grep {$_<@functions} @signature_ids) {
            $context_bytes+=$_->[1] for @{$functions[$callee]{signature_parts}};
        }
        push @problems,"complete logical body/context byte bound exceeded" if $context_bytes>$limit{context_bytes};
        my $status=@problems ? 'unassessed' : 'complete';
        my $folder=sprintf('functions/%02u-function-%u',$ordinal,$id);
        mkdir("$out/$folder",0700) or die "function output directory\n";
        my $meta={ordinal=>dec($ordinal),function=>dec($id),first_display_row=>dec($pick->{first_row}),
            name=>$f->{name},status=>$status,eligibility=>'not-assessed',
            byte_offset=>dec($f->{start}),body_bytes=>dec($f->{length}),instructions=>dec($f->{instructions}),
            local_count=>dec($f->{locals}),parameter_count=>dec($f->{params}),result_type=>dec($f->{result}),
            types=>dec(scalar @type_ids),signatures=>dec(scalar @signature_ids),context_bytes=>dec($context_bytes),
            reasons=>\@problems,destructor_behavior=>'not-extracted; signatures-and-complete-declarations-only'};
        if ($status eq 'complete') {
            my $body=$range->($f->{start},$f->{length});
            $meta->{body_sha256}=sha256_hex($body); bytes_write("$folder/body.lkjir",$body);
            my ($type_text,$type_index)=('',"type\tbyte_offset\tbytes\tsha256\tkind\tname_json\n");
            for my $ty (@type_ids) {
                my $t=$types[$ty]; my $raw=$range->($t->{start},$t->{length}); $type_text.=$raw;
                $type_index.=join("\t",dec($ty),dec($t->{start}),dec($t->{length}),sha256_hex($raw),$t->{kind},qjson($t->{name}))."\n";
            }
            bytes_write("$folder/types.lkjir",$type_text); bytes_write("$folder/types.tsv",$type_index);
            mkdir("$out/$folder/signatures",0700) or die "signature directory\n";
            my $signature_index="function\treasons\tbytes\tsha256\tname_json\n";
            my $parts_index="function\tpart\tbyte_offset\tbytes\tsha256\n";
            for my $callee (@signature_ids) {
                my $sig=$functions[$callee]; my $text=''; my $part=0;
                for my $span (@{$sig->{signature_parts}}) {
                    my $raw=$range->(@$span); $text.=$raw;
                    $parts_index.=join("\t",dec($callee),dec($part++),map {dec($_)} @$span)."\t".sha256_hex($raw)."\n";
                }
                bytes_write("$folder/signatures/function-$callee.lkjir",$text);
                $signature_index.=join("\t",dec($callee),join(',',sort keys %{$why{$callee}}),dec(length($text)),sha256_hex($text),qjson($sig->{name}))."\n";
            }
            bytes_write("$folder/signatures.tsv",$signature_index); bytes_write("$folder/signature-parts.tsv",$parts_index);
            my $ins="instruction\tbyte_offset\tbytes\top\texplicit_source_file\tsource_line\tsource_column\tsha256\n";
            for my $span (@{$f->{instruction_spans}}) {
                my ($at,$start,$length,$op,$file,$line,$column)=@$span;
                $ins.=join("\t",dec($at),dec($start),dec($length),$op,$file,$line,$column,sha256_hex($range->($start,$length)))."\n";
            }
            bytes_write("$folder/instructions.tsv",$ins);
            $meta->{type_indices}=[map {dec($_)} @type_ids]; $meta->{signature_indices}=[map {dec($_)} @signature_ids];
        } else {
            $exit=2; $meta->{partial_body_exported}='no'; $meta->{partial_context_exported}='no';
        }
        bytes_write("$folder/extraction.json",json($meta));
        push @verdicts,$meta;
        $selection.=join("\t",dec($ordinal),dec($id),dec($pick->{first_row}),qjson($f->{name}),$status,
            dec($f->{length}),dec($f->{instructions}),dec(scalar @type_ids),dec(scalar @signature_ids),dec($context_bytes),qjson(\@problems))."\n";
    }
    close $ir or die "IR close\n";
    bytes_write('selection.tsv',$selection);
    $result->{selected_functions}=dec(scalar @selected); $result->{functions}=\@verdicts;
    $result->{companion_functions}=dec(scalar @functions); $result->{companion_types}=dec(scalar @types);
    $result->{source_records}=dec(scalar @sources); $result->{destructor_records}=dec(scalar @destructors);
    $result->{shared_context_bytes}=dec($shared_bytes);
    $result->{status}=$exit ? 'has-unassessed-functions' : 'complete';
    1;
};
if (!$ok) { $result->{status}='unassessed-extraction'; $result->{error}="$@"; $exit=1; }
$result->{exit_status}=dec($exit);
bytes_write('extraction.json',json($result));
my $manifest=join('',map {"$written{$_}  $_\n"} sort keys %written);
sysopen(my $mf,"$out/manifest.sha256",O_WRONLY|O_CREAT|O_EXCL,0600) or die "manifest claim\n";
print $mf $manifest or die "manifest write\n"; close $mf or die "manifest close\n";
print json({status=>$result->{status},exit_status=>dec($exit),output=>$out,
    manifest_sha256=>sha256_hex($manifest),selected_functions=>$result->{selected_functions}//'NA',
    eligibility=>'not-assessed'});
exit $exit;
