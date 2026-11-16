#!/usr/bin/env perl
# Post-command census. Only the signed Budget and fixed snapshot fronts are
# authenticated here; complete rooted node/value authentication is a later oracle.
use strict;
use warnings;
use Digest::SHA qw(sha256 sha256_hex hmac_sha256);
use JSON::PP;
my ($directory, $key_path, $capacity) = @ARGV;
die "usage: census STORE KEY WRITER_CAPACITY\n" unless defined($capacity) && $capacity =~ /\A[0-9]+\z/;
my $magic = "lkjscript-store-2\n";
sub read_exact {
    my ($file, $count) = @_;
    my $out = '';
    while (length($out) < $count) {
        my $got = read($file, my $part, $count - length($out));
        die "short read\n" unless defined($got) && $got > 0;
        $out .= $part;
    }
    return $out;
}
sub file_bytes {
    my ($path, $maximum) = @_;
    die "nonregular input $path\n" unless -f $path && !-l $path;
    open my $file, '<:raw', $path or die "$path: $!";
    my $size = -s $file;
    die "input bound $path\n" unless defined($size) && $size <= $maximum;
    my $bytes = $size ? read_exact($file, $size) : '';
    close $file or die "$path: $!";
    return $bytes;
}
my $secret = file_bytes($key_path, 32);
die "key length\n" unless length($secret) == 32;
sub frame { return pack('Q<', length($_[0])) . $_[0]; }
sub word {
    my ($bytes, $at) = @_;
    die "short word\n" if $$at > length($bytes) || length($bytes) - $$at < 8;
    my $value = unpack('Q<', substr($bytes, $$at, 8)); $$at += 8; return $value;
}
sub field {
    my ($bytes, $at) = @_;
    my $count = word($bytes, $at);
    die "short field\n" if $count > length($bytes) - $$at;
    my $value = substr($bytes, $$at, $count); $$at += $count; return $value;
}
sub hex_id { return defined($_[0]) && $_[0] =~ /\A[0-9a-f]{64}\z/; }
sub authenticate {
    my ($bytes, $producer, $namespace, $key, $maximum) = @_;
    die "envelope framing\n" unless length($bytes) >= 90 && substr($bytes, 0, 18) eq $magic;
    my $size = unpack('Q<', substr($bytes, 18, 8));
    die "envelope size\n" unless $size <= $maximum && $size == length($bytes) - 90;
    my $digest = substr($bytes, 26, 32);
    my $body = substr($bytes, 90);
    my $message = frame('lkjscript-authenticated-store-root-2') . frame(pack('H*', $producer))
        . frame('linux-x86_64') . frame($namespace) . frame($key) . pack('Q<', $size) . frame($digest);
    die "envelope authentication\n" unless hex_id($producer)
        && hmac_sha256($message, $secret) eq substr($bytes, 58, 32) && sha256($body) eq $digest;
    return $body;
}
sub family {
    my ($name) = @_;
    return 'budget-index' if $name eq '.budget-index';
    return 'budget-lock' if $name eq '.budget-lock';
    return 'semantic-locator-v3' if $name =~ /\Asemantic-locator-v3-[0-9a-f]{64}\z/;
    return 'semantic-locator-v2' if $name =~ /\Asemantic-locator-[0-9a-f]{64}\z/;
    if ($name =~ /\A(semantic-container-v3|semantic-latest-v3|semantic-page|semantic-root|semantic-latest)-[0-9a-f]{64}-[0-9a-f]{64}\z/) {
        return $1;
    }
    return $1 if $name =~ /\A(inputs-semantic|inputs-image|inputs-test|front|native|check|image|test)-[0-9a-f]{64}-[0-9a-f]{64}\z/;
    return 'other';
}
opendir my $directory_handle, $directory or die "$directory: $!";
my @names = sort grep { $_ ne '.' && $_ ne '..' } readdir $directory_handle;
closedir $directory_handle;
my (%groups, %physical);
my ($files, $bytes, $header_bytes, $front_bytes) = (0, 0, 0, 0);
my (@snapshots, @nonregular);
my $manifest = Digest::SHA->new(256);
for my $name (@names) {
    my $path = "$directory/$name";
    my @stat = lstat($path); die "lstat $path: $!" unless @stat;
    if (!-f _ || -l _) {
        push @nonregular, {name=>$name, type=>(-d _ ? 'directory' : -l _ ? 'symlink' : 'other'), bytes=>$stat[7]};
        next;
    }
    my $size = $stat[7]; my $kind = family($name);
    $physical{$name} = $size;
    my $group = ($groups{$kind} //= {files=>0, stored_bytes=>0, fixed_header_bytes_read=>0,
        declared_front_or_payload_bytes=>0, framed_envelopes=>0});
    open my $file, '<:raw', $path or die "$path: $!";
    my $n = $size < 90 ? $size : 90;
    my $header = $n ? read_exact($file, $n) : '';
    $files++; $bytes += $size; $header_bytes += $n;
    $group->{files}++; $group->{stored_bytes} += $size; $group->{fixed_header_bytes_read} += $n;
    my $front = '';
    if (length($header) == 90 && substr($header, 0, 18) eq $magic) {
        my $length = unpack('Q<', substr($header, 18, 8));
        if ($kind eq 'semantic-container-v3') {
            die "container fixed front size\n" unless $length == 400 && $size >= 612;
            $front = read_exact($file, 400); $front_bytes += 400;
        } else {
            die "ordinary envelope physical length\n" unless $length + 90 == $size;
            if ($kind eq 'semantic-root') {
                die "v2 root size\n" unless $length == 376;
                $front = read_exact($file, 376); $front_bytes += 376;
            }
        }
        $group->{framed_envelopes}++; $group->{declared_front_or_payload_bytes} += $length;
    } elsif ($kind ne 'budget-lock' && $kind ne 'other') { die "unframed named envelope $path\n"; }
    close $file or die "$path: $!";
    if ($kind eq 'semantic-container-v3' || $kind eq 'semantic-root') {
        my ($slot, $id) = $name =~ /-([0-9a-f]{64})-([0-9a-f]{64})\z/;
        my $at = 0; my $version = word($front, \$at);
        die "snapshot version\n" unless $version == ($kind eq 'semantic-root' ? 2 : 3);
        my @values = map { field($front, \$at) } 1..5;
        die "snapshot fields\n" if grep { !hex_id($_) } @values;
        die "snapshot producer slot\n" unless sha256_hex(pack('H*', $values[0])) eq $slot;
        authenticate($header . $front, $values[0], $kind, $id, $version == 2 ? 376 : 400);
        die "snapshot identity\n" unless sha256_hex($front) eq $id;
        my $facts = word($front, \$at);
        my $snapshot = {version=>$version, identity=>$id, producer=>$values[0], source=>$values[1],
            checked_ir=>$values[2], root=>$values[3], selector=>$values[4], facts=>$facts,
            file=>$name, physical_bytes=>$size, fixed_front_authenticated=>JSON::PP::true,
            node_authentication=>'not-performed-by-census'};
        if ($version == 3) {
            my ($offset, $length, $total) = map { word($front, \$at) } 1..3;
            die "container extent\n" unless $total == $size && $total <= 9223372036854775807
                && $offset >= 490 && $offset <= $total && $length >= 122 && $length <= 32858
                && $length == $total - $offset;
            @$snapshot{qw(root_offset root_length total_bytes)} = ($offset, $length, $total);
        }
        die "snapshot trailing bytes\n" unless $at == length($front);
        my $locator = $version == 2 ? "semantic-locator-$id" : "semantic-locator-v3-$id";
        $snapshot->{locator_present} = (-f "$directory/$locator" && !-l "$directory/$locator") ? JSON::PP::true : JSON::PP::false;
        push @snapshots, $snapshot;
    }
    $manifest->add(frame($name), pack('Q<', $size), frame($header), frame($front));
}
my $budget = {status=>'absent'};
if (exists $physical{'.budget-index'}) {
    my $raw = file_bytes("$directory/.budget-index", $capacity);
    my $payload = authenticate($raw, sha256_hex('lkjscript-store-size-index-1'), 'size-index', '1', $capacity);
    my $at = 0; die "budget version\n" unless word($payload, \$at) == 2;
    my $n = word($payload, \$at); die "budget generation count\n" if $n > length($payload)/72;
    my (%generations, %entries); my @generation_order;
    for (1..$n) {
        my $generation = field($payload, \$at);
        die "budget generation\n" unless hex_id($generation) && !$generations{$generation}++;
        push @generation_order, $generation;
    }
    my $count = word($payload, \$at); die "budget entry count\n" if $count > length($payload)/24;
    my ($accounted, $missing, $mismatch) = (0, 0, 0);
    my %by_family;
    for (1..$count) {
        my $name = field($payload, \$at); my $generation = field($payload, \$at); my $length = word($payload, \$at);
        die "budget duplicate/generation/length\n" unless !$entries{$name}++ && $generations{$generation} && $length >= 90;
        $accounted += $length;
        $missing++ unless exists $physical{$name};
        $mismatch++ if exists($physical{$name}) && $physical{$name} != $length;
        my $g = ($by_family{family($name)} //= {entries=>0, bytes=>0});
        $g->{entries}++; $g->{bytes} += $length;
    }
    die "budget trailing bytes\n" unless $at == length($payload);
    my @unindexed = sort grep { $_ ne '.budget-index' && $_ ne '.budget-lock' && !exists($entries{$_}) } keys %physical;
    $budget = {status=>'authenticated', envelope_bytes=>length($raw), payload_bytes=>length($payload),
        envelope_sha256=>sha256_hex($raw), entries=>$count, generation_order=>\@generation_order,
        accounted_entry_bytes=>$accounted, accounted_total_bytes=>$accounted+length($raw),
        writer_capacity_bytes=>0+$capacity, within_capacity=>($accounted+length($raw)<=$capacity)?JSON::PP::true:JSON::PP::false,
        missing_entries=>$missing, physical_length_mismatches=>$mismatch, unindexed_regular_names=>\@unindexed,
        families=>\%by_family};
}
print JSON::PP->new->canonical->pretty->encode({
    measurement=>'untimed post-command metadata census; authenticates Budget and fixed snapshot fronts only',
    files=>$files, stored_bytes=>$bytes, fixed_header_bytes_read=>$header_bytes, fixed_snapshot_payload_bytes_read=>$front_bytes,
    inventory_header_manifest_sha256=>$manifest->hexdigest, namespaces=>\%groups,
    snapshots=>\@snapshots, nonregular=>\@nonregular, budget=>$budget
});
