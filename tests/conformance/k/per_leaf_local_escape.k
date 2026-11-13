type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "&i64" ref shared 3
type "Pair" struct move "left" 7 "right" 7
fn "leak" 1 7
return-leaf 0 exact 0
local "external" 7 let
local "local" 3 let
local "escaped" 7 let
@1 int 1 42
@2 borrow 2 shared 1
@3 drop 0
@4 return 2
endfn
