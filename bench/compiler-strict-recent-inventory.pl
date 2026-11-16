#!/usr/bin/env perl
# Static Git/proof inventory only. Never invokes lkjscript or rewrites history.
use strict; use warnings; use Digest::SHA qw(sha256_hex);
my ($repo,$prefix,$mode)=@ARGV;
defined $prefix && @ARGV<=3 && (!defined $mode || $mode eq '--check')
    or die "usage: $0 REPOSITORY PREFIX [--check]\n";
my $head='6a187b94fb763f7b0565436a98ab5feda3cd642f';
my $previous='a4fd685a9bb2e7736110a4ed2a43651dfe040d56';
my $old_producer='4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09';
sub git {
    open my $p,'-|','git','-C',$repo,@_ or die $!; binmode $p; local $/;
    my $v=<$p>//''; close $p or die "git failed: @_\n"; return $v;
}
sub bytes {
    open my $f,'<:raw',$_[0] or die "$_[0]: $!\n"; local $/; return <$f>//'';
}
sub line { my $v=shift; $v=~s/\n\z//; return $v; }
my %out;
my $window=git('rev-list','--max-count=300',$head);
sha256_hex($window) eq 'bee88306e1c12adfb3e0f8dfc6a96bf71c6b8ad344d4d08c2cdb0b808293cd20'
    or die "frozen population differs\n";
my @window=split /\n/,$window; @window==300 or die "wrong population count\n";
my %rank; $rank{$window[$_]}=$_+1 for 0..$#window;
$out{'window'}=$window;
my @selected=qw(
9047a677f9a353ae13883b4998adf9da2fc67b48
de65c09bab9d336e10dcfe245058d4ee14b3aea2
0a410badb7e96ed9f6cf185df7d1f781d71c5fdb
c5f67a5583aaba6dd338ba3e7278fdc775f36731
ba27e90add8c1a28e7d2c03984dbaa4aed1a8e72
71e821dfcaaf6a7940ed67bb7d2d3a721c169922);
$out{'order'}=join("\n",@selected)."\n";
sha256_hex($out{'order'}) eq 'dbefff80f9fd1164e81bf429a73a5978ef783868312309578e06cdf5ea1e4d21'
    or die "selected order differs\n";
my %expected=(
    $selected[0]=>[qw(src/semantic_facts.lkj)],
    $selected[1]=>[qw(src/structure_edit.lkj src/structure_write.lkj)],
    $selected[2]=>[qw(src/ast.lkj src/semantic.lkj src/source.lkj src/structure_edit.lkj src/structure_replay.lkj)],
    $selected[3]=>[qw(src/structure_write.lkj)],
    $selected[4]=>[qw(src/semantic_store.lkj)],
    $selected[5]=>[qw(src/driver.lkj src/store.lkj)],
);
my @prior=(
    ['self-use-23','bench/compiler-strict-history-imported.tsv',
     '0aafd3a891bf224bd3113ab9de58219fecba44b84e0c5605a28f8ced0f25acb1',23],
    ['separate-four-probe-1','bench/compiler-strict-history-four.proofs.tsv',
     '02bd19ad873b7b20ae0a947513c3603443b97633aeefa2555f751d058c696daa',1],
);
my %proved; my $inside=0; my $outside=0;
$out{'membership.tsv'}="revision\tcampaign\tproducer_sha256\tlast300\trank\tproof_table_sha256\n";
for my $group (@prior) {
    my ($campaign,$file,$hash,$count)=@$group;
    my $value=bytes($file); sha256_hex($value) eq $hash or die "$file changed\n";
    my @rows=split /\n/,$value; shift @rows;
    @rows==$count or die "wrong imported count\n";
    for my $row (@rows) {
        my @x=split /\t/,$row; my $rev=$x[1];
        !$proved{$rev}++ or die "duplicate prior proof\n";
        my $in=exists $rank{$rev}; $in?$inside++:$outside++;
        $out{'membership.tsv'}.=join("\t",$rev,$campaign,$old_producer,$in?'yes':'no',$rank{$rev}//'-',$hash)."\n";
    }
}
$inside==24 && $outside==0 or die "prior horizon membership differs\n";
$out{'candidates.tsv'}="ordinal\trevision\tparent\troot\tchanged_lines\tchanged_files\tbefore_src_tree\tafter_src_tree\tbefore_library_tree\tafter_library_tree\n";
$out{'required-paths.tsv'}="ordinal\trevision\tpath\tstatus\tbefore_sha256\tafter_sha256\n";
for my $i (0..$#selected) {
    my $rev=$selected[$i]; exists $rank{$rev} && !$proved{$rev} or die "selected membership/overlap\n";
    my $parent=line(git('rev-parse',"$rev^"));
    my @all=grep {length} split /\0/,git('diff','--name-only','--no-renames','-z',$parent,$rev);
    my @required=grep {m{^(src|library)/} || /\.(lkj|rs)$/} @all;
    join("\0",@required) eq join("\0",@{$expected{$rev}}) or die "$rev full required scope differs\n";
    my $status=git('diff','--name-status','--no-renames',$parent,$rev,'--',@required);
    $status eq join('',map {"M\t$_\n"} @required) or die "selected file set is not existing modifications\n";
    for my $side ($parent,$rev) {
        my $root=git('show',"$side:src/main.lkj");
        $root!~/\#\s*\[\s*cfg\b/ or die "root has configuration attributes; needs manual scope decision\n";
        for my $path (@required) {
            $path=~m{^src/([A-Za-z_][A-Za-z0-9_]*)\.lkj$} or die "selected file is not direct-root module\n";
            my $module=$1;
            $root=~/^(?:pub\s+)?mod\s+\Q$module\E\s*;/m or die "$side root omits $module\n";
        }
    }
    my $weight=0;
    for (split /\n/,git('diff','--numstat',$parent,$rev,'--',@required)) {
        my ($add,$del)=split /\t/; $add=~/^\d+$/ && $del=~/^\d+$/ or die "binary source\n"; $weight+=$add+$del;
    }
    $out{'candidates.tsv'}.=join("\t",$i+1,$rev,$parent,'src/main.lkj',$weight,scalar(@required),
        map {line(git('rev-parse',$_))} ("$parent:src","$rev:src","$parent:library","$rev:library"))."\n";
    for my $path (@required) {
        $out{'required-paths.tsv'}.=join("\t",$i+1,$rev,$path,'M',
            sha256_hex(git('show',"$parent:$path")),sha256_hex(git('show',"$rev:$path")))."\n";
    }
}
my %classification=map {$_=>'selected-main-root'} @selected;
$classification{'e03db719e77de6e6f3cb1ce4495058b646bb9220'}='alternate-existing-refactor-root-not-selected';
$classification{'579cf55e88177b9c9a4c59e2ff6c1bf7829e2878'}='excluded-mixed-roots-and-Rust-witness';
$classification{'1c5d964e03a505d9545f85da39a5b2975f7694b9'}='excluded-mixed-roots-and-Rust-witness';
$classification{'9c0d1797a31e3b67ad17ec3cf819951f8f51b4e0'}='excluded-main-and-semantic-test-roots';
$classification{'501db910c1e57b6647cf2b0f7cf972f034f5bbfd'}='excluded-Rust-witness-only';
$out{'population.tsv'}="revision\tparent\tclassification\tlanguage_files\tsubject\n";
$out{'population-paths.tsv'}="revision\tpath\n";
my $population_count=0;
for my $rev (split /\n/,git('rev-list','--reverse','--topo-order',"$previous..$head")) {
    my $parent=line(git('rev-parse',"$rev^"));
    my @paths=grep {length} split /\0/,git('diff','--name-only','--no-renames','-z',$parent,$rev,'--','*.lkj','*.rs');
    next unless @paths;
    exists $classification{$rev} or die "unclassified language commit $rev\n";
    my $subject=line(git('show','-s','--format=%s',$rev));
    $out{'population.tsv'}.=join("\t",$rev,$parent,$classification{$rev},scalar(@paths),$subject)."\n";
    $out{'population-paths.tsv'}.="$rev\t$_\n" for @paths;
    $population_count++;
}
$population_count==11 or die "language population differs\n";
$out{'inventory.meta'}="horizon=$head\nprevious_horizon=$previous\nwindow_count=300\nwindow_sha256=".sha256_hex($window)."\nwindow_oldest=$window[-1]\nprior_strict_inside=24\nprior_strict_outside=0\nselected_new_candidates=6\nselected_order_sha256=".sha256_hex($out{'order'})."\ncompiler_executions=0\nclaimed_new_successes=0\n";
for my $suffix (sort keys %out) {
    my $path="$prefix.$suffix";
    if (defined $mode) { bytes($path) eq $out{$suffix} or die "$path differs\n"; }
    else { open my $f,'>:raw',$path or die $!; print {$f} $out{$suffix}; close $f or die $!; }
}
print "verified named_horizon=$head prior_inside=24 prior_outside=0 selected=6 language_commits=11 compiler_executions=0\n";
