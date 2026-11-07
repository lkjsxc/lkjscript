//! Allocation-safe decoding of flat expression records.

use super::*;

linear_record!(ExpressionRecord {
    contract_version = Decode::decode,
    id = Decode::decode,
    operation = ExpressionOperation::decode_linear,
});
linear_record!(RecordExpressionField {
    selector = FieldSelector::decode_linear,
    value = Decode::decode,
});
linear_record!(OwnedProductExpressionField {
    name = Name::decode_linear,
    value = Decode::decode,
});
linear_record!(OwnedProductBinding {
    name = Name::decode_linear,
    binding = Decode::decode,
});
linear_record!(OwnedChoiceArm {
    name = Name::decode_linear,
    binding = Decode::decode,
    body = Decode::decode,
});

impl TextValue {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        match u32::decode(decoder)? {
            0 => Ok(Self::Inline {
                text: decode_string(decoder)?,
            }),
            1 => Ok(Self::Blob {
                digest: Decode::decode(decoder)?,
                bytes: Decode::decode(decoder)?,
            }),
            _ => Err(DecodeError::Other("unknown text storage tag")),
        }
    }
}

impl FieldSelector {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        match u32::decode(decoder)? {
            0 => Ok(Self::Nominal(Decode::decode(decoder)?)),
            1 => Ok(Self::Structural(Name::decode_linear(decoder)?)),
            _ => Err(DecodeError::Other("unknown field selector tag")),
        }
    }
}

impl ExpressionOperation {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        // Tags and field order are the existing derived Encode schema. Only
        // variable-field allocation changes; child expressions remain typed IDs.
        Ok(match u32::decode(decoder)? {
            0 => Self::Unit {},
            1 => Self::Bool {
                value: Decode::decode(decoder)?,
            },
            2 => Self::I64 {
                value: Decode::decode(decoder)?,
            },
            3 => Self::Text {
                value: TextValue::decode_linear(decoder)?,
            },
            4 => Self::StaticText {
                value: TextValue::decode_linear(decoder)?,
            },
            5 => Self::Local {
                value: Decode::decode(decoder)?,
            },
            6 => Self::Constant {
                declaration: Decode::decode(decoder)?,
            },
            7 => Self::If {
                condition: Decode::decode(decoder)?,
                when_true: Decode::decode(decoder)?,
                when_false: Decode::decode(decoder)?,
            },
            8 => Self::Let {
                bindings: decode_vec(decoder)?,
                body: Decode::decode(decoder)?,
            },
            9 => Self::Sequence {
                items: decode_vec(decoder)?,
            },
            10 => Self::Call {
                requirement_arguments: decode_vec(decoder)?,
                effect_arguments: decode_vec_with(decoder, EffectRow::decode_linear)?,
                function: Decode::decode(decoder)?,
                type_arguments: decode_vec(decoder)?,
                read_scope_arguments: decode_vec(decoder)?,
                arguments: decode_vec(decoder)?,
            },
            11 => Self::FunctionValue {
                requirement_arguments: decode_vec(decoder)?,
                effect_arguments: decode_vec_with(decoder, EffectRow::decode_linear)?,
                function: Decode::decode(decoder)?,
                type_arguments: decode_vec(decoder)?,
                read_scope_arguments: decode_vec(decoder)?,
            },
            12 => Self::Invoke {
                callee: Decode::decode(decoder)?,
                arguments: decode_vec(decoder)?,
            },
            13 => Self::Record {
                nominal_type: Decode::decode(decoder)?,
                type_arguments: decode_vec(decoder)?,
                fields: decode_vec_with(decoder, RecordExpressionField::decode_linear)?,
            },
            14 => Self::Variant {
                case: Decode::decode(decoder)?,
                type_arguments: decode_vec(decoder)?,
                payload: Decode::decode(decoder)?,
            },
            15 => Self::Field {
                value: Decode::decode(decoder)?,
                selector: FieldSelector::decode_linear(decoder)?,
            },
            16 => Self::List {
                item_type: Decode::decode(decoder)?,
                items: decode_vec(decoder)?,
            },
            17 => Self::Map {
                key_type: Decode::decode(decoder)?,
                value_type: Decode::decode(decoder)?,
                entries: decode_vec(decoder)?,
            },
            18 => Self::Match {
                value: Decode::decode(decoder)?,
                arms: decode_vec(decoder)?,
            },
            19 => Self::CapabilityCall {
                requirement: Decode::decode(decoder)?,
                operation: Decode::decode(decoder)?,
                arguments: decode_vec(decoder)?,
            },
            20 => Self::Transaction {
                requirement: Decode::decode(decoder)?,
                binding: Decode::decode(decoder)?,
                body: Decode::decode(decoder)?,
            },
            21 => Self::Bind {
                callee: Decode::decode(decoder)?,
                arguments: decode_vec(decoder)?,
            },
            22 => Self::TransactionOutcome {
                requirement: Decode::decode(decoder)?,
                binding: Decode::decode(decoder)?,
                body: Decode::decode(decoder)?,
                outcome: Decode::decode(decoder)?,
                type_argument: Decode::decode(decoder)?,
            },
            23 => Self::F64 {
                value: Decode::decode(decoder)?,
            },
            24 => Self::ImplementationCall {
                requirement_arguments: decode_vec(decoder)?,
                effect_arguments: decode_vec_with(decoder, EffectRow::decode_linear)?,
                function: Decode::decode(decoder)?,
                type_arguments: decode_vec(decoder)?,
                read_scope_arguments: decode_vec(decoder)?,
                implementations: decode_vec_with(decoder, ImplementationOperand::decode_linear)?,
                arguments: decode_vec(decoder)?,
            },
            25 => Self::MethodCall {
                witness: ImplementationOperand::decode_linear(decoder)?,
                contract: Decode::decode(decoder)?,
                method: Decode::decode(decoder)?,
                read_scope_arguments: decode_vec(decoder)?,
                arguments: decode_vec(decoder)?,
            },
            26 => Self::PackOwned {
                product_type: Decode::decode(decoder)?,
                fields: decode_vec_with(decoder, OwnedProductExpressionField::decode_linear)?,
            },
            27 => Self::UnpackOwned {
                product_type: Decode::decode(decoder)?,
                source: Decode::decode(decoder)?,
                fields: decode_vec_with(decoder, OwnedProductBinding::decode_linear)?,
                body: Decode::decode(decoder)?,
            },
            28 => Self::ChooseOwned {
                choice_type: Decode::decode(decoder)?,
                case: Name::decode_linear(decoder)?,
                value: Decode::decode(decoder)?,
            },
            29 => Self::MatchOwned {
                choice_type: Decode::decode(decoder)?,
                source: Decode::decode(decoder)?,
                arms: decode_vec_with(decoder, OwnedChoiceArm::decode_linear)?,
            },
            30 => Self::Parallel {
                left: Decode::decode(decoder)?,
                right: Decode::decode(decoder)?,
            },
            31 => Self::BorrowOwnedField {
                product_type: Decode::decode(decoder)?,
                source: Decode::decode(decoder)?,
                field: Name::decode_linear(decoder)?,
                binding: Decode::decode(decoder)?,
                body: Decode::decode(decoder)?,
            },
            32 => Self::MatchBorrowedOwned {
                choice_type: Decode::decode(decoder)?,
                source: Decode::decode(decoder)?,
                arms: decode_vec_with(decoder, OwnedChoiceArm::decode_linear)?,
            },
            33 => Self::SequenceEmpty {
                sequence_type: Decode::decode(decoder)?,
            },
            34 => Self::SequenceLength {
                sequence_type: Decode::decode(decoder)?,
                source: Decode::decode(decoder)?,
            },
            35 => Self::SequencePush {
                sequence_type: Decode::decode(decoder)?,
                value: Decode::decode(decoder)?,
                source: Decode::decode(decoder)?,
            },
            36 => Self::SequencePop {
                sequence_type: Decode::decode(decoder)?,
                result_type: Decode::decode(decoder)?,
                source: Decode::decode(decoder)?,
            },
            37 => Self::BorrowOwnedItem {
                sequence_type: Decode::decode(decoder)?,
                source: Decode::decode(decoder)?,
                index: Decode::decode(decoder)?,
                binding: Decode::decode(decoder)?,
                body: Decode::decode(decoder)?,
            },
            38 => Self::BorrowCall {
                call: Decode::decode(decoder)?,
                binding: Decode::decode(decoder)?,
                body: Decode::decode(decoder)?,
            },
            39 => Self::SequenceGet {
                sequence_type: Decode::decode(decoder)?,
                source: Decode::decode(decoder)?,
                index: Decode::decode(decoder)?,
            },
            40 => Self::SequenceReplace {
                sequence_type: Decode::decode(decoder)?,
                result_type: Decode::decode(decoder)?,
                index: Decode::decode(decoder)?,
                value: Decode::decode(decoder)?,
                source: Decode::decode(decoder)?,
            },
            41 => Self::PackOwnedRecord {
                record_type: Decode::decode(decoder)?,
                fields: decode_vec(decoder)?,
            },
            42 => Self::UnpackOwnedRecord {
                record_type: Decode::decode(decoder)?,
                source: Decode::decode(decoder)?,
                fields: decode_vec(decoder)?,
                body: Decode::decode(decoder)?,
            },
            43 => Self::BorrowOwnedRecordField {
                record_type: Decode::decode(decoder)?,
                source: Decode::decode(decoder)?,
                field: Decode::decode(decoder)?,
                binding: Decode::decode(decoder)?,
                body: Decode::decode(decoder)?,
            },
            44 => Self::CreateReadView {
                declaration: Decode::decode(decoder)?,
                type_arguments: decode_vec(decoder)?,
                read_scope: Decode::decode(decoder)?,
                source: Decode::decode(decoder)?,
                fields: decode_vec(decoder)?,
            },
            45 => Self::BorrowReadView {
                value: Decode::decode(decoder)?,
                binding: Decode::decode(decoder)?,
                body: Decode::decode(decoder)?,
            },
            46 => Self::ReborrowReadView {
                value: Decode::decode(decoder)?,
                read_scope: Decode::decode(decoder)?,
            },
            47 => Self::MoveReadView {
                value: Decode::decode(decoder)?,
            },
            48 => Self::DropReadView {
                value: Decode::decode(decoder)?,
            },
            49 => Self::ReadViewDataField {
                value: Decode::decode(decoder)?,
                field: Decode::decode(decoder)?,
            },
            50 => Self::BorrowReadViewField {
                value: Decode::decode(decoder)?,
                field: Decode::decode(decoder)?,
                binding: Decode::decode(decoder)?,
                body: Decode::decode(decoder)?,
            },
            51 => Self::Region {
                // The accepted RegionExpression contains only fixed scalar,
                // typed identity and reference fields, with no collections.
                operation: Decode::decode(decoder)?,
            },
            _ => return Err(DecodeError::Other("unknown expression operation tag")),
        })
    }
}
