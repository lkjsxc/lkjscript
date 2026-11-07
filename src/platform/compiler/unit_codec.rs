//! Progressive compiler-unit decoding. Every variable collection, including nested
//! kernel records, consumes encoded elements before fallible capacity growth.
//! The existing Encode implementations remain the canonical wire definition.

use super::*;
use crate::platform::packed::linear::{LinearDecode, decode_string, decode_vec, decode_vec_with};
use bincode::de::Decoder;
use bincode::error::{AllowedEnumVariants, DecodeError};

pub(super) struct UnitWire(pub(super) CompilationUnit);

impl Encode for UnitWire {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        Encode::encode(&self.0, encoder)
    }
}

impl<Context> Decode<Context> for UnitWire {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        CompilationUnit::decode_linear(decoder).map(Self)
    }
}

impl LinearDecode for UnitWire {}

trait DecodeLinear: Sized {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError>;
}

impl DecodeLinear for CompilationUnit {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            contract_version: Decode::decode(decoder)?,
            graph_contract_version: Decode::decode(decoder)?,
            bytecode_contract_version: Decode::decode(decoder)?,
            key: Decode::decode(decoder)?,
            source: Decode::decode(decoder)?,
            optimization: Decode::decode(decoder)?,
            tables: CompilationTables::decode_linear(decoder)?,
            payload: CompilationPayload::decode_linear(decoder)?,
        })
    }
}

impl DecodeLinear for CompilationTables {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            declarations: decode_vec(decoder)?,
            fields: decode_vec(decoder)?,
            cases: decode_vec(decoder)?,
            requirements: decode_vec(decoder)?,
            operations: decode_vec(decoder)?,
            ports: decode_vec(decoder)?,
            types: decode_vec(decoder)?,
            structural_names: decode_vec_with(decoder, Name::decode_linear)?,
            read_view_fields: decode_vec(decoder)?,
            texts: decode_vec_with(decoder, CompiledText::decode_linear)?,
        })
    }
}

impl DecodeLinear for CompiledHttpRoute {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            route: Decode::decode(decoder)?,
            method: decode_string(decoder)?,
            selector: crate::platform::kernel::HttpRouteSelector::decode_linear(decoder)?,
            port: Decode::decode(decoder)?,
            capture_parameters: decode_vec(decoder)?,
        })
    }
}

impl DecodeLinear for CompiledSignature {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            implementation_parameters: decode_vec_with(
                decoder,
                crate::platform::kernel::ImplementationParameter::decode_linear,
            )?,
            requirement_parameters: decode_vec(decoder)?,
            effect_parameters: decode_vec(decoder)?,
            effect: crate::platform::kernel::FunctionEffect::decode_linear(decoder)?,
            type_parameters: decode_vec(decoder)?,
            type_parameter_constraints: decode_vec(decoder)?,
            parameters: decode_vec_with(decoder, CompiledParameter::decode_linear)?,
            result: Decode::decode(decoder)?,
            read_scope_parameters: decode_vec_with(
                decoder,
                crate::platform::kernel::ReadScopeParameter::decode_linear,
            )?,
            read_scope_sources: decode_vec(decoder)?,
            task_requirements: decode_vec(decoder)?,
        })
    }
}

impl DecodeLinear for CompiledParameter {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            parameter: Decode::decode(decoder)?,
            name: Name::decode_linear(decoder)?,
            ty: Decode::decode(decoder)?,
            use_mode: Decode::decode(decoder)?,
            resource_requirement: Decode::decode(decoder)?,
        })
    }
}

impl DecodeLinear for CompiledOperationLayout {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            operation: Decode::decode(decoder)?,
            parameters: decode_vec_with(decoder, CompiledParameter::decode_linear)?,
            result: Decode::decode(decoder)?,
            idempotency: Decode::decode(decoder)?,
            external_visibility: Decode::decode(decoder)?,
        })
    }
}

impl DecodeLinear for CompiledRequirement {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            requirement: Decode::decode(decoder)?,
            interface: Decode::decode(decoder)?,
            operations: decode_vec(decoder)?,
            limits: decode_vec_with(decoder, ResourceLimit::decode_linear)?,
        })
    }
}

impl DecodeLinear for CompiledPort {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            port: Decode::decode(decoder)?,
            function_type: Decode::decode(decoder)?,
            implementation: CompiledPortImplementation::decode_linear(decoder)?,
        })
    }
}

impl DecodeLinear for CompiledCode {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            parameter_count: Decode::decode(decoder)?,
            local_count: Decode::decode(decoder)?,
            instructions: decode_vec_with(decoder, CompiledInstruction::decode_linear)?,
        })
    }
}

impl DecodeLinear for CompiledText {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        match u32::decode(decoder)? {
            0 => Ok(Self::Inline(decode_string(decoder)?)),
            1 => Ok(Self::Blob {
                digest: Decode::decode(decoder)?,
                bytes: Decode::decode(decoder)?,
            }),
            found => Err(DecodeError::UnexpectedVariant {
                type_name: "CompiledText",
                allowed: &AllowedEnumVariants::Range { min: 0, max: 1 },
                found,
            }),
        }
    }
}

impl DecodeLinear for CompilationPayload {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        match u32::decode(decoder)? {
            0 => Ok(Self::ReadView {
                declaration: crate::platform::kernel::ReadViewDeclaration::decode_linear(decoder)?,
                type_parameter_constraints: decode_vec(decoder)?,
            }),
            1 => Ok(Self::OwnedContract(
                crate::platform::kernel::OwnedContract::decode_linear(decoder)?,
            )),
            2 => Ok(Self::OwnedImplementation(
                crate::platform::kernel::OwnedImplementation::decode_linear(decoder)?,
            )),
            3 => Ok(Self::Record {
                type_parameters: decode_vec(decoder)?,
                type_parameter_constraints: decode_vec(decoder)?,
                fields: decode_vec(decoder)?,
            }),
            4 => Ok(Self::Variant {
                type_parameters: decode_vec(decoder)?,
                type_parameter_constraints: decode_vec(decoder)?,
                cases: decode_vec(decoder)?,
            }),
            5 => Ok(Self::Interface {
                operations: decode_vec_with(decoder, CompiledOperationLayout::decode_linear)?,
            }),
            6 => Ok(Self::External {
                signature: CompiledSignature::decode_linear(decoder)?,
                implementation: ImplementationName::decode_linear(decoder)?,
            }),
            7 => Ok(Self::Function {
                signature: CompiledSignature::decode_linear(decoder)?,
                code: CompiledCode::decode_linear(decoder)?,
            }),
            8 => Ok(Self::Constant {
                ty: Decode::decode(decoder)?,
                code: CompiledCode::decode_linear(decoder)?,
            }),
            9 => Ok(Self::Component {
                requirements: decode_vec_with(decoder, CompiledRequirement::decode_linear)?,
                ports: decode_vec_with(decoder, CompiledPort::decode_linear)?,
            }),
            10 => Ok(Self::Test {
                actual: CompiledCode::decode_linear(decoder)?,
                expected: CompiledCode::decode_linear(decoder)?,
                comparison: Decode::decode(decoder)?,
            }),
            11 => Ok(Self::Target {
                component: Decode::decode(decoder)?,
                port: Decode::decode(decoder)?,
                routes: decode_vec_with(decoder, CompiledHttpRoute::decode_linear)?,
                runner: Decode::decode(decoder)?,
            }),
            12 => Ok(Self::OwnedRecord {
                type_parameters: decode_vec(decoder)?,
                type_parameter_constraints: decode_vec(decoder)?,
                fields: decode_vec(decoder)?,
            }),
            found => Err(DecodeError::UnexpectedVariant {
                type_name: "CompilationPayload",
                allowed: &AllowedEnumVariants::Range { min: 0, max: 12 },
                found,
            }),
        }
    }
}

impl DecodeLinear for CompiledPortImplementation {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        match u32::decode(decoder)? {
            0 => Ok(Self::Function(Decode::decode(decoder)?)),
            1 => Ok(Self::Expression(CompiledCode::decode_linear(decoder)?)),
            found => Err(DecodeError::UnexpectedVariant {
                type_name: "CompiledPortImplementation",
                allowed: &AllowedEnumVariants::Range { min: 0, max: 1 },
                found,
            }),
        }
    }
}

impl DecodeLinear for CompiledInstruction {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        match u32::decode(decoder)? {
            0 => Ok(Self::ImplementationCall {
                requirement_arguments: decode_vec(decoder)?,
                effect_arguments: decode_vec_with(
                    decoder,
                    crate::platform::kernel::EffectRow::decode_linear,
                )?,
                function: Decode::decode(decoder)?,
                type_arguments: decode_vec(decoder)?,
                implementations: decode_vec_with(
                    decoder,
                    crate::platform::kernel::ImplementationOperand::decode_linear,
                )?,
                arguments: Decode::decode(decoder)?,
                read_scope_arguments: decode_vec(decoder)?,
            }),
            1 => Ok(Self::MethodCall {
                witness: crate::platform::kernel::ImplementationOperand::decode_linear(decoder)?,
                contract: Decode::decode(decoder)?,
                method: Decode::decode(decoder)?,
                arguments: Decode::decode(decoder)?,
                read_scope_arguments: decode_vec(decoder)?,
            }),
            2 => Ok(Self::Unit),
            3 => Ok(Self::Bool(Decode::decode(decoder)?)),
            4 => Ok(Self::I64(Decode::decode(decoder)?)),
            5 => Ok(Self::Text(Decode::decode(decoder)?)),
            6 => Ok(Self::StaticText(Decode::decode(decoder)?)),
            7 => Ok(Self::LoadLocal {
                local: Decode::decode(decoder)?,
                use_mode: Decode::decode(decoder)?,
            }),
            8 => Ok(Self::StoreLocal(Decode::decode(decoder)?)),
            9 => Ok(Self::Drop),
            10 => Ok(Self::JumpIfFalse(Decode::decode(decoder)?)),
            11 => Ok(Self::Jump(Decode::decode(decoder)?)),
            12 => Ok(Self::Call {
                requirement_arguments: decode_vec(decoder)?,
                effect_arguments: decode_vec_with(
                    decoder,
                    crate::platform::kernel::EffectRow::decode_linear,
                )?,
                function: Decode::decode(decoder)?,
                type_arguments: decode_vec(decoder)?,
                arguments: Decode::decode(decoder)?,
                read_scope_arguments: decode_vec(decoder)?,
            }),
            13 => Ok(Self::FunctionValue {
                requirement_arguments: decode_vec(decoder)?,
                effect_arguments: decode_vec_with(
                    decoder,
                    crate::platform::kernel::EffectRow::decode_linear,
                )?,
                function: Decode::decode(decoder)?,
                type_arguments: decode_vec(decoder)?,
                read_scope_arguments: decode_vec(decoder)?,
            }),
            14 => Ok(Self::Invoke {
                arguments: Decode::decode(decoder)?,
            }),
            15 => Ok(Self::Record {
                nominal_type: Decode::decode(decoder)?,
                type_arguments: decode_vec(decoder)?,
                fields: decode_vec(decoder)?,
            }),
            16 => Ok(Self::Variant {
                case: Decode::decode(decoder)?,
                type_arguments: decode_vec(decoder)?,
                has_payload: Decode::decode(decoder)?,
            }),
            17 => Ok(Self::Field(Decode::decode(decoder)?)),
            18 => Ok(Self::List {
                item_type: Decode::decode(decoder)?,
                items: Decode::decode(decoder)?,
            }),
            19 => Ok(Self::Map {
                key_type: Decode::decode(decoder)?,
                value_type: Decode::decode(decoder)?,
                entries: Decode::decode(decoder)?,
            }),
            20 => Ok(Self::SwitchVariant(decode_vec(decoder)?)),
            21 => Ok(Self::Perform {
                requirement: Decode::decode(decoder)?,
                operation: Decode::decode(decoder)?,
                arguments: Decode::decode(decoder)?,
            }),
            22 => Ok(Self::BeginTransaction {
                requirement: Decode::decode(decoder)?,
                binding: Decode::decode(decoder)?,
            }),
            23 => Ok(Self::CommitTransaction {
                requirement: Decode::decode(decoder)?,
                binding: Decode::decode(decoder)?,
            }),
            24 => Ok(Self::PerformParameter {
                parameter: Decode::decode(decoder)?,
                operation: Decode::decode(decoder)?,
                arguments: Decode::decode(decoder)?,
            }),
            25 => Ok(Self::BeginParameterTransaction {
                parameter: Decode::decode(decoder)?,
                binding: Decode::decode(decoder)?,
            }),
            26 => Ok(Self::CommitParameterTransaction {
                parameter: Decode::decode(decoder)?,
                binding: Decode::decode(decoder)?,
            }),
            27 => Ok(Self::Return),
            28 => Ok(Self::Bind {
                arguments: Decode::decode(decoder)?,
            }),
            29 => Ok(Self::BeginBind {
                arguments: Decode::decode(decoder)?,
            }),
            30 => Ok(Self::Capture {
                index: Decode::decode(decoder)?,
            }),
            31 => Ok(Self::BeginTransactionOutcome {
                requirement: Decode::decode(decoder)?,
                binding: Decode::decode(decoder)?,
                outcome: Decode::decode(decoder)?,
            }),
            32 => Ok(Self::CommitTransactionOutcome {
                requirement: Decode::decode(decoder)?,
                binding: Decode::decode(decoder)?,
                outcome: Decode::decode(decoder)?,
            }),
            33 => Ok(Self::F64(Decode::decode(decoder)?)),
            34 => Ok(Self::PackOwned {
                product_type: Decode::decode(decoder)?,
                fields: decode_vec(decoder)?,
            }),
            35 => Ok(Self::UnpackOwned {
                product_type: Decode::decode(decoder)?,
                locals: decode_vec(decoder)?,
            }),
            36 => Ok(Self::ChooseOwned {
                choice_type: Decode::decode(decoder)?,
                case: Decode::decode(decoder)?,
            }),
            37 => Ok(Self::MatchOwned {
                choice_type: Decode::decode(decoder)?,
                cases: decode_vec(decoder)?,
            }),
            38 => Ok(Self::Parallel {
                left_types: decode_vec(decoder)?,
                left_implementations: decode_vec_with(
                    decoder,
                    crate::platform::kernel::ImplementationOperand::decode_linear,
                )?,
                left_read_scope_arguments: decode_vec(decoder)?,
                right_types: decode_vec(decoder)?,
                right_implementations: decode_vec_with(
                    decoder,
                    crate::platform::kernel::ImplementationOperand::decode_linear,
                )?,
                right_read_scope_arguments: decode_vec(decoder)?,
                left: Decode::decode(decoder)?,
                left_arguments: Decode::decode(decoder)?,
                right: Decode::decode(decoder)?,
                right_arguments: Decode::decode(decoder)?,
                result_type: Decode::decode(decoder)?,
            }),
            39 => Ok(Self::BorrowOwnedField {
                product_type: Decode::decode(decoder)?,
                source_local: Decode::decode(decoder)?,
                field: Decode::decode(decoder)?,
                binding_local: Decode::decode(decoder)?,
                binding_type: Decode::decode(decoder)?,
            }),
            40 => Ok(Self::MatchBorrowedOwned {
                choice_type: Decode::decode(decoder)?,
                source_local: Decode::decode(decoder)?,
                cases: decode_vec(decoder)?,
            }),
            41 => Ok(Self::EndOwnedBorrow {
                binding_local: Decode::decode(decoder)?,
            }),
            42 => Ok(Self::SequenceEmpty {
                sequence_type: Decode::decode(decoder)?,
            }),
            43 => Ok(Self::SequenceLength {
                sequence_type: Decode::decode(decoder)?,
                source_local: Decode::decode(decoder)?,
            }),
            44 => Ok(Self::SequencePush {
                sequence_type: Decode::decode(decoder)?,
                source_local: Decode::decode(decoder)?,
            }),
            45 => Ok(Self::SequencePop {
                sequence_type: Decode::decode(decoder)?,
                result_type: Decode::decode(decoder)?,
                source_local: Decode::decode(decoder)?,
            }),
            46 => Ok(Self::BorrowOwnedItem {
                sequence_type: Decode::decode(decoder)?,
                source_local: Decode::decode(decoder)?,
                binding_local: Decode::decode(decoder)?,
                binding_type: Decode::decode(decoder)?,
            }),
            47 => Ok(Self::BeginBorrowCall {
                source_local: Decode::decode(decoder)?,
                source_position: Decode::decode(decoder)?,
            }),
            48 => Ok(Self::AdoptBorrowResult {
                source_local: Decode::decode(decoder)?,
                binding_local: Decode::decode(decoder)?,
                binding_type: Decode::decode(decoder)?,
            }),
            49 => Ok(Self::ReturnBorrowed),
            50 => Ok(Self::SequenceGet {
                sequence_type: Decode::decode(decoder)?,
                source_local: Decode::decode(decoder)?,
            }),
            51 => Ok(Self::SequenceReplace {
                sequence_type: Decode::decode(decoder)?,
                result_type: Decode::decode(decoder)?,
                source_local: Decode::decode(decoder)?,
            }),
            52 => Ok(Self::BeginReadView {
                view_type: Decode::decode(decoder)?,
                source_local: Decode::decode(decoder)?,
                fields: decode_vec(decoder)?,
            }),
            53 => Ok(Self::FinishReadView {
                view_type: Decode::decode(decoder)?,
                fields: decode_vec(decoder)?,
            }),
            54 => Ok(Self::BorrowReadView {
                view_type: Decode::decode(decoder)?,
                source_local: Decode::decode(decoder)?,
                binding_local: Decode::decode(decoder)?,
                binding_type: Decode::decode(decoder)?,
            }),
            55 => Ok(Self::EndReadViewBorrow {
                binding_local: Decode::decode(decoder)?,
            }),
            56 => Ok(Self::ReborrowReadView {
                view_type: Decode::decode(decoder)?,
                source_local: Decode::decode(decoder)?,
            }),
            57 => Ok(Self::MoveReadView {
                view_type: Decode::decode(decoder)?,
                source_local: Decode::decode(decoder)?,
            }),
            58 => Ok(Self::DropReadView {
                view_type: Decode::decode(decoder)?,
                source_local: Decode::decode(decoder)?,
            }),
            59 => Ok(Self::ReadViewDataField {
                view_type: Decode::decode(decoder)?,
                source_local: Decode::decode(decoder)?,
                field: Decode::decode(decoder)?,
            }),
            60 => Ok(Self::BorrowReadViewField {
                view_type: Decode::decode(decoder)?,
                source_local: Decode::decode(decoder)?,
                field: Decode::decode(decoder)?,
                binding_local: Decode::decode(decoder)?,
                binding_type: Decode::decode(decoder)?,
            }),
            61 => Ok(Self::AdoptViewResult {
                source_local: Decode::decode(decoder)?,
                binding_local: Decode::decode(decoder)?,
                binding_type: Decode::decode(decoder)?,
            }),
            62 => Ok(Self::ReturnView),
            63 => Ok(Self::BeginReadViewScope {
                read_scope: Decode::decode(decoder)?,
            }),
            64 => Ok(Self::EndReadViewScope {
                read_scope: Decode::decode(decoder)?,
            }),
            65 => Ok(Self::Region(Decode::decode(decoder)?)),
            found => Err(DecodeError::UnexpectedVariant {
                type_name: "CompiledInstruction",
                allowed: &AllowedEnumVariants::Range { min: 0, max: 65 },
                found,
            }),
        }
    }
}

#[cfg(test)]
#[path = "unit_codec_tests.rs"]
mod tests;
