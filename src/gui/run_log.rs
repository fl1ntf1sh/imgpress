use std::sync::Mutex;

pub(super) struct RunLog {
    lines: Mutex<Vec<String>>,
    max_lines: usize,
}

impl RunLog {
    pub(super) fn new(max_lines: usize) -> Self {
        Self {
            lines: Mutex::new(Vec::new()),
            max_lines,
        }
    }

    pub(super) fn append(&self, line: impl Into<String>) -> String {
        let mut lines = self.lines.lock().unwrap();
        lines.push(line.into());
        let keep_from = lines.len().saturating_sub(self.max_lines);
        lines[keep_from..].join("\n")
    }
}
