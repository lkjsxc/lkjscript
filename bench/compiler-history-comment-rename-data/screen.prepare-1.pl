#!/usr/bin/env perl
# Bounded static shortlist only: no compiler, semantic proof, or Git mutation.
use strict; use warnings; use Digest::SHA qw(sha256_hex);
my ($repo,$directory)=@ARGV;
@ARGV==2 && -d $directory or die "usage: $0 REPOSITORY EXISTING_OUTPUT_DIRECTORY\n";
sub git {
    open my $p,'-|','git','-c','core.packedGitWindowSize=64m','-c','core.packedGitLimit=128m','-C',$repo,@_ or die $!;
    binmode $p; local $/; my $v=<$p>//''; close $p or die "git failed: @_\n"; return $v;
}
sub write_file { my ($path,$value)=@_; !-e $path or die "already exists: $path\n"; open my $f,'>:raw',$path or die $!; print {$f} $value; close $f or die $!; }
my $head='6a187b94fb763f7b0565436a98ab5feda3cd642f';
my $window=git('rev-list','--max-count=300',$head);
sha256_hex($window) eq 'bee88306e1c12adfb3e0f8dfc6a96bf71c6b8ad344d4d08c2cdb0b808293cd20' or die "population changed\n";
my @window=split /\n/,$window; @window==300 or die "wrong population\n";
my @scope=(':\(glob\)**/*.lkj',':\(glob\)**/*.rs',':\(exclude\)**/.generation-one/**');
my %keyword=map {$_=>1} qw(as async await become box break const continue crate do dyn else enum extern false final fn for if impl in let loop macro match mod move mut override priv pub ref return self Self static struct super trait true try type typeof union unsafe unsized use virtual where while yield);
sub normalized {
    my ($s)=@_; my @literal;
    $s =~ s{("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])'|//[^\n]*|/\*.*?\*/)}{
        my $x=$1;
        if ($x =~ m{^/[/\*]}) {""} else {push @literal,$x; "\x01".scalar(@literal)."\x02"}
    }gse;
    $s =~ s/\b([A-Za-z_][A-Za-z0-9_]*)\b/$keyword{$1} ? $1 : "\x03"/ge;
    $s =~ s/\s+//g;
    return $s."\x00".join("\x00",@literal);
}
my $inventory="rank\trevision\tfirst_parent\tlanguage_files\tlanguage_diff_sha256\tpaths\n";
my $shortlist="revision\tfirst_parent\ttitle\n";
my ($scanned,$hits,$path_only)=(0,0,0);
for my $index (0..$#window) {
    my $rev=$window[$index];
    my $parents=git('show','-s','--format=%P',$rev); $parents=~s/\n\z//;
    my ($parent)=split / /,$parents;
    defined $parent or die "root commit requires explicit framing\n";
    my $diff=git('diff','--no-ext-diff','--find-renames','--unified=0',$parent,$rev,'--',@scope);
    my @paths=grep {length} split /\0/,git('diff','--name-only','--no-renames','-z',$parent,$rev,'--',@scope);
    !grep {/[\t\n;]/} @paths or die "path requires escaping\n";
    $inventory.=join("\t",$index+1,$rev,$parent,scalar(@paths),sha256_hex($diff),join(';',@paths))."\n";
    next if $diff eq ''; $scanned++;
    my $status=git('diff','--name-status','--find-renames',$parent,$rev,'--',@scope);
    my @status=grep {length} split /\n/,$status;
    $path_only++ if @status && !grep {!/^R100\t[^\t]+\t[^\t]+$/} @status;
    my ($old,$new)=('','');
    for (split /\n/,$diff) {next if /^(?:\+\+\+|---) /; $old.=substr($_,1)."\n" if /^-/; $new.=substr($_,1)."\n" if /^\+/;}
    next if $old eq $new;
    if (normalized($old) eq normalized($new)) {
        my $title=git('show','-s','--format=%s',$rev); $title=~s/\n\z//;
        $shortlist.=join("\t",$rev,$parent,$title)."\n"; $hits++;
    }
}
$scanned==204 && $hits==2 && $path_only==0 or die "static screen changed: $scanned/$hits/$path_only\n";
write_file("$directory/population.tsv",$inventory);
write_file("$directory/shortlist.tsv",$shortlist);
write_file("$directory/screen.txt","horizon=$head\nwindow_sha256=".sha256_hex($window)."\nwindow_commits=300\nlanguage_diffs=$scanned\nidentifier_or_comment_shortlist=$hits\npure_content_preserving_language_path_renames=$path_only\nsemantic_proof=false\ncompiler_executions=0\n");
for my $rev (qw(f71dd494d2c7b4ea04471bad6e75c26c4076eb67 2d143a1fd27ce8e475653cebf6cdb85332032e32)) {
    my $parent=git('rev-parse',"$rev^"); $parent=~s/\n\z//;
    write_file("$directory/$rev.diff",git('diff','--no-ext-diff','--no-renames',$parent,$rev,'--',@scope));
}
print "verified named population=300 language_diffs=204 shortlist=2 pure_path_renames=0 compiler_executions=0\n";
