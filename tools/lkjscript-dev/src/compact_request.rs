//! Compact request builder shared by semantic development probes.
//! These strings are requests only; the public executable owns validation and publication.

#[derive(Default)]
pub(super) struct Request {
    pub text: String,
    next: usize,
}

impl Request {
    pub(crate) fn http_request_type(&mut self) {
        self.text.push_str("type.structural-record as=@http-request-header\ntype.field parent=@http-request-header index=0 name=name type=text\ntype.field parent=@http-request-header index=1 name=value type=bytes\ntype.list as=@http-request-headers item=@http-request-header\ntype.stream as=@http-request-body item=bytes\ntype.list as=@http-query-values item=text\ntype.map as=@http-query key=text value=@http-query-values\ntype.structural-record as=@http-request\ntype.field parent=@http-request index=0 name=body type=@http-request-body\ntype.field parent=@http-request index=1 name=headers type=@http-request-headers\ntype.field parent=@http-request index=2 name=method type=text\ntype.field parent=@http-request index=3 name=path type=text\ntype.field parent=@http-request index=4 name=query type=text\ntype.field parent=@http-request index=5 name=query_parameters type=@http-query\n");
    }
    pub(crate) fn expression(&mut self, form: &str, fields: &str) -> String {
        let symbol = format!("$e{}", self.next);
        self.next += 1;
        self.text
            .push_str(&format!("expression.{form} as={symbol} {fields}\n"));
        symbol
    }

    pub(crate) fn local(&mut self, name: &str) -> String {
        self.expression("local", &format!("value={name}"))
    }

    pub(crate) fn integer(&mut self, value: i64) -> String {
        self.expression("i64", &format!("value={value}"))
    }

    pub(crate) fn arguments(&mut self, symbol: &str, arguments: &[String]) {
        for (index, argument) in arguments.iter().enumerate() {
            self.text.push_str(&format!(
                "expression.argument parent={symbol} index={index} expression={argument}\n"
            ));
        }
    }

    pub(crate) fn types(&mut self, symbol: &str, types: &[&str]) {
        for (index, ty) in types.iter().enumerate() {
            self.text.push_str(&format!(
                "type.argument parent={symbol} index={index} type={ty}\n"
            ));
        }
    }

    pub(crate) fn effects(&mut self, symbol: &str, rows: &[&str]) {
        for (index, row) in rows.iter().enumerate() {
            self.text.push_str(&format!(
                "effect.argument parent={symbol} index={index} effect={row}\n"
            ));
        }
    }

    pub(crate) fn call(&mut self, function: &str, types: &[&str], arguments: &[String]) -> String {
        let symbol = self.expression("call", &format!("function={function}"));
        self.types(&symbol, types);
        self.arguments(&symbol, arguments);
        symbol
    }

    pub(crate) fn function_value(&mut self, function: &str) -> String {
        self.expression("function-value", &format!("function={function}"))
    }

    pub(crate) fn bind(&mut self, callee: &str, captures: &[String]) -> String {
        let result = self.expression("bind", &format!("callee={callee}"));
        self.arguments(&result, captures);
        result
    }

    pub(crate) fn invoke(&mut self, callee: &str, arguments: &[String]) -> String {
        let result = self.expression("invoke", &format!("function={callee}"));
        self.arguments(&result, arguments);
        result
    }

    pub(crate) fn function(
        &mut self,
        name: &str,
        result: &str,
        body: &str,
        parameters: &[(&str, &str)],
    ) {
        self.text.push_str(&format!("create.function as=${name} module=$module name={name} visibility=public result={result} effect=pure body={body}\n"));
        for (parameter, ty) in parameters {
            self.text.push_str(&format!(
                "add.parameter as=${name}_{parameter} function=${name} name={parameter} type={ty}\n"
            ));
        }
    }

    pub(crate) fn target(&mut self, name: &str, result: &str, parameters: &[&str]) {
        self.text
            .push_str(&format!("type.function as=@{name} result={result}\n"));
        self.types(&format!("@{name}"), parameters);
        self.text.push_str(&format!("add.port as=${name}_port component=$component name={name} type=@{name} function=${name}\ncreate.target as=${name}_target name={name} component=$component port=${name}_port runner=command\n"));
    }

    pub(crate) fn choose(&mut self, condition: &str, yes: &str, no: &str) -> String {
        self.expression(
            "if",
            &format!("condition={condition} when-true={yes} when-false={no}"),
        )
    }

    pub(crate) fn field(&mut self, value: &str, name: &str) -> String {
        self.expression("field", &format!("value={value} name={name}"))
    }

    pub(crate) fn capability(
        &mut self,
        requirement: &str,
        operation: &str,
        arguments: &[String],
    ) -> String {
        let value = self.expression(
            "capability-call",
            &format!("requirement={requirement} operation={operation}"),
        );
        self.arguments(&value, arguments);
        value
    }
}
