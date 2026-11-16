use strict;
use warnings;
use Cwd qw(abs_path);
use Digest::SHA qw(sha256_hex);
use Fcntl qw(O_RDONLY O_WRONLY O_CREAT O_EXCL O_NOFOLLOW :mode);
use File::Find;
use IO::Uncompress::Gunzip;
use IPC::Open2;
use JSON::PP;
use POSIX qw(strftime);
use Time::HiRes qw(time);

# This program has no dry-run deletion mode, process observer, credential input,
# recursive removal helper, or fallback. Invoke only after the explicit root
# release of the exact committed bundle. Read-only preparation is not release.
my $repo='/home/coder/workspace/lkjscript';
my $target="$repo/target";
my $prefix='bench/compiler-legacy-root-cleanup-data';
my $evidence_pin='f9e4581ae815ba041510d31421e6b588d66b4059';
my $inventory_sha='bda3ce9d26ec494ba97779315bd1d731cb35ecf97c326f80ce6e58d542d69b8e';
my $result_dir='/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116/target/compiler-legacy-root-inventory-20261116/cleanup-results';
my @scopes=qw(
 compiler-origin-container-gate.uQZd6X
 compiler-container-root-audit
 compiler-generated-source-root-audit
 compiler-strict-scoped-root-audit
 compiler-current50-root-audit
 compiler-strict-derived-root-audit
 compiler-scale-refusal-root-audit
);
my %allowed_scope=map { $_=>1 } @scopes;
my @retention=qw(
 compiler-current50-root-audit/continuation-landing.log
 compiler-current50-root-audit/results-landing.log
 compiler-current50-root-audit/replay-pin-landing.log
 compiler-origin-container-gate.uQZd6X/owner-repository
 compiler-origin-container-gate.uQZd6X/owner-pid
 compiler-origin-container-gate.uQZd6X/start-marker
);
@ARGV==2 && $ARGV[0] eq '--execute-after-root-release' &&
 $ARGV[1]=~/^[0-9a-f]{40}$/ or die "requires explicit root release flag and exact committed bundle revision\n";
my $bundle_commit=$ARGV[1];
die "repository/target alias" unless abs_path($repo) eq $repo && abs_path($target) eq $target;
die "result path already exists; do not repeat cleanup" if -e $result_dir || -l $result_dir;
mkdir $result_dir,0700 or die "create result directory: $!";
my $started=time;
my $record={
 bundle_commit=>$bundle_commit,evidence_pin=>$evidence_pin,
 inventory_sha256=>$inventory_sha,started_utc=>strftime('%Y-%m-%dT%H:%M:%SZ',gmtime),
 state=>'preflight',deleted_files=>0,deleted_directories=>0,deleted_file_bytes=>0,
 disposable_cache_files=>16049,disposable_cache_bytes=>318819150,
 disposable_cache_content_archived=>JSON::PP::false,
 process_or_fd_observer=>JSON::PP::false,private_key_read=>JSON::PP::false,
 compiler_executed=>JSON::PP::false,
};
my ($git_pid,$reader,$writer);
my %rows;
sub slurp {
    my ($p)=@_;
    sysopen(my $f,$p,O_RDONLY|O_NOFOLLOW) or die "open $p: $!";
    binmode $f; local $/; my $value=<$f>; close $f or die "close $p: $!";
    return defined($value) ? $value : '';
}
sub git_blob {
    my ($reference)=@_;
    print $writer "$reference\n" or die "Git input: $!";
    my $header=<$reader>; defined($header) or die "Git header missing";
    my ($oid,$size)=$header=~/^([0-9a-f]{40}) blob ([0-9]+)\n$/;
    die "Git object missing/not a blob: $reference" unless defined $size;
    my $bytes='';
    while(length($bytes)<$size) {
        my $n=read($reader,my $part,$size-length($bytes));
        die "truncated Git object $reference" unless defined($n) && $n;
        $bytes.=$part;
    }
    my $newline;
    read($reader,$newline,1)==1 && $newline eq "\n" or die "Git object framing";
    return $bytes;
}
sub close_git {
    return unless defined $git_pid;
    close $writer or die "Git input close";
    close $reader or die "Git output close";
    waitpid($git_pid,0)==$git_pid && $?==0 or die "Git reader exit";
    undef $git_pid;
}
sub in_scope {
    my ($relative)=@_;
    die "unsafe relative path" unless $relative=~/^[A-Za-z0-9_.\/-]+$/ &&
      $relative !~ m{(?:^|/)\.\.?(?:/|$)};
    my ($scope)=split m{/},$relative;
    die "outside approved scopes" unless $allowed_scope{$scope};
}
sub stat_values {
    my (@s)=@_;
    die "missing stat" unless @s;
    my $type=S_ISREG($s[2]) ? 'f' : S_ISDIR($s[2]) ? 'd' : 'other';
    return ($type,sprintf('%o',$s[2]&07777),@s[4,5,3,7,12,0,1,9,10]);
}
sub check_stat {
    my ($relative,@s)=@_;
    my @actual=stat_values(@s);
    my $row=$rows{$relative} or die "unexpected path $relative";
    for my $i(0..10) {
        die "changed metadata $relative column $i" unless "$actual[$i]" eq "$row->[$i]";
    }
}
sub verify_file {
    my ($relative)=@_;
    my $path="$target/$relative";
    check_stat($relative,lstat($path));
    sysopen(my $f,$path,O_RDONLY|O_NOFOLLOW) or die "open live file $relative: $!";
    binmode $f;
    check_stat($relative,stat($f));
    my $digest=Digest::SHA->new(256); $digest->addfile($f);
    die "changed hash $relative" unless $digest->hexdigest eq $rows{$relative}[11];
    check_stat($relative,stat($f));
    close $f or die "close live file $relative";
    check_stat($relative,lstat($path));
}
sub verify_paths {
    my %found;
    for my $scope(@scopes) {
        my $path="$target/$scope";
        my @s=lstat($path);
        die "missing/non-directory scope $scope" unless @s && S_ISDIR($s[2]);
        die "scope alias $scope" unless abs_path($path) eq $path;
        find({no_chdir=>1,follow=>0,wanted=>sub{
            my $absolute=$File::Find::name;
            die "discovery escaped target" unless index($absolute,"$target/")==0;
            my $relative=substr($absolute,length($target)+1);
            in_scope($relative);
            die "extra/duplicate entry $relative" if !exists($rows{$relative}) || $found{$relative}++;
            check_stat($relative,lstat($absolute));
        }},$path);
    }
    die "missing inventoried entries" unless keys(%found)==keys(%rows);
}
sub verify_parent_identities {
    my ($relative)=@_;
    my @parts=split m{/},$relative; pop @parts;
    while(@parts) {
        my $parent=join('/',@parts);
        my @s=lstat("$target/$parent");
        my $expected=$rows{$parent} or die "uninventoried parent $parent";
        die "parent replaced $parent" unless @s && S_ISDIR($s[2]) &&
          sprintf('%o',$s[2]&07777) eq $expected->[1] &&
          $s[4]==$expected->[2] && $s[5]==$expected->[3] &&
          $s[0]==$expected->[7] && $s[1]==$expected->[8];
        pop @parts;
    }
}
my $ok=eval {
    system('git','-C',$repo,'merge-base','--is-ancestor',$bundle_commit,'HEAD')==0
      or die "bundle is not committed in main history";
    $git_pid=open2($reader,$writer,'git','-C',$repo,'cat-file','--batch');
    binmode $reader; binmode $writer;
    select((select($writer),$|=1)[0]);
    my $manifest=git_blob("$bundle_commit:$prefix/manifest.sha256");
    my %contents;
    for my $line(split /\n/,$manifest) {
        my ($sha,$name)=$line=~/^([0-9a-f]{64})  ([A-Za-z0-9_.\/-]+)$/;
        die "bad committed manifest" unless defined($name) &&
          $name!~m{(?:^|/)\.\.?(?:/|$)} && !exists($contents{$name});
        my $bytes=git_blob("$bundle_commit:$prefix/$name");
        die "committed bundle hash mismatch $name" unless sha256_hex($bytes) eq $sha;
        $contents{$name}=$bytes;
    }
    for my $required(qw(guard.pl full-inventory.tsv retained-inputs.json retained-counterparts-verified.json plan.json)) {
        die "bundle lacks $required" unless exists $contents{$required};
    }
    die "executing guard differs from committed bytes" unless
      sha256_hex(slurp(__FILE__)) eq sha256_hex($contents{'guard.pl'});
    die "inventory identity mismatch" unless sha256_hex($contents{'full-inventory.tsv'}) eq $inventory_sha;
    my @lines=split /\n/,$contents{'full-inventory.tsv'};
    die "inventory schema" unless shift(@lines) eq join("\t",qw(type mode uid gid links bytes blocks device inode mtime ctime sha256 path));
    my ($files,$dirs,$bytes,$cache_files,$cache_bytes)=(0,0,0,0,0);
    for my $line(@lines) {
        my @f=split /\t/,$line,-1;
        die "inventory framing" unless @f==13;
        my $relative=pop @f; in_scope($relative);
        die "duplicate inventory path" if exists $rows{$relative};
        die "inventory type" unless $f[0] eq 'f' || $f[0] eq 'd';
        die "inventory numeric metadata" if grep { $_ !~ /^[0-9]+$/ } @f[1..10];
        if($f[0] eq 'f') {
            die "unsafe file metadata" unless $f[4]==1 && $f[11]=~/^[0-9a-f]{64}$/;
            $files++; $bytes+=$f[5];
            if($relative=~m{^compiler-origin-container-gate\.uQZd6X/store/}) {
                $cache_files++; $cache_bytes+=$f[5];
            }
        } else { die "directory digest" unless $f[11] eq '-'; $dirs++; }
        $rows{$relative}=\@f;
    }
    die "scope totals differ" unless $files==16365 && $dirs==15 &&
      $bytes==365876759 && $cache_files==16049 && $cache_bytes==318819150;
    my %recoverable;
    my $counterparts=decode_json($contents{'retained-counterparts-verified.json'});
    die "counterpart count" unless @$counterparts==309;
    for my $item(@$counterparts) {
        die "counterpart path" unless index($item->{path},"$target/")==0;
        my $relative=substr($item->{path},length($target)+1);
        my $row=$rows{$relative} or die "counterpart outside inventory";
        die "counterpart source mismatch" unless $row->[0] eq 'f' &&
          $row->[5]==$item->{bytes} && $row->[11] eq $item->{sha256};
        die "counterpart pin" unless $item->{reference}=~m{^\Q$evidence_pin\E:bench/};
        my $bytes=git_blob($item->{reference});
        die "retained counterpart mismatch" unless length($bytes)==$item->{bytes} &&
          sha256_hex($bytes) eq $item->{sha256};
        $recoverable{$relative}=1;
    }
    my $retained=decode_json($contents{'retained-inputs.json'});
    die "new retention count" unless @$retained==@retention;
    my %required=map { $_=>1 } @retention;
    for my $item(@$retained) {
        die "retention path" unless index($item->{source_path},"$target/")==0;
        my $relative=substr($item->{source_path},length($target)+1);
        die "unexpected/duplicate retained file" unless delete $required{$relative};
        die "retained bundle path" unless $item->{bundle_path} eq "retained/$relative";
        my $bytes=$contents{$item->{bundle_path}};
        die "retained bytes missing" unless defined $bytes;
        my $row=$rows{$relative} or die "retention outside inventory";
        die "retained bytes mismatch" unless length($bytes)==$row->[5] &&
          sha256_hex($bytes) eq $row->[11] && $item->{sha256} eq $row->[11] &&
          $item->{bytes}==$row->[5];
        $recoverable{$relative}=1;
    }
    die "retention incomplete" if keys %required;
    my $seed=git_blob('72a259e8889cff6e7c4ac712d7940b77343a9f5b:seed/lkjscript.gz');
    die "historical compressed seed mismatch" unless length($seed)==6466085 &&
      sha256_hex($seed) eq '0f2b0b0f0d62dc31e625eea80cfb37e317f007f6fcb4a7e8ecf6ef5249d51522';
    my $unzip=IO::Uncompress::Gunzip->new(\$seed) or die "historical seed gzip";
    my $digest=Digest::SHA->new(256); my $decoded_bytes=0;
    while(1) {
        my $n=$unzip->read(my $buffer,1024*1024);
        die "historical seed inflate" if $n<0;
        last if $n==0;
        $decoded_bytes+=$n; $digest->add($buffer);
    }
    $unzip->close or die "historical seed close";
    my $canonical_sha=$digest->hexdigest;
    die "historical canonical mismatch" unless $decoded_bytes==32196802 &&
      $canonical_sha eq '34491766f5258dd3af3caf92db6b92dfeba09b405c00a8dd19cd955205c0d377';
    for my $name(qw(canonical seed-1.gz seed-2.gz)) {
        my $relative="compiler-origin-container-gate.uQZd6X/$name";
        my $row=$rows{$relative} or die "missing seed-related file";
        my ($expected_bytes,$expected_hash)=$name eq 'canonical' ?
          ($decoded_bytes,$canonical_sha) : (length($seed),sha256_hex($seed));
        die "seed-related inventory mismatch" unless $row->[5]==$expected_bytes && $row->[11] eq $expected_hash;
        $recoverable{$relative}=1;
    }
    for my $relative(keys %rows) {
        next unless $rows{$relative}[0] eq 'f';
        next if $relative=~m{^compiler-origin-container-gate\.uQZd6X/store/};
        die "non-cache file lacks committed recovery $relative" unless $recoverable{$relative};
    }
    close_git();
    verify_paths();
    for my $relative(sort keys %rows) { verify_file($relative) if $rows{$relative}[0] eq 'f'; }
    verify_paths();
    $record->{state}='deleting';
    for my $relative(sort keys %rows) {
        next unless $rows{$relative}[0] eq 'f';
        verify_parent_identities($relative);
        verify_file($relative);
        unlink("$target/$relative")==1 or die "unlink approved file $relative: $!";
        $record->{deleted_files}++; $record->{deleted_file_bytes}+=$rows{$relative}[5];
    }
    my @directories=grep { $rows{$_}[0] eq 'd' } keys %rows;
    for my $relative(sort { length($b)<=>length($a) || $a cmp $b } @directories) {
        verify_parent_identities($relative);
        my @s=lstat("$target/$relative");
        my $expected=$rows{$relative};
        die "directory identity changed $relative" unless @s && S_ISDIR($s[2]) &&
          $s[0]==$expected->[7] && $s[1]==$expected->[8] &&
          $s[4]==$expected->[2] && $s[5]==$expected->[3] &&
          sprintf('%o',$s[2]&07777) eq $expected->[1];
        rmdir("$target/$relative") or die "rmdir approved directory $relative: $!";
        $record->{deleted_directories}++;
    }
    my @absences;
    for my $scope(@scopes) {
        die "scope remains $scope" if lstat("$target/$scope");
        push @absences,"$target/$scope";
    }
    $record->{absent_paths}=\@absences;
    die "final totals" unless $record->{deleted_files}==16365 &&
      $record->{deleted_directories}==15 && $record->{deleted_file_bytes}==365876759;
    $record->{state}='complete';
    1;
};
my $error=$@;
if(defined $git_pid) {
    close $writer; close $reader; waitpid($git_pid,0);
}
$record->{finished_utc}=strftime('%Y-%m-%dT%H:%M:%SZ',gmtime);
$record->{elapsed_seconds}=time-$started;
$record->{exit_status}=$ok?0:1;
$record->{error}=$error unless $ok;
my $output=JSON::PP->new->canonical->pretty->encode($record);
sysopen(my $result,"$result_dir/result.json",O_WRONLY|O_CREAT|O_EXCL,0600) or die "write result: $!";
print $result $output; close $result or die "close result: $!";
print $output;
exit($ok?0:1);
