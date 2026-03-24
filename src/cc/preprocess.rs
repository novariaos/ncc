use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub struct Preprocessor {
    include_dirs: Vec<PathBuf>,
    defines: HashMap<String, String>,
    defined: HashSet<String>,
    included: HashSet<PathBuf>,
}

impl Preprocessor {
    pub fn new(include_dirs: Vec<PathBuf>) -> Self {
        Self {
            include_dirs,
            defines: HashMap::new(),
            defined: HashSet::new(),
            included: HashSet::new(),
        }
    }

    pub fn process(&mut self, source: &str, file_path: &Path) -> Result<String, String> {
        let mut output = String::new();
        let mut skip_depth: u32 = 0;

        for line in source.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with('#') {
                let directive = trimmed.trim_start_matches('#').trim();

                if let Some(rest) = directive.strip_prefix("ifndef") {
                    let name = rest.trim();
                    if skip_depth > 0 || self.defined.contains(name) {
                        skip_depth += 1;
                    }
                    continue;
                }

                if let Some(rest) = directive.strip_prefix("ifdef") {
                    let name = rest.trim();
                    if skip_depth > 0 || !self.defined.contains(name) {
                        skip_depth += 1;
                    }
                    continue;
                }

                if directive == "endif" {
                    if skip_depth > 0 {
                        skip_depth -= 1;
                    }
                    continue;
                }

                if directive.starts_with("else") {
                    continue;
                }

                if skip_depth > 0 {
                    continue;
                }

                if let Some(rest) = directive.strip_prefix("define") {
                    let rest = rest.trim();
                    if let Some(space_pos) = rest.find(|c: char| c.is_ascii_whitespace()) {
                        let name = &rest[..space_pos];
                        let value = rest[space_pos..].trim().to_string();
                        self.defined.insert(name.to_string());
                        self.defines.insert(name.to_string(), value);
                    } else {
                        self.defined.insert(rest.to_string());
                    }
                    continue;
                }

                if let Some(rest) = directive.strip_prefix("include") {
                    let rest = rest.trim();
                    let filename = if rest.starts_with('"') {
                        rest.trim_matches('"')
                    } else if rest.starts_with('<') {
                        rest.trim_start_matches('<').trim_end_matches('>')
                    } else {
                        return Err(format!("Invalid #include: {}", line));
                    };

                    let included_source = self.resolve_include(filename, file_path)?;
                    output.push_str(&included_source);
                    output.push('\n');
                    continue;
                }

                continue;
            }

            if skip_depth > 0 {
                continue;
            }

            let processed = strip_attributes(line);
            let processed = self.apply_defines(&processed);

            output.push_str(&processed);
            output.push('\n');
        }

        Ok(output)
    }

    fn resolve_include(&mut self, filename: &str, current_file: &Path) -> Result<String, String> {
        let candidates = {
            let mut c = Vec::new();
            if let Some(dir) = current_file.parent() {
                c.push(dir.join(filename));
            }
            for inc_dir in &self.include_dirs {
                c.push(inc_dir.join(filename));
            }
            c
        };

        for path in &candidates {
            if path.exists() {
                let canonical = path
                    .canonicalize()
                    .unwrap_or_else(|_| path.clone());

                if self.included.contains(&canonical) {
                    return Ok(String::new());
                }
                self.included.insert(canonical);

                let content = std::fs::read_to_string(path)
                    .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;

                return self.process(&content, path);
            }
        }

        Err(format!(
            "Cannot find include file '{}' (searched from {})",
            filename,
            current_file.display()
        ))
    }

    fn apply_defines(&self, line: &str) -> String {
        let mut result = line.to_string();
        for (name, value) in &self.defines {
            let mut new = String::new();
            let mut remaining = result.as_str();
            while let Some(pos) = remaining.find(name.as_str()) {
                let before_ok = pos == 0
                    || !remaining.as_bytes()[pos - 1].is_ascii_alphanumeric()
                        && remaining.as_bytes()[pos - 1] != b'_';
                let after_pos = pos + name.len();
                let after_ok = after_pos >= remaining.len()
                    || !remaining.as_bytes()[after_pos].is_ascii_alphanumeric()
                        && remaining.as_bytes()[after_pos] != b'_';

                if before_ok && after_ok {
                    new.push_str(&remaining[..pos]);
                    new.push_str(value);
                    remaining = &remaining[after_pos..];
                } else {
                    new.push_str(&remaining[..after_pos]);
                    remaining = &remaining[after_pos..];
                }
            }
            new.push_str(remaining);
            result = new;
        }
        result
    }
}

fn strip_attributes(line: &str) -> String {
    let mut result = String::new();
    let mut chars = line.chars().peekable();

    while chars.peek().is_some() {
        let remaining: String = chars.clone().collect();
        if remaining.starts_with("__attribute__") {
            for _ in 0.."__attribute__".len() {
                chars.next();
            }
            while chars.peek() == Some(&' ') || chars.peek() == Some(&'\t') {
                chars.next();
            }
            if chars.peek() == Some(&'(') {
                let mut depth = 0;
                loop {
                    match chars.next() {
                        Some('(') => depth += 1,
                        Some(')') => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        None => break,
                        _ => {}
                    }
                }
            }
            while chars.peek() == Some(&' ') || chars.peek() == Some(&'\t') {
                chars.next();
            }
        } else {
            result.push(chars.next().unwrap());
        }
    }

    result
}
