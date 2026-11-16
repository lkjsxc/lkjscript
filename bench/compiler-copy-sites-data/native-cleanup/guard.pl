#!/usr/bin/env perl
use strict;
use warnings;
use Archive::Tar;
use Cwd qw(abs_path);
use Digest::SHA qw(sha256_hex);
use Errno qw(ENOENT);
use File::Find;
$|=1;
die "explicit cleanup release argument required\n"
    unless @ARGV==1 && $ARGV[0] eq "--execute-after-root-release";
my $owner="/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116";
my $main="/home/coder/workspace/lkjscript";
my $commit="f9e4581ae815ba041510d31421e6b588d66b4059";
my $prefix="bench/compiler-copy-sites-data/proposals";
my @roots=("compiler-copy-sites-candidate","compiler-copy-sites-candidate-v2",
    "compiler-copy-sites-role-fixture-candidate");
my %pins=(
    "$prefix/compiler-copy-sites-candidate.tar.gz" =>
        "45a8157a1c98ff4372757c9b5fb452bfd37ce7c8b41510c4fd2d257fe0c0e31b",
    "$prefix/compiler-copy-sites-candidate-v2.tar.gz" =>
        "0b5892fe4e12b36d47b685d33d8a8d49aff262334d5407cd9ae8d384c9c5adda",
    "$prefix/compiler-copy-sites-role-fixture-candidate.tar.gz" =>
        "66918c7b74ea5fcf180b4aa4e1351957efb7aa59c12b9b7003e7ae37da32c7ed",
    "$prefix/native-cleanup-inventory.txt" =>
        "e9abcb75e5f23d1c2db95fe09cb354696c5baf1f3d1b6fa5f6542d2e2a3fe516",
);
my $manifest="bench/compiler-copy-sites-results.sha256";
my $manifest_hash="7b73b1dcd1b7de39230e35ca93149f4d44df311ec13bc23aa650cad13036008f";
sub bytes {
    my ($path)=@_;
    open my $f,"<",$path or die "read $path: $!\n";
    binmode $f; local $/; my $data=<$f>;
    close $f or die "close $path: $!\n";
    return $data;
}
sub committed {
    my ($path)=@_;
    open my $git,"-|","git","-C",$main,"cat-file","blob","$commit:$path"
        or die "open committed blob $path: $!\n";
    binmode $git; my $data; { local $/; $data=<$git>; }
    close $git or die "read committed blob $path\n";
    return $data;
}
sub same_set {
    my ($actual,$expected,$label)=@_;
    die "$label differs\n" unless join("\n",sort keys %$actual) eq join("\n",sort keys %$expected);
}
my %parent_inode=($owner=>37034259,"$owner/target"=>37041659);
for my $path (sort keys %parent_inode) {
    my @s=lstat($path);
    die "parent scope $path\n" unless @s && -d _ && !-l _
        && abs_path($path) eq $path && $s[0]==144 && $s[1]==$parent_inode{$path}
        && ($s[2]&07777)==0700 && $s[4]==1001 && $s[5]==1001;
}
my %retained;
for my $path (sort keys %pins) {
    my $git=committed($path);
    die "committed pin $path\n" unless sha256_hex($git) eq $pins{$path};
    my $live=bytes("$main/$path");
    die "live retained pin $path\n" unless $live eq $git;
    $retained{$path}=$git;
}
my $manifest_bytes=committed($manifest);
die "global manifest pin\n" unless sha256_hex($manifest_bytes) eq $manifest_hash
    && bytes("$main/$manifest") eq $manifest_bytes;
my %manifest_entries;
for my $line (split /\n/,$manifest_bytes) {
    die "global manifest row\n" unless $line =~ /^([0-9a-f]{64})  (.+)$/;
    die "duplicate global manifest path\n" if exists $manifest_entries{$2};
    $manifest_entries{$2}=$1;
}
die "global manifest record count\n" unless keys(%manifest_entries)==304;
for my $path (keys %pins) {
    die "global manifest selected identity $path\n"
        unless exists $manifest_entries{$path} && $manifest_entries{$path} eq $pins{$path};
}
print "committed_bundle=$commit global_manifest=$manifest_hash records=304 selected_records=4 pins=exact\n";
my @lines=split /\n/,$retained{"$prefix/native-cleanup-inventory.txt"};
die "inventory shape\n" if @lines%2;
my (%expected,%directories);
while(@lines) {
    my @m=split /\t/,shift @lines;
    my $hash=shift @lines;
    die "inventory fields\n" unless @m==10
        && $hash =~ /^([0-9a-f]{64})  (.+)$/ && $2 eq $m[9];
    my $digest=$1; my $relative=$m[9];
    die "inventory prefix\n" unless $relative =~ s{^target/}{};
    die "inventory path\n" if $relative =~ m{^/|(?:^|/)\.\.(?:/|$)|//};
    my $allowed=0;
    for my $root (@roots) { $allowed=1 if index($relative,"$root/")==0; }
    die "inventory outside released scope\n" unless $allowed;
    die "inventory duplicate\n" if exists $expected{$relative};
    $expected{$relative}={
        mode=>oct($m[0]),uid=>0+$m[1],gid=>0+$m[2],links=>0+$m[3],
        size=>0+$m[4],dev=>0+$m[6],inode=>0+$m[7],mtime=>0+$m[8],digest=>$digest,
    };
    my $d=$relative;
    while($d =~ s{/[^/]+$}{}) { $directories{$d}=1; }
}
die "inventory expected counts\n" unless keys(%expected)==22 && keys(%directories)==7;
my (%live,%live_dirs,%dir_metadata);
for my $root (@roots) {
    File::Find::find({no_chdir=>1,wanted=>sub {
        my $path=$File::Find::name; my @s=lstat($path);
        die "lstat $path\n" unless @s;
        die "symlink in release scope $path\n" if -l _;
        my $relative=substr($path,length("$owner/target/"));
        if(-d _) {
            die "directory authority $path\n" unless $s[4]==1001 && $s[5]==1001
                && ($s[2]&07777)==0700 && abs_path($path) eq $path;
            $live_dirs{$relative}=1;
            $dir_metadata{$relative}=[$s[0],$s[1]];
        } elsif(-f _) { $live{$relative}=1; }
        else { die "unexpected entry type $path\n"; }
    }}, "$owner/target/$root");
}
same_set(\%live,\%expected,"live regular path set");
same_set(\%live_dirs,\%directories,"live directory path set");
sub valid_live_file {
    my ($relative)=@_; my $m=$expected{$relative};
    my $path="$owner/target/$relative"; my @s=lstat($path);
    return @s && -f _ && !-l _
        && ($s[2]&07777)==$m->{mode} && $s[4]==$m->{uid} && $s[5]==$m->{gid}
        && $s[3]==1 && $s[3]==$m->{links} && $s[7]==$m->{size}
        && $s[0]==$m->{dev} && $s[1]==$m->{inode} && $s[9]==$m->{mtime};
}
my $total=0;
for my $relative (sort keys %expected) {
    die "live metadata changed $relative\n" unless valid_live_file($relative);
    die "live bytes changed $relative\n"
        unless sha256_hex(bytes("$owner/target/$relative")) eq $expected{$relative}->{digest};
    die "live file changed during hashing $relative\n" unless valid_live_file($relative);
    $total+=$expected{$relative}->{size};
}
die "live byte count\n" unless $total==295550;
for my $root (@roots) {
    my $archive="$main/$prefix/$root.tar.gz";
    my $tar=Archive::Tar->new;
    die "archive read $root\n" unless $tar->read($archive,1);
    my (%files,%dirs,%names); my $size=0;
    for my $entry ($tar->get_files) {
        my $name=$entry->full_path;
        die "unsafe archive name\n" if $name =~ m{^/|(?:^|/)\.\.(?:/|$)|//};
        die "duplicate archive name\n" if $names{$name}++;
        if($entry->type eq "5") {
            die "archive directory spelling\n" unless $name =~ s{/$}{};
            die "archive directory scope\n" unless $name eq $root || index($name,"$root/")==0;
            die "duplicate archive directory\n" if $dirs{$name}++;
        } elsif($entry->type eq "0" || $entry->type eq "") {
            die "archive regular scope\n" unless index($name,"$root/")==0 && exists $expected{$name};
            die "duplicate archive file\n" if $files{$name}++;
            my $data=$entry->get_content; my $m=$expected{$name};
            die "archive member mismatch $name\n" unless length($data)==$m->{size}
                && $entry->size==$m->{size} && ($entry->mode&07777)==$m->{mode}
                && sha256_hex($data) eq $m->{digest};
            $size+=length($data);
        } else { die "archive link or special member\n"; }
    }
    my %want_files=map {$_=>1} grep {index($_,"$root/")==0} keys %expected;
    my %want_dirs=map {$_=>1} grep {$_ eq $root || index($_,"$root/")==0} keys %directories;
    same_set(\%files,\%want_files,"archive files $root");
    same_set(\%dirs,\%want_dirs,"archive directories $root");
    die "archive changed during validation\n"
        unless sha256_hex(bytes($archive)) eq $pins{"$prefix/$root.tar.gz"};
    print "archive=$root members=exact files=",scalar(keys %files)," directories=",scalar(keys %dirs)," bytes=$size\n";
}
print "preflight=complete exact_live_files=22 exact_live_directories=7 bytes=295550 secret_reads=0 observer_started=no\n";
my $removed=0;
for my $relative (sort keys %expected) {
    die "file changed before unlink $relative\n" unless valid_live_file($relative);
    unlink("$owner/target/$relative")==1 or die "unlink $relative: $!\n";
    $removed++;
    print "removed_file=$relative bytes=$expected{$relative}->{size} sha256=$expected{$relative}->{digest}\n";
}
my $removed_dirs=0;
for my $relative (sort {length($b)<=>length($a) || $a cmp $b} keys %directories) {
    my $path="$owner/target/$relative"; my @s=lstat($path);
    die "directory changed before rmdir $relative\n" unless @s && -d _ && !-l _
        && $s[0]==$dir_metadata{$relative}->[0] && $s[1]==$dir_metadata{$relative}->[1]
        && $s[4]==1001 && $s[5]==1001 && ($s[2]&07777)==0700;
    rmdir($path) or die "rmdir $relative: $!\n";
    $removed_dirs++;
    print "removed_directory=$relative\n";
}
for my $root (@roots) {
    my @s=lstat("$owner/target/$root");
    die "released root still exists or is inaccessible\n" if @s || $! != ENOENT;
    print "absent=$owner/target/$root\n";
}
for my $path (sort keys %parent_inode) {
    my @s=lstat($path);
    die "preserved parent changed\n" unless @s && -d _ && !-l _
        && $s[0]==144 && $s[1]==$parent_inode{$path} && ($s[2]&07777)==0700
        && $s[4]==1001 && $s[5]==1001;
    print "preserved_parent=$path dev=$s[0] inode=$s[1]\n";
}
for my $path (sort keys %pins) {
    die "retained evidence changed\n" unless sha256_hex(bytes("$main/$path")) eq $pins{$path};
}
die "global manifest changed\n" unless sha256_hex(bytes("$main/$manifest")) eq $manifest_hash;
print "cleanup=complete files=$removed directories=$removed_dirs bytes=$total retained_archives=unchanged global_manifest=unchanged compiler_executions=0\n";
