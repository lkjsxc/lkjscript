use strict; use warnings;
use JSON::PP (); use Digest::SHA (); use Cwd qw(abs_path);
my ($repo,$script,$release,$out)=@ARGV;
sub die_if ($$) {
    die "E_COPY_RELEASE_GUARD expected condition and message\n"
        unless @_ == 2 && defined($_[1]);
    die "$_[1]\n" if $_[0];
}
sub raw {
    my ($p)=@_; my @s=lstat($p);
    die_if(!@s || !-f _ || -l _ || $s[3]!=1,"not a single-link regular file: $p");
    open my $f,"<:raw",$p or die "$p: $!"; local $/; return <$f>;
}
sub sha_file {
    my ($p,$allow_links)=@_; my @s=lstat($p);
    die_if(!@s || !-f _ || -l _ || (!$allow_links && $s[3]!=1),"not an admissible regular file: $p");
    open my $f,"<:raw",$p or die "$p: $!";
    return Digest::SHA->new(256)->addfile($f)->hexdigest;
}
sub git {
    open my $f,"-|","git",@_ or die "git: $!"; binmode $f; local $/;
    my $v=<$f>; close $f or die "git failed: @_\n"; return $v;
}
sub hash { die_if(!defined($_[0]) || ref($_[0]) || $_[0]!~/\A[0-9a-f]{64}\z/,"invalid sha256"); return $_[0]; }
sub oid { die_if(!defined($_[0]) || ref($_[0]) || $_[0]!~/\A[0-9a-f]{40}\z/,"invalid commit/tree"); return $_[0]; }
sub rel {
    my ($p)=@_;
    die_if(!defined($p) || ref($p) || $p!~m{\A[A-Za-z0-9._/-]+\z} ||
        $p=~m{(?:\A|/)\.\.?(?:/|\z)} || $p=~m{//} || $p=~m{\A/},"unsafe relative path");
    return $p;
}
sub exact_keys {
    my ($v,@allowed)=@_; die_if(ref($v) ne "HASH","expected object");
    my %ok=map {$_=>1} @allowed;
    die_if(scalar(grep(!$ok{$_},keys %$v)),"unknown release field");
    die_if(scalar(grep(!exists($v->{$_}),@allowed)),"missing release field");
}
sub committed {
    my ($path,$want,$commit)=@_; rel($path); hash($want); oid($commit);
    die_if(sha_file("$repo/$path") ne $want,"working file hash differs: $path");
    die_if(Digest::SHA::sha256_hex(git("show","$commit:$path")) ne $want,
        "committed bytes differ: $path");
}
my $bytes=raw($release);
my $r=JSON::PP->new->utf8->decode($bytes);
exact_keys($r,qw(schema cohort repository release_path evidence_directory run_directory files producer evidence witness bounds decode workers tools));
die_if($r->{schema} ne "compiler-copy-sites-release-v1" || $r->{repository} ne $repo,"schema/repository");
die_if($r->{cohort}!~/\Acompiler-copy-sites-[a-z0-9][a-z0-9._-]*\z/,"cohort");
my $head=git("rev-parse","HEAD"); chomp $head; oid($head);
die_if("$repo/".rel($r->{release_path}) ne $release,"release path mismatch");
die_if(Digest::SHA::sha256_hex(git("show","$head:$r->{release_path}")) ne
    Digest::SHA::sha256_hex($bytes),"release is not exact committed HEAD bytes");
die_if(raw("$out/release.json") ne $bytes,"release copy mismatch");
die_if($r->{evidence_directory}!~m{\Atarget/compiler-copy-sites-evidence\.[A-Za-z0-9][A-Za-z0-9._-]*\z} ||
       "$repo/$r->{evidence_directory}" ne $out ||
       $r->{run_directory}!~m{\Atarget/compiler-copy-sites-run\.[A-Za-z0-9][A-Za-z0-9._-]*\z},"result/run paths");
exact_keys($r->{files},qw(plan launcher accounting));
for my $kind (qw(plan launcher accounting)) {
    my $v=$r->{files}{$kind}; exact_keys($v,qw(path sha256));
    die_if(rel($v->{path})!~m{\Abench/[^/]+\z},"bench file required");
    committed($v->{path},$v->{sha256},$head);
}
die_if("$repo/$r->{files}{launcher}{path}" ne $script,"launcher path mismatch");
die_if($r->{workers} ne "1","one worker required");
exact_keys($r->{bounds},qw(address_space_bytes deadline_seconds grace_seconds free_space_bytes));
my %bounds=(address_space_bytes=>4294967296,deadline_seconds=>900,grace_seconds=>10,free_space_bytes=>3221225472);
for my $k (keys %bounds) { die_if($r->{bounds}{$k} ne "$bounds{$k}","bound mismatch: $k"); }
exact_keys($r->{decode},qw(limit bytes));
die_if($r->{decode}{limit} ne "200" || $r->{decode}{bytes} ne "1048576","detail bounds");
my $w=$r->{witness};
exact_keys($w,qw(revision source_tree library_tree runtime_sha256 optimizer_sha256 seed_archive_sha256 seed_archive_bytes expected_sha256 expected_bytes input_files));
my %witness=(
 revision=>"26bf2c3c021dd79810a81fa1d9054d59f3768b06",
 source_tree=>"3431671024d0a550b3130f2401bde067026d9bd3",
 library_tree=>"e442fb3920db7eda2377f92fde0d92761f0562be",
 runtime_sha256=>"eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14",
 optimizer_sha256=>"961d59858c83afd8c5e8e2f9ec00d0f3a90cc91953ba8903528589f41a9da705",
 seed_archive_sha256=>"85b4e70de34dfa759aaf3cfb7e1009ba6b49b57499e01e4a3d49ae9182a6193c",
 seed_archive_bytes=>"3145429",expected_sha256=>"8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132",
 expected_bytes=>"16228028",input_files=>"172");
for my $k (keys %witness) { die_if($w->{$k} ne $witness{$k},"witness mismatch: $k"); }
for my $pair (["src","source_tree"],["library","library_tree"]) {
    my $v=git("rev-parse","$w->{revision}:$pair->[0]"); chomp $v;
    die_if($v ne $w->{$pair->[1]},"witness tree mismatch");
}
for my $pair (["library/runtime.lkj","runtime_sha256"],["src/optimizer.lkj","optimizer_sha256"],["seed/lkjscript.gz","seed_archive_sha256"]) {
    die_if(Digest::SHA::sha256_hex(git("show","$w->{revision}:$pair->[0]")) ne $w->{$pair->[1]},"witness blob mismatch");
}
my $p=$r->{producer};
exact_keys($p,qw(path sha256 bytes source_commit source_tree library_tree runtime_sha256 seed_commit seed_archive_sha256 seed_archive_bytes bootstrap_revision bootstrap_path bootstrap_sha256));
die_if(!defined($p->{path}) || $p->{path}!~m{\A/[A-Za-z0-9._/-]+\z} ||
       abs_path($p->{path}) ne $p->{path},"producer path must be absolute/canonical");
hash($p->{sha256}); oid($p->{source_commit}); oid($p->{seed_commit}); oid($p->{bootstrap_revision});
die_if($p->{bytes}!~/\A[1-9][0-9]*\z/ || -s $p->{path} != $p->{bytes} ||
       sha_file($p->{path}) ne $p->{sha256},"producer identity");
die_if(!-x $p->{path},"producer not executable");
for my $pair (["src","source_tree"],["library","library_tree"]) {
    oid($p->{$pair->[1]}); my $v=git("rev-parse","$p->{source_commit}:$pair->[0]"); chomp $v;
    die_if($v ne $p->{$pair->[1]},"producer source tree mismatch");
}
hash($p->{runtime_sha256});
die_if(Digest::SHA::sha256_hex(git("show","$p->{source_commit}:library/runtime.lkj")) ne $p->{runtime_sha256},"producer runtime mismatch");
my $seed=git("show","$p->{seed_commit}:seed/lkjscript.gz");
die_if(length($seed) ne $p->{seed_archive_bytes} ||
       Digest::SHA::sha256_hex($seed) ne hash($p->{seed_archive_sha256}),"producer seed archive mismatch");
die_if(Digest::SHA::sha256_hex(git("show","$p->{bootstrap_revision}:".rel($p->{bootstrap_path}))) ne
       hash($p->{bootstrap_sha256}),"bootstrap recipe mismatch");
die_if(system("git","merge-base","--is-ancestor",$p->{source_commit},$head)!=0,"producer source not in release history");
die_if(system("git","diff","--quiet",$p->{source_commit},"--","src","library")!=0,"current source/library differs from gated producer");
for my $f (".lkjscript-edit.pending","..lkjscript-edit.pending.lkjscript-next","..lkjscript-edit.generation.lkjscript-next") {
    die_if(-e "$repo/$f" || -l "$repo/$f","pending source transaction");
}
die_if(ref($r->{evidence}) ne "ARRAY","evidence array required");
my %required=map {$_=>0} qw(gate fixed_point ci focused_profile);
for my $e (@{$r->{evidence}}) {
    exact_keys($e,qw(kind path commit sha256 status coverage_limits));
    die_if(!exists($required{$e->{kind}}) || $required{$e->{kind}}++,"duplicate/unknown acceptance kind");
    die_if($e->{status} ne "passed" || !defined($e->{coverage_limits}) || ref($e->{coverage_limits}),"acceptance must record passed status and limitations");
    die_if(rel($e->{path})!~m{\Abench/},"acceptance must be retained in bench");
    committed($e->{path},$e->{sha256},$e->{commit});
    die_if(system("git","merge-base","--is-ancestor",$e->{commit},$head)!=0,"acceptance commit not in release history");
}
die_if(scalar(grep($_!=1,values %required)),"missing required acceptance");
exact_keys($r->{tools},qw(time prlimit timeout perl git gzip tar bash));
for my $name (sort keys %{$r->{tools}}) {
    my $t=$r->{tools}{$name}; exact_keys($t,qw(path sha256));
    die_if($t->{path}!~m{\A/[A-Za-z0-9._/-]+\z} ||
           abs_path($t->{path}) ne $t->{path} || !-x $t->{path},"tool path: $name");
    die_if(sha_file($t->{path},1) ne hash($t->{sha256}),"tool hash: $name");
}
die_if($r->{tools}{time}{path} ne "/usr/bin/time","GNU time path differs");
my $report={schema=>"compiler-copy-sites-preflight-v1",status=>"passed",head=>$head,
    release_sha256=>Digest::SHA::sha256_hex($bytes),release=>$r};
print JSON::PP->new->canonical->pretty->encode($report);
