type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "&i64" ref shared 3
fn "probe" 0 3
local "condition" 1 let
local "number" 3 let
local "reference" 7 let
local "result" 3 let
@1 int 0 1
@2 int 1 42
@3 if 0
@4 borrow 2 shared 1
@5 end
@6 drop 1
@6 drop-if 2
@7 read 3 copy 1
@8 drop 1
@9 return 3
endfn
