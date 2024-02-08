use crate::string_builder::StringBuilder;

pub struct IfBuilder {
    string_builder: StringBuilder,
    has_conditions: bool,
}

impl IfBuilder {
    pub fn new() -> IfBuilder {
        IfBuilder {
            string_builder: StringBuilder::new(),
            has_conditions: false,
        }
    }

    pub fn add_condition(&mut self, condition: &str, callback: impl FnOnce(&mut StringBuilder)) {
        self.string_builder.push(format!(
            "{} {condition} then",
            if self.has_conditions { "elseif" } else { "if" }
        ));

        self.has_conditions = true;

        self.string_builder.indent();
        callback(&mut self.string_builder);
        self.string_builder.dedent();
    }

    pub fn with_else(mut self, callback: impl FnOnce(&mut StringBuilder)) -> Self {
        self.string_builder.push("else");
        self.string_builder.indent();
        callback(&mut self.string_builder);
        self.string_builder.dedent();
        self
    }

    pub fn build(self) -> String {
        self.into_string_builder().build()
    }

    pub fn indent_n(&mut self, n: usize) {
        self.string_builder.indent_n(n);
    }

    pub fn into_string_builder(mut self) -> StringBuilder {
        if self.has_conditions {
            self.string_builder.push("end");
        }

        self.string_builder
    }
}
