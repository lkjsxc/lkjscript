type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "Box" box 3
type "Pair" struct move "left" 7
fn "probe" 0 0
local "number" 3 let
local "boxed" 7 let
local "pair" 8 mut
local "stolen" 7 let
local "done" 0 let
@1 int 0 1
@2 intrinsic 1 "box_new" 0
@3 struct 2 1
@4 read 3 move 2 field 0
@5 drop 2 field 0
@6 drop 3
@7 drop 2
@8 unit 4
@9 return 4
endfn
