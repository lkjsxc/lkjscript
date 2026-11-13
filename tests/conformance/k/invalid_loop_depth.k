type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
fn "probe" 0 3
local "result" 3 let
@1 loop
@2 break-to 1
@3 end
@4 int 0 42
@5 return 0
endfn
