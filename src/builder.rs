pub struct CodeBuilder {
    code: String,
    indent_size: usize,
    indent_level: usize,
}

impl CodeBuilder {
    pub fn new(indent_size: usize) -> Self {
        CodeBuilder {
            code: String::new(),
            indent_size,
            indent_level: 0,
        }
    }

    pub fn push_indent(&mut self) {
        self.indent_level += 1;
    }

    pub fn pop_indent(&mut self) {
        assert!(self.indent_level > 0, "Indent level cannot be negative");
        self.indent_level -= 1;
    }

    pub fn add_line(&mut self, line: &str) {
        let indent = self.indent_level * self.indent_size;
        self.code.push_str(&" ".repeat(indent));
        self.code.push_str(line);
        self.code.push('\n');
    }

    pub fn _newline(&mut self) {
        self.code.push('\n');
    }

    pub fn build(self) -> String {
        assert!(self.indent_level == 0, "Unmatched indent level");
        self.code
    }
}
