use strict;
use warnings;
sub raw {
    my($p)=@_;
    -f $p && !-l $p or die "regular record required: $p\n";
    open my $f,'<:raw',$p or die "$p: $!\n";
    local $/;return <$f>;
}
sub object { my $v=decode_json(raw($_[0]));ref($v) eq 'HASH' or die "object required\n";return $v; }
sub digest {
    my($p)=@_;
    -f $p && !-l $p or die "regular digest input required\n";
    open my $f,'<:raw',$p or die "$p: $!\n";
    return Digest::SHA->new(256)->addfile($f)->hexdigest;
}
sub equal { defined($_[0]) && defined($_[1]) && !ref($_[0]) && !ref($_[1]) && $_[0] eq $_[1] or die "$_[2]\n"; }
my($path,$repo,$results)=@ARGV;
my $c=object($path);
equal($c->{version},1,'checkpoint version');
equal($c->{protocol},'lkjscript-current50-checkpoint-continuation-1','checkpoint protocol');
equal($c->{results},$results,'single declared result destination');
equal($c->{repository},$repo,'owned repository differs');
equal($c->{prior_query_launches},0,'prior query count differs');
equal($c->{query_launches},50,'query count differs');
for my $r(@{$c->{records}}) {
    ref($r) eq 'HASH' && ($r->{sha256}//'')=~/\A[0-9a-f]{64}\z/ or die "record shape\n";
    my $relative=$r->{path}//'';
    $relative=~m{\Abench/[A-Za-z0-9_./-]+\z} && $relative!~m{(?:\A|/)\.\.(?:/|\z)} or die "record path\n";
    equal(digest("$repo/$relative"),$r->{sha256},"changed record $relative");
}
my $original=$c->{original_evidence};
my $frozen="$repo/$c->{imported_attempt}";
my $manifest=raw("$repo/$c->{imported_attempt_manifest}");
for my $line(split /\n/,$manifest) {
    $line=~/\A([0-9a-f]{64})  (.+)\z/ or die "attempt manifest framing\n";
    my($hash,$record)=($1,$2);
    my $prefix="$c->{imported_attempt}/";
    index($record,$prefix)==0 or die "attempt manifest path\n";
    my $suffix=substr($record,length($prefix));
    equal(digest("$repo/$record"),$hash,"frozen attempt changed");
    equal(digest("$original/$suffix"),$hash,"live attempt changed");
}
my @prior_queries=glob("$original/raw/query-*");
!@prior_queries && !-e "$original/timed-completion" or die "questions already started\n";
raw("$original/completion")=~/\Aphase=unchanged-index\nstatus=1\n/ or die "original stop differs\n";
my $imported="$repo/$c->{validated_directory}";
my $builds=object("$imported/builds.json");
$builds->{valid} && ref($builds->{rows}) eq 'ARRAY' && @{$builds->{rows}}==2 or die "two accepted imported builds required\n";
my $pin=object("$imported/pins.json");
for my $key(qw(snapshot source checked_ir producer image_sha256)) { equal($pin->{$key},$c->{pins}{$key},"pin $key differs"); }
my $fixture=$c->{fixture};
my $keydir=$c->{key_directory};
for my $v($fixture,$keydir,$original,$imported,$pin->{snapshot}) { defined($v) && !ref($v) && $v!~/[\r\n\0]/ or die "invalid binding\n"; }
equal(raw("$fixture/owner-evidence"),"$original\n",'fixture evidence ownership');
equal(raw("$fixture/owner-repository"),"$repo\n",'fixture repository ownership');
equal(raw("$fixture/owner-key"),"$keydir\n",'fixture key ownership');
equal(raw("$keydir/owner-run"),"$fixture\n",'key run ownership');
-f "$keydir/key" && !-l "$keydir/key" && -s "$keydir/key"==32 or die "private key unavailable\n";
equal(digest("$fixture/bin/producer"),$c->{pins}{producer},'producer differs');
equal(-s "$fixture/bin/producer",$c->{producer_bytes},'producer bytes differ');
for my $name(qw(cold-index unchanged-index)) {
    equal(digest("$fixture/output/$name.image"),$c->{pins}{producer},"$name image differs");
    equal(-s "$fixture/output/$name.image",$c->{producer_bytes},"$name image size differs");
}
print "$fixture\n$keydir\n$pin->{snapshot}\n$original\n$imported\n";
