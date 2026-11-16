#!/usr/bin/env perl
# Offline validation for the predeclared current-compiler query cohort.
use strict;
use warnings;
use JSON::PP;
use Digest::SHA qw(sha256_hex);
use Encode qw(decode FB_CROAK);
my $json=JSON::PP->new->canonical;
my $question_hash='417a4adfe357aab04915c5cdee3ebe2f691ee46ca2fb74663735e5d35a6e81e9';
sub raw {
    my($path,$limit)=@_;$limit//=16777216;
    die "missing regular file $path\n" unless -f $path && !-l $path;
    die "input bound $path\n" if -s $path>$limit;
    open my $f,'<:raw',$path or die "$path: $!";local $/;my $b=<$f>;close $f;return $b;
}
sub digest {
    my($path)=@_;die "missing regular digest input\n" unless -f $path && !-l $path;
    open my $f,'<:raw',$path or die $!;my $h=Digest::SHA->new(256);$h->addfile($f);close $f;return $h->hexdigest;
}
sub object { my $v=decode_json(raw($_[0]));die "expected object\n" unless ref($v) eq 'HASH';return $v; }
sub save { my($path,$value)=@_;open my $f,'>:raw',$path or die $!;print $f JSON::PP->new->canonical->pretty->encode($value);close $f; }
sub write_tsv {
    my($path,$head,$rows)=@_;open my $f,'>:raw',$path or die $!;print $f join("\t",@$head),"\n";
    for my $row(@$rows){my @v=map{defined($row->{$_})?$row->{$_}:'NA'}@$head;for(@v){$_=$json->encode($_) if ref $_;die "TSV framing\n" if /[\t\r\n]/;}print $f join("\t",@v),"\n";}close $f;
}
sub git {
    my($repo,@args)=@_;open my $p,'-|','git','-C',$repo,@args or die $!;local $/;my $out=<$p>;close $p or die "git control failed\n";$out=~s/\n\z//;return $out;
}
sub finite_number { defined($_[0]) && !ref($_[0]) && $_[0]=~/\A(?:[0-9]+(?:\.[0-9]+)?|[0-9]+(?:\.[0-9]+)?[eE][+-]?[0-9]+)\z/ }
sub require_same { my($left,$right,$why)=@_;die "$why\n" unless defined($left)&&defined($right)&&!ref($left)&&!ref($right)&&$left eq $right; }
sub identities {
    my($path,$repo,$compiler)=@_;my $id=object($path);
    require_same($id->{version},1,'identity version');
    for my $key(qw(revision src_tree library_tree runtime_blob gate_revision)){die "identity $key\n" unless ($id->{$key}//'')=~/\A[0-9a-f]{40}\z/;}
    for my $key(qw(producer_sha256 gate_producer_sha256 questions_sha256)){die "identity $key\n" unless ($id->{$key}//'')=~/\A[0-9a-f]{64}\z/;}
    require_same($id->{revision},$id->{gate_revision},'source is not gated revision');
    require_same($id->{producer_sha256},$id->{gate_producer_sha256},'producer is not gated executable');
    require_same($id->{questions_sha256},$question_hash,'question population differs');
    require_same($id->{source_entry},'src/main.lkj','entry differs');
    die "source-directory must be absent to preserve canonical diagnostic names\n" if !exists($id->{source_directory}) || defined($id->{source_directory});
    require_same($id->{library_dir},'library','library naming differs');
    require_same($id->{runtime_source},'library/runtime.lkj','runtime naming differs');
    require_same($id->{inline_limit},64,'output inline policy differs');
    require_same($id->{source_grant},'.','source grant differs');
    die "canonical factory argv missing\n" unless ref($id->{factory_argv}) eq 'ARRAY' && @{$id->{factory_argv}}>0
        && !grep { ref($_)||!defined($_)||/[\x00]/ } @{$id->{factory_argv}};
    die "canonical factory cwd missing\n" unless defined($id->{factory_cwd}) && !ref($id->{factory_cwd})
        && $id->{factory_cwd}=~m{\A/[^\x00]+\z} && $id->{factory_cwd} ne '/';
    my @argv=@{$id->{factory_argv}};
    die "canonical factory source-directory prefix differs\n" if grep { $_ eq '--source-dir' || /^--source-dir=/ } @argv;
    for my $pair(['--library-dir','library'],['--runtime-source','library/runtime.lkj'],['--inline-limit','64']){
        my $matches=0;for my $i(0..$#argv-1){$matches++ if $argv[$i] eq $pair->[0] && $argv[$i+1] eq $pair->[1];}
        die "canonical factory selector $pair->[0]\n" unless $matches==1;
    }
    my $entries=0;for my $i(0..$#argv-1){$entries++ if $argv[$i] eq 'build' && $argv[$i+1] eq 'src/main.lkj';}
    die "canonical factory entry differs\n" unless $entries==1;
    die "gate evidence missing\n" unless ref($id->{gate_evidence}) eq 'ARRAY' && @{$id->{gate_evidence}}>0
        && !grep { ref($_)||!defined($_)||!length($_) } @{$id->{gate_evidence}};
    require_same(git($repo,'rev-parse','--verify',"$id->{revision}^{commit}"),$id->{revision},'revision lookup differs');
    require_same(git($repo,'rev-parse',"$id->{revision}:src"),$id->{src_tree},'source tree differs');
    require_same(git($repo,'rev-parse',"$id->{revision}:library"),$id->{library_tree},'library tree differs');
    require_same(git($repo,'rev-parse',"$id->{revision}:library/runtime.lkj"),$id->{runtime_blob},'runtime blob differs');
    require_same(digest($compiler),$id->{producer_sha256},'compiler bytes differ');
    require_same(-s $compiler,$id->{producer_bytes},'compiler length differs');
    return $id;
}
sub capture {
    my($dir,$tag)=@_;my $v=object("$dir/raw/$tag.capture.json");
    for my $k(qw(status start_ns finish_ns wrapper_wall_ns)){die "capture $k\n" unless ($v->{$k}//'')=~/\A[0-9]+\z/;}
    die "capture clock\n" unless $v->{finish_ns}>=$v->{start_ns}
        && $v->{wrapper_wall_ns}==$v->{finish_ns}-$v->{start_ns};
    for my $k(qw(load_before load_after)){die "load capture\n" unless ($v->{$k}//'')=~/\A[0-9.]+,[0-9.]+,[0-9.]+\z/;}
    my $time=raw("$dir/raw/$tag.time");
    my($wall,$user,$system,$rss)=$time=~/([0-9.]+) ([0-9.]+) ([0-9.]+) ([0-9]+)\n?\z/;
    die "missing GNU time\n" unless defined $rss;
    @$v{qw(subprocess_wall_s user_s system_s rss_kib)}=($wall,$user,$system,$rss);return $v;
}
sub stats {
    my($path,$expected)=@_;my($stats,$summary,%categories);my @extra;
    for my $line(split /\n/,raw($path)) {
        if($line=~s/^LKJSCRIPT_STATS //){die "duplicate stats\n" if $stats;$stats=decode_json($line);}
        elsif($line=~s/^LKJSCRIPT_STORE_IO //){my $row=decode_json($line);my $cat=$row->{category}//die "I/O category missing\n";
            if($cat eq 'summary'){die "duplicate I/O summary\n" if $summary;$summary=$row;}
            else{die "duplicate I/O category\n" if exists $categories{$cat};$categories{$cat}=$row;}}
        elsif(length $line){push @extra,$line;}
    }
    die "missing stats\n" unless ref($stats) eq 'HASH';
    die "unexpected diagnostic stderr\n" if @extra;
    my @required=$expected eq 'query'
        ? qw(total_ms snapshot_auth_ms semantic_query_ms producer_bytes_hashed)
        : qw(total_ms producer_hash_ms store_io_ms build_reused store_writes);
    for my $k(@required){die "missing numeric stats $k\n" unless finite_number($stats->{$k});}
    die "invalid optional producer hash timing\n" if exists($stats->{producer_hash_ms}) && !finite_number($stats->{producer_hash_ms});
    if($expected eq 'build'){
        die "invalid image reuse counter\n" unless $stats->{build_reused}==0 || $stats->{build_reused}==1;
        if($stats->{build_reused}==0){
            for my $k(qw(semantic_catalog_ms semantic_index_ms ir_functions ir_instructions)){
                die "missing numeric cold-build stats $k\n" unless finite_number($stats->{$k});
            }
        }
    }
    die "missing indexed Store counters\n" if $expected eq 'build' && (!$summary || !keys %categories);
    my $sum=0;
    for my $row(values %categories){for my $key(qw(read_ns pack_decode_ns pack_retain_ns seal_ns pack_reseal_ns publish_ns prepare_ns metadata_ns stream_write_ns)){
        my $value=$row->{$key}//0;die "invalid I/O counter\n" unless finite_number($value);$sum+=$value;}}
    if($summary){die "Store counter state failure\n" if $summary->{global_timing_failures}||$summary->{global_counter_overflow}||$summary->{pending_checkpoint};
        die "exclusive counter sum differs\n" unless $sum==$summary->{accounted_ns};}
    die "pinned query hashes current producer\n" if $expected eq 'query' && $stats->{producer_bytes_hashed}!=0;
    return {stats=>$stats,store_io_summary=>$summary,store_io_categories=>\%categories,exclusive_store_counter_sum_ns=>$sum};
}
sub builds {
    my($dir,$fixture,$idpath)=@_;my $id=object($idpath);my @rows;my @problems;my @pins;
    for my $tag(qw(cold-index unchanged-index)){
        my %row=(phase=>$tag);my @errors;
        if(!-f "$dir/raw/$tag.capture.json"){$row{status}='not-run';push @errors,'dependent command not run';}
        else {
            eval{my $cap=capture($dir,$tag);%row=(%row,%$cap);push @errors,"exit=$cap->{status}" if $cap->{status};
                my $valid=stats("$dir/raw/$tag.stderr",'build');$row{stats}=$valid->{stats};$row{io}=$valid;
                my $out=raw("$dir/raw/$tag.stdout");my @found=($out=~/^LKJSCRIPT_SNAPSHOT ([0-9a-f]{64}) source=([0-9a-f]{64}) checked-ir=([0-9a-f]{64})\n/gm);
                die "snapshot record count\n" unless @found==3;
                @row{qw(snapshot source checked_ir)}=@found;push @pins,\%row;
                my $image="$fixture/output/$tag.image";$row{image_sha256}=digest($image);$row{image_bytes}=-s $image;
                die "indexed image differs from gated canonical compiler\n" unless $row{image_sha256} eq $id->{producer_sha256} && $row{image_bytes}==$id->{producer_bytes};
                if($tag eq 'unchanged-index'){die "unchanged image was not reused\n" unless $valid->{stats}{build_reused}==1;}
                1;
            } or push @errors,$@||'unknown build validation failure';
        }
        for(@errors){s/\s+\z//;}
        $row{errors}=\@errors;$row{protocol_valid}=@errors?JSON::PP::false:JSON::PP::true;
        push @problems,map{"$tag: $_"}@errors;push @rows,\%row;
    }
    if(@pins==2){for my $k(qw(snapshot source checked_ir image_sha256)){push @problems,"unchanged $k differs" if $pins[0]{$k} ne $pins[1]{$k};}}
    save("$dir/builds.json",{valid=>@problems?JSON::PP::false:JSON::PP::true,errors=>\@problems,rows=>\@rows});
    my @flat;
    for my $r(@rows){my %f=%$r;my $s=$r->{stats}//{};
        @f{qw(total_ms producer_hash_ms semantic_catalog_ms semantic_index_ms store_io_ms build_reused store_writes ir_functions ir_instructions)}
            =@$s{qw(total_ms producer_hash_ms semantic_catalog_ms semantic_index_ms store_io_ms build_reused store_writes ir_functions ir_instructions)};push @flat,\%f;}
    write_tsv("$dir/builds.tsv",[qw(phase status protocol_valid wrapper_wall_ns subprocess_wall_s user_s system_s rss_kib total_ms producer_hash_ms semantic_catalog_ms semantic_index_ms store_io_ms build_reused store_writes ir_functions ir_instructions image_bytes image_sha256 snapshot source checked_ir load_before load_after errors)],\@flat);
    if(!@problems){save("$dir/pins.json",{(map{$_=>$pins[0]{$_}}qw(snapshot source checked_ir image_sha256)),producer=>$id->{producer_sha256}});}
    else{print STDERR join("\n",@problems),"\n";}
    return !@problems;
}
sub requests {
    my($path)=@_;die "question list differs\n" unless digest($path) eq $question_hash;
    my @lines=split /\n/,raw($path);require_same(shift(@lines),"ordinal\tgroup\tkind\tidentity\tdepth\tlimit\tbytes",'question header');
    die "question count\n" unless @lines==50;my @rows;my $n=0;
    for(@lines){my @v=split /\t/,$_,-1;die "question shape\n" unless @v==7 && $v[0]==++$n;my %r;@r{qw(ordinal group kind identity depth limit bytes)}=@v;push @rows,\%r;}return \@rows;
}
sub query_row {
    my($dir,$request,$pin)=@_;my %r=%$request;my $tag=sprintf('query-%02d',$r{ordinal});my @errors;
    eval {
        my $cap=capture($dir,$tag);%r=(%r,%$cap);push @errors,"exit=$cap->{status}" if $cap->{status};
        my $valid=stats("$dir/raw/$tag.stderr",'query');$r{stats}=$valid->{stats};
        my $path="$dir/raw/$tag.stdout";$r{answer_bytes}=-s $path;$r{answer_sha256}=digest($path);
        die "reply byte bound\n" if $r{answer_bytes}>$r{bytes};
        my $answer=raw($path,32768);my $utf8=$answer;decode('UTF-8',$utf8,FB_CROAK);
        my @lines=split /\n/,$answer,-1;die "reply final newline\n" unless @lines && pop(@lines) eq '';
        my $head=shift @lines;my $tail=pop @lines;
        my $quoted=$json->allow_nonref->encode($r{identity});
        my $expected="$quoted\tlevel=protocol\tfact=query\tversion=3 producer=$pin->{producer} "
            ."validation=authenticated-producing-compiler current-check=not-performed snapshot=$pin->{snapshot} "
            ."source=$pin->{source} checked-ir=$pin->{checked_ir} kind=$r{kind} depth=$r{depth} freshness=not-asserted "
            ."source-reference-coverage=recorded-sites instance-coverage=demanded selection=explicit";
        require_same($head,$expected,'query header/provenance differs');
        die "continuation framing\n" unless defined($tail) && $tail=~/\A\Q$quoted\E\tlevel=protocol\tfact=continuation\tcursor=(none|(?:[0-9a-f]{2})+) facts=([0-9]+) pages-read=([0-9]+) traversal-steps=([0-9]+)\z/;
        my($cursor,$facts,$pages,$steps)=($1,$2,$3,$4);
        die "reply fact bound/count\n" unless $facts==@lines && $facts<=$r{limit};
        @r{qw(facts pages_read traversal_steps continuation)}=($facts,$pages,$steps,$cursor eq 'none'?'none':'available');
        my(%levels,%kinds);my($substantive,$partial,$unavailable)=(0,0,0);
        for my $line(@lines) {
            my @fields=split /\t/,$line,4;die "fact framing\n" unless @fields==4;
            my $identity=decode_json($fields[0]);die "fact identity is not text\n" if ref($identity)||!defined($identity);
            my($level)=$fields[1]=~/\Alevel=(source-claim|checked-ir|protocol)\z/;
            my($fact)=$fields[2]=~/\Afact=([a-z0-9-]+)\z/;die "fact label missing\n" unless defined($level)&&defined($fact);
            die "duplicate query protocol record\n" if $fact eq 'query'||$fact eq 'continuation';
            $levels{$level}=1;$kinds{$fact}=1;
            if($fact=~/\A(?:authority|layout|variant-layout|instance|ownership|loan)\z/){die "trusted fact mislabelled\n" unless $level eq 'checked-ir';}
            if(($r{kind} eq 'authority'||$r{kind} eq 'layout') && $level ne 'protocol'){die "trusted question mislabelled\n" unless $level eq 'checked-ir';}
            $substantive++ if $level ne 'protocol';
            $partial++ if $fact eq 'unresolved-reference'||$fields[3]=~/\bclaim=partial\b/;
            $unavailable++ if $fact=~/unavailable|missing|unknown/;
        }
        @r{qw(fact_levels fact_kinds substantive partial_rows unavailable_rows)}
            =(join(',',sort keys %levels),join(',',sort keys %kinds),$substantive,$partial,$unavailable);
        $r{coverage}=!$substantive?'empty':$unavailable?'unavailable':$partial?'partial-recorded':'nonempty-recorded';
        1;
    } or push @errors,$@||'unknown query validation failure';
    for(@errors){s/\s+\z//;}
    $r{errors}=\@errors;$r{protocol_valid}=@errors?JSON::PP::false:JSON::PP::true;
    $r{under_100ms}=defined($r{wrapper_wall_ns})&&$r{wrapper_wall_ns}<100000000?JSON::PP::true:JSON::PP::false;
    $r{coverage}//='refused-or-malformed';
    my $s=$r{stats}//{};@r{qw(total_ms snapshot_auth_ms semantic_query_ms producer_hash_ms producer_bytes_hashed)}
        =@$s{qw(total_ms snapshot_auth_ms semantic_query_ms producer_hash_ms producer_bytes_hashed)};
    return \%r;
}
sub distribution {
    my @values=sort{$a<=>$b}@_;return {n=>0} unless @values;my $n=@values;
    return {n=>$n,min=>$values[0],median=>$n%2?$values[int($n/2)]:($values[$n/2-1]+$values[$n/2])/2,max=>$values[-1]};
}
sub queries {
    my($dir,$question_file)=@_;my $pin=object("$dir/pins.json");my @rows;
    for my $request(@{requests($question_file)}){my $row=query_row($dir,$request,$pin);push @rows,$row;save("$dir/raw/".sprintf('query-%02d',$request->{ordinal}).".validation.json",$row);}
    write_tsv("$dir/queries.tsv",[qw(ordinal group kind identity depth limit bytes status protocol_valid coverage wrapper_wall_ns under_100ms subprocess_wall_s user_s system_s rss_kib total_ms snapshot_auth_ms semantic_query_ms producer_hash_ms producer_bytes_hashed answer_bytes facts continuation pages_read traversal_steps substantive partial_rows unavailable_rows fact_levels fact_kinds answer_sha256 load_before load_after errors)],\@rows);
    my %sets=(all=>\@rows,core=>[grep{$_->{ordinal}<=40}@rows],supplemental=>[grep{$_->{ordinal}>40}@rows]);
    my %summary;
    for my $name(sort keys %sets){my @selected=@{$sets{$name}};my @valid=grep{$_->{protocol_valid}}@selected;
        my %coverage;for(@selected){$coverage{$_->{coverage}}++;}
        $summary{$name}={commands=>scalar(@selected),protocol_valid=>scalar(@valid),
          valid_under_100ms=>scalar(grep{$_->{under_100ms}}@valid),
          valid_at_or_over_100ms=>scalar(grep{!$_->{under_100ms}}@valid),coverage=>\%coverage,
          wrapper_wall_ns=>distribution(map{$_->{wrapper_wall_ns}}grep{defined($_->{wrapper_wall_ns})}@selected),
          semantic_query_ms=>distribution(map{$_->{semantic_query_ms}}grep{defined($_->{semantic_query_ms})}@selected)};
    }
    my $failed=grep{!$_->{protocol_valid}}@rows;
    save("$dir/summary.json",{status=>$failed?'observed-validation-failures':'all-captured-protocols-valid',
        pins=>$pin,threshold_ns=>100000000,sets=>\%summary,
        interpretation=>'One fixed cohort. Source/producer and indexed-output policy differ from historical v2; no causal speed comparison. Recorded source-reference coverage is not an exhaustive semantic proof.'});
    return !$failed;
}
my $mode=shift(@ARGV)//'';
if($mode eq 'identity'){my $id=identities(@ARGV);print $json->pretty->encode($id);}
elsif($mode eq 'builds'){exit(builds(@ARGV)?0:1);}
elsif($mode eq 'queries'){exit(queries(@ARGV)?0:1);}
elsif($mode eq 'questions'){my $q=requests(@ARGV);print scalar(@$q)," fixed requests\n";}
else{die "usage: validate identity JSON REPO COMPILER | builds EVIDENCE FIXTURE IDENTITIES | queries EVIDENCE QUESTIONS | questions QUESTIONS\n";}
