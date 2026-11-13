type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "Choice" enum copy
variant "A" "value" 3
variant "B" "value" 3
endtype
type "Choices" array 7 2
type "&Choice" ref shared 7
fn "probe" 0 3
local "left" 3 let
local "right" 3 let
local "a" 7 let
local "b" 7 let
local "choices" 8 let
local "index" 4 mut
local "zero" 4 let
local "condition" 1 let
local "one" 4 let
local "result" 3 let
local "borrowed" 9 let
@1 int 0 42
@2 int 1 22
@3 variant 2 0 0
@4 variant 3 1 1
@5 array 4 2 3
@6 int 6 0
@7 assign 6 5
@8 borrow 10 shared 4 index 5
@8 tag-test 7 10 0
@9 int 8 1
@10 assign 8 5
@11 if 7
@12 read 9 copy 10 deref variant 0 0
@13 drop 10
@13 drop 4
@14 drop 5
@14 return 9
@15 else
@16 trap "unreachable"
@17 end
endfn
