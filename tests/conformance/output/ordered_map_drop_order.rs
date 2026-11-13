use std::collections::BTreeMap;
struct Mark {
    out: Stdout,
    byte: u8,
}
impl Drop for Mark {
    fn drop(&mut self) {
        self.out.write(&[self.byte]);
    }
}
pub fn main(mut world: World) {
    let mut map: BTreeMap<i64, Mark> = BTreeMap::new();
    map.insert(
        3,
        Mark {
            out: world.stdout(),
            byte: 3u8,
        },
    );
    map.insert(
        1,
        Mark {
            out: world.stdout(),
            byte: 1u8,
        },
    );
    map.insert(
        4,
        Mark {
            out: world.stdout(),
            byte: 4u8,
        },
    );
    map.insert(
        2,
        Mark {
            out: world.stdout(),
            byte: 2u8,
        },
    );
    drop(map.remove(&3).unwrap());
    drop(
        map.insert(
            2,
            Mark {
                out: world.stdout(),
                byte: 22u8,
            },
        )
        .unwrap(),
    );
    map.clear();
    map.insert(
        5,
        Mark {
            out: world.stdout(),
            byte: 5u8,
        },
    );
    map.insert(
        0,
        Mark {
            out: world.stdout(),
            byte: 0u8,
        },
    );
    drop(map);
    let mut output = world.stdout();
    output.write(&[99u8]);
}
