// expect: output 6c697374656e6572206f6b0a
// witness: {}
fn errno<T>(value: Result<T, i64>, expected: i64) {
    match value {
        Err(error) => assert_eq!(error, expected),
        Ok(_) => panic!("expected an I/O error"),
    }
}

fn descriptors(world: &mut World) -> usize {
    let mut directory = world.directory("/proc/self/fd").unwrap();
    directory.list().unwrap().len()
}

pub fn main(mut world: World) {
    let addresses: Vec<String> = vec![
        "".to_string(), "localhost".to_string(), "127.0.0".to_string(),
        "127.0.0.1.2".to_string(), "127..0.1".to_string(), "256.0.0.1".to_string(),
        "-1.0.0.1".to_string(), "127.0.0.1\0".to_string(),
        "127.0.0.1:80".to_string(), "127.0.0.1.".to_string(),
    ];
    for address in addresses {
        errno(world.listen(address.as_str(), 0u16), 22);
    }
    errno(world.__lkjscript_listen(&[2u8, 0u8]), 22);
    let mut raw = [0u8; 16];
    errno(world.__lkjscript_listen(&raw), 22);
    raw[0] = 2;
    raw[4] = 127;
    raw[7] = 1;
    let port = {
        let mut listener = world.__lkjscript_listen(&raw).unwrap();
        let port = listener.port().unwrap();
        assert!(port != 0);
        let before = descriptors(&mut world);
        for _ in 0..64usize {
            errno(world.listen("127.0.0.1", port), 98);
        }
        assert_eq!(descriptors(&mut world), before);
        let mut short = [17u8; 15];
        assert_eq!(listener.__lkjscript_address(&mut short), -22);
        assert_eq!(short, [17u8; 15]);
        let mut long = [19u8; 17];
        assert_eq!(listener.__lkjscript_address(&mut long), -22);
        assert_eq!(long, [19u8; 17]);
        assert_eq!(listener.__lkjscript_address(&mut raw), 0);
        assert_eq!(&raw[0..2], &[2u8, 0u8]);
        assert_eq!(&raw[4..8], &[127u8, 0u8, 0u8, 1u8]);
        assert_eq!(((raw[2] as u16) << 8u64) | raw[3] as u16, port);
        port
    };
    {
        let mut listener = world.listen("127.000.0.1", port).unwrap();
        assert_eq!(listener.port().unwrap(), port);
    }
    for _ in 0..128usize {
        let mut listener = world.listen("127.0.0.1", port).unwrap();
        assert_eq!(listener.port().unwrap(), port);
    }
    let mut output = world.stdout();
    output.write(b"listener ok\n");
}
