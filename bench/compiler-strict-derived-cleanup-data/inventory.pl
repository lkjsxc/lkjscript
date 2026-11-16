# Read-only fixture inventory, shared by preparation and the released guard.
use strict; use warnings;
use Digest::SHA ();
use File::Find ();
use Cwd ();
sub derived_bytes {
    my ($path)=@_;
    open my $f,'<:raw',$path or die "$path: $!\n"; local $/; return <$f>//'';
}
sub derived_digest {
    my ($path)=@_;
    open my $f,'<:raw',$path or die "$path: $!\n";
    return Digest::SHA->new(256)->addfile($f)->hexdigest;
}
sub derived_inventory {
    my ($root)=@_;
    my $fixed='/home/coder/workspace/lkjscript-compiler-strict-derived-20261116/target/compiler-strict-derived.ZQv7cu';
    $root eq $fixed && Cwd::realpath($root) eq $fixed && !-l $root or die "fixture path differs\n";
    open my $mounts,'<:raw','/proc/self/mountinfo' or die "cannot inspect mount namespace\n";
    while (my $line=<$mounts>) {
        my @parts=split / /,$line;
        @parts>=6 or die "invalid mount record\n";
        my $point=$parts[4]; $point=~s/\\([0-7]{3})/chr(oct($1))/ge;
        $point ne $root && index($point,"$root/")!=0 or die "mount inside fixture\n";
    }
    close $mounts;
    my @rootstat=lstat($root);
    @rootstat && -d _ && $rootstat[4]==$< && ($rootstat[2]&0777)==0700 or die "fixture ownership/mode differs\n";
    my @paths;
    File::Find::find({no_chdir=>1,wanted=>sub {push @paths,$File::Find::name}},$root);
    my $digest=Digest::SHA->new(256);
    my ($files,$directories,$bytes,$keys)=(0,0,0,0);
    for my $path (sort @paths) {
        my $relative=$path eq $root?'.':substr($path,length($root)+1);
        $relative!~/[\r\n\t\0]/ && ($path eq $root || index($path,"$root/")==0) or die "unsafe path\n";
        my @s=lstat($path);
        @s && !-l _ && $s[4]==$< && $s[0]==$rootstat[0] or die "foreign/symlink/mounted path: $relative\n";
        $relative!~/(?:^|\/)\.lkjscript-edit\.pending$/ && index($relative,'.lkjscript-next')<0 or die "pending/staging metadata exists\n";
        my ($kind,$hash);
        if (-d _) {$directories++;$kind='directory';$hash='-';}
        elsif (-f _) {
            $s[3]==1 or die "hardlinked file: $relative\n";
            $files++;$bytes+=$s[7];$kind='file';
            if ($relative eq 'key') {
                $s[7]==32 && ($s[2]&0777)==0600 or die "private key metadata differs\n";
                $keys++;$hash='private-key-content-not-read';
            } else {$hash=derived_digest($path);}
        } else {die "special file: $relative\n";}
        $digest->add(join("\0",$relative,$kind,@s[0,1,2,3,4,5,7],$hash)."\n");
    }
    $files==3706 && $bytes==71949767 && $keys==1 or die "fixture count/size differs\n";
    return "version=1\nfixture=$fixed\nfiles=$files\ndirectories=$directories\nlogical_bytes=$bytes\nkey_content=not-read\ntree_identity_sha256=".$digest->hexdigest."\n";
}
1;
