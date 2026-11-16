use strict;
use warnings;
use File::Find;
use Fcntl qw(:DEFAULT :mode);
use Digest::SHA qw(sha256_hex);
use JSON::PP;
my $root = '/home/coder/workspace/lkjscript-compiler-scalar-20261116';
chdir $root or die "$root: $!";
my $out = 'target/compiler-scalar-retention-proposal-1';
my @scopes = qw(
 compiler-scalar-proposals
 compiler-scalar-edit-core-1
 compiler-scalar-core-check-v2
 compiler-scalar-edit-core-2
 compiler-scalar-tests-check-v3
 compiler-scalar-edit-core-apply-1
 compiler-scalar-tests-check-v3-complete
 compiler-scalar-edit-tests-apply-1
 compiler-scalar-edit-tests-explicit-1
 compiler-scalar-live-preliminary-1
 compiler-scalar-edit-mutation-apply-1
 compiler-scalar-bootstrap-1
 compiler-scalar-focused-1
);
my %binary = (
 'target/compiler-scalar-proposals/desired-tests-input.tar' => 'existing-input-archive; stat-only; no archive read',
 'target/compiler-scalar-live-preliminary-1/live' => 'native-image; stat-only; no binary read',
);
my @files;
my @scope_rows;
for my $scope (@scopes) {
 my $path = "target/$scope";
 my @top = lstat($path);
 die "not a real directory: $path\n" unless @top && S_ISDIR($top[2]);
 my ($count,$bytes,$text_count,$text_bytes,$dirs) = (0,0,0,0,0);
 find({ no_chdir=>1, follow=>0, wanted=>sub {
  my $p = $File::Find::name;
  my @s = lstat($p);
  die "cannot lstat: $p\n" unless @s;
  if (S_ISDIR($s[2])) { $dirs++; return; }
  die "nonregular evidence: $p\n" unless S_ISREG($s[2]);
  die "key-shaped evidence path: $p\n" if $p =~ m{(?:^|/)(?:store-key|key|private-key|[^/]+\.key)$};
  my $r = { path=>$p, bytes=>$s[7], mode=>sprintf('%04o', $s[2]&07777), dev=>$s[0], ino=>$s[1], mtime=>$s[9] };
  if (exists $binary{$p}) {
    $r->{class} = $binary{$p};
    $r->{sha256} = undef;
  } else {
    die "unexpected large closed record: $p\n" if $s[7] > 16*1024*1024;
    open my $f,'<:raw',$p or die "$p: $!";
    my $h = Digest::SHA->new(256); $h->addfile($f); close $f or die "$p close: $!";
    $r->{class}='retained-record';
    $r->{sha256}=$h->hexdigest;
    $text_count++; $text_bytes += $s[7];
  }
  $count++; $bytes += $s[7]; push @files,$r;
 }},$path);
 push @scope_rows,{ scope=>$path, files=>$count, bytes=>$bytes, directories_including_root=>$dirs,
    retained_record_files=>$text_count, retained_record_bytes=>$text_bytes };
}
@files=sort { $a->{path} cmp $b->{path} } @files;
my $sum=0; $sum += $_->{bytes} for @files;
my $records=0; my $recordbytes=0;
for(@files){ if($_->{class} eq 'retained-record'){ $records++; $recordbytes += $_->{bytes}; } }
my $j={schema=>1,kind=>'read-only closed evidence inventory for prospective retention',
 root=>$root, source_commit=>'5b5ce7759c035ae0437e3512e18efc440aa50d2e',
 base=>'e0b21e66659dc1830a5bebddf0c1f30fd2d8fa94',
 total_files=>scalar(@files), total_bytes=>$sum, retained_record_files=>$records,
 retained_record_bytes=>$recordbytes, binary_contents_read=>JSON::PP::false,
 compiler_execution=>JSON::PP::false, archive_or_compression_execution=>JSON::PP::false,
 scopes=>\@scope_rows, files=>\@files,
 excluded=>['target/gate','target/compiler-scalar-cold-store-1','target/compiler-scalar-cold-gate-1',
 'target/bootstrap.nG4W90','target/compiler-scalar-bootstrap-store-1',
 'target/compiler-scalar-desired','target/lkjscript','all keys and durable edit metadata',
 'main worktree and all unrelated worktrees']
};
sub save {
 my($p,$b)=@_; sysopen my $f,$p,O_WRONLY|O_CREAT|O_EXCL,0600 or die "$p: $!";
 binmode $f; print {$f} $b or die "$p write: $!"; close $f or die "$p close: $!";
 return sha256_hex($b);
}
my $json=JSON::PP->new->canonical->pretty->encode($j);
my $jh=save("$out/closed-inventory.json",$json);
my $manifest=join('',map{"$_->{sha256}  $_->{path}\n"}grep{$_->{class} eq 'retained-record'}@files);
my $mh=save("$out/closed-records.sha256",$manifest);
print "inventory_sha256=$jh\nrecord_manifest_sha256=$mh\n";
print "files=".scalar(@files)." bytes=$sum retained_records=$records retained_record_bytes=$recordbytes\n";
for(@scope_rows){print "$_->{scope}\tfiles=$_->{files}\tbytes=$_->{bytes}\trecords=$_->{retained_record_files}\trecord_bytes=$_->{retained_record_bytes}\n";}
for(grep{$_->{class} ne 'retained-record'}@files){ print "deferred_binary\t$_->{path}\t$_->{bytes}\t$_->{class}\n"; }
