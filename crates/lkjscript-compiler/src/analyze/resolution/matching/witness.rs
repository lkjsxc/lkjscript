use std::fmt::{self, Write};

use crate::analyze::*;

use super::usefulness::{
    checked_capacity, reserve, Constructor, Usefulness, WitnessId, WitnessNode,
};

impl Usefulness<'_> {
    pub(super) fn render_witness(&self, root: WitnessId) -> Result<String> {
        enum Work<'a> {
            Witness(WitnessId),
            Type(&'a Type),
            Text(&'static str),
            Borrowed(&'a str),
            Label(&'a Type, Constructor),
        }

        let mut output = FallibleString::new();
        let mut work = Vec::new();
        reserve(&mut work, 1, "match witness render stack")?;
        work.push(Work::Witness(root));
        while let Some(item) = work.pop() {
            match item {
                Work::Text(text) => output.push(text)?,
                Work::Borrowed(text) => output.push(text)?,
                Work::Label(ty, constructor) => {
                    self.write_constructor_label(&mut output, ty, constructor)?;
                }
                Work::Witness(id) => match self.witness(id)? {
                    WitnessNode::Wild(ty) => {
                        reserve(&mut work, 3, "match witness render stack")?;
                        work.push(Work::Text(">"));
                        work.push(Work::Type(ty));
                        work.push(Work::Text("wildcard<"));
                    }
                    WitnessNode::Constructor {
                        ty,
                        constructor,
                        fields,
                    } => {
                        let field_work = fields
                            .len()
                            .checked_mul(2)
                            .ok_or_else(|| Error::host("match witness render stack overflow"))?;
                        let additional =
                            checked_capacity(3, field_work, "match witness render stack")?;
                        reserve(&mut work, additional, "match witness render stack")?;
                        if !fields.is_empty() {
                            work.push(Work::Text(")"));
                            for (index, field) in fields.iter().enumerate().rev() {
                                work.push(Work::Witness(*field));
                                if index != 0 {
                                    work.push(Work::Text(","));
                                }
                            }
                            work.push(Work::Text("("));
                        }
                        work.push(Work::Label(ty, *constructor));
                        work.push(Work::Text("::"));
                        work.push(Work::Type(ty));
                    }
                },
                Work::Type(ty) => match ty {
                    Type::Never => output.push("never")?,
                    Type::Unit => output.push("unit")?,
                    Type::Bool => output.push("bool")?,
                    Type::I64 => output.push("i64")?,
                    Type::F64 => output.push("f64")?,
                    Type::Str => output.push("string")?,
                    Type::Bytes => output.push("bytes")?,
                    Type::ByteVector => output.push("byte-vector")?,
                    Type::ByteSlice => output.push("byte-slice")?,
                    Type::ByteSliceMut => output.push("byte-slice-mut")?,
                    Type::Path => output.push("path")?,
                    Type::Capability(kind) => {
                        output.push("capability ")?;
                        output.push(kind.as_str())?;
                    }
                    Type::Symbol => output.push("symbol")?,
                    Type::Resource(kind) => output.push(kind.as_str())?,
                    Type::Product(_) => output.push("product")?,
                    Type::Enum { id, arguments, .. } => {
                        let argument_work = arguments
                            .len()
                            .checked_mul(2)
                            .ok_or_else(|| Error::host("match witness type render overflow"))?;
                        let additional =
                            checked_capacity(2, argument_work, "match witness type render stack")?;
                        reserve(&mut work, additional, "match witness render stack")?;
                        output.push("Enum#")?;
                        for byte in id.bytes() {
                            output.write_format(format_args!("{byte:02x}"))?;
                        }
                        output.push("<")?;
                        work.push(Work::Text(">"));
                        for (index, argument) in arguments.iter().enumerate().rev() {
                            work.push(Work::Type(argument));
                            if index != 0 {
                                work.push(Work::Text(","));
                            }
                        }
                    }
                    Type::Param(name) => output.push(name)?,
                    Type::List(inner) => {
                        output.push("list ")?;
                        reserve(&mut work, 1, "match witness render stack")?;
                        work.push(Work::Type(inner));
                    }
                    Type::Fn { params, ret } => {
                        let parameter_work = params
                            .len()
                            .checked_mul(2)
                            .ok_or_else(|| Error::host("match witness type render overflow"))?;
                        let additional =
                            checked_capacity(2, parameter_work, "match witness type render stack")?;
                        reserve(&mut work, additional, "match witness render stack")?;
                        output.push("fn inputs")?;
                        work.push(Work::Type(ret));
                        work.push(Work::Text(" output "));
                        for parameter in params.iter().rev() {
                            work.push(Work::Type(parameter));
                            work.push(Work::Text(" "));
                        }
                    }
                    Type::Forall { vars, body } => {
                        let variable_work = vars
                            .len()
                            .checked_mul(2)
                            .ok_or_else(|| Error::host("match witness type render overflow"))?;
                        let additional =
                            checked_capacity(2, variable_work, "match witness type render stack")?;
                        reserve(&mut work, additional, "match witness render stack")?;
                        output.push("forall")?;
                        work.push(Work::Type(body));
                        work.push(Work::Text(" "));
                        for variable in vars.iter().rev() {
                            work.push(Work::Borrowed(variable));
                            work.push(Work::Text(" "));
                        }
                    }
                },
            }
        }
        Ok(output.finish())
    }

    fn write_constructor_label(
        &self,
        output: &mut FallibleString,
        ty: &Type,
        constructor: Constructor,
    ) -> Result<()> {
        match constructor {
            Constructor::Bool(value) => output.write_format(format_args!("{value}")),
            Constructor::I64(value) => output.write_format(format_args!("{value}")),
            Constructor::Product(id) => output.write_format(format_args!("product#{}", id.raw())),
            Constructor::Variant(id) => {
                let enum_id = match ty {
                    Type::Enum { id, .. } => *id,
                    _ => return Err(Error::msg("variant witness type mismatch")),
                };
                let index = self
                    .enum_def(enum_id)?
                    .variants
                    .iter()
                    .position(|item| item.id == id)
                    .ok_or_else(|| Error::msg("witness variant identity is stale"))?;
                output.write_format(format_args!("variant#{index}"))
            }
        }
    }
}

struct FallibleString {
    value: String,
    failed: bool,
}

impl FallibleString {
    fn new() -> Self {
        Self {
            value: String::new(),
            failed: false,
        }
    }

    fn push(&mut self, value: &str) -> Result<()> {
        self.value
            .try_reserve(value.len())
            .map_err(|_| Error::host("canonical match witness allocation failed"))?;
        self.value.push_str(value);
        Ok(())
    }

    fn write_format(&mut self, arguments: fmt::Arguments<'_>) -> Result<()> {
        fmt::write(self, arguments)
            .map_err(|_| Error::host("canonical match witness allocation failed"))
    }

    fn finish(self) -> String {
        self.value
    }
}

impl Write for FallibleString {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if self.failed {
            return Err(fmt::Error);
        }
        if self.value.try_reserve(value.len()).is_err() {
            self.failed = true;
            return Err(fmt::Error);
        }
        self.value.push_str(value);
        Ok(())
    }
}
