use regex::Regex;

pub struct Watchdog {
    patterns: Vec<Regex>,
    prompt_patterns: Vec<Regex>,
}

impl Watchdog {
    pub fn new() -> Self {
        let pattern_strs = [
            r"(?i)\[y/n\]",
            r"(?i)\(y/n\)",
            r"(?i)password:",
            r"(?i)allow\?",
            r"(?i)proceed\?",
            r"(?i)continue\?",
            r"(?i)do you want to",
            r"(?i)are you sure",
            r"(?i)\[yes/no\]",
            r"(?i)Esc to cancel",
        ];

        let prompt_strs = [
            r"^\s*[\$%#>]\s*$",
            r"➜\s+\S",
            r"\$\s*$",
            r"%\s*$",
            r"^\s*\w+@",
        ];

        Self {
            patterns: pattern_strs
                .iter()
                .filter_map(|p| Regex::new(p).ok())
                .collect(),
            prompt_patterns: prompt_strs
                .iter()
                .filter_map(|p| Regex::new(p).ok())
                .collect(),
        }
    }

    pub fn needs_attention(&self, cursor_line: &str, nearby_text: &str) -> bool {
        if self.is_shell_prompt(cursor_line) {
            return false;
        }
        self.patterns.iter().any(|re| re.is_match(nearby_text))
    }

    fn is_shell_prompt(&self, line: &str) -> bool {
        let trimmed = line.trim();
        !trimmed.is_empty() && self.prompt_patterns.iter().any(|re| re.is_match(trimmed))
    }
}
