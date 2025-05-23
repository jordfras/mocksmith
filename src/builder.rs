pub(crate) struct CodeBuilder {
    code: String,
    indent_size: usize,
    indent_level: usize,
}

impl CodeBuilder {
    pub(crate) fn new(indent_size: usize) -> Self {
        CodeBuilder {
            code: String::new(),
            indent_size,
            indent_level: 0,
        }
    }

    pub(crate) fn push_indent(&mut self) {
        self.indent_level += 1;
    }

    pub(crate) fn pop_indent(&mut self) {
        assert!(self.indent_level > 0, "Indent level cannot be negative");
        self.indent_level -= 1;
    }

    pub(crate) fn add_line(&mut self, line: &str) {
        let indent = self.indent_level * self.indent_size;
        self.code.push_str(&" ".repeat(indent));
        self.code.push_str(line);
        self.code.push('\n');
    }

    pub(crate) fn _newline(&mut self) {
        self.code.push('\n');
    }

    pub(crate) fn build(self) -> String {
        assert!(self.indent_level == 0, "Unmatched indent level");
        self.code
    }
}
