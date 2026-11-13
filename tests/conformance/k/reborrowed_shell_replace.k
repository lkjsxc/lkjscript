type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "&mut i64" ref mut 3
type "Holder" struct move "value" 7
fn "probe" 0 3
local "value" 3 mut
local "reference" 7 let
local "holder" 8 mut
local "returned" 7 let
local "result" 3 let
local "other_value" 3 mut
local "other_reference" 7 let
local "replacement" 8 let
@1 int 0 42
@2 borrow 1 mut 0
@3 struct 2 1
@4 borrow 3 mut 2 field 0 deref
@5 int 5 0
@6 borrow 6 mut 5
@7 struct 7 6
@8 assign 7 2
@9 drop 3
@10 read 4 copy 0
@11 drop 2
@12 drop 5
@13 drop 0
@14 return 4
endfn
