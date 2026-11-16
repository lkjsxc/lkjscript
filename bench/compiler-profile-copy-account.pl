#!/usr/bin/env perl
# Prospective bench-only LKJPROF4 accounting. This does not check or execute IR.
use strict;
use warnings;
use Config;
use Digest::SHA qw(sha256_hex);
use Encode qw(decode FB_CROAK LEAVE_SRC);
use Fcntl qw(O_RDONLY O_WRONLY O_CREAT O_EXCL O_NOFOLLOW S_ISREG);
use JSON::PP;
use Math::BigInt;

sub refuse { die "E_COPY_ACCOUNT $_[0]\n"; }
$Config{ivsize} == 8 && $Config{uvsize} == 8
    or refuse("a Perl with exact 64-bit IV/UV support is required");
my $MAX = unpack('Q<', "\xff" x 8);
my $MAX_TEXT = '18446744073709551615';
my $TAG_MASK = unpack('Q<', pack('V2', 0x07ffffff, 0x00000007));
my $MAX_BIG = Math::BigInt->new($MAX_TEXT);
my $IV_MAX_TEXT = '9223372036854775807';
my $HEADER = 160;
my $SYMBOL_ROW = 64;
my $COPY_ROW = 72;
my $KIND_COUNT = 16;
my $DETAIL_LIMIT = 200;
my $DETAIL_BYTES = 1048576;
my $json = JSON::PP->new->canonical->ascii;
my $pretty = JSON::PP->new->canonical->ascii->pretty;
my @kind_names = (
    'unknown', 'inline copy', 'checks', 'call instruction', 'frame load/store',
    'other memory', 'arithmetic / comparison / conversion', 'branch', 'allocator',
    'runtime', 'register-to-register move', 'address formation (LEA)',
    'immediate constant', 'argument setup / result capture', 'prologue / epilogue',
    'return',
);
my @role_names = (
    'invalid', 'local', 'typed-place', 'constructor-field', 'array-element',
    'abi-incoming', 'parameter-local', 'abi-result', 'vector-element',
    'result-variant-field', 'repeated-array-element', 'box-payload',
    'referent', 'view-header',
);
my %args;
while (@ARGV) {
    my $flag = shift @ARGV;
    $flag =~ /\A--(profile|companion|summary|detail|output)\z/
        or refuse("unknown or positional argument");
    my $key = $1;
    !exists $args{$key} && @ARGV or refuse("duplicate option or missing value: $flag");
    $args{$key} = shift @ARGV;
    length($args{$key}) && index($args{$key}, "\0") < 0
        or refuse("empty or NUL path: $flag");
}
for my $key (qw(profile companion summary detail output)) {
    exists $args{$key} or refuse("missing --$key");
}
# An exclusive claim makes a failed derivation visible; it is never auto-removed.
mkdir($args{output}, 0700) or refuse("output directory is not absent/creatable: $!");
my %inputs;

# Decimal strings are deliberately fresh PV values for JSON, never floating
# numbers. All decoded words are native UVs; only bounded offsets use arithmetic.
# Aggregated counters use BigInt, including overlapping inclusive totals.
sub dec { return join('', $_[0]); }
sub big { return Math::BigInt->new(dec($_[0])); }
sub canonical_u64 {
    my ($text, $label) = @_;
    defined($text) && $text =~ /\A(?:0|[1-9][0-9]*)\z/
        && (length($text) < 20 || (length($text) == 20 && $text le $MAX_TEXT))
        or refuse("noncanonical/out-of-range integer: $label");
    return $text;
}
sub add64 {
    my ($sum, $value, $label) = @_;
    $sum->badd(dec($value));
    $sum->bcmp($MAX_BIG) <= 0 or refuse("u64 counter overflow: $label");
}
sub regular_open {
    my ($path) = @_;
    sysopen(my $fh, $path, O_RDONLY | O_NOFOLLOW)
        or refuse("cannot open regular input $path: $!");
    binmode($fh);
    my @st = stat($fh);
    @st && S_ISREG($st[2]) or refuse("input is not a regular file: $path");
    my $size_text = dec($st[7]);
    canonical_u64($size_text, 'input size');
    big($size_text)->bcmp($IV_MAX_TEXT) <= 0
        or refuse("input size exceeds signed file representation: $path");
    return ($fh, \@st);
}
sub read_raw {
    my ($key, $path) = @_;
    my ($fh, $before) = regular_open($path);
    my $bytes = '';
    while (1) {
        my $n = read($fh, my $part, 1048576);
        defined($n) or refuse("input read failed: $path: $!");
        last if $n == 0;
        $bytes .= $part;
    }
    my @after = stat($fh);
    close($fh) or refuse("input close failed: $path: $!");
    @after && $before->[0] == $after[0] && $before->[1] == $after[1]
        && $before->[7] == $after[7] && length($bytes) == $before->[7]
        or refuse("input changed length/identity during read: $path");
    $inputs{$key} = {
        path => $path, bytes => dec(length($bytes)),
        sha256 => sha256_hex($bytes),
    };
    return $bytes;
}
sub hash_regular {
    my ($key, $path) = @_;
    my ($fh, $before) = regular_open($path);
    my $sha = Digest::SHA->new(256);
    my $length = Math::BigInt->new(0);
    while (1) {
        my $n = read($fh, my $part, 1048576);
        defined($n) or refuse("input read failed: $path: $!");
        last if $n == 0;
        $sha->add($part);
        $length->badd($n);
    }
    my @after = stat($fh);
    close($fh) or refuse("input close failed: $path: $!");
    @after && $before->[0] == $after[0] && $before->[1] == $after[1]
        && $before->[7] == $after[7] && $length->bcmp(dec($before->[7])) == 0
        or refuse("input changed length/identity during read: $path");
    $inputs{$key} = {
        path => $path, bytes => dec($length->bstr), sha256 => $sha->hexdigest,
    };
}
sub utf8 {
    my ($bytes, $label) = @_;
    my $text = eval { decode('UTF-8', $bytes, FB_CROAK | LEAVE_SRC) };
    defined($text) && !$@ or refuse("invalid UTF-8: $label");
    return $text;
}
sub lines {
    my ($bytes, $label) = @_;
    $bytes =~ /\n\z/ or refuse("missing final LF: $label");
    my $text = utf8($bytes, $label);
    my @lines = split /\n/, $text, -1;
    pop @lines;
    return \@lines;
}

my $profile = read_raw('profile', $args{profile});
hash_regular('companion', $args{companion});
my $summary_raw = read_raw('summary', $args{summary});
my $detail_raw = read_raw('detail', $args{detail});
hash_regular('helper', $0);
my $length = length($profile);
sub word {
    my ($at) = @_;
    $at >= 0 && $at <= $length && $length - $at >= 8
        or refuse("word is outside the profile");
    return unpack('Q<', substr($profile, $at, 8));
}
sub extent {
    my ($start, $count, $width, $label) = @_;
    $width > 0 && $start <= $length
        or refuse("invalid section start/width: $label");
    # All three operands in this division are <= signed file length. Integer
    # division is exact; count is checked before multiplication or conversion.
    my $available;
    { use integer; $available = ($length - $start) / $width; }
    $count <= $available or refuse("section exceeds file: $label");
    return $start + $count * $width;
}
$length >= $HEADER && substr($profile, 0, 8) eq 'LKJPROF4'
    or refuse("not a complete LKJPROF4 header");
word(16) == 1000000 && word(72) == 0 && word(152) == 0
    or refuse("period or reserved header fields differ");
unpack('H*', substr($profile, 80, 32)) eq $inputs{companion}{sha256}
    or refuse("exact selected-IR companion digest differs");

my $symbol_count = word(8);
my $serial = word(24);
my $kind_offset = word(32);
my $kind_count = word(40);
my $counter_offset = word(48);
my $code_start = word(56);
my $code_end = word(64);
my $copy_offset = word(112);
my $copy_count = word(120);
my $map_offset = word(128);
my $map_count = word(136);
my $unmapped = word(144);
$code_start < $code_end or refuse("invalid code range");

my $symbols_end = extent($HEADER, $symbol_count, $SYMBOL_ROW, 'symbols');
my $name_at = $symbols_end;
my @symbols;
my $prior_end = $code_start;
my $self_total = Math::BigInt->new(0);
my $inclusive_total = Math::BigInt->new(0);
for (my $index = 0; $index < $symbol_count; $index++) {
    my $at = $HEADER + $index * $SYMBOL_ROW;
    my @row = map { word($at + $_ * 8) } 0..7;
    my ($start, $end, $self, $inclusive, $seen, $name, $name_length, $frame) = @row;
    $start >= $code_start && $start < $end && $end <= $code_end
        && ($index == 0 || $start == $prior_end) && $name == $name_at
        && ($frame == 0 || ($frame >= $start && $frame <= $end))
        && $self <= $serial && $inclusive <= $serial && $seen <= $serial
        or refuse("invalid symbol row $index");
    my $next = extent($name, $name_length, 1, "symbol name $index");
    my $text = utf8(substr($profile, $name, $name_length), "symbol name $index");
    add64($self_total, $self, 'function self total');
    $inclusive_total->badd(dec($inclusive));
    push @symbols, {
        start => $start, end => $end, self => $self, inclusive => $inclusive,
        last_seen_serial => $seen, frame => $frame, name => $text,
    };
    $name_at = $next;
    $prior_end = $end;
}
($symbol_count == 0 || $prior_end == $code_end)
    && $kind_offset == $name_at && $kind_count > 0
    or refuse("symbol/name/kind framing differs");
my $kind_end = extent($kind_offset, $kind_count, 16, 'kind intervals');
$counter_offset == $kind_end or refuse("kind counter offset differs");
my $counter_end = extent($counter_offset, $KIND_COUNT, 8, 'kind counters');
my $previous_kind_start = 0;
my @interval_counts = (0) x $KIND_COUNT;
for (my $index = 0; $index < $kind_count; $index++) {
    my $at = $kind_offset + $index * 16;
    my ($start, $kind) = (word($at), word($at + 8));
    $kind < $KIND_COUNT && $start >= $code_start && $start < $code_end
        && ($index != 0 || $start == $code_start)
        && ($index == 0 || $start > $previous_kind_start)
        or refuse("invalid kind interval $index");
    $interval_counts[$kind]++;
    $previous_kind_start = $start;
}
my @counts;
my $total = Math::BigInt->new(0);
for my $kind (0..$KIND_COUNT-1) {
    my $count = word($counter_offset + $kind * 8);
    add64($total, $count, 'completed kind total');
    push @counts, $count;
}
my $serial_gap = big($serial)->bsub($total);
($serial_gap->is_zero || $serial_gap->is_one)
    && $self_total->bcmp($total) <= 0
    or refuse("serial/completed/function-self counters disagree");
$copy_offset == $counter_end or refuse("copy section offset differs");
my $copy_end = extent($copy_offset, $copy_count, $COPY_ROW, 'copy sites');
$map_offset == $copy_end or refuse("function-map offset differs");
extent($map_offset, $map_count, 16, 'function map') == $length
    or refuse("function map does not end exactly at EOF");
my @function_map;
my %selected_for_symbol;
for (my $index = 0; $index < $map_count; $index++) {
    my $at = $map_offset + $index * 16;
    my ($selected, $symbol) = (word($at), word($at + 8));
    $selected == $index or refuse("function map is not canonical at $index");
    if ($symbol != $MAX) {
        $symbol < $symbol_count && !exists $selected_for_symbol{dec($symbol)}
            or refuse("function map aliases/out-of-range symbol at $index");
        $selected_for_symbol{dec($symbol)} = $index;
    }
    push @function_map, $symbol;
}

# Parse the exact official first page, keeping independent admission explicitly
# delegated to its checked-ir result. This helper does not parse the companion.
my $detail_lines = lines($detail_raw, 'official detail');
length($detail_raw) <= $DETAIL_BYTES && @$detail_lines >= 3
    or refuse("official detail exceeds fixed bound or has no framing");
my $header = shift @$detail_lines;
my @header_fields = $header =~ /\Acopy-sites version=4 profile=([0-9a-f]{64}) companion=([0-9a-f]{64}) companion-check=checked-ir serial=([0-9]+) total-samples=([0-9]+) copy-samples=([0-9]+) mapped=([0-9]+) unmapped=([0-9]+) in-flight=([0-9]+) sites=([0-9]+) order=samples-desc\/function\/instruction\/subsite\/start\/end\z/;
@header_fields == 9 or refuse("official detail header differs");
for my $at (2..8) { canonical_u64($header_fields[$at], 'detail header'); }
$header_fields[0] eq $inputs{profile}{sha256}
    && $header_fields[1] eq $inputs{companion}{sha256}
    or refuse("official detail does not bind these exact bytes");
my $scope = shift @$detail_lines;
$scope eq 'scope=existing-kind-copy runtime=excluded variable-width-copy=excluded replace=excluded scalar-frame-address-work=excluded counters=sampled-native-observation'
    or refuse("official detail scope differs");
my $tail = pop @$detail_lines;
my @tail_fields = $tail =~ /\Acontinuation cursor=(none|4:[0-9a-f]{64}:[0-9a-f]{64}:[0-9]+) rows=([0-9]+) next=([0-9]+) total-sites=([0-9]+)\z/;
@tail_fields == 4 or refuse("official detail continuation differs");
for my $at (1..3) { canonical_u64($tail_fields[$at], 'detail continuation'); }
my $visible_count = scalar @$detail_lines;
$visible_count <= $DETAIL_LIMIT
    && $tail_fields[1] eq dec($visible_count)
    && $tail_fields[2] eq dec($visible_count)
    && $tail_fields[3] eq dec($copy_count)
    && ($copy_count == 0 ? $visible_count == 0 : $visible_count > 0)
    && $visible_count <= $copy_count
    or refuse("official detail is not the bounded first page");
my $expected_cursor = $visible_count == $copy_count ? 'none'
    : join(':', '4', $inputs{profile}{sha256}, $inputs{companion}{sha256}, $visible_count);
$tail_fields[0] eq $expected_cursor or refuse("official cursor bytes differ");
my @visible;
my %visible_start;
my $quoted = qr/"(?:[^"\\\x00-\x1f]|\\(?:["\\\/bfnrt]|u[0-9a-fA-F]{4}))*"/;
for my $line (@$detail_lines) {
    my @fields = $line =~ /\Acopy function=([0-9]+) instruction=(parameter|[0-9]+) subsite=([0-9]+) symbol=([0-9]+) samples=([0-9]+) type=([0-9]+) width=([0-9]+) source=([a-z-]+) destination=([a-z-]+) source-flags=([0-9]+) destination-flags=([0-9]+) start=([0-9]+) end=([0-9]+) name=($quoted) type-name=($quoted)\z/;
    @fields == 15 or refuse("malformed official copy row");
    for my $at (0, 2, 3, 4, 5, 6, 9, 10, 11, 12) {
        canonical_u64($fields[$at], 'official copy row');
    }
    canonical_u64($fields[1], 'official instruction') if $fields[1] ne 'parameter';
    for my $at (13, 14) {
        my $value = eval { $json->decode($fields[$at]) };
        !$@ && defined($value) && !ref($value)
            or refuse("invalid official quoted name");
        $fields[$at] = $value;
    }
    !exists $visible_start{$fields[11]} or refuse("duplicate official copy row");
    $visible_start{$fields[11]} = scalar @visible;
    push @visible, { fields => \@fields };
}

sub copy_tags {
    my ($tags) = @_;
    ($tags & ~$TAG_MASK) == 0 or refuse("unknown copy tag bits");
    my $origin = $tags & 255;
    my $source = ($tags >> 8) & 255;
    my $destination = ($tags >> 16) & 255;
    my $source_flags = ($tags >> 24) & 7;
    my $destination_flags = ($tags >> 32) & 7;
    ($source == 2 || $source_flags == 0)
        && ($destination == 2 || $destination_flags == 0)
        or refuse("flags on a non-place role");
    my $valid = $origin == 2 ? ($source == 5 && $destination == 6)
        : $origin == 1 && (
            ($source == 1 && ($destination == 1 || $destination == 2
                || $destination == 3 || $destination == 4 || $destination == 7
                || $destination == 8 || $destination == 10 || $destination == 11))
            || ($source == 2 && $destination == 1)
            || ($source == 8 && $destination == 9)
            || ($source == 12 && $destination == 1)
            || ($source == 13 && $destination == 13)
        );
    $valid or refuse("unknown origin/role pair");
    return ($origin, $source, $destination, $source_flags, $destination_flags);
}
sub bucket {
    my ($map, $key, $fields, $samples) = @_;
    my $value = $map->{$key};
    if (!defined($value)) {
        $value = { fields => $fields, sites => 0, nonzero => 0, samples => Math::BigInt->new(0) };
        $map->{$key} = $value;
    }
    $value->{sites}++;
    $value->{nonzero}++ if $samples != 0;
    add64($value->{samples}, $samples, 'bucket samples');
}
sub copy_tuple {
    my ($index) = @_;
    my $at = $copy_offset + $index * $COPY_ROW;
    return [word($at + 16), word($at + 24), word($at + 32),
        word($at + 40), word($at), word($at + 8)];
}
sub rank_compare {
    my ($left, $right) = @_;
    my $order = $right->[0] <=> $left->[0];
    return $order if $order;
    for my $at (1..5) {
        $order = $left->[$at] <=> $right->[$at];
        return $order if $order;
    }
    return 0;
}
my (%widths, %roles, %width_roles);
my $mapped = Math::BigInt->new(0);
my $nonzero_sites = 0;
my $prior_copy_end = $code_start;
my $kind_index = 0;
for (my $index = 0; $index < $copy_count; $index++) {
    my $at = $copy_offset + $index * $COPY_ROW;
    my @row = map { word($at + $_ * 8) } 0..8;
    my ($start, $end, $samples, $function, $instruction, $subsite, $ty, $width, $tags) = @row;
    $start >= $code_start && $start < $end && $end <= $code_end
        && $start >= $prior_copy_end && $function < $map_count && $width > 0
        or refuse("invalid copy coordinates at row $index");
    my ($origin, $source, $destination, $source_flags, $destination_flags) = copy_tags($tags);
    (($origin == 2 && $instruction == $MAX) || ($origin == 1 && $instruction != $MAX))
        or refuse("copy instruction/synthetic-origin mismatch at row $index");
    my $symbol = $function_map[$function];
    $symbol != $MAX && $start >= $symbols[$symbol]{start} && $end <= $symbols[$symbol]{end}
        or refuse("copy range crosses its selected function at row $index");
    while ($kind_index + 1 < $kind_count
        && word($kind_offset + ($kind_index + 1) * 16) <= $start) { $kind_index++; }
    word($kind_offset + $kind_index * 16 + 8) == 1
        && ($kind_index + 1 == $kind_count
            || word($kind_offset + ($kind_index + 1) * 16) >= $end)
        or refuse("copy range crosses its machine kind at row $index");
    add64($mapped, $samples, 'mapped copy samples');
    $nonzero_sites++ if $samples != 0;
    my @role_fields = (
        $origin == 1 ? 'body' : 'incoming-parameter',
        $role_names[$source], $role_names[$destination],
        dec($source_flags), dec($destination_flags),
    );
    my $role_key = join("\t", @role_fields);
    bucket(\%widths, dec($width), [dec($width)], $samples);
    bucket(\%roles, $role_key, \@role_fields, $samples);
    bucket(\%width_roles, join("\t", dec($width), @role_fields),
        [dec($width), @role_fields], $samples);
    if (exists $visible_start{dec($start)}) {
        my $visible = $visible[$visible_start{dec($start)}];
        my $fields = $visible->{fields};
        my @expected = (
            dec($function), $instruction == $MAX ? 'parameter' : dec($instruction),
            dec($subsite), dec($symbol), dec($samples), dec($ty), dec($width),
            $role_names[$source], $role_names[$destination],
            dec($source_flags), dec($destination_flags), dec($start), dec($end),
        );
        for my $field (0..12) {
            $fields->[$field] eq $expected[$field]
                or refuse("official copy row disagrees at field $field");
        }
        # The official IR checker proves the complete names and type semantics.
        # The function name can additionally be tied to this exact raw symbol.
        my $symbol_name = $symbols[$symbol]{name};
        $symbol_name =~ /\A(.*) \[file [0-9]+:[0-9]+\]\z/s
            && $fields->[13] eq $1
            or refuse("official copy function name differs from raw symbol");
        $visible->{tuple} = [$samples, $function, $instruction, $subsite, $start, $end];
        $visible->{raw_index} = $index;
    }
    $prior_copy_end = $end;
}
my $detail_total = $mapped->copy;
add64($detail_total, $unmapped, 'mapped plus unmapped');
my $in_flight = $detail_total->copy->bsub(dec($counts[1]));
($in_flight->is_zero || $in_flight->is_one)
    && ($in_flight->is_zero || $serial_gap->is_one)
    or refuse("copy detail/kind/in-flight counters disagree");
my @expected_header = (
    dec($serial), dec($total->bstr), dec($counts[1]), dec($mapped->bstr),
    dec($unmapped), dec($in_flight->bstr), dec($copy_count),
);
for my $at (0..6) {
    $header_fields[$at + 2] eq $expected_header[$at]
        or refuse("official detail accounting differs at field $at");
}
for my $index (0..$#visible) {
    exists $visible[$index]{tuple} or refuse("official copy row is absent from raw table");
    ($index == 0 || rank_compare($visible[$index - 1]{tuple}, $visible[$index]{tuple}) < 0)
        or refuse("official detail order differs");
}
if ($visible_count) {
    my $last = $visible[-1]{tuple};
    my $earlier = 0;
    for (my $index = 0; $index < $copy_count; $index++) {
        $earlier++ if rank_compare(copy_tuple($index), $last) < 0;
    }
    $earlier == $visible_count - 1
        or refuse("official bounded rows are not the exact deterministic prefix");
}

# Cross-check every integer in the ordinary report; percentages are explicitly
# display-only f64 truncations and are not sources of accounting.
sub display_name {
    my ($name) = @_;
    $name =~ s/([\x00-\x1f\x7f\x{2028}\x{2029}])/sprintf('\\u{%x}', ord($1))/ge;
    return $name;
}
sub report_percent {
    my ($value) = @_;
    $value =~ /\A(?:[0-9]|[1-9][0-9]|100)\.[0-9]\z/
        && ($value !~ /\A100\./ || $value eq '100.0')
        or refuse("malformed displayed percentage");
}
my $summary_lines = lines($summary_raw, 'official summary');
@$summary_lines <= 37 or refuse("ordinary summary exceeds its format");
my $summary_header = shift @$summary_lines;
$summary_header eq 'CPU samples: ' . dec($total->bstr)
        . '; requested period 1000 us; inclusive counts each function once per sample'
    or refuse("official summary total/period differs");
for my $mode (qw(self inclusive)) {
    my $title = shift @$summary_lines;
    my $columns = shift @$summary_lines;
    defined($title) && $title eq ($mode eq 'self' ? 'Top self time' : 'Top inclusive time')
        && defined($columns) && $columns eq 'self% incl% samples function'
        or refuse("official function summary headers differ");
    my @order = sort {
        $symbols[$b]{$mode} <=> $symbols[$a]{$mode} || $a <=> $b
    } grep { $symbols[$_]{$mode} != 0 } 0..$#symbols;
    splice(@order, 7) if @order > 7;
    for my $index (@order) {
        my $line = shift @$summary_lines;
        defined($line) && $line =~ /\A([0-9]+\.[0-9]) ([0-9]+\.[0-9]) ([0-9]+) (.*)\z/
            or refuse("malformed official function summary row");
        my ($self_percent, $inclusive_percent, $count, $name) = ($1, $2, $3, $4);
        report_percent($self_percent); report_percent($inclusive_percent);
        canonical_u64($count, 'official function count');
        $count eq dec($symbols[$index]{$mode}) && $name eq display_name($symbols[$index]{name})
            or refuse("official function marginal differs");
    }
}
my $kind_title = shift @$summary_lines;
my $kind_columns = shift @$summary_lines;
defined($kind_title) && $kind_title eq 'Emitted instruction kinds (self samples)'
    && defined($kind_columns) && $kind_columns eq 'kind self% samples class'
    or refuse("official kind summary headers differ");
for my $kind (0..$KIND_COUNT-1) {
    my $line = shift @$summary_lines;
    defined($line) && $line =~ /\Akind ([0-9]+\.[0-9]) ([0-9]+) (.*)\z/
        or refuse("malformed official kind row");
    my ($percent, $count, $name) = ($1, $2, $3);
    report_percent($percent); canonical_u64($count, 'official kind count');
    $count eq dec($counts[$kind]) && $name eq $kind_names[$kind]
        or refuse("official kind marginal differs");
}
!@$summary_lines or refuse("extra official summary content");

# These are independent coverage invariants for every emitted aggregation.
for my $map (\%widths, \%roles, \%width_roles) {
    my $sites = Math::BigInt->new(0);
    my $nonzero = Math::BigInt->new(0);
    my $samples = Math::BigInt->new(0);
    for my $row (values %$map) {
        $sites->badd($row->{sites});
        $nonzero->badd($row->{nonzero});
        add64($samples, $row->{samples}->bstr, 'bucket coverage');
    }
    $sites->bcmp(dec($copy_count)) == 0 && $nonzero->bcmp(dec($nonzero_sites)) == 0
        && $samples->bcmp($mapped) == 0
        or refuse("bucket coverage differs from the complete raw table");
}

my %output_hashes;
sub write_file {
    my ($name, $bytes) = @_;
    my $path = "$args{output}/$name";
    sysopen(my $fh, $path, O_WRONLY | O_CREAT | O_EXCL, 0600)
        or refuse("cannot exclusively create output $name: $!");
    binmode($fh);
    print {$fh} $bytes or refuse("cannot write output $name: $!");
    close($fh) or refuse("cannot close output $name: $!");
    $output_hashes{$name} = { sha256 => sha256_hex($bytes), bytes => dec(length($bytes)) };
}
sub tsv {
    my ($fields) = @_;
    return join("\t", @$fields) . "\n";
}
my $kinds_tsv = "kind\tname_json\tsamples\tintervals\n";
for my $kind (0..$KIND_COUNT-1) {
    $kinds_tsv .= tsv([dec($kind), $json->encode($kind_names[$kind]),
        dec($counts[$kind]), dec($interval_counts[$kind])]);
}
write_file('kinds.tsv', $kinds_tsv);
my $functions_tsv = "symbol\tstart\tend\tself_samples\tinclusive_samples\tlast_seen_serial\tframe_start\tselected_function\tname_json\n";
for (my $index = 0; $index < $symbol_count; $index++) {
    my $row = $symbols[$index];
    my $selected = exists $selected_for_symbol{dec($index)}
        ? dec($selected_for_symbol{dec($index)}) : 'none';
    $functions_tsv .= tsv([dec($index), (map { dec($row->{$_}) }
        qw(start end self inclusive last_seen_serial frame)), $selected, $json->encode($row->{name})]);
}
write_file('functions.tsv', $functions_tsv);
my $map_tsv = "selected_function\tsymbol\tencoded_symbol_u64\temitted\n";
for (my $index = 0; $index < $map_count; $index++) {
    my $symbol = $function_map[$index];
    $map_tsv .= tsv([dec($index), $symbol == $MAX ? 'none' : dec($symbol),
        dec($symbol), $symbol == $MAX ? 'no' : 'yes']);
}
write_file('function-map.tsv', $map_tsv);
sub decimal_order {
    return length($_[0]) <=> length($_[1]) || $_[0] cmp $_[1];
}
my $width_tsv = "width_bytes\tsites\tnonzero_sites\tsamples\n";
for my $key (sort { decimal_order($a, $b) } keys %widths) {
    my $row = $widths{$key};
    $width_tsv .= tsv([@{$row->{fields}}, dec($row->{sites}),
        dec($row->{nonzero}), $row->{samples}->bstr]);
}
$width_tsv .= tsv(['unmapped', 'NA', 'NA', dec($unmapped)]);
write_file('widths.tsv', $width_tsv);
my $role_tsv = "origin\tsource\tdestination\tsource_flags\tdestination_flags\tsites\tnonzero_sites\tsamples\n";
for my $key (sort keys %roles) {
    my $row = $roles{$key};
    $role_tsv .= tsv([@{$row->{fields}}, dec($row->{sites}),
        dec($row->{nonzero}), $row->{samples}->bstr]);
}
$role_tsv .= tsv(['unmapped', 'unknown', 'unknown', 'NA', 'NA', 'NA', 'NA', dec($unmapped)]);
write_file('roles.tsv', $role_tsv);
my $joint_tsv = "width_bytes\torigin\tsource\tdestination\tsource_flags\tdestination_flags\tsites\tnonzero_sites\tsamples\n";
for my $key (sort {
    my ($wa, $ra) = split /\t/, $a, 2;
    my ($wb, $rb) = split /\t/, $b, 2;
    decimal_order($wa, $wb) || $ra cmp $rb
} keys %width_roles) {
    my $row = $width_roles{$key};
    $joint_tsv .= tsv([@{$row->{fields}}, dec($row->{sites}),
        dec($row->{nonzero}), $row->{samples}->bstr]);
}
$joint_tsv .= tsv(['unmapped', 'unknown', 'unknown', 'unknown', 'NA', 'NA', 'NA', 'NA', dec($unmapped)]);
write_file('width-roles.tsv', $joint_tsv);

my $increment = big($copy_count)->bmul($COPY_ROW)->badd(big($map_count)->bmul(16))->badd(80);
my $accounting = {
    schema => 'lkjscript-copy-site-accounting-1',
    format => 'LKJPROF4',
    complete => JSON::PP::true,
    integers => 'all observed integer values are exact decimal strings',
    inputs => \%inputs,
    perl => { version => sprintf('%vd', $^V), iv_bytes => dec($Config{ivsize}), uv_bytes => dec($Config{uvsize}),
        bigint_version => dec($Math::BigInt::VERSION) },
    counters => {
        serial_S => dec($serial), completed_T => dec($total->bstr),
        copy_kind_K => dec($counts[1]), mapped => dec($mapped->bstr),
        unmapped => dec($unmapped), detail_D => dec($detail_total->bstr),
        serial_minus_completed => dec($serial_gap->bstr), detail_minus_copy_in_flight => dec($in_flight->bstr),
        function_self_total => dec($self_total->bstr),
        function_unattributed => dec($total->copy->bsub($self_total)->bstr),
        overlapping_inclusive_sum_not_total => dec($inclusive_total->bstr),
        in_flight_policy => 'no bucket or row was decremented; the possible detail-leading sample is not located',
    },
    tables => {
        file_bytes => dec($length), header_bytes => dec($HEADER),
        symbol_count => dec($symbol_count), symbol_row_bytes => dec($SYMBOL_ROW),
        symbols_offset => dec($HEADER), symbols_bytes => dec($symbols_end - $HEADER),
        names_offset => dec($symbols_end), names_bytes => dec($kind_offset - $symbols_end),
        kind_offset => dec($kind_offset), kind_count => dec($kind_count), kind_row_bytes => '16',
        kind_bytes => dec($kind_end - $kind_offset),
        counters_offset => dec($counter_offset), counters_count => dec($KIND_COUNT),
        counters_bytes => dec($counter_end - $counter_offset),
        copy_offset => dec($copy_offset), C => dec($copy_count), nonzero_copy_sites => dec($nonzero_sites),
        copy_row_bytes => dec($COPY_ROW), copy_bytes => dec($copy_end - $copy_offset),
        map_offset => dec($map_offset), F => dec($map_count), map_row_bytes => '16',
        map_bytes => dec($length - $map_offset),
        encoded_increment_over_otherwise_equal_v3 => dec($increment->bstr), alignment_bytes => '0',
        increment_is_a_format_calculation_not_an_observed_old_new_delta => JSON::PP::true,
        code_start => dec($code_start), code_end => dec($code_end),
    },
    official => {
        summary_integer_marginals_match => JSON::PP::true,
        summary_percentages => 'syntax/range checked; displayed f64 truncations are not accounting inputs',
        detail_claim => 'checked-ir',
        detail_profile_and_companion_digests_match => JSON::PP::true,
        detail_accounting_matches => JSON::PP::true,
        detail_exact_first_page_prefix_matches => JSON::PP::true,
        detail_rows => dec($visible_count), detail_limit => dec($DETAIL_LIMIT),
        detail_max_bytes => dec($DETAIL_BYTES), continuation => $tail_fields[0],
        type_names_and_operand_layouts => 'checked only by the recorded official detail command; not rechecked by this helper',
        command_success_and_capture_completeness => 'separate launcher records; stdout alone is not an execution attestation',
    },
    coverage => {
        kind_marginals => 'all 16, including unknown',
        function_marginals => 'all symbol rows including zero counters; inclusive rows overlap',
        function_map => 'all selected slots, with explicit unemitted sentinel',
        buckets => 'all raw copy rows, exact widths and origin/roles/flags, including zero-count sites; unmapped kept separate',
        bucket_sample_denominator => 'D (mapped plus unmapped), which can be K+1 for one interrupted handler',
        semantic_eligibility => 'not assessed; no SROA, escape, ownership, traffic, removable-time or causal claims',
        retained_input_semantics => 'hashes bind bytes read; no freshness or kernel-sealing claim',
    },
    outputs => \%output_hashes,
};
write_file('accounting.json', $pretty->encode($accounting));
my $manifest = '';
for my $name (sort keys %output_hashes) {
    $manifest .= "$output_hashes{$name}{sha256}  $name\n";
}
write_file('manifest.sha256', $manifest);
print $json->encode({
    status => 'ok', schema => 'lkjscript-copy-site-accounting-1',
    output => $args{output}, manifest_sha256 => $output_hashes{'manifest.sha256'}{sha256},
    profile_sha256 => $inputs{profile}{sha256}, companion_sha256 => $inputs{companion}{sha256},
    C => dec($copy_count), F => dec($map_count), completed_T => dec($total->bstr),
    copy_kind_K => dec($counts[1]), mapped => dec($mapped->bstr), unmapped => dec($unmapped),
    in_flight => dec($in_flight->bstr),
}), "\n" or refuse("cannot write success output");
