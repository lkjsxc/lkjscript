#!/usr/bin/env perl
# Diagnostic only: exact corrected guard/rel, with no Git or compiler invocation.
use strict; use warnings; use JSON::PP ();
sub die_if ($$) {
    die "E_COPY_RELEASE_GUARD expected condition and message\n"
        unless @_ == 2 && defined($_[1]);
    die "$_[1]\n" if $_[0];
}
sub rel {
    my ($p)=@_;
    die_if(!defined($p) || ref($p) || $p!~m{\A[A-Za-z0-9._/-]+\z} ||
        $p=~m{(?:\A|/)\.\.?(?:/|\z)} || $p=~m{//} || $p=~m{\A/},"unsafe relative path");
    return $p;
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
    my $returned=eval {rel($path)}; my $error=$@;
    die "unexpected result:$name\n" if (!!$error)!=!!$unsafe;
    die "unexpected refusal:$name:$error\n" if $unsafe && $error ne "unsafe relative path\n";
    die "returned path differs:$name\n" if !$unsafe && $returned ne $path;
    print JSON::PP->new->canonical->encode({case=>$name,
        status=>$unsafe?"refused":"accepted",diagnostic=>$unsafe?$error:undef}),"\n";
}
my @messages;
for my $test (
    ["missing-message",sub { &die_if(0) }],
    ["undefined-message",sub { &die_if(0,undef) }],
    ["extra-argument",sub { &die_if(0,"message","extra") }],
) {
    eval { $test->[1]->() }; my $error=$@;
    die "arity diagnostic absent:$test->[0]\n"
        unless $error eq "E_COPY_RELEASE_GUARD expected condition and message\n";
    print JSON::PP->new->canonical->encode({case=>$test->[0],status=>"refused",diagnostic=>$error}),"\n";
}
my $error=eval {die_if("relative" =~ m{\A/},"must not fire"); 1};
die "prototype failed scalar context\n" unless $error && !$@;
print JSON::PP->new->canonical->encode({case=>"direct-false-match",status=>"accepted"}),"\n";
