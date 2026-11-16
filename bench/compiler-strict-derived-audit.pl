#!/usr/bin/env perl
# Verify retained replay evidence; this program never invokes the compiler.
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
my $mode=shift @ARGV // '';
my $repository=shift @ARGV // '';
sub bytes {
    open my $file,'<:raw',$_[0] or die "$_[0]: $!\n";
    local $/; return <$file>//'';
}
sub git {
    open my $pipe,'-|','git','-C',$repository,@_ or die $!;
    binmode $pipe; local $/; my $value=<$pipe>//'';
    close $pipe or die "git failed: @_\n"; return $value;
}
sub line { my $value=shift; $value=~s/\n\z//; return $value; }
my $file_header="path\trequired\thistorical_before_sha256\tproof_before_sha256\tproof_after_sha256\tpublished_sha256\ttarget_sha256\texact_target_text\n";
sub verify_case {
    my ($stem,$before,$revision,$parent,$live)=@_;
    git('rev-parse',"$revision^") eq "$parent\n" or die "first parent differs\n";
    my $all=git('diff','--name-only','--no-renames','-z',$parent,$revision);
    bytes("$stem.all-changed-paths") eq $all or die "all changed paths differ\n";
    my @required=grep {m{^(src|library)/} || /\.(lkj|rs)$/} grep {length} split /\0/,$all;
    @required or die "empty required language scope\n";
    bytes("$stem.changed-paths") eq join('',map {"$_\0"} @required)
        or die "required changed paths differ\n";
    my %required=map {$_=>1} @required;
    my $out=bytes("$stem.out");
    my @replay=($out=~/^replay desired=([0-9a-f]{64}) candidate=([0-9a-f]{64}) verified=true operations=([1-9][0-9]*)$/mg);
    @replay==3 && $replay[0] eq $replay[1] or die "missing or ambiguous canonical equality proof\n";
    my @transaction=($out=~/^transaction source-before=([0-9a-f]{64}) source-after=([0-9a-f]{64}) mode=apply$/mg);
    @transaction==2 or die "missing or ambiguous applied transaction\n";
    my %applied;
    for my $fact (split /\n/,$out) {
        next unless $fact=~/^("(?:\\.|[^"])*") level=source-claim file before=([0-9a-f]{64}) after=([0-9a-f]{64})$/;
        my ($quoted,$old,$new)=($1,$2,$3);
        my $absolute=decode_json($quoted);
        index($absolute,"$before/")==0 or die "file fact outside historical source tree\n";
        my $path=substr($absolute,length($before)+1);
        $path!~m{(?:^|/)\.\.?(?:/|$)} && $path!~/[\t\r\n\0]/ or die "unsafe proof path\n";
        !$applied{$path} or die "duplicate file proof\n";
        $old eq sha256_hex(git('show',"$parent:$path")) or die "before fact differs from historical bytes: $path\n";
        $applied{$path}=[$old,$new];
    }
    for my $path (@required) { exists $applied{$path} or die "required changed language file is uncovered: $path\n"; }
    my %recorded;
    if (!$live) {
        my @rows=split /\n/,bytes("$stem.files.tsv");
        shift(@rows)."\n" eq $file_header or die "wrong published-file schema\n";
        for (@rows) {
            my @row=split /\t/,$_,-1;
            @row==8 && !$recorded{$row[0]} or die "bad/duplicate published-file witness\n";
            $recorded{$row[0]}=\@row;
        }
    }
    my $files=$file_header;
    my $text_equal=0;
    for my $path (sort keys %applied) {
        my ($old,$new)=@{$applied{$path}};
        my $published;
        if ($live || -d $before) {
            -f "$before/$path" && !-l "$before/$path" or die "published source is not a regular file: $path\n";
            $published=sha256_hex(bytes("$before/$path"));
        } else {
            my $row=$recorded{$path}//die "missing retained publication digest: $path\n";
            $published=$row->[5];
        }
        $published eq $new or die "published bytes differ from after fact: $path\n";
        my $target=sha256_hex(git('show',"$revision:$path"));
        my $equal=$new eq $target;
        $text_equal++ if $required{$path} && $equal;
        $files.=join("\t",$path,$required{$path}?'yes':'no',$old,$old,$new,$published,$target,$equal?'yes':'no')."\n";
    }
    if (!$live) { bytes("$stem.files.tsv") eq $files or die "retained publication digest rows differ\n"; }
    !-e "$before/.lkjscript-edit.pending" or die "successful transaction has a pending source intent\n";
    return {
        files=>$files, required=>scalar(@required), operations=>$replay[2],
        desired=>$replay[0], candidate=>$replay[1], source_before=>$transaction[0],
        source_after=>$transaction[1], text_equal=>$text_equal,
        stdout_sha=>sha256_hex($out), stderr_sha=>sha256_hex(bytes("$stem.err"))
    };
}
if ($mode eq '--case') {
    @ARGV==4 or die "usage: $0 --case REPOSITORY STEM BEFORE REVISION PARENT\n";
    my $result=verify_case(@ARGV,1);
    print $result->{files};
    exit;
}
$mode eq '--report' && @ARGV==1
    or die "usage: $0 --report REPOSITORY OUTPUT_TSV\n";
my $output=$ARGV[0];
my $pin_bytes=bytes('bench/compiler-strict-derived.producer');
my $launch_manifest=join('',map {sha256_hex(bytes($_))."  $_\n"}
    qw(bench/compiler-strict-derived.prepared.sha256
       bench/compiler-strict-derived.producer bench/compiler-strict-derived.producer.md));
bytes('bench/compiler-strict-derived.launch-ready.sha256') eq $launch_manifest
    or die "launch-ready manifest omits or changes a required pin record\n";
$pin_bytes=~m{\Aversion=1\ncampaign=one-strict-case-validated-derived-origin\nproducer_sha256=([0-9a-f]{64})\nproducer_bytes=([1-9][0-9]*)\nproducer_source_commit=([0-9a-f]{40})\nproducer_src_tree=([0-9a-f]{40})\nproducer_library_tree=([0-9a-f]{40})\ngated_commit=([0-9a-f]{40})\n\z}
    or die "missing or malformed frozen producer pin\n";
my ($producer_sha,$producer_size,$producer_source,$producer_src,$producer_library,$gate)=($1,$2,$3,$4,$5,$6);
git('rev-parse',"$gate^{commit}") eq "$gate\n" or die "gated commit differs\n";
for my $pair ([src=>$producer_src],[library=>$producer_library]) {
    git('rev-parse',"$producer_source:$pair->[0]") eq "$pair->[1]\n"
        && git('rev-parse',"$gate:$pair->[0]") eq "$pair->[1]\n"
        or die "producer/gate source trees differ\n";
}
sha256_hex(bytes('bench/compiler-strict-recent.tsv')) eq 'e99bc48ad638bbe75ec014f9c2a7686f36bc494c832f34b506177baf0f631486'
    or die "the prior six-refusal cohort changed\n";
my $order=bytes('bench/compiler-strict-recent.order');
sha256_hex($order) eq 'dbefff80f9fd1164e81bf429a73a5978ef783868312309578e06cdf5ea1e4d21'
    or die "predeclared order differs\n";
my @original_order=split /\n/,$order;
@original_order==6 && $original_order[2] eq '0a410badb7e96ed9f6cf185df7d1f781d71c5fdb'
    or die "original third candidate differs\n";
my @candidate_rows=split /\n/,bytes('bench/compiler-strict-recent.candidates.tsv');
my @selected=grep {/^3\t/} @candidate_rows;
@selected==1 && bytes('bench/compiler-strict-derived.candidates.tsv')
    eq "$candidate_rows[0]\n$selected[0]\n" or die "selected complete candidate record differs\n";
my @order=($original_order[2]);
sha256_hex(bytes('bench/compiler-strict-scoped.tsv'))
    eq 'edfcb2f924fb1934d9b6e80ad7bda71f755e235fc034980595a6da184bc8dd83'
    or die "prior five-pass one-refusal cohort changed\n";
my $window=git('rev-list','--max-count=300','6a187b94fb763f7b0565436a98ab5feda3cd642f');
sha256_hex($window) eq 'bee88306e1c12adfb3e0f8dfc6a96bf71c6b8ad344d4d08c2cdb0b808293cd20'
    or die "named-horizon window differs\n";
my %member=map {$_=>1} split /\n/,$window;
my %prior;
for my $control (
    ['bench/compiler-strict-history-imported.tsv','0aafd3a891bf224bd3113ab9de58219fecba44b84e0c5605a28f8ced0f25acb1',23],
    ['bench/compiler-strict-history-four.proofs.tsv','02bd19ad873b7b20ae0a947513c3603443b97633aeefa2555f751d058c696daa',1],
    ['bench/compiler-strict-scoped.proofs.tsv','dbff3d431b1a33375116eabc90091ba3c68feecff53bb7f8c5961a20c4934667',5]
) {
    my $value=bytes($control->[0]); sha256_hex($value) eq $control->[1] or die "prior proof table differs\n";
    my @rows=split /\n/,$value; shift @rows; @rows==$control->[2] or die "prior count differs\n";
    for (@rows) { my @row=split /\t/; !$prior{$row[1]}++ && $member{$row[1]} or die "prior duplicate/outside frozen window\n"; }
}
my %meta;
for (split /\n/,bytes("$output.meta")) {
    /^([^=]+)=(.*)$/ or die "bad metadata row\n";
    !exists $meta{$1} or die "duplicate metadata key $1\n";
    $meta{$1}=$2;
}
for my $expected (
    [segment=>'separate-derived-origin-strict-case'],
    [history_head=>'6a187b94fb763f7b0565436a98ab5feda3cd642f'],
    [compiler_sha256=>$producer_sha],[compiler_bytes=>$producer_size],
    [producer_source_commit=>$producer_source],[gated_commit=>$gate],
    [producer_pin_sha256=>sha256_hex($pin_bytes)],
    [prepared_manifest_sha256=>sha256_hex(bytes('bench/compiler-strict-derived.prepared.sha256'))],
    [launch_ready_manifest_sha256=>sha256_hex(bytes('bench/compiler-strict-derived.launch-ready.sha256'))],
    [mode=>'replay'],[root_policy=>'explicit-src-main'],[inline_limit=>'0'],
    [harness_sha256=>sha256_hex(bytes('bench/compiler-strict-derived.sh'))],
    [acceptance=>'canonical-checked-meaning-and-every-changed-language-file'],
    [store_state=>'cold-single-case'],[file_cache_reset=>'false'],
    [source_binding=>'explicit-owner-provided-gated-build-provenance'],
    [library_policy=>'physical-exact-historical-library-on-both-sides'],
    [runtime_policy=>'no-runtime-selector-no-native-historical-image-execution'],
    [prior_proofs=>'29'],[original_candidate_ordinal=>'3'],
    [prior_producer_sha256=>'4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09'],
    [preparation_version=>'1'],[working_directory=>'case-after'],
    [against_policy=>'relative-src-main'],[against_source_policy=>'explicit-case-after'],
    [key_grant_fence=>'canonical-before-after-and-both-libraries'],
    [prior_failed_cohort=>'compiler-strict-recent'],[prior_failed_passed=>'0'],
    [prior_failed_producer_sha256=>'adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7'],
    [prior_scoped_cohort=>'compiler-strict-scoped'],[prior_scoped_passed=>'5'],
    [prior_scoped_producer_sha256=>'ab2fa80787261e2fec6d872d989099d3b1dd0984c26ac59bf89c7237d9e97504'],
    [memory_bytes=>'4294967296'],[timeout_seconds=>'900'],[store_bytes=>'2147483648'],
    [workers=>'1'],[resume=>'false'],[retries=>'false'],
    [identity_domain=>'lkjscript-checked-meaning-1']
) { ($meta{$expected->[0]}//'') eq $expected->[1] or die "metadata differs: $expected->[0]\n"; }
for my $part (qw(src library)) {
    ($meta{"producer_${part}_tree"}//'') eq line(git('rev-parse',"$meta{producer_source_commit}:$part"))
        or die "producer source tree differs\n";
}
my $fixture=$meta{fixture}//'';
$fixture ne '' && ($meta{retained_fixture}//'') eq $fixture or die "fixture provenance missing\n";
if (-f "$fixture/compiler") {
    (-s "$fixture/compiler")==$producer_size
        && sha256_hex(bytes("$fixture/compiler")) eq $meta{compiler_sha256}
        or die "retained executing producer differs\n";
}
my @records=split /\n/,bytes($output);
my $expected_header="ordinal\trevision\tparent\tstatus\texit_code\toperations\tstdout_lines\twall_s\tuser_s\tsys_s\tpeak_kib\tload_before\tload_after\treason\tlibrary_environment\tsource_root";
shift(@records) eq $expected_header or die "wrong result schema\n";
@records<=1 && @records==($meta{completed_rows}//-1) or die "result count differs\n";
print "ordinal\trevision\tparent\toperations\tchanged_source_files\tdesired_meaning\tcandidate_meaning\tsource_before\tsource_after\tsource_root\tlibrary_environment\tstdout_sha256\tstderr_sha256\n";
my ($passed,$launched,$operations,$facts,$text_equal)=(0,0,0,0,0);
my %reasons;
for my $index (0..$#records) {
    my @row=split /\t/,$records[$index],-1;
    @row==16 or die "bad result columns\n";
    my ($ordinal,$revision,$parent,$status,$exit,$count)=@row;
    $index==0 && $ordinal==3 && $revision eq $order[$index] && !$prior{$revision} && $member{$revision}
        or die "candidate order/overlap differs\n";
    git('rev-parse',"$revision^") eq "$parent\n" or die "recorded parent differs\n";
    my $stem=sprintf "%s.logs/%03d-%s",$output,$ordinal,$revision;
    bytes("$stem.ordinal") eq "$ordinal\n" or die "ordinal evidence differs\n";
    my $all=git('diff','--name-only','--no-renames','-z',$parent,$revision);
    bytes("$stem.all-changed-paths") eq $all or die "full changed set differs\n";
    my @required=grep {m{^(src|library)/} || /\.(lkj|rs)$/} grep {length} split /\0/,$all;
    bytes("$stem.changed-paths") eq join('',map {"$_\0"} @required) or die "required scope differs\n";
    $reasons{$row[13]}++;
    if ($exit eq '-') {
        $status eq 'not-run' || $status eq 'refused' or die "unexecuted row mislabelled\n";
        $count==0 or die "unexecuted row has operations\n";
        next;
    }
    $launched++;
    bytes("$stem.exit") eq "$exit\n" or die "exit witness differs\n";
    $row[14] eq 'historical-roots' && $row[15] eq 'src/main.lkj' or die "root/library policy differs\n";
    my $before="$fixture/case-".sprintf('%03d',$ordinal)."/before";
    bytes("$stem.before") eq "$before\n" or die "historical source path differs\n";
    (my $after=$before)=~s{/before$}{/after};
    my @args=("$fixture/compiler",'edit','src/main.lkj','--source-dir',$before,
        '--against','src/main.lkj','--against-source-dir',$after,'--library-dir',"$before/library",
        '--against-library-dir',"$after/library",'--inline-limit','0',
        '--store',"$fixture/store",'--store-bytes','2147483648','--store-key',"$fixture/key",'--stats');
    bytes("$stem.argv0") eq join('',map {"$_\0"} @args) or die "actual replay arguments differ\n";
    bytes("$stem.cwd-requested") eq "$after\n"
        or die "requested desired-source working directory differs\n";
    if (-f "$stem.cwd") {
        bytes("$stem.cwd") eq "$after\n" or die "actual desired-source working directory differs\n";
    } else {
        $status eq 'stopped-failure' && $exit eq '126'
            or die "missing actual cwd without a recorded launch failure\n";
    }
    my @grants=(['before-source',$before],['desired-source',$after],
        ['before-library',"$before/library"],['desired-library',"$after/library"]);
    my $grant_text="role\tcanonical_path\n";
    my $key="$fixture/key";
    for my $grant (@grants) {
        $key ne $grant->[1] && index($key,"$grant->[1]/")!=0
            or die "private key is inside a granted source/library root\n";
        $grant_text.=join("\t",@$grant)."\n";
    }
    $grant_text.="private-key\t$key\n";
    bytes("$stem.grants.tsv") eq $grant_text or die "source/library/key grant evidence differs\n";

    my @capture=split /\t/,line(bytes("$stem.capture"));
    @capture==4 && $capture[0]=~/^\d+$/ && $capture[1]=~/^\d+$/ && $capture[1]>=$capture[0]
        && $capture[2] eq $row[11] && $capture[3] eq $row[12] or die "capture/load witness differs\n";
    for my $load (@capture[2,3]) { $load=~/^\d+\.\d+,\d+\.\d+,\d+\.\d+$/ or die "bad three-window load\n"; }
    if (-f "$stem.time") {
        my @times=split /\n/,bytes("$stem.time");
        join("\t",@row[7..10]) eq $times[-1] or die "GNU time metrics differ\n";
    } else {
        $status eq 'stopped-failure' && join("\t",@row[7..10]) eq "-\t-\t-\t-"
            or die "missing GNU time data without a recorded launch failure\n";
    }
    my $out=bytes("$stem.out"); my $lines=($out=~tr/\n//);
    $lines==$row[6] or die "stdout line count differs\n";
    next unless $status eq 'passed';
    $exit eq '0' or die "nonzero command accepted\n";
    bytes("$stem.group-cleanup") eq "0\n" or die "accepted command left an undrained private group\n";
    my $verified=verify_case($stem,$before,$revision,$parent,0);
    $verified->{operations}==$count && bytes("$stem.scope") eq '' or die "accepted operation/scope record differs\n";
    print join("\t",$ordinal,$revision,$parent,$count,$verified->{required},
        $verified->{desired},$verified->{candidate},$verified->{source_before},$verified->{source_after},
        $row[15],$row[14],$verified->{stdout_sha},$verified->{stderr_sha}),"\n";
    $passed++; $operations+=$count; $facts+=$verified->{required}; $text_equal+=$verified->{text_equal};
}
($meta{passed}//-1)==$passed && ($meta{launched}//-1)==$launched or die "completion counts differ\n";
printf STDERR "verified new_planned=1 new_launched=%d new_passed=%d new_operations=%d prior_strict_inside_named_window=29 conditional_union=%d\n",
    $launched,$passed,$operations,29+$passed;
printf STDERR "new_required_file_facts=%d exact_historical_before=%d exact_target_text=%d target_text_differs=%d missing_result_rows=%d\n",
    $facts,$facts,$text_equal,$facts-$text_equal,1-scalar(@records);
print STDERR "claim=complete-changed-language-coverage-plus-selected-root-canonical-checked-meaning\n";
print STDERR "not_claimed=serialized-IR-byte-identity,non-language-Git-effects,unmeasured-cases\n";
for my $reason (sort keys %reasons) { print STDERR "reason $reason $reasons{$reason}\n"; }
