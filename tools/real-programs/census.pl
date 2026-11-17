#!/usr/bin/env perl
use strict;
use warnings;
use utf8;
use Cwd qw(abs_path);
use Digest::SHA qw(sha256_hex);
use Encode qw(decode FB_CROAK LEAVE_SRC);
use File::Find qw(find);
use File::Spec;
use Getopt::Long qw(GetOptions);
use JSON::PP;

# This is an inventory of source tokens, not a Rust parser or a refusal census.
my @FEATURES = qw(macro_rules_definitions macro_invocations
  nonbuiltin_macro_invocations derive_attributes derived_traits test_attributes
  async await dyn unsafe extern);
my %KEYWORD = map { $_ => 1 } qw(async await dyn unsafe extern);
# Explicit heuristic: exclude these unqualified names, or std/core/alloc-qualified
# versions, from the nonbuiltin inventory. Keep every invocation in the full one.
my @STANDARD_MACROS = sort qw(addr_of addr_of_mut asm assert assert_eq assert_ne
  cfg column compile_error concat concat_bytes concat_idents dbg debug_assert debug_assert_eq
  debug_assert_ne env eprint eprintln file format format_args format_args_nl
  global_asm include include_bytes include_str is_aarch64_feature_detected
  is_arm_feature_detected is_loongarch_feature_detected is_mips_feature_detected
  is_mips64_feature_detected is_powerpc_feature_detected
  is_powerpc64_feature_detected is_riscv_feature_detected
  is_s390x_feature_detected is_x86_feature_detected line log_syntax matches
  module_path naked_asm offset_of option_env panic pin print println stringify thread_local
  todo trace_macros try unimplemented unreachable vec write writeln);
my %STANDARD_MACRO = map { $_ => 1 } @STANDARD_MACROS;

sub json { JSON::PP->new->canonical->pretty->ascii->encode($_[0]) }
sub fail { die "real-programs-census: $_[0]\n" }
sub physical_lines {
    my ($bytes) = @_;
    return 0 unless length $bytes;
    return scalar(() = $bytes =~ /\n/g) + (substr($bytes, -1) eq "\n" ? 0 : 1);
}

sub lex_source {
    my ($source) = @_;
    my ($i, $n, $masked) = (0, length($source), $source);
    my @tokens;
    my $mask = sub {
        my ($start, $end) = @_;
        my $blank = substr($source, $start, $end - $start);
        $blank =~ s/[^\r\n]/ /g;
        substr($masked, $start, $end - $start) = $blank;
        $i = $end;
    };
    while ($i < $n) {
        pos($source) = $i;
        if ($source =~ /\G\s+/gc) { $i = pos($source); next }
        if (substr($source, $i, 2) eq '//') {
            my $end = index($source, "\n", $i + 2);
            $mask->($i, $end < 0 ? $n : $end);
            next;
        }
        if (substr($source, $i, 2) eq '/*') {
            my ($start, $depth) = ($i, 1);
            pos($source) = $i + 2;
            while ($source =~ m{(/\*|\*/)}g) {
                $depth += $1 eq '/*' ? 1 : -1;
                last unless $depth;
            }
            fail("unterminated block comment at character $start") if $depth;
            $mask->($start, pos($source));
            next;
        }
        pos($source) = $i;
        if ($source =~ /\G(?:br|cr|r)(\#*)"/gc) {
            my ($start, $hashes, $body) = ($i, $1, pos($source));
            my $close = '"' . $hashes;
            my $end = index($source, $close, $body);
            fail("unterminated raw string at character $start") if $end < 0;
            $mask->($start, $end + length($close));
            next;
        }
        pos($source) = $i;
        if ($source =~ /\G(?:b|c)?"/gc) {
            my ($start, $closed) = ($i, 0);
            while ($source =~ /["\\]/g) {
                if ($& eq '"') { $closed = 1; last }
                # Escaped quotes, backslashes and escaped newlines cannot close it.
                pos($source) = pos($source) + 1 if pos($source) < $n;
            }
            fail("unterminated string at character $start") unless $closed;
            $mask->($start, pos($source));
            next;
        }
        pos($source) = $i;
        if ($source =~ /\G(?:b)?'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]+\}|[^\r\n])|[^\\'\r\n])'/gc) {
            $mask->($i, pos($source));
            next;
        }
        pos($source) = $i;
        if ($source =~ /\G'(?:r\#)?[_\p{XID_Start}]\p{XID_Continue}*/gc) {
            push @tokens, { text => $&, kind => 'lifetime', at => $i };
            $i = pos($source);
            next;
        }
        pos($source) = $i;
        if ($source =~ /\G(?:0x[0-9a-fA-F_]*|0o[0-7_]*|0b[01_]*|[0-9][0-9_]*(?:\.(?![._\p{XID_Start}])[0-9_]*)?(?:[eE][+-]?[0-9_]+)?)(?:[_\p{XID_Start}]\p{XID_Continue}*)?/gc) {
            # Macro input can contain arbitrary literal suffixes, e.g. 1async.
            # A suffix is part of its literal, not a keyword token.
            $i = pos($source);
            next;
        }
        pos($source) = $i;
        if ($source =~ /\G(r\#)?([_\p{XID_Start}]\p{XID_Continue}*)/gc) {
            push @tokens, { text => $&, value => $2, raw => defined($1) ? 1 : 0,
                            kind => 'ident', at => $i };
            $i = pos($source);
            next;
        }
        my $punct = substr($source, $i, 2) eq '::' ? '::' : substr($source, $i, 1);
        push @tokens, { text => $punct, kind => 'punct', at => $i };
        $i += length $punct;
    }
    return ($masked, \@tokens);
}

sub path_at {
    my ($t, $start, $end) = @_;
    my $i = $start;
    my $leading = $i < $end && $t->[$i]{text} eq '::' ? '::' : '';
    ++$i if length $leading;
    return unless $i < $end && $t->[$i]{kind} eq 'ident';
    my (@text, @normal);
    while ($i < $end && $t->[$i]{kind} eq 'ident') {
        push @text, $t->[$i]{text};
        push @normal, $t->[$i]{value};
        ++$i;
        last unless $i + 1 < $end && $t->[$i]{text} eq '::'
                    && $t->[$i + 1]{kind} eq 'ident';
        ++$i;
    }
    return ($leading . join('::', @text), $leading . join('::', @normal), $i);
}

sub close_at {
    my ($t, $start, $end) = @_;
    my %close = ('(' => ')', '[' => ']', '{' => '}');
    return unless exists $close{$t->[$start]{text}};
    my @stack;
    for my $i ($start .. $end - 1) {
        my $s = $t->[$i]{text};
        if (exists $close{$s}) { push @stack, $close{$s} }
        elsif ($s eq ')' || $s eq ']' || $s eq '}') {
            return unless @stack && pop(@stack) eq $s;
            return $i unless @stack;
        }
    }
    return;
}

sub comma_parts {
    my ($t, $start, $end) = @_;
    my (@parts, $part);
    $part = $start;
    for (my $i = $start; $i < $end; ++$i) {
        if ($t->[$i]{text} =~ /\A[([{]\z/) {
            my $close = close_at($t, $i, $end);
            return () unless defined $close;
            $i = $close;
        } elsif ($t->[$i]{text} eq ',') {
            push @parts, [$part, $i] if $part < $i;
            $part = $i + 1;
        }
    }
    push @parts, [$part, $end] if $part < $end;
    return @parts;
}

sub inventory {
    my ($source) = @_;
    my ($masked, $t) = lex_source($source);
    my @starts = (0);
    push @starts, pos($source) while $source =~ /\n/g;
    my $location = sub {
        my ($at) = @_;
        my ($lo, $hi) = (0, scalar @starts);
        while ($lo + 1 < $hi) {
            my $mid = int(($lo + $hi) / 2);
            if ($starts[$mid] <= $at) { $lo = $mid } else { $hi = $mid }
        }
        return (line => $lo + 1, column => $at - $starts[$lo] + 1);
    };
    my %sites = map { $_ => [] } @FEATURES;
    my $add = sub {
        my ($kind, $at, %extra) = @_;
        push @{$sites{$kind}}, { $location->($at), %extra };
    };
    my $attribute;
    $attribute = sub {
        my ($start, $end, $conditional, $inner, $anchor) = @_;
        my ($path, $normal, $next) = path_at($t, $start, $end);
        return unless defined $path;
        my %extra = (path => $path, conditional => $conditional ? JSON::PP::true : JSON::PP::false,
                     inner => $inner ? JSON::PP::true : JSON::PP::false);
        if ($normal =~ /(?:\A|::)test\z/) {
            $add->('test_attributes', $anchor, %extra,
                   qualified => $normal eq 'test' ? JSON::PP::false : JSON::PP::true);
        }
        return unless $next < $end && $t->[$next]{text} eq '(';
        my $close = close_at($t, $next, $end);
        return unless defined $close;
        my @parts = comma_parts($t, $next + 1, $close);
        if ($normal eq 'derive') {
            $add->('derive_attributes', $anchor, %extra);
            for my $part (@parts) {
                my ($trait, undef, $after) = path_at($t, @$part);
                next unless defined($trait) && $after == $part->[1];
                $add->('derived_traits', $t->[$part->[0]]{at},
                       path => $trait, conditional => $extra{conditional});
            }
        } elsif ($normal eq 'cfg_attr') {
            shift @parts; # The first argument is a condition, never an attribute.
            for my $part (@parts) {
                $attribute->(@$part, 1, $inner, $t->[$part->[0]]{at});
            }
        }
    };
    for (my $i = 0; $i < @$t; ++$i) {
        my $token = $t->[$i];
        if ($token->{kind} eq 'ident' && !$token->{raw}) {
            $add->($token->{value}, $token->{at}) if $KEYWORD{$token->{value}};
            if ($token->{value} eq 'macro_rules' && $i + 1 < @$t
                && $t->[$i + 1]{text} eq '!') {
                my %name;
                if ($i + 2 < @$t && $t->[$i + 2]{kind} eq 'ident') {
                    $name{name} = $t->[$i + 2]{text};
                } elsif ($i + 3 < @$t && $t->[$i + 2]{text} eq '$'
                         && $t->[$i + 3]{kind} eq 'ident') {
                    $name{name} = '$' . $t->[$i + 3]{text};
                    $name{name_is_metavariable} = JSON::PP::true;
                }
                $add->('macro_rules_definitions', $token->{at}, %name);
            }
        }
        if ($token->{text} eq '#' && $i + 1 < @$t) {
            my $open = $i + 1;
            my $inner = $t->[$open]{text} eq '!' ? 1 : 0;
            ++$open if $inner;
            if ($open < @$t && $t->[$open]{text} eq '[') {
                my $close = close_at($t, $open, scalar @$t);
                $attribute->($open + 1, $close, 0, $inner, $token->{at}) if defined $close;
            }
        }
        next unless $token->{text} eq '!' && $i > 0 && $i + 1 < @$t
                 && $t->[$i + 1]{text} =~ /\A[([{]\z/ && $t->[$i - 1]{kind} eq 'ident';
        next if !$t->[$i - 1]{raw} && $t->[$i - 1]{value} eq 'macro_rules';
        my $start = $i - 1;
        $start -= 2 while $start >= 2 && $t->[$start - 1]{text} eq '::'
                             && $t->[$start - 2]{kind} eq 'ident';
        --$start if $start > 0 && ($t->[$start - 1]{text} eq '$' || $t->[$start - 1]{text} eq '::');
        my $path = join '', map { $_->{text} } @$t[$start .. $i - 1];
        my $normal = $path;
        $normal =~ s/(\A|::)r#/$1/g;
        my $base = $t->[$i - 1]{value};
        my $standard = $STANDARD_MACRO{$base}
            && ($normal eq $base || $normal =~ /\A(?:::)?(?:std|core|alloc)::/);
        $add->('macro_invocations', $t->[$start]{at}, path => $path,
               standard_name_match => $standard ? JSON::PP::true : JSON::PP::false);
        $add->('nonbuiltin_macro_invocations', $t->[$start]{at}, path => $path) unless $standard;
    }
    undef $attribute; # Break its recursive closure cycle for large file sets.
    return { map { $_ => { count => scalar @{$sites{$_}}, sites => $sites{$_} } } @FEATURES };
}

sub self_test {
    my $checks = 0;
    my $check = sub { ++$checks; fail("self-test: $_[1]") unless $_[0] };
    my $source = <<'RUST';
// unsafe async await dyn extern fake!() #[test]
/* unsafe /* async */ extern derive!(x) */
const A: &str = "escaped \" unsafe fake!() \\";
const B: &[u8] = b"async /* extern */";
const C: &str = r###" "## unsafe // fake!()
#[test] derive!(x) "###;
const D: &[u8] = br##"unsafe "# await"##;
const E: &CStr = cr#"extern unsafe"#;
const F: &CStr = c"unsafe";
let chars = ('\'', '\\', '\u{1f980}', '🦀', b'"', b'\x2f');
let r#unsafe = r#async;
let r#await = r#dyn;
let r#extern = 0;
let suffixes = (1async, 0x1unsafe, 2.0extern, 1e2dyn);
fn borrow<'unsafe>(x: &'unsafe str) {}
fn raw_lifetime<'r#async>(x: &'r#async str) {}
macro_rules! demo { () => { custom!(); } }
macro_rules! factory { ($name:ident) => { macro_rules! $name { () => {} } } }
vec![]; std::vec![]; project::vec![]; external::make!{}; $dynamic!();
#[cfg(test)]
#[test]
fn plain() {}
#[tokio::test]
async fn network() { job.await; }
#[cfg_attr(feature = "fake #[test]", test, derive(Debug, serde::Serialize))]
#[derive(Clone)]
struct Thing;
#[cfg_attr(x, cfg_attr(y, test))]
unsafe extern "C" { fn imported(); }
let object: &dyn Trait;
RUST
    my ($masked) = lex_source($source);
    $check->(length($masked) == length($source), 'mask preserves character offsets');
    $check->(join(',', map { $_->[0] } newline_positions($masked)) eq
             join(',', map { $_->[0] } newline_positions($source)), 'mask preserves every newline');
    my $features = inventory($source);
    my %expected = (macro_rules_definitions => 3, macro_invocations => 6,
        nonbuiltin_macro_invocations => 4, derive_attributes => 2, derived_traits => 3,
        test_attributes => 4, async => 1, await => 1, dyn => 1, unsafe => 1, extern => 1);
    for my $kind (@FEATURES) {
        $check->($features->{$kind}{count} == $expected{$kind}, "$kind count");
    }
    my $unsafe_at = index($source, 'unsafe extern');
    my $line = 1 + scalar(() = substr($source, 0, $unsafe_at) =~ /\n/g);
    $check->($features->{unsafe}{sites}[0]{line} == $line
             && $features->{unsafe}{sites}[0]{column} == 1, 'location after multiline literals');
    $check->(scalar(grep { $_->{conditional} } @{$features->{test_attributes}{sites}}) == 2,
             'nested cfg_attr test inventory');
    $check->(physical_lines('') == 0 && physical_lines("a\r\nb\n") == 2
             && physical_lines('a') == 1, 'physical line convention');
    for my $bad ('/* nested /* */', 'r##"missing"#', 'b"missing') {
        my $ok = eval { lex_source($bad); 1 };
        $check->(!$ok, 'unterminated lexical region rejected');
    }
    return { passed => JSON::PP::true, checks => $checks };
}

sub newline_positions {
    my ($s) = @_;
    my @positions;
    push @positions, [pos($s)] while $s =~ /\n/g;
    return @positions;
}

sub relative_root {
    my ($value) = @_;
    fail("root must be package-relative: $value") if File::Spec->file_name_is_absolute($value);
    my @parts = split m{/+}, $value;
    fail("parent traversal in root: $value") if grep { $_ eq '..' } @parts;
    @parts = grep { length($_) && $_ ne '.' } @parts;
    return @parts ? join('/', @parts) : '.';
}

sub run {
    my (@source_roots, @test_roots, $self_test, $help);
    Getopt::Long::Configure(qw(no_auto_abbrev no_ignore_case));
    GetOptions('source-root=s@' => \@source_roots, 'test-root=s@' => \@test_roots,
               'self-test' => \$self_test, 'help' => \$help) or fail('use --help for usage');
    if ($help) {
        print <<'HELP';
Usage: perl real-programs-census.pl [options] PACKAGE_ROOT
  --source-root REL   Source file/directory, repeatable (default: src, else .).
  --test-root REL     External-test file/directory, repeatable (default: tests if present).
  --self-test         Run in-memory lexer controls; PACKAGE_ROOT may be omitted.
Only .rs files are read. Test roots take precedence over overlapping source roots.
Inline tests stay in source. Roots are package-relative; symlinks are rejected.
Output is deterministic JSON. Counts inventory lexical source, including inactive
cfg branches and macro bodies; they do not report compiler refusals or expansions.
HELP
        return;
    }
    my $controls = $self_test ? self_test() : undef;
    if (!@ARGV && $self_test) { print json({ self_test => $controls }); return }
    fail('exactly one package root is required; use --help') unless @ARGV == 1;
    my $root = abs_path($ARGV[0]);
    fail("not a directory: $ARGV[0]") unless defined($root) && -d $root;
    @source_roots = (-d "$root/src" ? 'src' : '.') unless @source_roots;
    @test_roots = ('tests') if !@test_roots && -d "$root/tests";
    @source_roots = sort keys %{ { map { relative_root($_) => 1 } @source_roots } };
    @test_roots = sort keys %{ { map { relative_root($_) => 1 } @test_roots } };
    my %files;
    for my $group (['source', \@source_roots], ['external_test', \@test_roots]) {
        my ($kind, $roots) = @$group;
        for my $rel (@$roots) {
            my $path = $root;
            for my $part (split m{/}, $rel) {
                $path .= "/$part";
                fail("symlink root component: $rel") if -l $path;
            }
            fail("missing selected root: $rel") unless -e $path;
            find({ no_chdir => 1, wanted => sub {
                my $file = $File::Find::name;
                fail("symlink under selected root: $file") if -l $file;
                return unless -f $file && $file =~ /\.rs\z/;
                my $relative = File::Spec->abs2rel($file, $root);
                $relative =~ s{\A\./}{};
                $files{$relative} = $kind; # Explicit test roots override source roots.
            } }, $path);
        }
    }
    fail('no .rs files under selected roots') unless keys %files;
    my @records;
    my %summary = (files => 0, bytes => 0, physical_lines => 0,
        source => { files => 0, bytes => 0, physical_lines => 0 },
        external_test => { files => 0, bytes => 0, physical_lines => 0 },
        feature_counts => { map { $_ => 0 } @FEATURES },
        test_attribute_counts => { direct_unqualified => 0, conditional_unqualified => 0, qualified => 0 });
    for my $kind (qw(source external_test)) {
        $summary{$kind}{feature_counts} = { map { $_ => 0 } @FEATURES };
        $summary{$kind}{test_attribute_counts} = {
            direct_unqualified => 0, conditional_unqualified => 0, qualified => 0 };
    }
    for my $path (sort keys %files) {
        open my $fh, '<:raw', "$root/$path" or fail("read $path: $!");
        my $bytes = do { local $/; <$fh> };
        close $fh or fail("close $path: $!");
        $bytes = '' unless defined $bytes;
        my $text = eval { decode('UTF-8', $bytes, FB_CROAK | LEAVE_SRC) };
        fail("$path is not UTF-8: $@") if $@;
        my $features = eval { inventory($text) };
        fail("$path: $@") if $@;
        my $display_path = eval { decode('UTF-8', $path, FB_CROAK | LEAVE_SRC) };
        fail("non-UTF-8 file name under selected roots") if $@;
        my $record = { path => $display_path, kind => $files{$path}, sha256 => sha256_hex($bytes),
            bytes => length($bytes), physical_lines => physical_lines($bytes), features => $features };
        push @records, $record;
        for my $target (\%summary, $summary{$files{$path}}) {
            ++$target->{files};
            $target->{bytes} += $record->{bytes};
            $target->{physical_lines} += $record->{physical_lines};
            $target->{feature_counts}{$_} += $features->{$_}{count} for @FEATURES;
            for my $site (@{$features->{test_attributes}{sites}}) {
                my $kind = $site->{qualified} ? 'qualified'
                         : $site->{conditional} ? 'conditional_unqualified' : 'direct_unqualified';
                ++$target->{test_attribute_counts}{$kind};
            }
        }
    }
    my $result = {
        schema => 1, inventory_kind => 'lexical_rust_source',
        scope => { source_roots => [map { decode('UTF-8', $_, FB_CROAK | LEAVE_SRC) } @source_roots],
                   external_test_roots => [map { decode('UTF-8', $_, FB_CROAK | LEAVE_SRC) } @test_roots],
                   suffix => '.rs', overlap_precedence => 'external_test' },
        conventions => {
            physical_lines => 'LF terminators plus one unterminated final line; empty file is zero',
            locations => 'one-based lines and Unicode-scalar columns in original UTF-8 source',
            inline_tests => 'remain in source files; external_test is determined only by selected roots',
            macro_classification => 'nonbuiltin means no lexical match to the explicit standard macro-name list; all invocations are also retained',
            standard_macro_names => \@STANDARD_MACROS,
            limitations => [
                'No parsing, name resolution, cfg evaluation, macro expansion, type checking, refusal classification, or blocked-line estimate.',
                'Keyword counts include inactive branches and literal macro-body tokens; they are not counts of semantic constructs.',
                'Unqualified standard macro names may be shadowed; renamed/imported macros cannot be resolved.',
                'Test attributes include paths ending in test and recursively nested cfg_attr; generated tests and custom attribute semantics are unknown. Doctests are comments, not test-attribute sites.',
                'Derive traits are lexical paths, including built-in derives; no expansion size is inferred.',
                'Only selected .rs roots are inventoried; dependency, generated, example and build-script sources require separately selected roots.',
                'Inputs must be UTF-8. Unterminated comments/strings and symlinks fail; other Rust syntax is not validated.'
            ],
        },
        summary => \%summary, files => \@records,
    };
    $result->{self_test} = $controls if defined $controls;
    print json($result);
}

run();
