type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "&i64" ref shared 3
type "Pair" struct move "left" 7 "right" 7
fn "choose" 3 8
return-leaf 0 field 0 exact 0 field 0 or 0 field 0
return-leaf 0 field 1 exact 0 field 1 or 1 field 1
local "first" 8 let
local "second" 8 let
local "condition" 1 let
local "result" 8 let
@1 if 2
@2 read 3 move 0
@3 else
@4 read 3 move 1
@5 end
@6 drop-if 0
@7 drop-if 1
@8 return 3
endfn
