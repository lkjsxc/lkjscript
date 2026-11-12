type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "Box" box 3
type "&mut Box" ref mut 7
fn "probe" 0 3
local "number" 3 let
local "boxed" 7 mut
local "reference" 8 let
local "call_result" 0 let
local "answer" 3 let
@1 int 0 42
@2 intrinsic 1 "box_new" 0
@3 borrow 2 mut 1
@4 call 3 1 2
@5 read 4 copy 1 deref
@6 drop 1
@7 drop 3
@8 return 4
endfn
fn "leave_hole" 1 0
local "slot" 8 let
local "done" 0 let
@9 drop-if 0 deref
@10 drop 0
@11 unit 1
@12 return 1
endfn
