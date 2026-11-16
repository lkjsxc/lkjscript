#!/usr/bin/perl
# Read-only prelaunch check. Does not execute or apply the semantic transaction.
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use File::Find;
use Fcntl qw(:mode);
my $repo='/home/coder/workspace/lkjscript';
my $root="$repo/target/compiler-fingerprint-index-integration-1";
sub raw { my($p)=@_; my @s=lstat($p); die "not regular single-link: $p\n" unless @s&&S_ISREG($s[2])&&$s[3]==1; open my $f,'<:raw',$p or die "$p: $!"; local $/; my $b=<$f>; close $f or die "close"; return $b; }
sub hash_file { my($p)=@_; my @s=lstat($p); die "not regular single-link: $p\n" unless @s&&S_ISREG($s[2])&&$s[3]==1; open my $f,'<:raw',$p or die "$p: $!"; my $h=Digest::SHA->new(256)->addfile($f)->hexdigest; close $f or die "close"; return $h; }
sub command { open my $f,'-|',@_ or die "command: $!"; binmode $f; local $/; my $b=<$f>; close $f or die "command failed"; return $b; }
my $pin_bytes=raw("$root/pins.json");
die "pins changed\n" unless sha256_hex($pin_bytes) eq 'c3cfc2ca37551ee06d98d8273041d67f8ed6efe2e6532bf87bb75c04d7f03bd6';
my $p=JSON::PP->new->utf8->decode($pin_bytes);
my(%language,%inputs,%all);
for my $spec(['tracked-language.sha256','language_manifest_sha256',\%language],['inputs.sha256','input_manifest_sha256',\%inputs]) {
 my $b=raw("$root/before/$spec->[0]"); die "manifest changed\n" unless sha256_hex($b) eq $p->{$spec->[1]};
 for my $line(split /\n/,$b) {
  die "manifest framing\n" unless $line=~/\A([0-9a-f]{64})  ([\x20-\x7e]+)\z/;
  my($sha,$path)=($1,$2); die "unsafe/duplicate path\n" if $path=~m{\A/|(?:\A|/)\.\.?(?:/|\z)} || exists($spec->[2]->{$path});
  $spec->[2]->{$path}=$sha;
  die "conflicting manifest overlap\n" if exists($all{$path})&&$all{$path} ne $sha; $all{$path}=$sha;
 }
}
die "manifest counts\n" unless keys(%language)==$p->{tracked_language_files} && keys(%inputs)==$p->{input_files};
my @tracked=grep{/\.lkj\z/}split /\0/,command('git','-C',$repo,'ls-files','-z');
die "tracked language population changed\n" unless join("\n",sort @tracked) eq join("\n",sort keys %language);
my %physical;
for my $sub(qw(src library tests)) {
 find({no_chdir=>1,wanted=>sub {
  my $f=$File::Find::name; my @s=lstat($f); die "source lstat\n" unless @s; return if S_ISDIR($s[2]);
  die "source special/linked\n" unless S_ISREG($s[2])&&$s[3]==1;
  (my $rel=$f)=~s/^\Q$repo\E\///; $physical{$rel}=1;
 }}, "$repo/$sub");
}
die "source input population changed\n" unless join("\n",sort keys %physical) eq join("\n",sort keys %inputs);
for my $path(sort keys %all) { die "source changed: $path\n" unless hash_file("$repo/$path") eq $all{$path}; }
die "request changed\n" unless hash_file("$root/request.json") eq $p->{request_sha256};
die "filled module changed\n" unless hash_file("$root/proposals/incremental-fingerprint-module.txt") eq $p->{module_sha256};
die "runner proposal changed\n" unless hash_file("$root/proposals/closed-semantics.sh.txt") eq $p->{runner_after_sha256};
die "runner before changed\n" unless hash_file("$repo/tools/closed-semantics.sh") eq $p->{runner_before_sha256};
die "producer changed\n" unless -s "$repo/target/lkjscript" == $p->{producer_bytes} && hash_file("$repo/target/lkjscript") eq $p->{producer_sha256};
for my $path($p->{new_module},$p->{alternate_module}) {
 die "projected module candidate occupied: $path\n" if lstat("$repo/$path"); die "negative probe not ENOENT\n" unless $!{ENOENT};
}
my $parent=$p->{parent_identity}; my @s=lstat("$repo/$parent->{path}");
die "projection parent identity/type changed\n" unless @s&&S_ISDIR($s[2])&&$s[0]==$parent->{dev}&&$s[1]==$parent->{inode}&&$s[4]==$parent->{uid}&&$s[5]==$parent->{gid}&&($s[2]&07777)==$parent->{mode};
print JSON::PP->new->canonical->pretty->encode({status=>'prelaunch_source_bytes_match',tracked_language_files=>scalar(keys %language),input_files=>scalar(keys %inputs),request_sha256=>$p->{request_sha256},module_sha256=>$p->{module_sha256},producer_sha256=>$p->{producer_sha256},compiler_or_edit_executed=>JSON::PP::false,
 qualification=>'A read-only point-in-time guard. The compiler still resolves identities, captures negative inputs, checks the full candidate and rechecks publication inputs. No semantic snapshot or item expect digest was invented.'});
