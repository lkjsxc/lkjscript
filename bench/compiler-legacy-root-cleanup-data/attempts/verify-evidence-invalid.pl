use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use IO::Uncompress::Gunzip;
use IPC::Open2;
use JSON::PP;
my $out='/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116/target/compiler-legacy-root-inventory-20261116';
my $repo='/home/coder/workspace/lkjscript';
sub slurp { open my $f,'<:raw',$_[0] or die "$_[0]: $!"; local $/; return <$f>; }
my $rows=decode_json(slurp("$out/nonstore-evidence.json"));
my $pid=open2(my $reader,my $writer,'git','-C',$repo,'cat-file','--batch');
binmode $reader; binmode $writer;
select((select($writer),$|=1)[0]);
sub blob {
    my ($reference)=@_;
    print $writer "$reference\n" or die "git input: $!";
    my $header=<$reader>; defined $header or die "missing git header";
    if ($header =~ /^(.+) missing\n$/) { return undef; }
    my ($oid,$type,$size)=$header =~ /^([0-9a-f]{40}) (blob) ([0-9]+)\n$/;
    die "unexpected Git object $header" unless defined $size;
    my $content='';
    while (length($content)<$size) {
        my $n=read($reader,my $part,$size-length($content));
        die "truncated Git blob" unless defined($n) && $n;
        $content.=$part;
    }
    read($reader,my $newline,1)==1 && $newline eq "\n" or die "Git framing";
    return {oid=>$oid,bytes=>0+$size,sha256=>sha256_hex($content),content=>$content};
}
my (@verified,@cross_family,@not_in_objects);
for my $row (@$rows) {
    if ($row->{retained_path}) {
        my $ref="$row->{retained_commit}:$row->{retained_path}";
        my $got=blob($ref); defined $got or die "retained object missing $ref";
        die "counterpart mismatch $row->{path}" unless
            $got->{oid} eq $row->{git_blob} &&
            $got->{bytes}==$row->{bytes} &&
            $got->{sha256} eq $row->{sha256};
        my $record={path=>$row->{path},bytes=>$got->{bytes},sha256=>$got->{sha256},reference=>$ref,git_blob=>$got->{oid}};
        push @verified,$record;
        push @cross_family,$record unless $row->{same_family};
    } elsif ($row->{relative} !~ /^(?:canonical|seed-[12]\.gz)$/) {
        my $got=blob($row->{git_blob});
        push @not_in_objects,{path=>$row->{path},git_blob=>$row->{git_blob},object_present=>defined($got)?JSON::PP::true:JSON::PP::false};
    }
}
my $seed_ref='72a259e8889cff6e7c4ac712d7940b77343a9f5b:seed/lkjscript.gz';
my $seed=blob($seed_ref); defined $seed or die "historical seed missing";
my $unzip=IO::Uncompress::Gunzip->new(\$seed->{content}) or die "seed gzip: $IO::Uncompress::Gunzip::GunzipError";
my $digest=Digest::SHA->new(256); my $length=0;
while (1) {
    my $n=$unzip->read(my $bytes,1024*1024);
    die "decompress failure: $IO::Uncompress::Gunzip::GunzipError" if $n<0;
    last if $n==0;
    $length+=$n; $digest->add($bytes);
}
$unzip->close or die "gzip close";
my $decoded_sha=$digest->hexdigest;
my @seed_rows=grep { $_->{scope} eq 'compiler-origin-container-gate.uQZd6X' && $_->{relative}=~/^(?:canonical|seed-[12]\.gz)$/ } @$rows;
die "seed row count" unless @seed_rows==3;
for (@seed_rows) {
    my ($bytes,$hash) = $_->{relative} eq 'canonical' ? ($length,$decoded_sha) : ($seed->{bytes},$seed->{sha256});
    die "seed recovery mismatch $_->{path}" unless $_->{bytes}==$bytes && $_->{sha256} eq $hash;
}
close $writer or die "git close input";
close $reader or die "git close output";
waitpid($pid,0)==$pid && $?==0 or die "git cat-file failed";
my $json=JSON::PP->new->canonical->pretty;
my $summary={
    retained_files_verified=>scalar @verified,
    retained_bytes_verified=>0,
    cross_family_matches=>\@cross_family,
    unmatched_git_object_checks=>\@not_in_objects,
    seed_recovery=>{
        reference=>$seed_ref,git_blob=>$seed->{oid},compressed_bytes=>$seed->{bytes},
        compressed_sha256=>$seed->{sha256},decoded_bytes=>$length,decoded_sha256=>$decoded_sha,
        exact_live_paths=>[map { $_->{path} } @seed_rows],
        executed=>JSON::PP::false,
    }
};
$summary->{retained_bytes_verified}+=$_->{bytes} for @verified;
for my $pair (['retained-counterparts-verified.json',\@verified],['verification-summary.json',$summary]) {
    open my $f,'>:raw',"$out/$pair->[0]" or die $!;
    print $f $json->encode($pair->[1]); close $f or die $!;
}
print $json->encode($summary);
