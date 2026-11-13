fn forward<'a>(value: &'a str) -> &'a str {
    value
}

pub fn probe() -> i64 {
    let retained = &forward("keeper");
    {
        let view0 = "A";
        let view1 = "BB";
        let view2 = "CCC";
        let view3 = "DDDD";
        let view4 = "EEEEE";
        let view5 = "FFFFFF";
        let view6 = "GGGGGGG";
        let view7 = "HHHHHHHH";
        let view8 = "IIIIIIIII";
        let view9 = "JJJJJJJJJJ";
        let view10 = "KKKKKKKKKKK";
        let view11 = "LLLLLLLLLLLL";
        let view12 = "MMMMMMMMMMMMM";
        let view13 = "NNNNNNNNNNNNNN";
        let view14 = "OOOOOOOOOOOOOOO";
        let view15 = "PPPPPPPPPPPPPPPP";
        let view16 = "QQQQQQQQQQQQQQQQQ";
        let view17 = "RRRRRRRRRRRRRRRRRR";
        let view18 = "SSSSSSSSSSSSSSSSSSS";
        let view19 = "TTTTTTTTTTTTTTTTTTTT";
        let view20 = "UUUUUUUUUUUUUUUUUUUUU";
        let view21 = "VVVVVVVVVVVVVVVVVVVVVV";
        let view22 = "WWWWWWWWWWWWWWWWWWWWWWW";
        let view23 = "XXXXXXXXXXXXXXXXXXXXXXXX";
        let view24 = "YYYYYYYYYYYYYYYYYYYYYYYYY";
        let view25 = "ZZZZZZZZZZZZZZZZZZZZZZZZZZ";
        let view26 = "AAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let view27 = "BBBBBBBBBBBBBBBBBBBBBBBBBBBB";
        let view28 = "CCCCCCCCCCCCCCCCCCCCCCCCCCCCC";
        let view29 = "DDDDDDDDDDDDDDDDDDDDDDDDDDDDDD";
        let view30 = "EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE";
        let view31 = "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF";
        let view32 = "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG";
        let view33 = "HHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHH";
        let view34 = "IIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIII";
        let view35 = "JJJJJJJJJJJJJJJJJJJJJJJJJJJJJJJJJJJJ";
        let view36 = "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK";
        let view37 = "LLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLL";
        let view38 = "MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM";
        let view39 = "NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN";
        assert!(view0.len() == 1 && view0.as_bytes()[0] == 65u8);
        assert!(view1.len() == 2 && view1.as_bytes()[0] == 66u8);
        assert!(view2.len() == 3 && view2.as_bytes()[0] == 67u8);
        assert!(view3.len() == 4 && view3.as_bytes()[0] == 68u8);
        assert!(view4.len() == 5 && view4.as_bytes()[0] == 69u8);
        assert!(view5.len() == 6 && view5.as_bytes()[0] == 70u8);
        assert!(view6.len() == 7 && view6.as_bytes()[0] == 71u8);
        assert!(view7.len() == 8 && view7.as_bytes()[0] == 72u8);
        assert!(view8.len() == 9 && view8.as_bytes()[0] == 73u8);
        assert!(view9.len() == 10 && view9.as_bytes()[0] == 74u8);
        assert!(view10.len() == 11 && view10.as_bytes()[0] == 75u8);
        assert!(view11.len() == 12 && view11.as_bytes()[0] == 76u8);
        assert!(view12.len() == 13 && view12.as_bytes()[0] == 77u8);
        assert!(view13.len() == 14 && view13.as_bytes()[0] == 78u8);
        assert!(view14.len() == 15 && view14.as_bytes()[0] == 79u8);
        assert!(view15.len() == 16 && view15.as_bytes()[0] == 80u8);
        assert!(view16.len() == 17 && view16.as_bytes()[0] == 81u8);
        assert!(view17.len() == 18 && view17.as_bytes()[0] == 82u8);
        assert!(view18.len() == 19 && view18.as_bytes()[0] == 83u8);
        assert!(view19.len() == 20 && view19.as_bytes()[0] == 84u8);
        assert!(view20.len() == 21 && view20.as_bytes()[0] == 85u8);
        assert!(view21.len() == 22 && view21.as_bytes()[0] == 86u8);
        assert!(view22.len() == 23 && view22.as_bytes()[0] == 87u8);
        assert!(view23.len() == 24 && view23.as_bytes()[0] == 88u8);
        assert!(view24.len() == 25 && view24.as_bytes()[0] == 89u8);
        assert!(view25.len() == 26 && view25.as_bytes()[0] == 90u8);
        assert!(view26.len() == 27 && view26.as_bytes()[0] == 65u8);
        assert!(view27.len() == 28 && view27.as_bytes()[0] == 66u8);
        assert!(view28.len() == 29 && view28.as_bytes()[0] == 67u8);
        assert!(view29.len() == 30 && view29.as_bytes()[0] == 68u8);
        assert!(view30.len() == 31 && view30.as_bytes()[0] == 69u8);
        assert!(view31.len() == 32 && view31.as_bytes()[0] == 70u8);
        assert!(view32.len() == 33 && view32.as_bytes()[0] == 71u8);
        assert!(view33.len() == 34 && view33.as_bytes()[0] == 72u8);
        assert!(view34.len() == 35 && view34.as_bytes()[0] == 73u8);
        assert!(view35.len() == 36 && view35.as_bytes()[0] == 74u8);
        assert!(view36.len() == 37 && view36.as_bytes()[0] == 75u8);
        assert!(view37.len() == 38 && view37.as_bytes()[0] == 76u8);
        assert!(view38.len() == 39 && view38.as_bytes()[0] == 77u8);
        assert!(view39.len() == 40 && view39.as_bytes()[0] == 78u8);
    }
    let reused_a = forward("abc");
    let reused_b = forward("thirteenbytes");
    let reused_c = forward("λ🦀");
    assert!(reused_a.len() == 3 && reused_a.as_bytes()[2] == 99);
    assert!(reused_b.len() == 13 && reused_b.as_bytes()[12] == 115);
    assert!(reused_c.len() == 6 && reused_c.as_bytes()[0] == 206);
    assert!(retained.len() == 6 && retained.as_bytes()[5] == 114);
    retained.len() as i64 * 7
}
