type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "&i64" ref shared 3
fn "invalid" 1 3
local "condition" 1 let
local "number" 3 let
local "reference" 7 let
local "result" 3 let
@1 int 1 42
@2 if 0
@3 borrow 2 shared 1
@4 end
@5 read 3 copy 2 deref
@6 drop-if 2
@7 drop 1
@8 return 3
endfn
