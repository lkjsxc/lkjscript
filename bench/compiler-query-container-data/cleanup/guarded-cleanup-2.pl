use strict;
use warnings;
use Cwd qw(abs_path);
use File::Find;
use File::Path qw(remove_tree);
use Digest::SHA qw(sha256_hex);
use JSON::PP;
my $root='/home/coder/workspace/lkjscript-compiler-query-container-20261116';
die "wrong working tree\n" unless abs_path('.') eq $root;
my $result="$root/bench/compiler-query-container-data/cleanup/accepted-cleanup.json";
die "already recorded\n" if -e $result;
sub raw { my($p)=@_; open my $f,'<:raw',$p or die "$p: $!"; local $/; my $b=<$f>;close $f;return $b; }
sub line { my $b=raw($_[0]); $b =~ s/\n\z//;return $b; }
sub regular { my($p)=@_;my @s=lstat $p;die "not owned regular $p\n" unless @s && -f _ && !-l _ && $s[4]==$<;return \@s; }
sub directory { my($p)=@_;my @s=lstat $p;die "not owned directory $p\n" unless @s && -d _ && !-l _ && $s[4]==$<;die "noncanonical $p\n" unless abs_path($p) eq $p;return \@s; }
sub manifest {
    my($path,$base)=@_;my $n=0;
    for my $line(split /\n/,raw($path)) {
        $line =~ /\A([0-9a-f]{64})  (.+)\z/ or die "manifest framing\n";
        my($want,$relative)=($1,$2);die "unsafe manifest path\n" if $relative =~ m{\A/|(?:\A|/)\.\.(?:/|\z)};
        my $file="$base/$relative";regular($file);die "manifest differs $file\n" unless sha256_hex(raw($file)) eq $want;$n++;
    }
    return $n;
}
my %verified=(
 implementation=>manifest("$root/bench/compiler-query-container-data/implementation.manifest.sha256","$root/bench/compiler-query-container-data"),
 results=>manifest("$root/bench/compiler-query-container-data/results.manifest.sha256",$root),
 case_copy=>manifest("$root/bench/compiler-query-container-data/implementation/cases-after-2.sha256","$root/target/compiler-container-case-migration"),
);
my @runs=(
 ["$root/target/compiler-query-container-run.jKz4Mn","$root/target/compiler-container-measurement-1","/tmp/compiler-query-container-key.7KdwbR"],
 ["$root/target/compiler-container-helper-run.6JS2DU","$root/target/compiler-container-helper-preflight-1","/tmp/compiler-container-helper-key.ABXkeg"],
);
my @removals;
for my $r(@runs) {
    my($run,$evidence,$keydir)=@$r;directory($run);directory($keydir);
    die "repository marker\n" unless line("$run/owner-repository") eq $root;
    die "evidence marker\n" unless line("$run/owner-evidence") eq $evidence;
    die "key marker\n" unless line("$run/owner-key") eq $keydir && line("$keydir/owner-run") eq $run;
    my $s=regular("$keydir/key");die "key metadata\n" unless $s->[7]==32 && ($s->[2]&0777)==0600;
    if(-e "$run/owner-pid"){my $pid=line("$run/owner-pid");die "owner pid framing\n" unless $pid =~ /\A[1-9][0-9]*\z/;die "owner still exists\n" if -e "/proc/$pid";}
    push @removals,$run,$keydir;
}
my %executables=(
 'compiler-container-baseline'=>'adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7',
 'compiler-container-candidate'=>'73314aa80758acbd62f6aabbcce6c16dc8c3e6f48463c319763e014c3e9d1140',
 'compiler-container-candidate-2'=>'a72d386bbb6af3bf0a1782503ed4ae7f1ea9bf7bb8c3078eb4682acfe2f9c5ad',
 'compiler-container-inline64-g1'=>'eed6ca32edcc182a92c4e697f4702ec685dff8e7778b0374a27552aa9c938947',
 'compiler-container-inline64-g2'=>'eed6ca32edcc182a92c4e697f4702ec685dff8e7778b0374a27552aa9c938947',
);
for my $name(sort keys %executables) {
    my $file="$root/target/$name";regular($file);die "executable digest $name\n" unless sha256_hex(raw($file)) eq $executables{$name};push @removals,$file;
}
push @removals,"$root/target/compiler-container-case-migration/src","$root/target/compiler-container-case-migration/library";
my @preserved=("$root/.lkjscript-edit.lock","$root/.lkjscript-edit.generation",
 "$root/target/compiler-container-case-migration/.lkjscript-edit.lock","$root/target/compiler-container-case-migration/.lkjscript-edit.generation");
my %preserved_hash=map {regular($_);($_=>sha256_hex(raw($_)))} @preserved;
my $operation_key='/tmp/compiler-container-key.m4u6F6';
directory($operation_key);
die "operation key owner\n" unless line("$operation_key/owner-worktree") eq $root
 && line("$root/target/compiler-container-evidence/operation-key.path") eq "$operation_key/key";
my $key_before=regular("$operation_key/key");
my %ancestors;
my $pid=$$;
while($pid && !$ancestors{$pid}++) {
    my $status=eval{raw("/proc/$pid/status")};last unless defined $status;
    my($parent)=$status =~ /^PPid:\s+([0-9]+)/m;last unless defined $parent;$pid=$parent;
}
my @active;
for my $proc(glob '/proc/[0-9]*') {
    my($id)=$proc =~ m{/([0-9]+)\z};next if $ancestors{$id};
    my $cmd=eval{raw("$proc/cmdline")};my $cwd=readlink "$proc/cwd";
    for my $path(@removals) {
        if((defined($cmd) && index($cmd,$path)>=0) || (defined($cwd) && ($cwd eq $path || index($cwd,"$path/")==0))) {push @active,$id;last;}
    }
}
die "live process reference: @active\n" if @active;
my @inventory;
for my $path(@removals) {
    my @s=lstat $path;die "removal path missing\n" unless @s;
    my($files,$bytes)=(0,0);
    my $inspect=sub {
        my $p=$File::Find::name;my @st=lstat $p;
        die "not owned $p\n" unless @st && $st[4]==$<;
        die "durable edit metadata in removable tree $p\n" if $p =~ m{/\.lkjscript-edit\.};
        die "nonregular removal child $p\n" unless -d _ || -f _;
        die "symlink removal child $p\n" if -l _;
        if(-f _){$files++;$bytes+=$st[7];}
    };
    if(-d _){directory($path);find({no_chdir=>1,wanted=>$inspect},$path);}
    else{regular($path);$files=1;$bytes=$s[7];}
    push @inventory,{path=>$path,device=>$s[0],inode=>$s[1],uid=>$s[4],files=>$files,bytes=>$bytes};
}
# All ownership, proof, digest and process checks precede the first removal.
for my $entry(@inventory) {
    my $p=$entry->{path};my @s=lstat $p;die "changed removal slot\n" unless @s && $s[0]==$entry->{device} && $s[1]==$entry->{inode};
    if(-d _){my $error;remove_tree($p,{error=>\$error});die "remove error $p\n" if @$error;}
    else{unlink $p or die "unlink $p: $!";}
    die "removal remains $p\n" if -e $p || -l $p;$entry->{removed}=JSON::PP::true;
}
for my $p(@preserved){regular($p);die "durable metadata changed\n" unless sha256_hex(raw($p)) eq $preserved_hash{$p};}
my $key_after=regular("$operation_key/key");
die "operation key changed\n" unless join(':',@$key_before[0,1,2,4,7]) eq join(':',@$key_after[0,1,2,4,7]);
my $record={status=>'owned-cleanup-complete',worktree=>$root,accepted_report_commit=>'d06ff0553071c71cdcfb2630cd3fb66ec72c3233',
 manifests_verified=>\%verified,process_reference_check=>'no matching live process observed; own tool ancestry excluded',
 removed=>\@inventory,preserved_metadata=>\%preserved_hash,
 preserved_operation_key_directory=>$operation_key,key_contents_read=>JSON::PP::false,
 scope=>'Only named owned fixture paths and executable copies. Worktree, evidence, durable edit metadata, global/main keys preserved.'};
open my $out,'>:raw',$result or die $!;print $out JSON::PP->new->canonical->pretty->encode($record);close $out;
print scalar(@inventory)," owned paths removed; proofs and durable metadata preserved\n";
