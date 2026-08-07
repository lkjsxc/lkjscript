fn place_and_slot(vm: &mut Vm<'_>) -> Result<(usize, usize)> {
    vm.read_place_local()
}

pub(super) fn clear_consumed_owner(vm: &mut Vm<'_>, owner: u64) {
    let Some(frame) = vm.frames.last_mut() else {
        return;
    };
    for place in &mut frame.unique_places {
        if matches!(
            place,
            unique::RuntimePlace::Active {
                owner: Some(actual),
                ..
            } | unique::RuntimePlace::Active {
                transferred: Some(actual),
                ..
            } if *actual == owner
        ) {
            *place = unique::RuntimePlace::Active {
                owner: None,
                transferred: None,
            };
        }
    }
}

fn current_places<'vm>(vm: &'vm Vm<'_>) -> &'vm [unique::RuntimePlace] {
    vm.frames
        .last()
        .map(|frame| frame.unique_places.as_slice())
        .unwrap_or_default()
}

fn place_mut<'vm>(
    vm: &'vm mut Vm<'_>,
    place: usize,
) -> Result<&'vm mut unique::RuntimePlace> {
    vm.frames
        .last_mut()
        .and_then(|frame| frame.unique_places.get_mut(place))
        .ok_or_else(|| Error::msg("structural place index is out of range"))
}

fn require_place(vm: &Vm<'_>, place: usize, owner: u64) -> Result<()> {
    if current_places(vm).get(place)
        == Some(&unique::RuntimePlace::Active {
            owner: Some(owner),
            transferred: None,
        })
    {
        Ok(())
    } else {
        Err(Error::msg("structural operation names a stale place owner"))
    }
}
