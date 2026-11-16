#!/usr/bin/env perl
# Pure Perl argument-context reproduction; no file, Git, compiler or profile calls.
use strict; use warnings; use JSON::PP ();
sub arguments { return [@_]; }
sub original {
    my ($p)=@_;
    return arguments(!defined($p) || ref($p) || $p!~m{\A[A-Za-z0-9._/-]+\z} ||
        $p=~m{(?:\A|/)\.\.?(?:/|\z)} || $p=~m{//} || $p=~m{\A/},"unsafe relative path");
}
sub fixed {
    my ($p)=@_;
    return arguments(scalar(!defined($p) || ref($p) || $p!~m{\A[A-Za-z0-9._/-]+\z} ||
        $p=~m{(?:\A|/)\.\.?(?:/|\z)} || $p=~m{//} || $p=~m{\A/}),"unsafe relative path");
}
my @cases=(
    ["release-relative","bench/compiler-profile-copy-sites-release-20261116.json",0],
    ["ordinary-relative","tools/bootstrap.sh",0],
    ["absolute","/tmp/source",1],["parent","../source",1],
    ["inner-parent","bench/../source",1],["empty","",1],
    ["undefined",undef,1],["reference",{},1],
);
for my $case (@cases) {
    my ($name,$path,$unsafe)=@$case;
    my $old=original($path); my $new=fixed($path);
    die "fixed arity:$name\n" unless @$new==2;
    die "fixed predicate:$name\n" unless ($new->[0]?1:0)==$unsafe;
    die "fixed message:$name\n" unless $new->[1] eq "unsafe relative path";
    if (!$unsafe) {
        die "original failure absent:$name\n" unless @$old==1 && $old->[0] eq "unsafe relative path";
    }
    print JSON::PP->new->canonical->encode({
        case=>$name,original_arity=>scalar(@$old),original_arguments=>$old,
        fixed_arity=>scalar(@$new),fixed_unsafe=>$new->[0]?JSON::PP::true:JSON::PP::false,
        expected_unsafe=>$unsafe?JSON::PP::true:JSON::PP::false,
    }),"\n";
}
