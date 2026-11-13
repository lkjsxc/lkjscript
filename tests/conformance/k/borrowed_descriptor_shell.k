type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "&mut i64" ref mut 3
type "Holder" struct move "value" 7
type "&&mut i64" ref shared 7
fn "probe" 0 3
local "value" 3 mut
local "reference" 7 let
local "holder" 8 let
local "descriptor" 9 let
local "result" 3 let
@1 int 0 42
@2 borrow 1 mut 0
@3 struct 2 1
@4 borrow 3 shared 2 field 0
@5 drop 2
@6 drop 3
@7 read 4 copy 0
@8 drop 0
@9 return 4
endfn
