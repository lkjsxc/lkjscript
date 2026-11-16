use strict; use warnings;
use JSON::PP qw(decode_json);
use Digest::SHA qw(sha256_hex);
my $root=shift // die "root\n"; my $data="$root/bench/compiler-generated-source-data";
sub bytes { open my $f,'<:raw',shift or die $!; local $/; return <$f>; }
sub hashes {
    my ($path)=@_; my %r;
    for my $line(split /\n/,bytes($path)) {
        $line =~ /^([0-9a-f]{64})  (.+)$/ or die "bad hash row $line\n";
        $r{$2}=$1;
    }
    return \%r;
}
my $base="f31949f5e3b09125378365871b4fd15ebe00b44a";
my $initial=hashes("$data/before-1.sha256");
for my $path(sort keys %$initial) {
    open my $git,'-|','git','-C',$root,'show',"$base:$path" or die $!;
    local $/; my $content=<$git>; close $git or die "git show $path\n";
    sha256_hex($content) eq $initial->{$path} or die "initial mismatch $path\n";
}
my $previous=$initial; my $operations=0; my $facts=0;
print "base=$base\n";
for my $n(1..7) {
    my $before=hashes("$data/before-$n.sha256");
    my $after=hashes("$data/after-$n.sha256");
    for my $path(keys %$previous) { ($before->{$path}//"") eq $previous->{$path} or die "chain before$n $path\n"; }
    keys(%$before)==keys(%$previous) or die "before$n inventory\n";
    my $request=decode_json(bytes("$data/request-$n.json"));
    my $status=bytes("$data/apply-$n.status"); $status =~ s/\s+//g;
    my $out=bytes("$data/apply-$n.stdout");
    my %record;
    if($status eq "0") {
        $operations += @{$request->{operations}};
        while($out =~ /^("(?:[^"\\]|\\.)*") level=source-claim file before=(\S+) after=([0-9a-f]{64})$/mg) {
            my ($path,$from,$to)=(decode_json($1),$2,$3);
            $path =~ s/^\Q$root\E\/// or die "outside file fact $path\n";
            !exists $record{$path} or die "duplicate fact $path\n";
            $record{$path}=1; ++$facts;
            $from eq ($before->{$path}//"absent") or die "fact before$n $path\n";
            $to eq ($after->{$path}//"") or die "fact after$n $path\n";
        }
        for my $path(keys %$after) {
            next if ($before->{$path}//"absent") eq $after->{$path};
            $record{$path} or die "uncovered source change$n $path\n";
        }
        print "request$n applied operations=".scalar(@{$request->{operations}})." file_facts=".scalar(keys %record)." request_sha256=".sha256_hex(bytes("$data/request-$n.json"))."\n";
    } else {
        $out eq "" or die "refusal output$n\n";
        keys(%$after)==keys(%$before) or die "refusal inventory$n\n";
        for my $path(keys %$before) { $after->{$path} eq $before->{$path} or die "refusal changed$n $path\n"; }
        print "request$n refused status=$status unchanged=true request_sha256=".sha256_hex(bytes("$data/request-$n.json"))."\n";
    }
    for my $path(keys %$before) { exists $after->{$path} or die "source removed$n $path\n"; }
    $previous=$after;
}
for my $path(sort keys %$previous) {
    sha256_hex(bytes("$root/$path")) eq $previous->{$path} or die "final mismatch $path\n";
    print "final $path $previous->{$path}\n";
}
my $first=decode_json(bytes("$data/request-3.json"));
sha256_hex($first->{operations}[0]{projection}{source}) eq hashes("$data/after-3.sha256")->{"src/semantic/origins.lkj"} or die "new helper projection mismatch\n";
bytes("$data/actual-compiler.before.sha256") eq bytes("$data/actual-compiler.after.sha256") or die "self catalog altered source\n";
print "successful_operations=$operations file_facts=$facts final_source_files=".scalar(keys %$previous)." actual_compiler_source_unchanged=true\n";
