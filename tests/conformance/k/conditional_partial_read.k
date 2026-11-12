type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "Box" box 3
type "Pair" struct move "left" 7
fn "probe" 0 3
local "condition" 1 let
local "number" 3 let
local "owner" 7 mut
local "pair" 8 mut
local "result" 3 let
@1 int 1 7
@2 intrinsic 2 "box_new" 1
@3 struct 3 2
@4 int 0 1
@5 if 0
@6 read 2 move 3 field 0
@7 drop 2
@8 end
@9 read 2 move 3 field 0
@10 drop 2
@11 drop 3
@12 int 4 42
@13 return 4
endfn
