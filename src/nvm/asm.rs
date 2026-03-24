pub struct AsmBuilder {
    lines: Vec<String>,
}

impl AsmBuilder {
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    pub fn directive(&mut self, s: &str) {
        self.lines.push(s.to_string());
    }

    pub fn label(&mut self, name: &str) {
        self.lines.push(format!("{name}:"));
    }

    pub fn emit(&mut self, mnemonic: &str) {
        self.lines.push(format!("    {mnemonic}"));
    }

    pub fn emit_u8(&mut self, mnemonic: &str, val: u8) {
        self.lines.push(format!("    {mnemonic} {val}"));
    }

    pub fn emit_i32(&mut self, mnemonic: &str, val: i32) {
        self.lines.push(format!("    {mnemonic} {val}"));
    }

    pub fn emit_label(&mut self, mnemonic: &str, label: &str) {
        self.lines.push(format!("    {mnemonic} {label}"));
    }

    pub fn emit_syscall(&mut self, name: &str) {
        self.lines.push(format!("    syscall {name}"));
    }

    pub fn comment(&mut self, text: &str) {
        self.lines.push(format!("; {text}"));
    }

    pub fn blank(&mut self) {
        self.lines.push(String::new());
    }

    pub fn finish(&self) -> String {
        self.lines.join("\n") + "\n"
    }
}
