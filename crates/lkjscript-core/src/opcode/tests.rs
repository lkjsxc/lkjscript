#[cfg(test)]
use super::{ControlFlow, Op, StackEffect};

#[test]
fn every_known_opcode_has_truthful_metadata_and_round_trips() {
    let mut seen = [false; 256];
    for op in Op::ALL {
        let byte = *op as u8;
        assert!(!seen[usize::from(byte)]);
        seen[usize::from(byte)] = true;
        assert_eq!(Op::from_byte(byte), Some(*op));
        assert!(op.operand_width() <= 2);
    }
    assert_eq!(Op::from_byte(21), None);
    assert_eq!(Op::from_byte(85), None);
    assert_eq!(Op::from_byte(145), None);
    assert_eq!(Op::Jump.info().control, ControlFlow::Jump);
    assert_eq!(Op::Return.info().control, ControlFlow::Return);
    assert_eq!(Op::Call.info().stack, StackEffect::Call);
    assert_eq!(Op::MakeProduct.info().stack, StackEffect::MakeProduct);
}
