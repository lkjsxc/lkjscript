#!/usr/bin/env perl
# Independent bounded readers for the fixed v2/v3 comparison; no production code.
use strict;
use warnings;
use Digest::SHA qw(sha256 sha256_hex hmac_sha256);
use JSON::PP;
my $mode = shift @ARGV // '';
my $secret;
my $magic = "lkjscript-store-2\n";
my $maximum_offset = unpack('Q<', pack('H*', 'ffffffffffffff7f'));
my $no_next = unpack('Q<', pack('H*', 'ffffffffffffffff'));
sub raw {
    my ($path, $maximum) = @_;
    die "nonregular input $path\n" unless -f $path && !-l $path;
    open my $f, '<:raw', $path or die "$path: $!";
    my $size = -s $f;
    die "input size $path\n" unless defined($size) && $size <= $maximum;
    local $/;
    my $bytes = <$f>;
    die "input changed $path\n" unless length($bytes) == $size;
    return $bytes;
}
sub frame { return pack('Q<', length($_[0])) . $_[0]; }
sub word {
    my ($bytes, $at) = @_;
    die "short word\n" if $$at > length($bytes) || length($bytes) - $$at < 8;
    my $value = unpack('Q<', substr($bytes, $$at, 8)); $$at += 8; return $value;
}
sub field {
    my ($bytes, $at) = @_;
    my $length = word($bytes, $at);
    die "short field\n" if $length > length($bytes) - $$at;
    my $value = substr($bytes, $$at, $length); $$at += $length; return $value;
}
sub hex_id { return defined($_[0]) && $_[0] =~ /\A[0-9a-f]{64}\z/; }
sub authenticate {
    my ($bytes, $producer, $namespace, $key, $maximum) = @_;
    die "owner identity\n" unless hex_id($producer);
    die "envelope framing\n" unless length($bytes) >= 90 && substr($bytes, 0, 18) eq $magic;
    my $size = unpack('Q<', substr($bytes, 18, 8));
    die "envelope length\n" unless $size <= $maximum && $size == length($bytes) - 90;
    my $digest = substr($bytes, 26, 32);
    my $tag = substr($bytes, 58, 32);
    my $body = substr($bytes, 90);
    my $message = frame('lkjscript-authenticated-store-root-2') . frame(pack('H*', $producer))
        . frame('linux-x86_64') . frame($namespace) . frame($key) . pack('Q<', $size) . frame($digest);
    die "envelope authentication\n" unless hmac_sha256($message, $secret) eq $tag && sha256($body) eq $digest;
    return $body;
}
sub envelope {
    my ($path, $producer, $namespace, $key, $maximum) = @_;
    return authenticate(raw($path, $maximum + 90), $producer, $namespace, $key, $maximum);
}
sub extent {
    my ($ref, $total, $before) = @_;
    die "extent identity\n" unless hex_id($ref->{digest});
    die "extent representation\n" unless $total <= $maximum_offset && $before <= $total
        && $ref->{offset} >= 490 && $ref->{offset} <= $before
        && $ref->{length} >= 122 && $ref->{length} <= 32858
        && $ref->{length} <= $before - $ref->{offset};
}
sub snapshot {
    my ($directory, $producer, $id, $version) = @_;
    die "snapshot identity\n" unless hex_id($id);
    my $slot = sha256_hex(pack('H*', $producer));
    my ($body, $handle, $total);
    if ($version == 2) {
        $body = envelope("$directory/semantic-root-$slot-$id", $producer, 'semantic-root', $id, 376);
    } else {
        my $path = "$directory/semantic-container-v3-$slot-$id";
        die "nonregular container\n" unless -f $path && !-l $path;
        open $handle, '<:raw', $path or die "$path: $!";
        read($handle, my $front, 490) == 490 or die "short fixed front\n";
        $body = authenticate($front, $producer, 'semantic-container-v3', $id, 400);
        die "fixed manifest length\n" unless length($body) == 400;
        $total = -s $handle;
    }
    die "snapshot payload identity\n" unless sha256_hex($body) eq $id;
    my $at = 0;
    die "snapshot version\n" unless word($body, \$at) == $version;
    my @values = map { field($body, \$at) } 1..5;
    die "snapshot field identity\n" unless !grep { !hex_id($_) } @values;
    die "snapshot producer\n" unless $values[0] eq $producer;
    my $facts = word($body, \$at);
    my $out = {producer=>$producer, source=>$values[1], checked_ir=>$values[2],
        root=>$values[3], selector=>$values[4], facts=>$facts, slot=>$slot, identity=>$id,
        directory=>$directory, version=>$version};
    if ($version == 3) {
        my ($offset, $length, $claimed_total) = map { word($body, \$at) } 1..3;
        die "physical EOF differs\n" unless $claimed_total == $total && $total <= $maximum_offset;
        my $ref = {digest=>$out->{root}, offset=>$offset, length=>$length};
        extent($ref, $total, $total);
        die "root is not last\n" unless $offset + $length == $total;
        $out->{root_extent} = $ref; $out->{total} = $total; $out->{handle} = $handle;
    }
    die "snapshot trailing bytes\n" unless $at == length($body);
    my $namespace = $version == 2 ? 'semantic-locator' : 'semantic-locator-v3';
    my $protocol = sha256_hex("lkjscript-semantic-locator-$version");
    my $locator = envelope("$directory/$namespace-$id", $protocol, $namespace, $id, 104);
    my $expected = pack('Q<', $version) . pack('H*', $producer) . pack('H*', $out->{selector}) . pack('H*', $out->{root});
    die "locator relation\n" unless $locator eq $expected;
    return $out;
}
sub node {
    my ($snapshot, $reference, $before) = @_;
    if ($snapshot->{version} == 2) {
        my $body = envelope("$snapshot->{directory}/semantic-page-$snapshot->{slot}-$reference",
            $snapshot->{producer}, 'semantic-page', $reference, 32768);
        die "node payload identity\n" unless sha256_hex($body) eq $reference;
        return $body;
    }
    extent($reference, $snapshot->{total}, $before);
    my $handle = $snapshot->{handle};
    seek($handle, $reference->{offset}, 0) or die "extent seek\n";
    read($handle, my $bytes, $reference->{length}) == $reference->{length} or die "short extent\n";
    my $body = authenticate($bytes, $snapshot->{producer}, 'semantic-node-v3', $reference->{digest}, 32768);
    die "node payload identity\n" unless sha256_hex($body) eq $reference->{digest};
    return $body;
}
sub map_comparison {
    my ($left, $right, $key, $pa, $pb, $sa, $sb) = @_;
    die "maps arguments\n" unless defined $sb;
    $secret = raw($key, 32); die "key length\n" unless length($secret) == 32;
    my $left_snapshot = snapshot($left, $pa, $sa, 2);
    my $right_snapshot = snapshot($right, $pb, $sb, 3);
    for my $name (qw(source checked_ir selector facts)) {
        die "snapshot $name differs\n" unless $left_snapshot->{$name} eq $right_snapshot->{$name};
    }
    die "producer/pin separation\n" if $pa eq $pb || $sa eq $sb;
    my @todo = ([$left_snapshot->{root}, $right_snapshot->{root_extent}, 0, '', $right_snapshot->{total}]);
    my (%seen_a, %seen_b, %logical, %chains, %kind);
    my ($nodes, $branches, $leaves, $facts, $fact_bytes, $value_bytes, $key_bytes, $payload_a, $payload_b, $max_depth) = (0)x10;
    my @ranges;
    while (@todo) {
        my ($ra, $rb, $depth, $prefix, $before) = @{pop @todo};
        die "repeated v2 rooted page\n" if $seen_a{$ra}++;
        my $descriptor = join(':', $rb->{digest}, $rb->{offset}, $rb->{length});
        die "repeated v3 rooted extent\n" if $seen_b{$descriptor}++;
        my $x = node($left_snapshot, $ra, 0); my $y = node($right_snapshot, $rb, $before);
        push @ranges, [$rb->{offset}, $rb->{length}];
        $nodes++; $payload_a += length($x); $payload_b += length($y);
        $max_depth = $depth if $depth > $max_depth;
        my ($ia, $ib) = (0, 0);
        die "node versions\n" unless word($x, \$ia) == 1 && word($y, \$ib) == 3;
        my ($ta, $tb) = (word($x, \$ia), word($y, \$ib));
        my ($da, $db) = (word($x, \$ia), word($y, \$ib));
        my ($ca, $cb) = (word($x, \$ia), word($y, \$ib));
        die "node shape differs\n" unless $ta == $tb && $da == $depth && $db == $depth && $ca == $cb && $depth <= 64;
        if ($ta == 0) {
            $leaves++; die "leaf count\n" if $ca > 64;
            my $previous = '';
            for (1..$ca) {
                my ($name, $hash, $value) = map { field($x, \$ia) } 1..3;
                my ($other_name, $other_hash, $other_value) = map { field($y, \$ib) } 1..3;
                die "logical key/value bytes differ\n" unless $name eq $other_name && $hash eq $other_hash && $value eq $other_value;
                die "leaf path/order\n" unless hex_id($hash) && $hash eq sha256_hex($name)
                    && substr($hash, 0, $depth) eq $prefix && (!$previous || $previous lt $hash);
                $previous = $hash;
                die "duplicate logical key\n" if exists $logical{$name};
                $logical{$name} = [sha256($value), length($value)];
                $key_bytes += length($name); $value_bytes += length($value);
                die "logical page key\n" unless $name =~ /\A(.+)\t(0|[1-9][0-9]*)\z/s;
                my ($base, $page) = ($1, 0+$2);
                die "page number bounds\n" if $page > $maximum_offset;
                my $at = 0;
                die "logical page version\n" unless word($value, \$at) == 1;
                my $next = word($value, \$at); my $count = word($value, \$at);
                die "logical count\n" if $count > 16;
                die "logical continuation\n" unless $next == $no_next || $next == $page + 1;
                my $chain = ($chains{$base} //= {minimum=>$page, maximum=>$page, count=>0, tails=>0});
                $chain->{minimum} = $page if $page < $chain->{minimum};
                $chain->{maximum} = $page if $page > $chain->{maximum};
                $chain->{count}++;
                if ($next == $no_next) { $chain->{tails}++; $chain->{tail} = $page; }
                my ($category) = split /\t/, $name;
                my $k = ($kind{$category} //= {pages=>0, facts=>0, fact_bytes=>0, value_bytes=>0});
                $k->{pages}++; $k->{value_bytes} += length($value);
                for (1..$count) {
                    my $line = field($value, \$at);
                    die "fact size\n" if length($line) + 8 > 16000;
                    $facts++; $fact_bytes += length($line); $k->{facts}++; $k->{fact_bytes} += length($line);
                }
                die "logical trailing bytes\n" unless $at == length($value);
            }
        } elsif ($ta == 1) {
            $branches++; die "branch count/depth\n" if $ca < 1 || $ca > 16 || $depth >= 64;
            my @children; my $previous = -1;
            for (1..$ca) {
                my $digit = word($x, \$ia); my $child_a = field($x, \$ia);
                my $other_digit = word($y, \$ib); my $digest = field($y, \$ib);
                my $offset = word($y, \$ib); my $length = word($y, \$ib);
                die "branch digit/order\n" unless $digit == $other_digit && $digit <= 255
                    && chr($digit) =~ /\A[0-9a-f]\z/ && $digit > $previous && hex_id($child_a);
                my $child_b = {digest=>$digest, offset=>$offset, length=>$length};
                extent($child_b, $right_snapshot->{total}, $rb->{offset});
                $previous = $digit;
                push @children, [$child_a, $child_b, $depth+1, $prefix.chr($digit), $rb->{offset}];
            }
            push @todo, reverse @children;
        } else { die "node tag\n"; }
        die "node trailing bytes\n" unless $ia == length($x) && $ib == length($y);
    }
    for my $base (keys %chains) {
        my $c = $chains{$base};
        die "incomplete logical continuation chain\n" unless $c->{minimum} == 0
            && $c->{maximum} + 1 == $c->{count} && $c->{tails} == 1 && $c->{tail} == $c->{maximum};
    }
    my $end = 490;
    for my $r (sort { $a->[0] <=> $b->[0] } @ranges) {
        die "container stream gap/overlap\n" unless $r->[0] == $end;
        $end += $r->[1];
    }
    die "container complete extent\n" unless $end == $right_snapshot->{total} && (-s $right_snapshot->{handle}) == $right_snapshot->{total};
    die "snapshot fact count\n" unless $facts == $left_snapshot->{facts};
    my $map_hash = Digest::SHA->new(256);
    $map_hash->add('lkjscript-logical-map-oracle-1');
    for my $name (sort keys %logical) { $map_hash->add(frame($name), frame($logical{$name}[0]), pack('Q<', $logical{$name}[1])); }
    print JSON::PP->new->canonical->pretty->encode({
        status=>'exact-logical-map', version_a=>2, version_b=>3,
        producer_a=>$pa, producer_b=>$pb, snapshot_a=>$sa, snapshot_b=>$sb,
        source=>$left_snapshot->{source}, checked_ir=>$left_snapshot->{checked_ir}, selector=>$left_snapshot->{selector},
        root_a=>$left_snapshot->{root}, root_b=>$right_snapshot->{root}, facts=>$facts, physical_nodes=>$nodes,
        branches=>$branches, leaves=>$leaves, logical_pages=>scalar(keys %logical), logical_chains=>scalar(keys %chains),
        continuation_chains_verified=>JSON::PP::true, max_depth=>$max_depth,
        logical_key_bytes=>$key_bytes, logical_value_bytes=>$value_bytes, fact_utf8_bytes=>$fact_bytes,
        authenticated_node_payload_bytes_a=>$payload_a, authenticated_node_payload_bytes_b=>$payload_b,
        authenticated_node_envelope_bytes_a=>$payload_a+90*$nodes,
        authenticated_node_envelope_bytes_b=>$payload_b+90*$nodes,
        container_bytes_b=>$right_snapshot->{total}, sorted_logical_map_fingerprint=>$map_hash->hexdigest, kinds=>\%kind
    });
}
sub normalized_query {
    my ($path, $proof, $side, $kind, $id, $depth) = @_;
    my $version = $proof->{"version_$side"};
    my $producer = $proof->{"producer_$side"};
    my $snapshot = $proof->{"snapshot_$side"};
    my $out = raw($path, 32768);
    my ($header, $body) = split /\n/, $out, 2;
    die "missing query body\n" unless defined $body;
    my $source = $proof->{source}; my $checked = $proof->{checked_ir};
    my $quoted_id = JSON::PP->new->allow_nonref->encode($id);
    my $expected_header = "$quoted_id\tlevel=protocol\tfact=query\tversion=$version producer=$producer "
        . "validation=authenticated-producing-compiler current-check=not-performed snapshot=$snapshot "
        . "source=$source checked-ir=$checked kind=$kind depth=$depth freshness=not-asserted "
        . "source-reference-coverage=recorded-sites instance-coverage=demanded selection=explicit";
    die "query provenance\n" unless $header eq $expected_header;
    $header =~ s/version=\Q$version\E /version=VERIFIED_FORMAT /;
    $header =~ s/producer=\Q$producer\E /producer=VERIFIED_PRODUCER /;
    $header =~ s/snapshot=\Q$snapshot\E /snapshot=VERIFIED_SNAPSHOT /;
    my @rows = split /\n/, $body, -1;
    die "query final newline\n" unless @rows && pop(@rows) eq '';
    my $continuation = pop @rows;
    die "continuation framing\n" unless defined($continuation)
        && $continuation =~ /\A\Q$quoted_id\E\tlevel=protocol\tfact=continuation\tcursor=([^ ]+) facts=([0-9]+) pages-read=([0-9]+) traversal-steps=([0-9]+)\z/;
    my ($cursor, $emitted) = ($1, $2);
    die "fact count/bound\n" unless $emitted == @rows && $emitted <= 96;
    die "duplicate continuation\n" if grep { /\tlevel=protocol\tfact=continuation\t/ } @rows;
    if ($cursor ne 'none') {
        die "cursor hex\n" unless $cursor =~ /\A(?:[0-9a-f]{2})+\z/;
        my $question = sha256_hex("lkjscript-semantic-query-$version" . frame($snapshot)
            . frame($kind) . frame($id) . pack('Q<', $depth));
        my $cursor_key = sha256_hex("lkjscript-semantic-cursor-question-$version"
            . frame(pack('H*', $producer)) . frame($question));
        my $state = authenticate(pack('H*', $cursor), sha256_hex("lkjscript-semantic-cursor-$version"),
            "semantic-cursor-$version", $cursor_key, 32768);
        my $at = 0;
        die "cursor state version\n" unless word($state, \$at) == $version;
        my $count = word($state, \$at);
        die "cursor state count\n" if $count > length($state)/40;
        for (1..$count) {
            field($state, \$at); field($state, \$at);
            word($state, \$at); word($state, \$at);
            die "cursor state offset\n" if word($state, \$at) > 16;
        }
        die "cursor state trailing bytes\n" unless $at == length($state);
        substr($state, 0, 8, pack('Q<', 0));
        my $normalized_cursor = 'VERIFIED_STATE_' . unpack('H*', $state);
        my $changed = $continuation =~ s/\A(\Q$quoted_id\E\tlevel=protocol\tfact=continuation\t)cursor=\Q$cursor\E /${1}cursor=$normalized_cursor /;
        die "cursor normalization count\n" unless $changed == 1;
    }
    $body = join("\n", @rows, $continuation, '');
    return ("$header\n$body", $cursor ne 'none');
}
sub query_comparison {
    my ($metadata, $key, $left, $right, $kind, $id, $depth) = @_;
    die "query arguments\n" unless defined $depth && $depth =~ /\A[0-9]+\z/;
    my $proof = decode_json(raw($metadata, 1048576));
    die "map proof status\n" unless $proof->{status} eq 'exact-logical-map';
    $secret = raw($key, 32); die "key length\n" unless length($secret) == 32;
    my ($a, $ca) = normalized_query($left, $proof, 'a', $kind, $id, $depth);
    my ($b, $cb) = normalized_query($right, $proof, 'b', $kind, $id, $depth);
    die "query facts, protocol or authenticated traversal state differ\n" unless $a eq $b && $ca == $cb;
    print JSON::PP->new->canonical->encode({status=>'exact-facts-and-decoded-state', kind=>$kind, identity=>$id,
        continued=>$ca?JSON::PP::true:JSON::PP::false, normalized_sha256=>sha256_hex($a)}), "\n";
}
if ($mode eq 'maps') { map_comparison(@ARGV); }
elsif ($mode eq 'query') { query_comparison(@ARGV); }
else { die "usage: compare maps LEFT RIGHT KEY PA PB SA SB | query MAP_JSON KEY LEFT RIGHT KIND ID DEPTH\n"; }
