#!/usr/bin/env perl
# Read-only Git inventory for the eleven predeclared source-creation blockers.
# Declaration matches are raw evidence for manual route classification, not a parser.
use strict; use warnings; use JSON::PP;
my ($repository, $input, $output) = @ARGV;
@ARGV == 3 or die "usage: $0 REPOSITORY BLOCKERS_TSV OUTPUT_PREFIX\n";
sub read_bytes { open my $f, '<:raw', $_[0] or die $!; local $/; return <$f> // ''; }
sub git {
    open my $f, '-|', 'git', '-C', $repository, @_ or die $!;
    local $/; my $v=<$f> // ''; close $f;
    my $exit=$? >> 8;
    ($exit==0 || ($_[0] eq 'grep' && $exit==1)) or die "git failed ($exit): @_\n";
    return $v;
}
open my $paths, '>', "$output.paths.tsv" or die $!;
open my $raw, '>', "$output.declarations.txt" or die $!;
print $paths join("\t",qw(ordinal revision parent action projected_path destination_parent parent_exists original_source_path)), "\n";
my @rows=split /\n/, read_bytes($input); shift @rows;
for my $line (@rows) {
    my @row=split /\t/,$line;
    my ($ordinal,$revision,$parent)=@row[0,2,3];
    my (@added,@removed);
    @added=@{decode_json($row[7])}; @removed=@{decode_json($row[9])};
    my (%after,%before);
    for my $which ([$revision,\%after],[$parent,\%before]) {
        for my $path (split /\n/,git('ls-tree','-r','--name-only',$which->[0],'--','src','library')) {
            next unless $path =~ /\.(lkj|rs)$/;
            (my $projected=$path) =~ s/\.rs$/.lkj/;
            exists $which->[1]{$projected} and die "ambiguous projection at $projected\n";
            $which->[1]{$projected}=$path;
        }
    }
    for my $action (['added',\@added,\%after],['deleted',\@removed,\%before]) {
        for my $path (@{$action->[1]}) {
            (my $directory=$path) =~ s{/[^/]+$}{};
            my $parent_entry=git('ls-tree','-d',$parent,'--',$directory);
            my $exists=$parent_entry =~ /^040000 tree [0-9a-f]+\t\Q$directory\E\n$/ ? 'yes' : 'no';
            print $paths join("\t",$ordinal,$revision,$parent,$action->[0],$path,$directory,$exists,$action->[2]{$path}),"\n";
        }
    }
    my @names=map { m{([^/]+)\.lkj$}; $1 } @added;
    for (@names) { /^[A-Za-z_][A-Za-z_0-9]*$/ or die "unexpected name $_\n"; }
    my $pattern='(^|[^A-Za-z_0-9])mod[[:space:]]+(' . join('|',@names) . ')[[:space:]]*;';
    print $raw "=== original ordinal $ordinal revision $revision parent $parent root $row[12] ===\n";
    print $raw git('grep','-n','-C','2','-E','-e',$pattern,$revision,'--','src','library');
    print $raw "\n";
}

close $paths or die $!;
close $raw or die $!;
if (-f "$output.routes.tsv") {
    my %paths;
    my @path_rows=split /\n/,read_bytes("$output.paths.tsv"); shift @path_rows;
    for (@path_rows) {
        my @r=split /\t/;
        $paths{"$r[0]\t$r[4]"}=\@r;
    }
    open my $classification,'>',"$output.classification.tsv" or die $!;
    print $classification join("\t",qw(ordinal projected_path action parent_exists declaration_owner owner_state declaration_kind selected_root_route evidence)),"\n";
    my @routes=split /\n/,read_bytes("$output.routes.tsv"); shift @routes;
    my (%seen,%owner_state,%route_count,%kind_count,%parent_count,%case_count);
    for my $line (@routes) {
        my @r=split /\t/,$line;
        @r==6 or die "bad manual route row\n";
        my $key="$r[0]\t$r[1]";
        !$seen{$key}++ or die "duplicate route row\n";
        my $path=$paths{$key} // die "route outside frozen added paths\n";
        my $state='standalone';
        if ($r[2] ne '-') {
            my $in_after=git('ls-tree','-r','--name-only',$path->[1],'--',$r[2]);
            $in_after eq "$r[2]\n" or die "declaring owner absent in target: $r[2]\n";
            my $in_before=git('ls-tree','-r','--name-only',$path->[2],'--',$r[2]);
            $state=$in_before eq "$r[2]\n" ? 'existing' : 'new';
        }
        $owner_state{$state}++;
        $route_count{$r[4]}++;
        $kind_count{$r[3]}++;
        $parent_count{$path->[6]}++;
        $case_count{$r[0]}{$path->[6]}++;
        print $classification join("\t",$r[0],$r[1],$path->[3],$path->[6],$r[2],$state,@r[3..5]),"\n";
    }
    keys(%seen)==keys(%paths) or die "unclassified added path\n";
    printf "paths=%d cases=%d\n",scalar(keys %paths),scalar(keys %case_count);
    for my $key (sort keys %owner_state) { print "declaration_owner $key $owner_state{$key}\n"; }
    for my $key (sort keys %route_count) { print "static_route $key $route_count{$key}\n"; }
    for my $key (sort keys %kind_count) { print "declaration_kind $key $kind_count{$key}\n"; }
    for my $key (sort keys %parent_count) { print "destination_parent_exists $key $parent_count{$key}\n"; }
    for my $ordinal (sort {$a<=>$b} keys %case_count) {
        printf "case %d existing_parent=%d missing_parent=%d\n",$ordinal,$case_count{$ordinal}{yes}//0,$case_count{$ordinal}{no}//0;
    }
}
