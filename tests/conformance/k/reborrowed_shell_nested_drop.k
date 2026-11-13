type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "&mut i64" ref mut 3
type "Holder" struct move "value" 7
type "&mut Holder" ref mut 8
type "Outer" struct move "inner" 8
destructor 8 1
fn "probe" 0 3
local "value" 3 mut
local "reference" 7 let
local "holder" 10 let
local "returned" 7 let
local "result" 3 let
local "inner" 8 let
@1 int 0 42
@2 borrow 1 mut 0
@3 struct 5 1
@3 struct 2 5
@4 borrow 3 mut 2 field 0 field 0 deref
@5 drop 2
@6 drop 3
@7 read 4 copy 0
@8 drop 0
@9 return 4
endfn
fn "drop_holder" 1 0
local "self" 9 let
local "zero" 3 let
local "result" 0 let
@10 int 1 0
@11 assign 1 0 deref field 0 deref
@12 drop 0
@13 unit 2
@14 return 2
endfn
