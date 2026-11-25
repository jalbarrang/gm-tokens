use std::fs;
use std::iter::once;
use std::path::{Path, PathBuf};

struct CommentRules {
    line: String,
    four: String,
    doc: String,
    doc_bang: String,
    block: String,
}

impl CommentRules {
    fn new() -> Self {
        Self {
            line: once('/').chain(once('/')).collect(),
            four: once('/')
                .chain(once('/'))
                .chain(once('/'))
                .chain(once('/'))
                .collect(),
            doc: once('/').chain(once('/')).chain(once('/')).collect(),
            doc_bang: once('/').chain(once('/')).chain(once('!')).collect(),
            block: once('/').chain(once('*')).collect(),
        }
    }

    fn offends(&self, text_line: &str) -> bool {
        let trimmed = text_line.trim_start();
        let is_doc = (trimmed.starts_with(self.doc.as_str())
            && !trimmed.starts_with(self.four.as_str()))
            || trimmed.starts_with(self.doc_bang.as_str());
        !is_doc
            && (trimmed.starts_with(self.line.as_str())
                || trimmed.contains(self.block.as_str()))
    }
}

#[test]
fn plain_comments_are_abolished() {
    let rules = CommentRules::new();
    let mut offenders: Vec<String> = Vec::new();

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    scan_dir(&root.join("src"), &rules, &mut offenders);
    scan_dir(&root.join("tests"), &rules, &mut offenders);

    assert!(
        offenders.is_empty(),
        "plain comments are abolished here (short doc comments are fine); found at:\n{}",
        offenders.join("\n")
    );
}

fn scan_dir(dir: &Path, rules: &CommentRules, offenders: &mut Vec<String>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|_| panic!("readable dir: {}", dir.display()));
    for entry in entries {
        let path = entry.expect("valid dir entry").path();
        if path.is_dir() {
            scan_dir(&path, rules, offenders);
        } else {
            check_file(&path, rules, offenders);
        }
    }
}

fn check_file(path: &Path, rules: &CommentRules, offenders: &mut Vec<String>) {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("readable file: {}", path.display()));
    for (index, text_line) in text.lines().enumerate() {
        if rules.offends(text_line) {
            offenders.push(format!("{}:{}", path.display(), index + 1));
        }
    }
}
