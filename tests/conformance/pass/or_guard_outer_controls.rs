pub fn probe()->i64{let mut count=0i64;loop{count=count+1;match(0i64,0i64){(0,_)|(_,0) if {if count==1{continue;}if count==3{break 42i64;}false}=>(),_=>()}}}
