#!/usr/bin/env perl
# Offline regression coverage for probe.pl's byte-path/text boundary.
# Uses only temporary Git fixtures and fixture compiler/time programs.
use strict;
use warnings;
use Cwd qw(abs_path);
use Digest::SHA qw(sha256_hex);
use Encode qw(decode FB_CROAK LEAVE_SRC);
use File::Copy qw(copy);
use File::Path qw(make_path);
use File::Temp qw(tempdir);
use FindBin;
use Getopt::Long qw(GetOptions);
use JSON::PP;
use MIME::Base64 qw(decode_base64 encode_base64);
use POSIX ();

my $projector="$FindBin::Bin/project.pl";
my $probe="$FindBin::Bin/probe.pl";
my $help;
Getopt::Long::Configure(qw(no_auto_abbrev no_ignore_case));
GetOptions('projector=s'=>\$projector,'probe=s'=>\$probe,'help'=>\$help)
    or die "use --help\n";
if ($help) {
    print "Usage: perl probe-test.pl [--projector project.pl] [--probe probe.pl]\n";
    print "Runs temporary offline fixtures; no real compiler or network is used.\n";
    exit 0;
}
die "unexpected positional arguments\n" if @ARGV;
for ($projector,$probe) {
    $_=abs_path($_) // die "test input script does not exist\n";
    die "test input is not a regular file\n" unless -f $_;
}
my $tmp=tempdir('lkj-probe-unicode-XXXXXX',TMPDIR=>1,CLEANUP=>1);
local %ENV=%ENV;
delete @ENV{qw(GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_CONFIG_PARAMETERS GIT_CONFIG_COUNT PERL_UNICODE)};
delete $ENV{$_} for grep { /\AGIT_CONFIG_(?:KEY|VALUE)_\d+\z/ } keys %ENV;
$ENV{GIT_CONFIG_NOSYSTEM}=1;$ENV{GIT_CONFIG_GLOBAL}='/dev/null';
my $checks=0;my $sequence=0;
sub check {++$checks;die "review check failed: $_[1]\n" unless $_[0]}
sub contents {open my $f,'<:raw',$_[0] or die $!;local $/;my $s=<$f>//"";close $f;return$s}
sub write_file {open my $f,'>:raw',$_[0] or die $!;print $f $_[1] or die$!;close $f or die$!}
sub text_value {return decode('UTF-8',$_[0],FB_CROAK|LEAVE_SRC)}
sub run {
 my(@command)=@_;my $n=$sequence++;my $out="$tmp/run-$n.stdout";my $err="$tmp/run-$n.stderr";
 my $pid=fork();die$! unless defined$pid;
 if(!$pid){open STDOUT,'>:raw',$out or POSIX::_exit(120);open STDERR,'>:raw',$err or POSIX::_exit(121);exec{$command[0]}@command or POSIX::_exit(127)}
 waitpid($pid,0)==$pid or die$!;my $status=$?;return(($status&127?128+($status&127):$status>>8),contents($out),contents($err));
}
sub git {
 my($root,@args)=@_;my($status,$out,$err)=run('git','-c','core.hooksPath=/dev/null','-C',$root,@args);
 die "git failed: $err" if$status;return$out;
}
make_path("$tmp/tools");
copy($projector,"$tmp/tools/project.pl") or die$!;
copy($probe,"$tmp/tools/probe.pl") or die$!;
my $compiler_code="#!$^X\n".<<'COMPILER';
use strict;use warnings;
my($verb,$entry,$sf,$root,$lf,$library)=@ARGV;
die "bad compiler fixture arguments" unless @ARGV==6 && $verb eq 'check' && $sf eq '--source-dir' && $lf eq '--library-dir' && -d $library;
open my $f,'<:raw',"$root/$entry" or die "cannot open exact byte entry: $!";
local $/;my $payload=<$f>//"";close $f;
binmode STDOUT;binmode STDERR;
print STDOUT "entry\0".$entry."\0root\0".$root."\0payload\0".$payload."\xff";
print STDERR "diagnostic \xe2\x98\x83\n";
COMPILER
my $time_code="#!$^X\n".<<'TIME';
use strict;use warnings;
my($f,$format,$o,$path)=splice(@ARGV,0,4);
die "bad time fixture arguments" unless $f eq '-f' && $format eq '%M' && $o eq '-o';
my $status=system{$ARGV[0]}@ARGV;
open my $out,'>:raw',$path or die$!;print $out "4321\n";close$out or die$!;
exit($status==-1?127:($status&127?128+($status&127):$status>>8));
TIME
my $utf=pack('H*','e6ba90');
my $compiler_ascii="$tmp/compiler";my $time_ascii="$tmp/time";
my $compiler_utf="$tmp/compiler-$utf";my $time_utf="$tmp/time-$utf";
write_file($_,$compiler_code) for($compiler_ascii,$compiler_utf);
write_file($_,$time_code) for($time_ascii,$time_utf);
chmod 0755,$compiler_ascii,$compiler_utf,$time_ascii,$time_utf or die$!;
my $body="fn original() {}\r\n// caf".pack('H*','c3a9')."\r\n";
sub fixture {
 my($label,$entry,$unicode_roots)=@_;
 my $dir="$tmp/$label";make_path($dir);
 my $source="$dir/source".($unicode_roots?"-$utf":"");
 my $projection="$dir/projection".($unicode_roots?"-$utf":"");
 my $library="$dir/library".($unicode_roots?"-$utf":"");
 my $lib_relative=$label eq 'ascii' ? 'std/plain.lkj' : 'std/caf'.pack('H*','c3a9').'.lkj';
 make_path("$source/src","$library/std");
 git($source,'-c','init.defaultBranch=main','init','--quiet','--object-format=sha1');
 git($source,'config','user.name','Local review fixture');
 git($source,'config','user.email','fixture@example.invalid');
 git($source,'config','commit.gpgsign','false');
 git($source,'config','core.autocrlf','false');
 git($source,'config','remote.origin.url','https://example.invalid/fixture.git');
 write_file("$source/$entry",$body);write_file("$library/$lib_relative","fixture library\n");
 git($source,'add','--','.');git($source,'commit','--quiet','-m','fixture');
 my $commit=git($source,'rev-parse','HEAD');chomp$commit;
 my($status,$out,$err)=run($^X,"$tmp/tools/project.pl",$source,$projection);
 check($status==0,"$label projection succeeds: $err");
 return{dir=>$dir,source=>$source,projection=>$projection,library=>$library,entry=>$entry,lib_relative=>$lib_relative,commit=>$commit,
 compiler=>$unicode_roots?$compiler_utf:$compiler_ascii,time=>$unicode_roots?$time_utf:$time_ascii};
}
sub invoke {
 my($case,$output,%override)=@_;
 my %c=(%$case,%override);
 return run($^X,"$tmp/tools/probe.pl",'--compiler',$c{compiler},'--time',$c{time},'--library',$c{library},
 '--source',$c{source},'--projection',$c{projection},'--package','fixture','--version','1.0.0','--entry',$c{entry},'--output',$output);
}
sub validate_receipt {
 my($case,$receipt,$output)=@_;
 my $manifest_sha=sha256_hex(contents("$case->{projection}/projection.json"));
 check(@{$receipt->{observations}}==2 && $receipt->{projection_manifest_sha256} eq $manifest_sha
  && $receipt->{verification_before}{manifest_sha256} eq $manifest_sha
  && $receipt->{verification_after}{manifest_sha256} eq $manifest_sha,'both routes bind verified manifest');
 check($receipt->{package}{root} eq text_value($case->{source}) && $receipt->{package}{commit} eq $case->{commit}
  && $receipt->{verification_before}{commit} eq $case->{commit}
  && $receipt->{verification_after}{commit} eq $case->{commit},'source root and Git commit binding');
 my $lib_bytes=contents("$case->{library}/$case->{lib_relative}");
 my $expected_library=[{path=>text_value($case->{lib_relative}),bytes=>length($lib_bytes),sha256=>sha256_hex($lib_bytes)}];
 check($receipt->{library}{root} eq text_value($case->{library})
  && $receipt->{library}{manifest_sha256} eq sha256_hex(JSON::PP->new->canonical->ascii->encode($expected_library)),'library root and input manifest');
 check(JSON::PP->new->canonical->ascii->encode($receipt->{library}{files})
  eq JSON::PP->new->canonical->ascii->encode($expected_library),'library file inventory and Unicode path');
 check($receipt->{compiler}{path} eq text_value($case->{compiler})
  && $receipt->{compiler}{sha256} eq sha256_hex(contents($case->{compiler})),'compiler path and byte identity');
 check($receipt->{time_launcher}{path} eq text_value($case->{time})
  && $receipt->{time_launcher}{sha256} eq sha256_hex(contents($case->{time})),'time launcher path and byte identity');
 for my $index(0,1){
  my $o=$receipt->{observations}[$index];my $path=$case->{entry};$path =~ s/\.rs\z/.lkj/ if$index;
  my $root=$index?$case->{projection}:$case->{source};
  check($o->{exit_status}==0 && !$o->{launcher_signal},"route $index opens exact OS byte path");
  check($o->{command}[2] eq text_value($path) && $o->{command}[4] eq text_value($root),"route $index command text");
  check($o->{upstream_entry} eq text_value($case->{entry}),"route $index entry text");
  check($o->{entry_sha256} eq sha256_hex($body) && $o->{entry_bytes}==length($body),"route $index payload identity");
  my $expected="entry\0".$path."\0root\0".$root."\0payload\0".$body."\xff";
  check(decode_base64($o->{stdout}{base64}) eq $expected && $o->{stdout}{sha256} eq sha256_hex($expected),"route $index diagnostic bytes preserved");
  check($o->{stderr}{text} eq text_value("diagnostic \xe2\x98\x83\n"),"route $index UTF-8 diagnostics");
 }
}
sub ascii_receipt_fingerprint {
 my($receipt)=@_;
 my $r=decode_json(JSON::PP->new->canonical->ascii->encode($receipt));
 delete$r->{timestamp_utc};
 $r->{package}{commit}='FIXTURE_COMMIT';
 $r->{compiler}{sha256}='FIXTURE_COMPILER_SHA256';
 $r->{time_launcher}{sha256}='FIXTURE_TIME_SHA256';
 $r->{projection_manifest_sha256}='FIXTURE_PROJECTION_SHA256';
 for my $verification($r->{verification_before},$r->{verification_after}){
  $verification->{commit}='FIXTURE_COMMIT';
  $verification->{manifest_sha256}='FIXTURE_PROJECTION_SHA256';
 }
 for my $path(\$r->{package}{root},\$r->{compiler}{path},\$r->{time_launcher}{path},\$r->{library}{root},
              \$r->{verification_before}{manifest},\$r->{verification_after}{manifest}){
  $$path =~ s{\A\Q$tmp\E}{/fixture};
 }
 for my $o(@{$r->{observations}}){
  delete@{$o}{qw(wall_seconds load_before load_after)};
  s{\A\Q$tmp\E}{/fixture} for @{$o->{command}};
  my $raw=decode_base64($o->{stdout}{base64});
  $raw =~ s{\Q$tmp\E}{/fixture}g;
  $o->{stdout}{base64}=encode_base64($raw,'');
  $o->{stdout}{sha256}=sha256_hex($raw);
  $o->{stdout}{bytes}=length($raw);
 }
 return sha256_hex(JSON::PP->new->canonical->ascii->encode($r));
}
my $ascii=fixture('ascii','src/lib.rs',0);
my($new_status,$new_stdout,$new_err)=invoke($ascii,"$ascii->{dir}/receipt.json");
check($new_status==0,"ASCII control runs: $new_err");
my $new_receipt=decode_json(contents("$ascii->{dir}/receipt.json"));
validate_receipt($ascii,$new_receipt,"$ascii->{dir}/receipt.json");
# Frozen from the pre-UTF-8 producer c9308e6be90767d20d3a7be66637a5c1be8caaed0330b5bf26b0d19be1dcaa04.
# The normalizer removes timing, temporary prefixes and independently checked
# fixture identities; every stable receipt field and its type remain bound.
check(ascii_receipt_fingerprint($new_receipt) eq
 '0b4f12062b98aace48425ebbbaa2cd67e6b7526feeaf594efd7ac042195cbc22',
 'ASCII receipt schema and stable semantics unchanged');
for my $case (
 fixture('utf-entry','src/caf'.pack('H*','c3a9').'.rs',0),
 fixture('utf-roots','src/caf'.pack('H*','c3a9').'.rs',1)
){
 my $output="$case->{dir}/receipt-$utf.json";
 my($status,$stdout,$stderr)=invoke($case,$output);
 check($status==0,"UTF-8 probe runs: $stderr");
 my $summary=decode_json($stdout);
 check($summary->{receipt} eq text_value($output),'published receipt path is Unicode text');
 validate_receipt($case,decode_json(contents($output)),$output);
}
my @invalid=(
 ['entry',{entry=>"src/bad\xff.rs"},qr/non-UTF-8 entry path/],
 ['source',{source=>"$tmp/source-\xff"},qr/non-UTF-8 input path/],
 ['projection',{projection=>"$tmp/projection-\xff"},qr/non-UTF-8 input path/],
);
for my $case(@invalid){
 my($name,$override,$reason)=@$case;my $output="$tmp/invalid-$name.json";
 my($status,$stdout,$stderr)=invoke($ascii,$output,%$override);
 check($status!=0 && $stderr =~ $reason && !-e$output,"invalid UTF-8 $name refused");
}
my $bad_output="$tmp/invalid-\xff.json";
my($out_status,$out_stdout,$out_stderr)=invoke($ascii,$bad_output);
check($out_status!=0 && $out_stderr =~ /non-UTF-8 output path/ && !-e$bad_output,'invalid UTF-8 output refused');
write_file("$ascii->{library}/bad-\xff.lkj","invalid path fixture\n");
my($lib_status,$lib_stdout,$lib_stderr)=invoke($ascii,"$tmp/invalid-library.json");
check($lib_status!=0 && $lib_stderr =~ /non-UTF-8 library path/ && !-e"$tmp/invalid-library.json",'invalid UTF-8 library member refused');
print JSON::PP->new->canonical->ascii->encode({passed=>JSON::PP::true,checks=>$checks,
 cases=>['ASCII receipt control','UTF-8 entry under ASCII roots','UTF-8 source/projection/library/compiler/time/output paths','invalid UTF-8 entry/source/projection/output/library-member refusals'],
 diagnostic_bytes_preserved=>JSON::PP::true,temporary_fixture_cleanup=>'File::Temp CLEANUP enabled',real_compiler_invoked=>JSON::PP::false}),"\n";

