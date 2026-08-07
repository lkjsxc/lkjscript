fn list_node(vm: &Vm<'_>, value: Value) -> Result<Option<(Value, Value)>> {
    vm.list_view(value)
}

pub(crate) fn list_values_equal(
    vm: &mut Vm<'_>,
    mut left: Value,
    mut right: Value,
) -> Result<bool> {
    loop {
        let left_node = list_node(vm, left)?;
        let right_node = list_node(vm, right)?;
        let (left_car, left_cdr, right_car, right_cdr) = match (left_node, right_node) {
            (None, None) => return Ok(true),
            (None, Some(_)) | (Some(_), None) => return Ok(false),
            (Some((left_car, left_cdr)), Some((right_car, right_cdr))) => {
                (left_car, left_cdr, right_car, right_cdr)
            }
        };
        if !value_equal(vm, left_car, right_car)? {
            return Ok(false);
        }
        left = left_cdr;
        right = right_cdr;
    }
}

pub(crate) fn list_equal(vm: &mut Vm<'_>) -> Result<()> {
    let right = vm.pop()?;
    let left = vm.pop()?;
    let equal = list_values_equal(vm, left, right)?;
    vm.push(Value::from_bool(equal));
    Ok(())
}
