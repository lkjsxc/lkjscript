use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use Errno qw(ENOENT);
use JSON::PP;
use POSIX qw(strftime);
my $base='/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116/target/compiler-legacy-root-inventory-20261116';
my $capture="$base/cleanup-capture";
my $target='/home/coder/workspace/lkjscript/target';
sub bytes { open my $f,'<:raw',$_[0] or die "$_[0]: $!"; local $/; my $v=<$f>; return defined($v)?$v:''; }
my $raw=bytes("$base/cleanup-results/result.json");
my $result=decode_json($raw);
die "cleanup result" unless $result->{state} eq 'complete' && $result->{exit_status}==0 &&
  $result->{deleted_files}==16365 && $result->{deleted_directories}==15 &&
  $result->{deleted_file_bytes}==365876759 &&
  $result->{bundle_commit} eq 'cd1a63435ee856a334fc08340b4748c987f4838b';
die "stdout mismatch" unless bytes("$capture/stdout") eq $raw;
die "nonzero wrapper/bundle status" unless bytes("$capture/status") eq "0\n" &&
  bytes("$capture/bundle-check.status") eq "0\n";
die "unexpected stderr" unless bytes("$capture/stderr") eq '' && bytes("$capture/bundle-check.stderr") eq '';
my @scopes=qw(compiler-origin-container-gate.uQZd6X compiler-container-root-audit compiler-generated-source-root-audit compiler-strict-scoped-root-audit compiler-current50-root-audit compiler-strict-derived-root-audit compiler-scale-refusal-root-audit);
my @absent;
for my $scope(@scopes) {
    my $path="$target/$scope";
    my @s=lstat($path);
    die "scope still present $path" if @s;
    die "absence not ENOENT $path: $!" unless $! == ENOENT;
    push @absent,{path=>$path,result=>'ENOENT'};
}
my $main_manifest=bytes('/home/coder/workspace/lkjscript/bench/compiler-legacy-root-cleanup-data/manifest.sha256');
die "committed bundle manifest changed" unless sha256_hex($main_manifest) eq '436360a7987b8d34a015dede97589b60cd5b89ef565ee1bc8fd51e9e60945807';
my @checks=grep { length $_ } split /\n/,bytes("$capture/bundle-check.stdout");
die "bundle check count" unless @checks==27 && !grep { $_ !~ /: OK$/ } @checks;
for my $path('/home/coder/workspace/lkjscript','/home/coder/workspace/lkjscript/target','/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116') {
    die "shared directory missing $path" unless -d $path;
}
my $start=bytes("$capture/start.ns"); chomp $start;
my $end=bytes("$capture/end.ns"); chomp $end;
die "capture times" unless $start=~/^[0-9]+$/ && $end=~/^[0-9]+$/ && $end >= $start;
my $before=bytes("$capture/load.before"); chomp $before;
my $after=bytes("$capture/load.after"); chomp $after;
my $report={
 checked_utc=>strftime('%Y-%m-%dT%H:%M:%SZ',gmtime),
 cleanup_request=>'compiler-legacy-root-cleanup-once-20261116-082',
 drain_request=>'compiler-legacy-root-cleanup-drain-20261116-083',
 result_sha256=>sha256_hex($raw),absences=>\@absent,
 retained_bundle_entries_verified=>27,retained_bundle_manifest_sha256=>sha256_hex($main_manifest),
 capture_wall_seconds=>($end-$start)/1000000000,
 guard_seconds=>$result->{elapsed_seconds},
 gnu_time=>bytes("$capture/time"),load_before=>$before,load_after=>$after,
 original_inventory_preserved=>JSON::PP::true,
 guard_invocations=>1,compiler_executed=>JSON::PP::false,
 process_or_fd_observer=>JSON::PP::false,private_key_read=>JSON::PP::false,
};
my $json=JSON::PP->new->canonical->pretty->encode($report);
open my $f,'>:raw',"$capture/postcheck.json" or die $!;
print $f $json; close $f or die $!;
print $json;
