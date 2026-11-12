type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "&i64" ref shared 3
type "Box" box 3
type "ViewOwner" struct move "view" 7 "owned" 8
fn "probe" 0 3
local "condition" 1 let
local "external" 3 let
local "reference" 7 let
local "number" 3 let
local "payload" 8 let
local "container" 9 let
local "result" 3 let
@1 int 1 0
@2 borrow 2 shared 1
@3 int 3 1
@4 intrinsic 4 "box_new" 3
@5 struct 5 2 4
@6 int 0 1
@7 if 0
@8 drop 5 field 1
@9 end
@10 drop 5
@11 drop 1
@12 int 6 42
@13 return 6
endfn
