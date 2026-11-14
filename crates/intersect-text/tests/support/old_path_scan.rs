//! Handwritten test support (not generated): counts the non-test source files
//! of the workspace that use an old (non-frozen) parse/store entry point.
//! Shared by the Xtext and KerML `old_path_guard` tests. The inventory is
//! docs/old-path-inventory.md.
//!
//! Scanned: every `.rs` file under `crates/*/src` and `crates/*/examples`.
//! Not counted as a use:
//! - test files: a file or directory named `tests`/`test`, `*_tests.rs`,
//!   `*_test.rs` or `*_tests/`, any path containing `test_support`, and every
//!   module declared under `#[cfg(test)]` (with or without `#[path]`),
//!   recursively;
//! - inside a counted file, whole-line comments and `#[cfg(test)]` items (an
//!   item with a body ends at the first line that is its indentation followed
//!   by `}`, which `tools/format_workspace.py` guarantees).
//!
//! The scan is textual and deliberately simple: it sees spellings, not name
//! resolution, so an alias (`use ... as X`) of an old entry point is not seen.
//! A file that defines an entry point counts like a caller.
// Each including suite uses a different subset of these helpers.
#![allow(dead_code)]
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// One guarded entry point.
pub struct Entry {
    pub name: &'static str,
    /// Recorded non-test files under `crates/*/src` that use it.
    pub src: usize,
    /// Recorded files under `crates/*/examples` that use it.
    pub examples: usize,
    pub uses: fn(&File) -> bool,
}

/// One scanned non-test source file.
pub struct File {
    /// Workspace-relative path with `/` separators.
    pub path: String,
    /// Source with comment lines and `#[cfg(test)]` items removed.
    pub code: String,
}

impl File {
    /// The crate directory name, for example `intersect-text`.
    pub fn krate(&self) -> &str {
        self.path.split('/').nth(1).unwrap_or("")
    }
    /// True when `word` occurs delimited by non-identifier characters.
    pub fn has_word(&self, word: &str) -> bool {
        count_word(&self.code, word) > 0
    }
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Occurrences of `word` not preceded or followed by an identifier character.
pub fn count_word(code: &str, word: &str) -> usize {
    code.match_indices(word)
        .filter(|(at, _)| {
            let before = code[..*at].chars().next_back();
            let after = code[at + word.len()..].chars().next();
            !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
        })
        .count()
}

/// Identifiers named right after each `prefix` (`prefix ident` or the
/// identifiers of a `prefix{...}` group up to its `;`).
pub fn qualified_names<'c>(code: &'c str, prefix: &str) -> Vec<&'c str> {
    let mut names = Vec::new();
    for (at, _) in code.match_indices(prefix) {
        if code[..at].chars().next_back().is_some_and(is_ident) {
            continue;
        }
        let rest = &code[at + prefix.len()..];
        let scope = if rest.starts_with('{') {
            &rest[..rest.find(';').unwrap_or(rest.len())]
        } else {
            &rest[..rest.find(|c: char| !is_ident(c)).unwrap_or(rest.len())]
        };
        names.extend(
            scope
                .split(|c: char| !is_ident(c))
                .filter(|s| !s.is_empty()),
        );
    }
    names
}

pub fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            walk(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn is_test_path(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    let (file, dirs) = parts.split_last().map_or(("", &[][..]), |(f, d)| (*f, d));
    path.contains("test_support")
        || matches!(file, "tests.rs" | "test.rs")
        || file.ends_with("_tests.rs")
        || file.ends_with("_test.rs")
        || dirs
            .iter()
            .any(|d| matches!(*d, "tests" | "test") || d.ends_with("_tests"))
}

/// `(child path, declared under #[cfg(test)])` for every `mod name;` of `path`.
fn child_modules(path: &str, text: &str) -> Vec<(String, bool)> {
    let (dir, file) = path.rsplit_once('/').unwrap_or(("", path));
    let mut children = Vec::new();
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    for (index, line) in lines.iter().enumerate() {
        let declaration = line
            .trim_start_matches("pub(crate) ")
            .trim_start_matches("pub(super) ")
            .trim_start_matches("pub ");
        let Some(name) = declaration
            .strip_prefix("mod ")
            .and_then(|rest| rest.strip_suffix(';'))
        else {
            continue;
        };
        let attributes = lines[..index]
            .iter()
            .rev()
            .take_while(|l| l.starts_with("#["));
        let mut test = false;
        let mut explicit = None;
        for attribute in attributes {
            test |= *attribute == "#[cfg(test)]";
            if let Some(target) = attribute
                .strip_prefix("#[path = \"")
                .and_then(|rest| rest.strip_suffix("\"]"))
            {
                explicit = Some(target);
            }
        }
        let child = match explicit {
            Some(target) => normalize(&format!("{dir}/{target}")),
            None if matches!(file, "lib.rs" | "main.rs" | "mod.rs") => format!("{dir}/{name}.rs"),
            None => format!("{dir}/{}/{name}.rs", file.trim_end_matches(".rs")),
        };
        children.push((child, test));
        if explicit.is_none() {
            let nested = children
                .last()
                .and_then(|(c, _)| c.strip_suffix(".rs"))
                .map(|stem| format!("{stem}/mod.rs"));
            if let Some(nested) = nested {
                children.push((nested, test));
            }
        }
    }
    children
}

fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

/// Drops whole-line comments and `#[cfg(test)]` items.
fn strip_test_code(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = String::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if line.trim() == "#[cfg(test)]" {
            let indent = &line[..line.len() - line.trim_start().len()];
            let mut item = index + 1;
            while item < lines.len() && lines[item].trim().starts_with("#[") {
                item += 1;
            }
            if item < lines.len() && lines[item].trim_end().ends_with('{') {
                let close = format!("{indent}}}");
                let mut end = item + 1;
                while end < lines.len() && lines[end].trim_end() != close {
                    end += 1;
                }
                index = end + 1;
            } else {
                index = item + 1;
            }
            continue;
        }
        if !line.trim_start().starts_with("//") {
            out.push_str(line);
            out.push('\n');
        }
        index += 1;
    }
    out
}

/// Every non-test source file of the workspace, sorted by path.
pub fn non_test_files() -> Result<Vec<File>, String> {
    let root = workspace();
    let crates = root.join("crates");
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(&crates).map_err(|e| format!("{}: {e}", crates.display()))? {
        let krate = entry.map_err(|e| e.to_string())?.path();
        walk(&krate.join("src"), &mut paths)?;
        walk(&krate.join("examples"), &mut paths)?;
    }
    let mut texts = BTreeMap::new();
    for path in paths {
        let relative = path
            .strip_prefix(&root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{relative}: {e}"))?;
        texts.insert(relative, text);
    }
    let mut test: BTreeSet<String> = texts.keys().filter(|p| is_test_path(p)).cloned().collect();
    loop {
        let mut added = false;
        for (path, text) in &texts {
            let parent_test = test.contains(path);
            for (child, cfg_test) in child_modules(path, text) {
                if (cfg_test || parent_test) && texts.contains_key(&child) && !test.contains(&child)
                {
                    test.insert(child);
                    added = true;
                }
            }
        }
        if !added {
            break;
        }
    }
    Ok(texts
        .into_iter()
        .filter(|(path, _)| !test.contains(path))
        .map(|(path, text)| File {
            code: strip_test_code(&text),
            path,
        })
        .collect())
}

/// Fails when an entry point is used by more files than recorded; reports
/// every recorded number that can be lowered.
pub fn check(entries: &[Entry]) -> Result<(), String> {
    let files = non_test_files()?;
    let mut failures = Vec::new();
    for entry in entries {
        let users: Vec<&File> = files.iter().filter(|f| (entry.uses)(f)).collect();
        let src: Vec<&str> = users
            .iter()
            .filter(|f| f.path.contains("/src/"))
            .map(|f| f.path.as_str())
            .collect();
        let examples: Vec<&str> = users
            .iter()
            .filter(|f| f.path.contains("/examples/"))
            .map(|f| f.path.as_str())
            .collect();
        println!(
            "{}: src {} (recorded {}), examples {} (recorded {})",
            entry.name,
            src.len(),
            entry.src,
            examples.len(),
            entry.examples
        );
        if src.len() < entry.src || examples.len() < entry.examples {
            println!(
                "  {} has fewer users than recorded: lower the recorded numbers and update docs/old-path-inventory.md",
                entry.name
            );
        }
        if src.len() > entry.src || examples.len() > entry.examples {
            failures.push(format!(
                "{} is used by {} src files (recorded {}) and {} example files (recorded {}). \
                 Do not add uses of the old parse/store path; see docs/old-path-inventory.md. \
                 Users:\n  {}",
                entry.name,
                src.len(),
                entry.src,
                examples.len(),
                entry.examples,
                src.iter()
                    .chain(&examples)
                    .copied()
                    .collect::<Vec<_>>()
                    .join("\n  ")
            ));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("\n"))
    }
}
