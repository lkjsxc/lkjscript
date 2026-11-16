#!/usr/bin/env perl
# Post-campaign read-only verification of retained archives; never runs lkjscript.
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use IPC::Open2;
use File::Find;
my ($repository,$output)=@ARGV;
@ARGV==2 or die "usage: $0 REPOSITORY OUTPUT_TSV\n";
sub bytes {
    open my $f,'<:raw',$_[0] or die "$_[0]: $!\n"; local $/; return <$f>//'';
}
sub digest {
    -f $_[0] && !-l $_[0] or die "not a regular file: $_[0]\n";
    open my $f,'<:raw',$_[0] or die "$_[0]: $!\n";
    return Digest::SHA->new(256)->addfile($f)->hexdigest;
}
sub git {
    open my $p,'-|','git','-C',$repository,@_ or die $!; binmode $p; local $/;
    my $value=<$p>//''; close $p or die "git failed\n"; return $value;
}
my $meta=bytes("$output.meta");
$meta=~/^retained_fixture=(.+)$/m or die "missing retained fixture\n";
my $fixture=$1;
-d $fixture or die "retained fixture is gone\n";
my @rows=split /\n/,bytes($output); shift @rows;
@rows==1 or die "not the complete fixed single-case campaign\n";
my ($reader,$writer);
my $pid=open2($reader,$writer,'git','-C',$repository,'cat-file','--batch');
binmode $reader; binmode $writer;
my $selected=select($writer); $|=1; select($selected);
sub object {
    my ($oid)=@_;
    print {$writer} "$oid\n" or die "git request: $!\n";
    my $header=<$reader>//die "missing object header\n";
    $header=~/^\Q$oid\E blob ([0-9]+)\n$/ or die "unexpected object header\n";
    my $remaining=$1; my $body='';
    while ($remaining) {
        my $read=read($reader,my $part,$remaining);
        defined $read && $read>0 or die "short Git object\n";
        $body.=$part; $remaining-=$read;
    }
    my $newline;
    read($reader,$newline,1)==1 && $newline eq "\n" or die "bad object delimiter\n";
    return $body;
}
my $archives="ordinal\trevision\tside\tgit_revision\tfiles\tapplied_file_facts\texpected_manifest_sha256\tactual_manifest_sha256\tcontent_equal\texact_path_set\tpending_intent\n";
my $runtime="ordinal\trevision\tbefore_runtime_sha256\tafter_runtime_sha256\tequal\tbefore_include_expression\tafter_include_expression\n";
my $refusals="ordinal\trevision\tstatus\treason\tstderr_sha256\n";
my ($archive_count,$runtime_count,$applied_count)=(0,0,0);
for my $record (@rows) {
    my @row=split /\t/,$record,-1;
    my ($ordinal,$revision,$parent,$status)=@row;
    @row==16 && $ordinal==3 && $revision eq '0a410badb7e96ed9f6cf185df7d1f781d71c5fdb'
        or die "unexpected selected case/schema\n";
    my $stem=sprintf '%s.logs/%03d-%s',$output,$ordinal,$revision;
    my %applied;
    if ($status eq 'passed') {
        my @facts=split /\n/,bytes("$stem.files.tsv");
        shift(@facts) eq "path\trequired\thistorical_before_sha256\tproof_before_sha256\tproof_after_sha256\tpublished_sha256\ttarget_sha256\texact_target_text"
            or die "unexpected publication digest schema\n";
        for my $fact (@facts) {
            my @f=split /\t/,$fact,-1;
            @f==8 && $f[2] eq $f[3] && $f[4] eq $f[5]
                && $f[2]=~/^[0-9a-f]{64}$/ && $f[4]=~/^[0-9a-f]{64}$/
                && !exists $applied{$f[0]} or die "invalid/duplicate publication digest row\n";
            $applied{$f[0]}=[$f[2],$f[4]];
        }
        scalar(keys %applied)>0 or die "successful replay has no published files\n";
    } elsif ($row[4] ne '-') {
        $refusals.=join("\t",$ordinal,$revision,$status,$row[13],sha256_hex(bytes("$stem.err")))."\n";
    }
    my $case=sprintf '%s/case-%03d',$fixture,$ordinal;
    my @rt; my @expression;
    for my $pair (['before',$parent],['after',$revision]) {
        my ($side,$git_revision)=@$pair;
        my $root="$case/$side";
        next unless -d $root;
        my ($expected,$actual,$files,$used_facts)=('','',0,0);
        my @expected_paths;
        for my $entry (grep {length} split /\0/,git('ls-tree','-rz',$git_revision,'--','src','library','tools','tests')) {
            $entry=~/^(100644|100755) blob ([0-9a-f]+)\t(.+)$/s or die "unexpected archived entry\n";
            my ($oid,$path)=($2,$3);
            push @expected_paths,$path;
            my $wanted=sha256_hex(object($oid));
            if ($side eq 'before' && exists $applied{$path}) {
                $wanted eq $applied{$path}[0] or die "applied before digest differs from Git\n";
                $wanted=$applied{$path}[1];
                $used_facts++;
            }
            my $found=digest("$root/$path");
            $expected.="$path\0$wanted\n";
            $actual.="$path\0$found\n";
            $wanted eq $found or die "archive changed: case $ordinal $side $path\n";
            $files++;
        }
        if ($side eq 'before') {
            $used_facts==scalar(keys %applied) or die "published file is outside the archived historical tree\n";
            $applied_count+=$used_facts;
        }
        my @actual_paths;
        my @areas=grep {-e $_} map {"$root/$_"} qw(src library tools tests);
        find({no_chdir=>1,wanted=>sub {
            my $path=$File::Find::name;
            !-l $path or die "unexpected source symlink: $path\n";
            return if -d $path;
            -f $path or die "unexpected special source entry: $path\n";
            push @actual_paths,substr($path,length($root)+1);
        }},@areas);
        join("\0",sort @expected_paths) eq join("\0",sort @actual_paths)
            or die "archive path set changed in case $ordinal $side\n";
        my $pending=(-e "$root/.lkjscript-edit.pending")?'yes':'no';
        $pending eq 'no' or die "pending intent remains: case $ordinal $side\n";
        $archives.=join("\t",$ordinal,$revision,$side,$git_revision,$files,$used_facts,
            sha256_hex($expected),sha256_hex($actual),'yes','yes',$pending)."\n";
        $archive_count++;
        push @rt,digest("$root/library/runtime.lkj");
        my @lines=grep {/include_str!\("\.\.\/library\/runtime\.lkj"\)/} split /\n/,bytes("$root/src/embedded.lkj");
        @lines==1 or die "runtime include expression differs\n";
        $lines[0]=~s/^\s+|\s+$//g;
        push @expression,$lines[0];
    }
    if (@rt==2) {
        $rt[0] eq $rt[1] or die "runtime bytes changed within historical pair\n";
        $runtime.=join("\t",$ordinal,$revision,@rt,'yes',@expression)."\n";
        $runtime_count++;
    }
}
close $writer or die "git request close: $!\n";
close $reader or die "git output close: $!\n";
waitpid($pid,0)==$pid && $?==0 or die "git object reader failed\n";
for my $pair (['archives.tsv',$archives],['runtime-inputs.tsv',$runtime],['refusals.tsv',$refusals]) {
    (my $prefix=$output)=~s/\.tsv$//;
    my $path="$prefix.$pair->[0]";
    open my $f,'>:raw',$path or die "$path: $!\n";
    print {$f} $pair->[1]; close $f or die $!;
}
print "verified archives=$archive_count runtime_pairs=$runtime_count applied_file_facts=$applied_count compiler_executions=0\n";
