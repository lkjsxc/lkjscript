type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "Ref" ref shared 3
type "Numbers" struct move "left" 3 "right" 3
type "Holder" struct move "left" 7 "right" 7
type "HolderMut" ref mut 9
type "SlotMut" ref mut 7
destructor 9 1
fn "choose" 1 11 from 0
local "holder" 10 let
local "result" 11 let
@1 borrow 1 mut 0 deref field 1
@2 drop 0
@3 return 1
endfn
fn "drop_holder" 1 0
local "self" 10 let
local "value" 3 let
local "one" 3 let
local "checked" 3 let
local "unit" 0 let
@1 read 1 copy 0 deref field 0 deref
@2 int 2 1
@3 binary 3 "/" 2 1
@4 drop 3
@5 drop 0
@6 unit 4
@7 return 4
endfn
fn "probe" 0 0
local "one" 3 let
local "two" 3 let
local "values" 8 mut
local "left" 7 let
local "right" 7 let
local "holder" 9 mut
local "carrier" 10 let
local "slot" 11 let
local "replacement" 7 let
local "old" 7 let
local "zero" 3 let
local "unit" 0 let
@1 int 0 1
@2 int 1 2
@3 struct 2 0 1
@4 borrow 3 shared 2 field 0
@5 borrow 4 shared 2 field 1
@6 struct 5 3 4
@7 borrow 6 mut 5
@8 call 7 0 6
@9 borrow 8 shared 2 field 1
@10 intrinsic 9 "replace" 7 8
@11 drop 9
@12 int 10 0
@13 assign 10 2 field 0
@14 drop 5
@15 drop 2
@16 unit 11
@17 return 11
endfn
