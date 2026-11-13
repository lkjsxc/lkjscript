type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "&mut i64" ref mut 3
fn "probe" 0 3
local "owner" 3 mut
local "pointer" 7 let
local "replacement" 3 let
local "answer" 3 let
@1 int 0 0
@2 borrow 1 mut 0
@3 drop 1
@4 int 2 42
@5 assign 2 0
@6 read 3 copy 0
@7 drop 0
@8 return 3
endfn
