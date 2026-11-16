#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
profile_test=$(mktemp -d target/profile-test.XXXXXX)
profile_key_directory=
trap 'chmod u+w "$profile_test/publish-stage/blocked" 2>/dev/null || :; rm -rf "$profile_test"; if [ -n "$profile_key_directory" ]; then rm -rf "$profile_key_directory"; fi' EXIT HUP INT TERM
"$compiler" build tests/fixtures/profile.lkj -o "$profile_test/plain"
"$compiler" build tests/fixtures/profile.lkj --profile -o "$profile_test/sampled"
"$profile_test/plain" > "$profile_test/plain.out"
"$profile_test/sampled" > "$profile_test/sampled.out"
cmp "$profile_test/plain.out" "$profile_test/sampled.out"
# The explicit sampling prefix is a grant. If its directory disappears, the
# authorized entry's one diagnostic names that prefix and the refused syscall.
cat > "$profile_test/refusal.lkj" <<'SOURCE'
pub fn main(mut world: World) { world.stdout().write(b"unreached\n"); }
SOURCE
mkdir "$profile_test/removed"
"$compiler" build "$profile_test/refusal.lkj" --profile -o "$profile_test/removed/sampled"
mv "$profile_test/removed/sampled" "$profile_test/refused"
rmdir "$profile_test/removed"
profile_refusal_status=0
"$profile_test/refused" > "$profile_test/refused.out" 2> "$profile_test/refused.err" || profile_refusal_status=$?
test "$profile_refusal_status" -eq 1
test ! -s "$profile_test/refused.out"
test "$(wc -l < "$profile_test/refused.err")" -eq 1
grep -qxF "$profile_test/refusal.lkj:1: R0007 Profile $profile_test/removed/sampled.samples openat: ENOENT (2)" "$profile_test/refused.err"
"$compiler" profile "$profile_test"/sampled.samples.* > "$profile_test/report"
awk '
  /^CPU samples:/ { total=$3+0; if (total < 10) exit 1 }
  /^kind [0-9]/ {
    if ($2 < 0 || $2 > 100 || $3 > total) exit 1
    kinds++
    kind_samples += $3
    if ($4 == "arithmetic" && $3 > 0) arithmetic=1
  }
  /^[0-9]/ {
    if ($1 < 0 || $1 > 100 || $2 < 0 || $2 > 100 || $3 > total) exit 1
    if ($4 == "work" && $1 > 50) hot=1
    if ($4 == "recurse" && $2 > 50) recursive=1
    if ($4 == "main" && $2 > 50) root=1
  }
  END { if (NR > 37 || !hot || !recursive || !root || kinds != 16 || kind_samples != total || !arithmetic) exit 1 }
' "$profile_test/report"
"$compiler" ir tests/fixtures/profile.lkj -o "$profile_test/plain.lkjir"
sed 's/^fn "work" /fn "work\\ncontrol" /' "$profile_test/plain.lkjir" > "$profile_test/named.lkjir"
"$compiler" build "$profile_test/named.lkjir" --profile -o "$profile_test/named"
"$profile_test/named" > "$profile_test/named.out"
cmp "$profile_test/plain.out" "$profile_test/named.out"
"$compiler" profile "$profile_test"/named.samples.* > "$profile_test/named.report"
grep -qF 'work\u{a}control' "$profile_test/named.report"
test "$(wc -l < "$profile_test/named.report")" -le 37
cp "$profile_test"/sampled.samples.* "$profile_test/invalid-metadata"
printf '\377\377\377\377\377\377\377\377' |
    dd of="$profile_test/invalid-metadata" bs=1 seek=48 conv=notrunc status=none
if "$compiler" profile "$profile_test/invalid-metadata" > "$profile_test/out" 2> "$profile_test/error"; then
    echo 'profiler accepted invalid category metadata' >&2
    exit 1
fi
grep -q E_PROFILE "$profile_test/error"
cp "$profile_test"/sampled.samples.* "$profile_test/invalid-count"
printf '\377\377\377\377\377\377\377\377' |
    dd of="$profile_test/invalid-count" bs=1 seek=24 conv=notrunc status=none
if "$compiler" profile "$profile_test/invalid-count" > "$profile_test/out" 2> "$profile_test/error"; then
    echo 'profiler accepted inconsistent sample totals' >&2
    exit 1
fi
grep -q E_PROFILE "$profile_test/error"
# Termination can interrupt the handler after its serial advances but before
# its category counter. Such a file reports the same completed samples.
profile_samples=$(awk '/^CPU samples:/ { print $3+0 }' "$profile_test/report")
for gap in -1 1 2; do
    cp "$profile_test"/sampled.samples.* "$profile_test/serial-$gap"
    LC_ALL=C awk -v serial="$((profile_samples + gap))" '
        BEGIN {
            for (byte=0; byte<8; byte++) {
                printf "%c", serial % 256
                serial=int(serial / 256)
            }
        }
    ' | dd of="$profile_test/serial-$gap" bs=1 seek=24 conv=notrunc status=none
    if [ "$gap" = 1 ]; then
        "$compiler" profile "$profile_test/serial-$gap" > "$profile_test/interrupted.report"
        cmp "$profile_test/report" "$profile_test/interrupted.report"
    else
        if "$compiler" profile "$profile_test/serial-$gap" > "$profile_test/out" 2> "$profile_test/error"; then
            echo "profiler accepted invalid sample serial gap $gap" >&2
            exit 1
        fi
        grep -q E_PROFILE "$profile_test/error"
    fi
done
printf 'invalid samples\n' > "$profile_test/invalid"
if "$compiler" profile "$profile_test/invalid" > "$profile_test/out" 2> "$profile_test/error"; then
    echo 'profiler accepted invalid input' >&2
    exit 1
fi
grep -q E_PROFILE "$profile_test/error"
# The copy-site entry is separate: the original arithmetic/default-report test
# above keeps its existing workload and assertions.
cat > "$profile_test/copy-data.pl" <<'PERL'
use strict;
use warnings;
use Digest::SHA qw(sha256 sha256_hex);
use JSON::PP ();
sub read_bytes {
    my ($path)=@_;
    open my $fh, '<:raw', $path or die "$path: $!\n";
    local $/; my $data=<$fh>; close $fh or die "$path: $!\n";
    return defined($data) ? $data : '';
}
sub write_bytes {
    my ($path,$data)=@_;
    open my $fh, '>:raw', $path or die "$path: $!\n";
    print {$fh} $data or die "$path: $!\n";
    close $fh or die "$path: $!\n";
}
sub word {
    my ($data,$at)=@_;
    die "short word\n" if $at<0 || $at+8>length($data);
    return unpack('Q<',substr($data,$at,8));
}
sub put {
    my ($ref,$at,$value)=@_;
    die "short output word\n" if $at<0 || $at+8>length($$ref);
    substr($$ref,$at,8)=pack('Q<',$value);
}
sub metadata {
    my ($data)=@_;
    die "not v4\n" unless length($data)>=160 && substr($data,0,8) eq 'LKJPROF4';
    my ($offset,$count,$map,$functions)=map { word($data,$_) } (112,120,128,136);
    die "bad fixture extent\n" unless $count>1 && $offset+$count*72==$map
        && $map+$functions*16==length($data);
    return ($offset,$count,$map,$functions);
}
sub stats {
    my ($path)=@_;
    my @lines=read_bytes($path)=~/^LKJSCRIPT_STATS (.+)$/mg;
    die "one stats record required\n" unless @lines==1;
    return JSON::PP->new->decode($lines[0]);
}
sub rows {
    my ($data)=@_;
    return [ $data =~ /^(copy [^\n]*\n)/mg ];
}
my $mode=shift @ARGV // die "mode\n";
if ($mode eq 'selection') {
    my ($original,$selected,$log)=map { read_bytes($_) } @ARGV;
    my @before=$original=~/^fn "([^"\n]*)" /mg;
    my @after=$selected=~/^fn "([^"\n]*)" /mg;
    die "selected function slots changed\n" unless join("\0",@before) eq join("\0",@after);
    my @target=grep { $before[$_] eq 'profile_value' } 0..$#before;
    die "profile_value identity\n" unless @target==1;
    my $target=$target[0];
    die "original call missing\n" unless $original=~/^\@\S+ call \d+ $target(?:\s|$)/m;
    die "inline selection not exposed\n" if $selected=~/^\@\S+ call \d+ $target(?:\s|$)/m;
    die "companion unexpectedly original\n" if $selected eq $original;
    my @records=$log=~/^LKJSCRIPT_STATS (.+)$/mg;
    die "cold stats\n" unless @records==1;
    my $s=JSON::PP->new->decode($records[0]);
    die "inlining was not accepted\n" unless $s->{inline_calls}>0 && $s->{inline_accepted}>0;
    die "companion byte count\n" unless $s->{profile_ir_bytes}==length($selected);
} elsif ($mode eq 'warm') {
    my ($cold_path,$warm_path,$ir_path)=@ARGV;
    my $cold=stats($cold_path); my $warm=stats($warm_path);
    die "unexpected whole-image return\n" unless $cold->{build_reused}==0 && $warm->{build_reused}==0;
    die "cold fragments were not isolated\n" unless $cold->{emitted_reused}==0;
    die "warm fragments were not used\n" unless $warm->{emitted_reused}>0;
    die "companion not regenerated\n" unless $warm->{profile_ir_bytes}==length(read_bytes($ir_path));
} elsif ($mode eq 'binding') {
    my ($sample,$ir)=map { read_bytes($_) } @ARGV;
    my ($offset,$count)=metadata($sample);
    die "selected digest mismatch\n" unless substr($sample,80,32) eq sha256($ir);
    my $mapped=0; $mapped+=word($sample,$offset+$_*72+16) for 0..$count-1;
    my $counters=word($sample,48);
    die "copy workload produced no mapped observation\n" unless $mapped>0 && word($sample,$counters+8)>0;
    print "$count\n";
} elsif ($mode eq 'page') {
    my ($reply_path,$sample_path,$ir_path,$start,$limit,$bytes,$row_path)=@ARGV;
    my $reply=read_bytes($reply_path); my $sample=read_bytes($sample_path); my $ir=read_bytes($ir_path);
    my ($offset,$count)=metadata($sample);
    die "reply bound\n" if length($reply)>$bytes;
    my ($header,$scope)=split /\n/,$reply,3;
    my $profile_id=sha256_hex($sample); my $ir_id=sha256_hex($ir);
    die "reply pins\n" unless $header=~/^copy-sites version=4 profile=$profile_id companion=$ir_id companion-check=checked-ir /;
    die "reply kind\n" unless $scope=~/^scope=existing-kind-copy /;
    my $listed=rows($reply);
    my @lines=split /\n/,$reply;
    die "unexpected protocol line\n" unless @lines==@$listed+3;
    my @tail=$reply=~/^continuation cursor=(\S+) rows=(\d+) next=(\d+) total-sites=(\d+)\n\z/mg;
    die "one terminal continuation\n" unless @tail==4;
    my ($cursor,$emitted,$next,$total)=@tail;
    die "pagination accounting\n" unless $emitted==@$listed && $emitted<=$limit
        && $next==$start+$emitted && $total==$count && $next<=$count;
    my $expected=$next==$count ? 'none' : "4:$profile_id:$ir_id:$next";
    die "continuation pin\n" unless $cursor eq $expected;
    die "zero progress\n" if $start<$count && $emitted==0;
    open my $out, '>>:raw', $row_path or die "$row_path: $!\n";
    print {$out} @$listed or die "$row_path: $!\n"; close $out or die "$row_path: $!\n";
    print "$cursor $next $total\n";
} elsif ($mode eq 'mutate') {
    my ($sample_path,$ir_path,$dir)=@ARGV;
    my $sample=read_bytes($sample_path); my $ir=read_bytes($ir_path);
    my ($offset,$count,$map,$functions)=metadata($sample);
    my @body=grep { (word($sample,$offset+$_*72+64)&255)==1 } 0..$count-1;
    die "no body copy fixture\n" unless @body;
    my $at=$offset+$body[0]*72;
    my @mutations=(
        ['range',sub { put($_[0],$at+8,word($sample,$at)); }],
        ['owner',sub { put($_[0],$at+24,$functions); }],
        ['type',sub { my $types=()=$ir=~/^type /mg; put($_[0],$at+48,$types); }],
        ['width',sub { put($_[0],$at+56,word($sample,$at+56)+1); }],
        ['instruction',sub { my $ref=$_[0]; substr($$ref,$at+32,8)=pack('H*','feffffffffffffff'); }],
        ['subsite',sub { my $ref=$_[0]; substr($$ref,$at+40,8)=pack('H*','feffffffffffffff'); }],
        ['sentinel',sub { my $ref=$_[0]; substr($$ref,$at+32,8)="\xff"x8; }],
        ['tag',sub { my $ref=$_[0]; substr($$ref,$at+71,1)=chr(ord(substr($$ref,$at+71,1))|128); }],
        ['map',sub { put($_[0],$map,1); }],
        ['tail',sub { my $ref=$_[0]; $$ref.="\0"; }],
        ['short',sub { my $ref=$_[0]; chop $$ref; }],
    );
    for my $mutation (@mutations) {
        my $copy=$sample; $mutation->[1]->(\$copy);
        write_bytes("$dir/bad-$mutation->[0]",$copy);
    }
    write_bytes("$dir/wrong.lkjir",$ir."\n");
    my $invalid=$ir;
    $invalid=~s/^(local "(?:[^"\\]|\\.)*" )\d+( (?:let|mut))$/$1.'999999999'.$2/me
        or die "invalid checked companion fixture\n";
    write_bytes("$dir/unchecked.lkjir",$invalid);
    my $bound=$sample; substr($bound,80,32)=sha256($invalid);
    write_bytes("$dir/unchecked.samples",$bound);
    my $zero=$sample;
    my $symbols=word($zero,8);
    for my $i (0..$symbols-1) { put(\$zero,160+$i*64+$_,0) for (16,24,32); }
    my $counters=word($zero,48);
    put(\$zero,$counters+$_*8,0) for 0..15;
    put(\$zero,$offset+$_*72+16,0) for 0..$count-1;
    put(\$zero,144,0);
    my @states=(
        ['complete',4,4,0], ['serial-leading',5,4,0],
        ['site-leading',5,5,0], ['unmapped-leading',5,4,1],
        ['detail-early',4,5,0], ['detail-two',5,6,0],
        ['detail-missing',4,3,0], ['serial-behind',3,4,0], ['serial-two',6,4,0],
    );
    for my $state (@states) {
        my ($name,$serial,$mapped,$unmapped)=@$state;
        my $copy=$zero; put(\$copy,24,$serial); put(\$copy,$counters+8,4);
        put(\$copy,$offset+16,$mapped); put(\$copy,144,$unmapped);
        write_bytes("$dir/count-$name",$copy);
    }
    my $overflow=$zero; put(\$overflow,24,5); put(\$overflow,$counters+8,4);
    substr($overflow,$offset+16,8)="\xff"x8; put(\$overflow,$offset+72+16,1);
    write_bytes("$dir/count-overflow",$overflow);
} elsif ($mode eq 'roles') {
    my ($sample,$ir)=map { read_bytes($_) } @ARGV;
    my ($offset,$count)=metadata($sample);
    die "roles digest\n" unless substr($sample,80,32) eq sha256($ir);
    my @names=$ir=~/^fn "([^"\n]*)" /mg;
    my @types=$ir=~/^type "([^"\n]*)" /mg;
    my @sites;
    for my $i (0..$count-1) {
        my $at=$offset+$i*72; my $tags=word($sample,$at+64);
        push @sites, {
            function=>$names[word($sample,$at+24)], instruction=>word($sample,$at+32),
            subsite=>word($sample,$at+40), type=>$types[word($sample,$at+48)],
            width=>word($sample,$at+56), origin=>$tags&255,
            source=>($tags>>8)&255, destination=>($tags>>16)&255,
            source_flags=>($tags>>24)&7, destination_flags=>($tags>>32)&7,
        };
    }
    my $need=sub {
        my ($label,$predicate)=@_;
        die "missing retained role $label\n" unless grep { $predicate->($_) } @sites;
    };
    for my $row (['copy_role_byte',1],['copy_role_pair',16],['copy_role_wide',256],['copy_role_text',16]) {
        my ($function,$width)=@$row;
        $need->("$function incoming",sub {
            $_[0]{function} eq $function && $_[0]{origin}==2 && $_[0]{subsite}==0
                && $_[0]{source}==5 && $_[0]{destination}==6 && $_[0]{width}==$width
                && pack('Q<',$_[0]{instruction}) eq "\xff"x8
        });
        $need->("$function result",sub {
            $_[0]{function} eq $function && $_[0]{origin}==1
                && $_[0]{source}==1 && $_[0]{destination}==7 && $_[0]{width}==$width
        });
    }
    for my $row ([1,16],[3,256]) {
        my ($subsite,$width)=@$row;
        $need->("constructor original subsite $subsite",sub {
            $_[0]{function} eq 'copy_roles' && $_[0]{source}==1 && $_[0]{destination}==3
                && $_[0]{subsite}==$subsite && $_[0]{width}==$width
        });
    }
    $need->('whole local',sub { $_[0]{function} eq 'copy_roles'
        && $_[0]{source}==1 && $_[0]{destination}==1 && $_[0]{width}==16 });
    for my $flags (1,2,3,4) {
        $need->("projected read flags $flags",sub { $_[0]{function} eq 'copy_roles'
            && $_[0]{source}==2 && $_[0]{destination}==1 && $_[0]{source_flags}==$flags });
    }
    for my $flags (1,2) {
        $need->("projected write flags $flags",sub { $_[0]{function} eq 'copy_roles'
            && $_[0]{source}==1 && $_[0]{destination}==2 && $_[0]{destination_flags}==$flags });
    }
    # Compound-reference/indexed writes in this selected IR use Replace.
    # Its byte swap is outside the existing KIND_COPY interval contract.
    my ($owner,$instruction)=(-1,0);
    my %replace;
    for my $line (split /\n/,$ir) {
        if ($line=~/^fn /) { ++$owner; $instruction=0; }
        elsif ($line=~/^\@/) {
            $replace{"$owner:$instruction"}=1 if $line=~/^\@\S+ intrinsic \d+ "replace"(?:\s|$)/;
            ++$instruction;
        }
    }
    die "Replace control missing\n" unless keys %replace;
    for my $i (0..$count-1) {
        my $at=$offset+$i*72;
        die "Replace incorrectly attributed as copy\n"
            if $replace{word($sample,$at+24).':'.word($sample,$at+32)};
    }
    for my $subsite (0,1) {
        $need->("array original subsite $subsite",sub { $_[0]{function} eq 'copy_roles'
            && $_[0]{source}==1 && $_[0]{destination}==4 && $_[0]{subsite}==$subsite });
    }
    for my $row ([1,8,'vector push'],[8,9,'vector pop'],[1,10,'array repeat'],[1,11,'box payload']) {
        my ($source,$destination,$label)=@$row;
        $need->($label,sub { $_[0]{type} eq 'CopyRolePair' && $_[0]{width}==16
            && $_[0]{source}==$source && $_[0]{destination}==$destination });
    }
    $need->('view header',sub { $_[0]{function} eq 'copy_roles' && $_[0]{width}==16
        && $_[0]{source}==13 && $_[0]{destination}==13 });
} elsif ($mode eq 'in-flight') {
    my ($path,$wanted)=@ARGV;
    my $text=read_bytes($path);
    die "in-flight report\n" unless $text=~/^copy-sites .* in-flight=$wanted sites=/m;
} else { die "unknown helper mode $mode\n"; }
PERL
"$compiler" run tests/fixtures/profile.lkj --entry copy_once --engine interpreter --no-store \
    > "$profile_test/copy-once.interpreter"
"$compiler" run tests/fixtures/profile.lkj --entry copy_once --engine native --inline-limit 64 --no-store \
    > "$profile_test/copy-once.native"
cmp "$profile_test/copy-once.interpreter" "$profile_test/copy-once.native"
"$compiler" run tests/fixtures/profile.lkj --entry copy_roles --engine interpreter --inline-limit 0 --no-store \
    > "$profile_test/copy-roles.interpreter"
"$compiler" run tests/fixtures/profile.lkj --entry copy_roles --engine native --inline-limit 0 --no-store \
    > "$profile_test/copy-roles.native"
printf '297\n' > "$profile_test/copy-roles.expected"
cmp "$profile_test/copy-roles.expected" "$profile_test/copy-roles.interpreter"
cmp "$profile_test/copy-roles.interpreter" "$profile_test/copy-roles.native"
"$compiler" build tests/fixtures/profile.lkj --entry copy_roles --inline-limit 0 --no-store \
    --profile --profile-ir "$profile_test/copy-roles.lkjir" -o "$profile_test/copy-roles"
"$profile_test/copy-roles" > "$profile_test/copy-roles.out"
# Standalone scalar images discard their result; run above owns the value ABI.
test ! -s "$profile_test/copy-roles.out"
set -- "$profile_test"/copy-roles.samples.*
test "$#" -eq 1
"$compiler" profile "$1" --copy-sites --profile-ir "$profile_test/copy-roles.lkjir" \
    --limit 100000 --bytes 16777216 > "$profile_test/copy-roles.report"
grep -q '^continuation cursor=none ' "$profile_test/copy-roles.report"
perl "$profile_test/copy-data.pl" roles "$1" "$profile_test/copy-roles.lkjir"
"$compiler" build tests/fixtures/profile.lkj --entry copy_samples --inline-limit 64 --no-store \
    -o "$profile_test/copy-plain"
"$profile_test/copy-plain" > "$profile_test/copy-plain.out"
"$compiler" ir tests/fixtures/profile.lkj --entry copy_samples --inline-limit 64 --no-store \
    -o "$profile_test/original-inline.lkjir"
"$compiler" ir tests/fixtures/profile.lkj --entry copy_samples --no-opt --no-store \
    -o "$profile_test/original-no-opt.lkjir"
cmp "$profile_test/original-inline.lkjir" "$profile_test/original-no-opt.lkjir"
# The key is outside the default '.' source grant and every repository library
# or runtime path. This fixture owns the store and the temporary key directory.
profile_key_directory=$(mktemp -d /tmp/compiler-profile-key.XXXXXX)
chmod 700 "$profile_key_directory"
(umask 077; head -c 32 /dev/urandom > "$profile_key_directory/key")
mkdir "$profile_test/copy-store"
"$compiler" build tests/fixtures/profile.lkj --entry copy_samples --inline-limit 64 --profile \
    --profile-ir "$profile_test/copy.lkjir" --store "$profile_test/copy-store" \
    --store-key "$profile_key_directory/key" --stats -o "$profile_test/copy" \
    > "$profile_test/copy-cold.out" 2> "$profile_test/copy-cold.log"
perl "$profile_test/copy-data.pl" selection "$profile_test/original-inline.lkjir" \
    "$profile_test/copy.lkjir" "$profile_test/copy-cold.log"
cp "$profile_test/copy" "$profile_test/copy-cold"
cp "$profile_test/copy.lkjir" "$profile_test/copy-cold.lkjir"
rm "$profile_test/copy.lkjir"
"$compiler" build tests/fixtures/profile.lkj --entry copy_samples --inline-limit 64 --profile \
    --profile-ir "$profile_test/copy.lkjir" --store "$profile_test/copy-store" \
    --store-key "$profile_key_directory/key" --stats -o "$profile_test/copy" \
    > "$profile_test/copy-warm.out" 2> "$profile_test/copy-warm.log"
cmp "$profile_test/copy-cold" "$profile_test/copy"
cmp "$profile_test/copy-cold.lkjir" "$profile_test/copy.lkjir"
perl "$profile_test/copy-data.pl" warm "$profile_test/copy-cold.log" \
    "$profile_test/copy-warm.log" "$profile_test/copy.lkjir"
"$profile_test/copy" > "$profile_test/copy.out"
cmp "$profile_test/copy-plain.out" "$profile_test/copy.out"
set -- "$profile_test"/copy.samples.*
test "$#" -eq 1
copy_samples=$1
copy_count=$(perl "$profile_test/copy-data.pl" binding "$copy_samples" "$profile_test/copy.lkjir")
"$compiler" profile "$copy_samples" > "$profile_test/copy-default.report"
test "$(wc -l < "$profile_test/copy-default.report")" -le 37
"$compiler" profile "$copy_samples" --copy-sites --profile-ir "$profile_test/copy.lkjir" \
    --limit 100000 --bytes 16777216 > "$profile_test/copy-all.report"
: > "$profile_test/copy-all.rows"
set -- $(perl "$profile_test/copy-data.pl" page "$profile_test/copy-all.report" \
    "$copy_samples" "$profile_test/copy.lkjir" 0 100000 16777216 "$profile_test/copy-all.rows")
test "$1" = none
test "$2" -eq "$copy_count"

# First page forces a continuation. Later requests widen only the output bound;
# concatenated rows must equal the same fixed full ordering.
"$compiler" profile "$copy_samples" --copy-sites --profile-ir "$profile_test/copy.lkjir" \
    --limit 1 --bytes 4096 > "$profile_test/copy-first.report"
: > "$profile_test/copy-pages.rows"
set -- $(perl "$profile_test/copy-data.pl" page "$profile_test/copy-first.report" \
    "$copy_samples" "$profile_test/copy.lkjir" 0 1 4096 "$profile_test/copy-pages.rows")
copy_cursor=$1
copy_next=$2
test "$copy_cursor" != none
copy_first_cursor=$copy_cursor
copy_page=1
while [ "$copy_cursor" != none ]; do
    test "$copy_page" -le 64
    "$compiler" profile "$copy_samples" --copy-sites --profile-ir "$profile_test/copy.lkjir" \
        --limit 64 --bytes 65536 --cursor "$copy_cursor" > "$profile_test/copy-page-$copy_page.report"
    set -- $(perl "$profile_test/copy-data.pl" page "$profile_test/copy-page-$copy_page.report" \
        "$copy_samples" "$profile_test/copy.lkjir" "$copy_next" 64 65536 "$profile_test/copy-pages.rows")
    copy_cursor=$1
    copy_next=$2
    copy_page=$((copy_page + 1))
done
cmp "$profile_test/copy-all.rows" "$profile_test/copy-pages.rows"
copy_first_bytes=$(wc -c < "$profile_test/copy-first.report")
"$compiler" profile "$copy_samples" --copy-sites --profile-ir "$profile_test/copy.lkjir" \
    --limit 1 --bytes "$copy_first_bytes" > "$profile_test/copy-exact.report"
cmp "$profile_test/copy-first.report" "$profile_test/copy-exact.report"
copy_refuse() {
    case_name=$1
    case_message=$2
    shift 2
    if "$@" > "$profile_test/$case_name.out" 2> "$profile_test/$case_name.err"; then
        echo "profiler accepted refused case $case_name" >&2
        exit 1
    fi
    test ! -s "$profile_test/$case_name.out"
    grep -qF "$case_message" "$profile_test/$case_name.err"
}
copy_refuse bound-first E_PROFILE_BOUND "$compiler" profile "$copy_samples" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes "$((copy_first_bytes - 1))"
copy_refuse bound-header E_PROFILE_BOUND "$compiler" profile "$copy_samples" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes 1
copy_profile_id=$(sha256sum "$copy_samples" | cut -d ' ' -f 1)
copy_ir_id=$(sha256sum "$profile_test/copy.lkjir" | cut -d ' ' -f 1)
copy_end_cursor="4:$copy_profile_id:$copy_ir_id:$copy_count"
copy_refuse bound-empty E_PROFILE_BOUND "$compiler" profile "$copy_samples" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes 1 --cursor "$copy_end_cursor"
"$compiler" profile "$copy_samples" --copy-sites --profile-ir "$profile_test/copy.lkjir" \
    --limit 1 --bytes 4096 --cursor "$copy_end_cursor" > "$profile_test/copy-empty.report"
: > "$profile_test/copy-empty.rows"
perl "$profile_test/copy-data.pl" page "$profile_test/copy-empty.report" "$copy_samples" \
    "$profile_test/copy.lkjir" "$copy_count" 1 4096 "$profile_test/copy-empty.rows" > "$profile_test/copy-empty.meta"
test ! -s "$profile_test/copy-empty.rows"
copy_refuse cursor-noncanonical E_PROFILE_CURSOR "$compiler" profile "$copy_samples" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes 4096 --cursor "4:$copy_profile_id:$copy_ir_id:00"
copy_refuse cursor-range E_PROFILE_CURSOR "$compiler" profile "$copy_samples" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes 4096 \
    --cursor "4:$copy_profile_id:$copy_ir_id:$((copy_count + 1))"
copy_refuse cursor-companion E_PROFILE_CURSOR "$compiler" profile "$copy_samples" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes 4096 \
    --cursor "4:$copy_profile_id:$copy_profile_id:1"
copy_refuse usage-bounds E_USAGE "$compiler" profile "$copy_samples" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir"
copy_refuse usage-zero E_USAGE "$compiler" profile "$copy_samples" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir" --limit 0 --bytes 4096
mkdir "$profile_test/copy-mutations"
perl "$profile_test/copy-data.pl" mutate "$copy_samples" "$profile_test/copy.lkjir" "$profile_test/copy-mutations"
for mutation in range owner sentinel tag map tail short; do
    copy_refuse "invalid-$mutation" E_PROFILE "$compiler" profile \
        "$profile_test/copy-mutations/bad-$mutation"
done
for mutation in instruction subsite type width; do
    copy_refuse "invalid-$mutation" E_PROFILE_IR "$compiler" profile \
        "$profile_test/copy-mutations/bad-$mutation" --copy-sites \
        --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes 4096
done
copy_refuse wrong-companion 'companion bytes differ' "$compiler" profile "$copy_samples" \
    --copy-sites --profile-ir "$profile_test/copy-mutations/wrong.lkjir" --limit 1 --bytes 4096
copy_refuse unchecked-companion 'companion failed independent IR checking' "$compiler" profile \
    "$profile_test/copy-mutations/unchecked.samples" --copy-sites \
    --profile-ir "$profile_test/copy-mutations/unchecked.lkjir" --limit 1 --bytes 4096
for state in complete serial-leading site-leading unmapped-leading; do
    "$compiler" profile "$profile_test/copy-mutations/count-$state" > "$profile_test/count-$state.default"
    "$compiler" profile "$profile_test/copy-mutations/count-$state" --copy-sites \
        --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes 4096 > "$profile_test/count-$state.detail"
    case "$state" in complete|serial-leading) flight=0;; *) flight=1;; esac
    perl "$profile_test/copy-data.pl" in-flight "$profile_test/count-$state.detail" "$flight"
done
cmp "$profile_test/count-complete.default" "$profile_test/count-serial-leading.default"
for state in detail-early detail-two detail-missing serial-behind serial-two overflow; do
    copy_refuse "count-$state" E_PROFILE "$compiler" profile "$profile_test/copy-mutations/count-$state"
done
copy_refuse cursor-profile E_PROFILE_CURSOR "$compiler" profile \
    "$profile_test/copy-mutations/count-complete" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes 4096 --cursor "$copy_first_cursor"
# Publication errors are deterministic owned-file fixtures. Permission denial
# must still be real under a root-run gate; only that child's DAC bypass is
# dropped. No observer, race, signal injection or global filesystem change.
copy_without_dac_override() {
    if [ "$(id -u)" -eq 0 ]; then
        command -v setpriv >/dev/null
        setpriv --bounding-set=-dac_override,-dac_read_search "$@"
    else
        "$@"
    fi
}
copy_no_publication() {
    test -z "$(find "$1" -maxdepth 1 -type f \( -name 'image-*' -o -name 'inputs-image-*' \) -print)"
    test -z "$(find "$2" -name '.lkjscript-*.tmp' -print)"
}
mkdir "$profile_test/publish-same" "$profile_test/publish-same/store"
printf 'old same output\n' > "$profile_test/publish-same/image"
cp "$profile_test/publish-same/image" "$profile_test/publish-same/expected"
copy_refuse publish-same E_PROFILE_OUTPUT "$compiler" build tests/fixtures/profile.lkj \
    --entry copy_samples --inline-limit 64 --profile \
    --profile-ir "$profile_test/publish-same/image" -o "$profile_test/publish-same/image" \
    --store "$profile_test/publish-same/store" --store-key "$profile_key_directory/key"
cmp "$profile_test/publish-same/image" "$profile_test/publish-same/expected"
copy_no_publication "$profile_test/publish-same/store" "$profile_test/publish-same"
copy_refuse publish-parent-alias E_PROFILE_OUTPUT "$compiler" build tests/fixtures/profile.lkj \
    --entry copy_samples --inline-limit 64 --profile \
    --profile-ir "$profile_test/publish-same/./image" -o "$profile_test/publish-same/image" \
    --store "$profile_test/publish-same/store" --store-key "$profile_key_directory/key"
cmp "$profile_test/publish-same/image" "$profile_test/publish-same/expected"
copy_no_publication "$profile_test/publish-same/store" "$profile_test/publish-same"

mkdir "$profile_test/publish-stage" "$profile_test/publish-stage/store" "$profile_test/publish-stage/blocked"
printf 'old image\n' > "$profile_test/publish-stage/image"
printf 'old companion\n' > "$profile_test/publish-stage/blocked/companion"
cp "$profile_test/publish-stage/image" "$profile_test/publish-stage/expected-image"
cp "$profile_test/publish-stage/blocked/companion" "$profile_test/publish-stage/expected-companion"
chmod 0555 "$profile_test/publish-stage/blocked"
copy_refuse publish-stage "$profile_test/publish-stage/blocked/companion:1:1: E_IO create temporary failed (errno 13)" \
    copy_without_dac_override "$compiler" build tests/fixtures/profile.lkj \
    --entry copy_samples --inline-limit 64 --profile \
    --profile-ir "$profile_test/publish-stage/blocked/companion" -o "$profile_test/publish-stage/image" \
    --store "$profile_test/publish-stage/store" --store-key "$profile_key_directory/key"
chmod u+w "$profile_test/publish-stage/blocked"
cmp "$profile_test/publish-stage/image" "$profile_test/publish-stage/expected-image"
cmp "$profile_test/publish-stage/blocked/companion" "$profile_test/publish-stage/expected-companion"
copy_no_publication "$profile_test/publish-stage/store" "$profile_test/publish-stage"

mkdir "$profile_test/publish-image" "$profile_test/publish-image/store" "$profile_test/publish-image/image"
printf 'old directory image\n' > "$profile_test/publish-image/image/marker"
cp "$profile_test/publish-image/image/marker" "$profile_test/publish-image/expected-marker"
printf 'old companion\n' > "$profile_test/publish-image/companion"
copy_refuse publish-image "$profile_test/publish-image/image:1:1: E_IO rename failed (errno 21)" \
    "$compiler" build tests/fixtures/profile.lkj --entry copy_samples --inline-limit 64 --profile \
    --profile-ir "$profile_test/publish-image/companion" -o "$profile_test/publish-image/image" \
    --store "$profile_test/publish-image/store" --store-key "$profile_key_directory/key"
test -d "$profile_test/publish-image/image"
cmp "$profile_test/publish-image/image/marker" "$profile_test/publish-image/expected-marker"
# The companion rename completed before the image rename failed. Its exact
# checked bytes are valid, but the pair did not publish successfully.
cmp "$profile_test/publish-image/companion" "$profile_test/copy.lkjir"
copy_no_publication "$profile_test/publish-image/store" "$profile_test/publish-image"
printf 'ok profiler (ordinary report, selected IR, copy roles/value, fragment reuse, pagination/bounds, malformed coordinates, independent checking, interrupted accounting, publication refusals)\n'
