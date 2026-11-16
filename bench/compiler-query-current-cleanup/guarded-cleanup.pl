use strict;
use warnings;
use Cwd qw(abs_path);
use File::Find;
use File::Path qw(remove_tree);
use Digest::SHA qw(sha256_hex);
use IPC::Open2;
use IO::Handle;
use JSON::PP;
die "explicit reviewed cleanup selector required\n"
    unless @ARGV == 1 && $ARGV[0] eq '--execute-reviewed-six-path-cleanup';
my $root='/home/coder/workspace/lkjscript-compiler-query-container-20261116';
my $accepted='6a834248a3b5e28912a034343de5640eff2f6a89';
my $evidence="$root/bench/compiler-query-current-cleanup";
die "wrong working tree\n" unless abs_path('.') eq $root;
for my $name(qw(preflight.json progress.json accepted-cleanup.json)) {
    die "cleanup already started: $name\n" if -e "$evidence/$name" || -l "$evidence/$name";
}
sub raw {
    my($p)=@_;open my $f,'<:raw',$p or die "$p: $!";local $/;
    my $b=<$f>;close $f or die "$p close: $!";return defined($b) ? $b : '';
}
sub line {my $b=raw($_[0]);$b =~ s/\n\z//;return $b;}
sub regular {
    my($p)=@_;my @s=lstat $p;
    die "not owned regular $p\n" unless @s && -f _ && !-l _ && $s[4]==$<;
    return \@s;
}
sub directory {
    my($p)=@_;my @s=lstat $p;
    die "not owned canonical directory $p\n"
        unless @s && -d _ && !-l _ && $s[4]==$< && abs_path($p) eq $p;
    return \@s;
}
sub digest {
    my($p)=@_;regular($p);open my $f,'<:raw',$p or die "$p: $!";
    my $sha=Digest::SHA->new(256);$sha->addfile($f);close $f or die $!;
    return $sha->hexdigest;
}
sub save {
    my($name,$value)=@_;open my $f,'>:raw',"$evidence/$name" or die $!;
    print $f JSON::PP->new->canonical->pretty->encode($value) or die $!;
    close $f or die $!;
}
open my $head,'-|','git','rev-parse','HEAD' or die $!;
my $actual=<$head>;close $head or die "git rev-parse failed\n";chomp $actual;
die "accepted HEAD differs\n" unless $actual eq $accepted;
system('git','diff','--exit-code','HEAD','--') == 0 or die "tracked working bytes differ\n";
my($gitout,$gitin);
my $gitpid=open2($gitout,$gitin,'git','cat-file','--batch');
$gitin->autoflush(1);binmode $gitout;
my %blobs;
sub blob {
    my($path)=@_;return $blobs{$path} if exists $blobs{$path};
    die "unsafe Git path\n" unless $path =~ m{\Abench/[A-Za-z0-9_./-]+\z}
        && $path !~ m{(?:\A|/)\.\.?(?:/|\z)};
    print $gitin "$accepted:$path\n" or die "git batch write\n";
    my $header=<$gitout>;die "Git blob absent $path\n"
        unless defined($header) && $header =~ /\A([0-9a-f]{40}) blob ([0-9]+)\n\z/;
    my($oid,$length)=($1,0+$2);my $bytes='';
    while(length($bytes)<$length) {
        my $n=read($gitout,my $part,$length-length($bytes));
        die "short Git blob $path\n" unless defined($n) && $n>0;$bytes.=$part;
    }
    my $newline;read($gitout,$newline,1)==1 && $newline eq "\n" or die "Git framing\n";
    my $sha=sha256_hex($bytes);
    die "retained working copy differs $path\n" unless digest("$root/$path") eq $sha;
    return $blobs{$path}={git_oid=>$oid,sha256=>$sha,bytes=>$length};
}
my @spec=(
 ['target/compiler-query-current-run.TDPm4g',1879,50,891045312,36946644],
 ['/tmp/compiler-query-current-key.seAq58',2,1,139,36946645],
 ['target/compiler-query-current-62fa76f4',35,2,269674,36946633,'bench/compiler-query-current-data/attempt-1'],
 ['target/compiler-query-current-continuation-62fa76f4',380,2,1308050,36959687,'bench/compiler-query-current-results-62fa76f4'],
 ['target/compiler-query-current-revalidated-62fa76f4',22,2,46274,36950088,'bench/compiler-query-current-data/offline-validation-1'],
 ['target/compiler-query-current-revalidated-62fa76f4-2',22,2,46278,36951040,'bench/compiler-query-current-data/offline-validation-2'],
);
my(@inventory,@copies);
for my $s(@spec) {
    my($relative,$want_files,$want_dirs,$want_bytes,$want_inode,$retained)=@$s;
    my $path=$relative =~ m{\A/} ? $relative : "$root/$relative";
    my $st=directory($path);
    die "scope identity differs $path\n" unless $st->[0]==144 && $st->[1]==$want_inode && $st->[4]==1001;
    my($files,$dirs,$bytes)=(0,0,0);
    find({no_chdir=>1,wanted=>sub {
        my $p=$File::Find::name;my @v=lstat $p;
        die "unsafe removal child $p\n" unless @v && $v[4]==$< && !-l _ && (-d _ || -f _);
        die "durable edit metadata in removable tree $p\n" if $p =~ m{/\.lkjscript-edit\.};
        if(-d _){$dirs++;return;}
        $files++;$bytes+=$v[7];
        if(defined $retained) {
            my $suffix=substr($p,length($path)+1);my $proof=blob("$retained/$suffix");
            die "uncommitted disposable bytes $p\n" unless digest($p) eq $proof->{sha256};
            push @copies,{disposable=>$p,retained=>"$retained/$suffix",%$proof};
        }
    }},$path);
    die "scope inventory changed $path\n"
        unless $files==$want_files && $dirs==$want_dirs && $bytes==$want_bytes;
    push @inventory,{path=>$path,device=>$st->[0],inode=>$st->[1],uid=>$st->[4],
        files=>$files,directories=>$dirs,logical_file_bytes=>$bytes};
}
die "text correspondence count\n" unless @copies==459;
my $run=$inventory[0]{path};my $keydir=$inventory[1]{path};
die "repository ownership marker\n" unless line("$run/owner-repository") eq $root;
die "evidence ownership marker\n" unless line("$run/owner-evidence") eq $inventory[2]{path};
die "key ownership markers\n"
    unless line("$run/owner-key") eq $keydir && line("$keydir/owner-run") eq $run;
my $key=regular("$keydir/key");
die "key metadata\n" unless $key->[7]==32 && ($key->[2]&0777)==0600;
my $owner_pid=line("$run/owner-pid");
die "owner pid framing\n" unless $owner_pid =~ /\A[1-9][0-9]*\z/;
die "owner process still exists\n" if -e "/proc/$owner_pid";
my $source_manifest='bench/compiler-query-current-data/attempt-1/inputs-before.sha256';
die "source manifest digest\n" unless blob($source_manifest)->{sha256}
    eq '533c5773742dae792ce8118c47aa136c5d8031fbd52fddd0c7ce2062228030ca';
my %source_seen;
for my $entry(split /\n/,raw("$root/$source_manifest")) {
    $entry =~ /\A([0-9a-f]{64})  ((?:src|library|tools|tests)\/[^\r\n]+)\z/
        or die "source manifest framing\n";
    my($want,$relative)=($1,$2);
    die "source manifest path\n" if $relative =~ m{(?:\A|/)\.\.?(?:/|\z)} || $source_seen{$relative}++;
    die "owned source changed $relative\n" unless digest("$run/source/$relative") eq $want;
}
die "source count\n" unless keys(%source_seen)==1860;
my $canonical='34491766f5258dd3af3caf92db6b92dfeba09b405c00a8dd19cd955205c0d377';
my @images=qw(bin/producer output/image output/cold-index.image output/unchanged-index.image);
for my $image(@images) {
    my $st=regular("$run/$image");die "owned image differs $image\n"
        unless $st->[7]==32196802 && digest("$run/$image") eq $canonical;
}
my %preserved=(
 "$root/.lkjscript-edit.generation"=>'ce49582f1b93e2b87f353866f9e508033cb205322ceccb9702b734eb812cf8fa',
 "$root/.lkjscript-edit.lock"=>'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
 "$root/target/compiler-container-case-migration/.lkjscript-edit.generation"=>'e96a0e891392ab097354bcd3e01a728ab263058744954289d5f15acc1602ecb5',
 "$root/target/compiler-container-case-migration/.lkjscript-edit.lock"=>'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
);
for my $p(keys %preserved){die "preserved metadata differs\n" unless digest($p) eq $preserved{$p};}
my $operation_key='/tmp/compiler-container-key.m4u6F6';directory($operation_key);
die "preserved operation key ownership\n" unless line("$operation_key/owner-worktree") eq $root
    && line("$root/target/compiler-container-evidence/operation-key.path") eq "$operation_key/key";
my $operation_before=regular("$operation_key/key");
die "preserved key metadata\n" unless $operation_before->[7]==32 && ($operation_before->[2]&0777)==0600;
close $gitin or die $!;close $gitout or die $!;waitpid($gitpid,0);
die "Git batch failed\n" unless $?==0;
# This existing ownership guard observes argv/cwd only; it neither signals nor traces processes.
my %ancestors;my $pid=$$;
while($pid) {
    die "cyclic process ancestry\n" if $ancestors{$pid}++;
    my $status=raw("/proc/$pid/status");my($parent)=$status =~ /^PPid:\s+([0-9]+)$/m;
    die "incomplete process ancestry\n" unless defined $parent;$pid=$parent;
}
my @active;
for my $proc(glob '/proc/[0-9]*') {
    my($id)=$proc =~ m{/([0-9]+)\z};next if $ancestors{$id};
    my $cmd=eval{raw("$proc/cmdline")};my $cwd=readlink "$proc/cwd";
    for my $e(@inventory) {
        my $path=$e->{path};
        if((defined($cmd) && index($cmd,$path)>=0)
            || (defined($cwd) && ($cwd eq $path || index($cwd,"$path/")==0))) {
            push @active,$id;last;
        }
    }
}
die "live process reference: @active\n" if @active;
my $record={
 status=>'all-preflight-guards-passed',accepted_commit=>$accepted,worktree=>$root,
 removed=>\@inventory,committed_text_copies=>\@copies,
 source_manifest=>{path=>$source_manifest,sha256=>$blobs{$source_manifest}{sha256},records=>1860},
 images=>{paths=>\@images,sha256=>$canonical,bytes_each=>32196802},
 preserved_metadata=>\%preserved,preserved_operation_key_directory=>$operation_key,
 key_contents_read=>JSON::PP::false,
 process_reference_check=>'No matching live argv/cwd observed; own verified ancestry excluded. This is not a concurrent hostile-mutation guarantee.',
 scope=>'Exactly six named owned paths; all committed evidence, worktree, durable edit metadata, other credentials and root canonical executable preserved.',
};
# All proof, ownership, count and process guards finish before the first removal.
save('preflight.json',$record);
for my $e(@inventory) {
    my $path=$e->{path};my $st=directory($path);
    die "removal slot changed\n" unless $st->[0]==$e->{device} && $st->[1]==$e->{inode};
    my $errors;remove_tree($path,{error=>\$errors});die "remove failed $path\n" if @$errors;
    die "removal remains $path\n" if -e $path || -l $path;
    $e->{removed}=JSON::PP::true;save('progress.json',$record);
}
for my $p(keys %preserved){die "preserved metadata changed\n" unless digest($p) eq $preserved{$p};}
my $operation_after=regular("$operation_key/key");
die "preserved operation key changed\n"
    unless join(':',@$operation_before[0,1,2,4,7]) eq join(':',@$operation_after[0,1,2,4,7]);
$record->{status}='owned-cleanup-complete';
save('accepted-cleanup.json',$record);
print "Removed exactly six owned paths: 2340 files, 892715727 logical bytes; committed text proofs and durable metadata preserved.\n";
