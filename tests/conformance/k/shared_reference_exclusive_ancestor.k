type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "&i64" ref shared 3
type "Env" struct copy "capture" 7
type "&Env" ref shared 8
type "&mut i64" ref mut 3
fn "probe" 0 3
local "value" 3 mut
local "reference" 7 let
local "environment" 8 let
local "self" 9 let
local "output" 7 let
local "copied_environment" 8 let
local "result" 3 let
local "exclusive_parent" 10 let
@1 int 0 42
@2 borrow 7 mut 0
@2 borrow 1 shared 7 deref
@3 struct 2 1
@4 borrow 3 shared 2
@5 call 4 1 3
@6 read 5 copy 2
@7 drop 5
@8 drop-if 2 field 0
@9 read 6 copy 4 deref
@10 drop 4
@11 drop 2
@12 drop 7
@12 drop 0
@13 return 6
endfn
fn "borrow_capture" 1 7
return-leaf 0 bound 0 deref field 0
local "self" 9 let
local "result" 7 let
@20 borrow 1 shared 0 deref field 0 deref
@21 drop 0
@22 return 1
endfn
