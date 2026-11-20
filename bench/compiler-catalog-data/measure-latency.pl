#!/usr/bin/env perl
# Paired, sequential ask/change timings. Index construction is a separate step.
use strict;
use warnings;
use Getopt::Long qw(GetOptions);
use Time::HiRes qw(clock_gettime CLOCK_MONOTONIC);
use Digest::SHA qw(sha256_hex);
use File::Path qw(make_path);
use File::Spec;
use JSON::PP;
use POSIX qw(ceil);

my %opt = (rounds => 3);
GetOptions(\%opt, qw(baseline=s candidate=s source=s baseline-index=s candidate-index=s key=s questions=s change-source=s change-request=s out=s rounds=i))
    or die "invalid arguments\n";
for my $name (qw(baseline candidate source baseline-index candidate-index key questions change-source change-request out)) {
    die "missing --$name\n" unless defined $opt{$name};
    $opt{$name} = File::Spec->rel2abs($opt{$name});
}
die "need at least three rounds\n" if $opt{rounds} < 3;
die "output already exists\n" if -e $opt{out};
make_path($opt{out});
sub read_file {
    my ($path)=@_;
    open my $f,'<:raw',$path or die "$path: $!";
    local $/; my $text=<$f>; close $f; return $text;
}
sub write_file {
    my ($path,$text)=@_;
    open my $f,'>:raw',$path or die "$path: $!"; print $f $text; close $f or die $!;
}
sub invoke {
    my ($label,@cmd)=@_;
    my $start=clock_gettime(CLOCK_MONOTONIC);
    my $pid=fork(); die $! unless defined $pid;
    if (!$pid) {
        open STDOUT,'>:raw',"$opt{out}/$label.out" or die $!;
        open STDERR,'>:raw',"$opt{out}/$label.err" or die $!;
        exec @cmd; die "$cmd[0]: $!";
    }
    waitpid($pid,0); my $status=$?; my $elapsed=(clock_gettime(CLOCK_MONOTONIC)-$start)*1000;
    die "$label failed: status=$status (see retained .err)\n" if $status;
    return ($elapsed, read_file("$opt{out}/$label.out"));
}
my (%pin,%indexed_source,%indexed_ir);
for my $producer (qw(baseline candidate)) {
    my $index = read_file("$opt{\"$producer-index\"}/index.out");
    $index =~ /^LKJSCRIPT_SNAPSHOT ([0-9a-f]{64}) source=([0-9a-f]{64}) checked-ir=([0-9a-f]{64})/m or die "missing $producer snapshot";
    ($pin{$producer},$indexed_source{$producer},$indexed_ir{$producer})=($1,$2,$3);
}
die "producer input source differs\n" unless $indexed_source{baseline} eq $indexed_source{candidate};
die "producer checked IR differs\n" unless $indexed_ir{baseline} eq $indexed_ir{candidate};
my $question_text=read_file($opt{questions});
my @lines=split /\n/,$question_text;
shift @lines;
my @questions=map {
    my @q=split /\t/;
    die "malformed retained question\n" unless @q==7;
    \@q
} @lines;
die "expected retained 50-question cohort\n" unless @questions==50;
my $original=read_file($opt{'change-source'});
my $desired=$original;
$desired =~ s/add\(self\.get\(\), self\.get\(\)\)/add(self.get(), (\nadd(self.get(), 1i64)\n))/ or die "missing retained call expression\n";
my $request=read_file($opt{'change-request'});
my $json=JSON::PP->new->decode($request);
die "retained request has wrong body owner\n" unless $json->{operations}[0]{id} =~ /^Number::doubled::body:/;
write_file("$opt{out}/change-before.lkj",$original);
write_file("$opt{out}/change-desired.lkj",$desired);

my %meta = (
    schema => 1, rounds => $opt{rounds}, timing => 'CLOCK_MONOTONIC fork/exec wall milliseconds',
    order => 'one worker; alternate baseline/candidate order by round and question; one full untimed warmup',
    quantiles => 'nearest-rank p95; ordinary midpoint median',
    source_revision => '62fa76f4f863a169b906094f0dd9998c8421e77a',
    snapshots => \%pin,
    indexed_source => \%indexed_source, indexed_ir => \%indexed_ir,
    questions_sha256 => sha256_hex($question_text),
    change_source_sha256 => sha256_hex($original),
    change_request_sha256 => sha256_hex($request),
    compiler_sha256 => {map {$_=>sha256_hex(read_file($opt{$_}))} qw(baseline candidate)},
    load_before => scalar(read_file('/proc/loadavg')),
    limitations => 'Single host run; no statistical confidence or general performance claim. Index setup is measured separately. Query outputs are bounded and may contain more explicit namespace metadata.',
);
open my $query_tsv,'>',"$opt{out}/queries.tsv" or die $!;
print $query_tsv "round\torder\tproducer\tordinal\tkind\tidentity\tstatus\twall_ms\tfacts\tbytes\tpages_read\n";
my @query_rows;
my %reply_hash;
for my $round (0..$opt{rounds}) {
    for my $q (@questions) {
        my ($n,$group,$kind,$id,$depth,$limit,$bytes)=@$q;
        my @producers = ($round+$n)%2 ? qw(baseline candidate) : qw(candidate baseline);
        for my $order (0..1) {
            my $producer=$producers[$order];
            my $label=($round ? "query-r$round" : 'query-warm')."-$n-$producer";
            my ($ms,$reply)=invoke($label,$opt{$producer},'query','src/main.lkj',
                '--source-dir',$opt{source},'--library-dir',"$opt{source}/library",
                '--store',"$opt{\"$producer-index\"}/store",'--store-key',$opt{key},
                '--snapshot',$pin{$producer},'--kind',$kind,'--id',$id,
                '--depth',$depth,'--limit',$limit,'--bytes',$bytes);
            my ($facts)=$reply=~/ facts=([0-9]+)/;
            my ($pages)=$reply=~/ pages-read=([0-9]+)/;
            die "$label missing/invalid bounds\n" unless $facts && defined($pages) && $facts<=$limit && length($reply)<=$bytes;
            die "$label queried current source\n" unless $reply =~ /current-check=not-performed/ && $reply =~ /freshness=not-asserted/;
            my $digest=sha256_hex($reply);
            my $key="$producer/$n";
            die "$label changed its pinned answer\n" if exists($reply_hash{$key}) && $reply_hash{$key} ne $digest;
            $reply_hash{$key}=$digest;
            next unless $round;
            my @row=($round,$order+1,$producer,$n,$kind,$id,0,sprintf('%.6f',$ms),$facts,length($reply),$pages);
            print $query_tsv join("\t",@row),"\n"; push @query_rows,\@row;
        }
    }
}
close $query_tsv;
open my $change_tsv,'>',"$opt{out}/changes.tsv" or die $!;
print $change_tsv "round\torder\tproducer\tmode\tstatus\twall_ms\tbytes\tresult_native\tresult_interpreter\n";
my @change_rows;
for my $round (0..$opt{rounds}) {
    my @producers = $round%2 ? qw(baseline candidate) : qw(candidate baseline);
    for my $order (0..1) {
        my $producer=$producers[$order];
        my $project="$opt{out}/change-r$round-$producer";
        make_path($project);write_file("$project/main.lkj",$original);
        my @cmd=($opt{$producer},'edit','main.lkj','--source-dir',$project,'--no-library','--no-store','--store-key',$opt{key},'--request',$opt{'change-request'});
        my ($dry_ms,$dry)=invoke("change-r$round-$producer-dry",@cmd,'--dry-run');
        die 'dry-run mutated source' unless read_file("$project/main.lkj") eq $original;
        die 'dry-run published generation' if -e "$project/.lkjscript-edit.generation" || -e "$project/.lkjscript-edit.pending";
        my ($apply_ms,$apply)=invoke("change-r$round-$producer-apply",@cmd);
        die 'unexpected checked source projection' unless read_file("$project/main.lkj") eq $desired;
        die 'apply left pending intent' if -e "$project/.lkjscript-edit.pending";
        die 'wrong transaction modes' unless $dry =~ /mode=dry-run/ && $apply =~ /mode=apply/;
        my %result;
        for my $engine (qw(native interpreter)) {
            my (undef,$reply)=invoke("change-r$round-$producer-$engine",$opt{$producer},'run','main.lkj','--source-dir',$project,'--no-library','--no-store','--engine',$engine);
            $reply =~ s/\s+\z//; die "$producer $engine did not produce 43" unless $reply eq '43';
            $result{$engine}=$reply;
        }
        next unless $round;
        for my $mode (qw(dry-run apply)) {
            my ($ms,$reply)= $mode eq 'apply' ? ($apply_ms,$apply) : ($dry_ms,$dry);
            my @row=($round,$order+1,$producer,$mode,0,sprintf('%.6f',$ms),length($reply),$result{native},$result{interpreter});
            print $change_tsv join("\t",@row),"\n";push @change_rows,\@row;
        }
    }
}
close $change_tsv;
sub summary {
    my @v=sort {$a<=>$b} @_;
    my $n=@v; my $sum=0; $sum+=$_ for @v;
    return {n=>$n,total_ms=>$sum,mean_ms=>$sum/$n,median_ms=>$n%2 ? $v[int($n/2)] : ($v[$n/2-1]+$v[$n/2])/2,
        p95_ms=>$v[ceil(.95*$n)-1],min_ms=>$v[0],max_ms=>$v[-1]};
}
my %summary;
for my $producer (qw(baseline candidate)) {
    my @rows=grep {$_->[2] eq $producer} @query_rows;
    my ($facts,$bytes,$pages)=(0,0,0);for (@rows) {$facts+=$_->[8];$bytes+=$_->[9];$pages+=$_->[10];}
    $summary{query}{$producer}=summary(map {$_->[7]} @rows);
    $summary{query}{$producer}{facts}=$facts;
    $summary{query}{$producer}{bytes}=$bytes;
    $summary{query}{$producer}{pages_read}=$pages;
    for my $mode (qw(dry-run apply)) {
        $summary{change}{$mode}{$producer}=summary(map {$_->[5]} grep {$_->[2] eq $producer && $_->[3] eq $mode} @change_rows);
    }
}
$meta{load_after}=read_file('/proc/loadavg');
write_file("$opt{out}/metadata.json",JSON::PP->new->canonical->pretty->encode(\%meta));
write_file("$opt{out}/summary.json",JSON::PP->new->canonical->pretty->encode(\%summary));
print JSON::PP->new->canonical->pretty->encode(\%summary);
