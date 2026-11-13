type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "[u8;3]" array 5 3
type "&[u8;3]" ref shared 7
fn "probe" 0 4
local "seed" 5 let
local "array" 7 let
local "reference" 8 let
local "length" 4 let
@1 int 0 42
@2 intrinsic 1 "array_repeat" 0
@3 borrow 2 shared 1
@4 intrinsic 3 "vec_len" 2
@5 drop 1
@6 return 3
endfn
