use strict;use warnings;use JSON::PP;use Digest::SHA qw(sha256_hex);use Fcntl qw(:DEFAULT);
my$p='/home/coder/workspace/lkjscript/target/compiler-scalar-cohort-package-1';
sub raw{my$f=shift;open my$h,'<:raw',$f or die"$f: $!";local$/;my$b=<$h>;close$h;return defined($b)?$b:'';}
sub pinned{my($f,$h)=@_;my$b=raw($f);die"metadata input pin: $f\n"unless sha256_hex($b)eq$h;return$b;}
my$original_hash='c9e4be78f61bd1922bc5cebfda0b3dec76cbb81680807a485e861e65eb2d280e';
my$v=decode_json(pinned("$p/verification.json",$original_hash));
my$policy=decode_json(pinned("$p/retention-proposal/later-artifacts.json",'5f99c5746ce18811468904ff4ff6a979fa994634f242df518a2076abaed6e1de'));
pinned("$p/pack.pl",'300d4288dec42eda23badb2e323a4e9b2146bce3d4c9ecc5e952dc80859da189');
die"successful captured execution required\n"unless raw("$p/capture/status")eq"0\n"&&raw("$p/capture/stderr")eq''&&raw("$p/capture/stdout")=~/PASS raw=154 source_members=172 new_gzips=3 existing_E=verified_once shared_seed_outputs=7 proposal_copies=18\n\z/;
my%a=map{$_->{artifact}=>$_}@{$policy->{new_gzips}};die"artifact population\n"unless keys(%a)==3&&@{$v->{new_gzips}}==3;
my@changes;my$empty='e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855';
for my$r(@{$v->{new_gzips}}){
 my$e=$a{$r->{artifact}}or die"unknown artifact\n";die"incorrect failure shape\n"unless$r->{original_sha256}eq$empty&&$r->{original_bytes}==$e->{original_bytes}&&$r->{full_bytes_equal};
 push@changes,{field=>'new_gzips/'.$r->{artifact}.'/original_sha256',original=>$r->{original_sha256},corrected=>$e->{original_sha256},basis=>'successful expanded pin mismatch guard evaluated the first hexdigest against this exact policy value before return consumed the reset state'};
 $r->{original_sha256}=$e->{original_sha256};
}
my$e=$policy->{existing_emitter_gzip};
die"E failure shape\n"unless$v->{existing_emitter}{compressed_sha256}eq$empty&&$v->{existing_emitter}{original_sha256}eq$empty&&$v->{existing_emitter}{git_blob}eq'da39a3ee5e6b4b0d3255bfef95601890afd80709'&&$v->{existing_emitter}{compressed_bytes}==$e->{bytes}&&$v->{existing_emitter}{original_bytes}==$e->{original_bytes}&&$v->{existing_emitter}{full_bytes_equal}&&!$v->{existing_emitter}{copied};
for my$pair(['compressed_sha256',$e->{sha256}],['original_sha256',$e->{original_sha256}],['git_blob','95a68fd1c3c298e3686138bb28ad6bf63036e8e2']){
 my($k,$x)=@$pair;push@changes,{field=>"existing_emitter/$k",original=>$v->{existing_emitter}{$k},corrected=>$x,basis=>'successful first-digest guards in the exact retained pack.pl, with policy/committed Git blob values; no new E read'};$v->{existing_emitter}{$k}=$x;
}
my$s=$policy->{shared_output_recovery};
die"seed failure shape\n"unless$v->{shared_seed}{sha256}eq$empty&&$v->{shared_seed}{bytes}==$s->{original_bytes}&&$v->{shared_seed}{all_seven_byte_equal};
push@changes,{field=>'shared_seed/sha256',original=>$v->{shared_seed}{sha256},corrected=>$s->{original_sha256},basis=>'successful shared seed pin guard evaluated the first digest against this exact policy value after all seven byte comparisons'};
$v->{shared_seed}{sha256}=$s->{original_sha256};
die"correction count\n"unless@changes==7;
$v->{metadata_correction}={kind=>'report-only correction; no archive/binary/check rerun',original_verification_sha256=>$original_hash,original_retained=>'verification.json',corrected_fields=>\@changes,
 reason=>'Digest::SHA::hexdigest resets its digest object. The first calls in exact pinned-digest guards passed, but second calls used for JSON fields reported the reset empty-stream digest.',
 check_execution=>'compiler-scalar-cohort-package-run-1346 exited 0; its original captures and helper are unchanged',
 data_artifacts_changed=>JSON::PP::false,existing_E_read_again=>JSON::PP::false};
my$b=JSON::PP->new->canonical->pretty->encode($v);
sysopen my$f,"$p/verification-corrected.json",O_WRONLY|O_CREAT|O_EXCL,0600 or die"create correction: $!";print{$f}$b;close$f or die"close correction: $!";
print"corrected_fields=7\ncorrected_verification_sha256=".sha256_hex($b)."\noriginal_verification_retained=$original_hash\n";
