use std::fmt::Display;

// TODO: Remove all this because stylua
pub struct StringBuilder {
    lines: Vec<String>,
    depth: usize,
}

impl StringBuilder {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            depth: 0,
        }
    }

    pub fn build(self) -> String {
        self.lines.join("\n")
    }

    pub fn push<T: Display>(&mut self, text: T) {
        let text = text.to_string();

        for line in text.split('\n') {
            self.lines
                .push(format!("{}{line}", "\t".repeat(self.depth)));
        }
    }

    pub fn append(&mut self, other: &mut Self) {
        for line in &other.lines {
            self.push(line.trim_start_matches('\t'));
        }
    }

    pub fn blank(&mut self) {
        self.lines.push("".to_owned());
    }

    pub fn indent(&mut self) {
        self.depth += 1;
    }

    pub fn indent_n(&mut self, n: usize) {
        self.depth += n;
    }

    pub fn dedent(&mut self) {
        self.depth -= 1;
    }
}

impl FromIterator<String> for StringBuilder {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let mut builder = StringBuilder::new();

        for line in iter {
            builder.push(line);
        }

        builder
    }
}
