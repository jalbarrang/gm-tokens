use std::fs;
use std::iter::once;
use std::path::{Path, PathBuf};

#[test]
fn source_files_contain_no_comments() {
    let line_comment: String = once('/').chain(once('/')).collect();
    let block_comment: String = once('/').chain(once('*')).collect();
    let mut offenders: Vec<String> = Vec::new();

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    scan_dir(&root.join("src"), &line_comment, &block_comment, &mut offenders);
    scan_dir(&root.join("tests"), &line_comment, &block_comment, &mut offenders);

    assert!(
        offenders.is_empty(),
        "comments are abolished in this crate; found at:\n{}",
        offenders.join("\n")
    );
}

fn scan_dir(dir: &Path, line: &str, block: &str, offenders: &mut Vec<String>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|_| panic!("readable dir: {}", dir.display()));
    for entry in entries {
        let path = entry.expect("valid dir entry").path();
        if path.is_dir() {
            scan_dir(&path, line, block, offenders);
        } else {
            check_file(&path, line, block, offenders);
        }
    }
}

fn check_file(path: &Path, line: &str, block: &str, offenders: &mut Vec<String>) {
    let text =
        fs::read_to_string(path).unwrap_or_else(|_| panic!("readable file: {}", path.display()));
    for (index, text_line) in text.lines().enumerate() {
        if text_line.contains(line) || text_line.contains(block) {
            offenders.push(format!("{}:{}", path.display(), index + 1));
        }
    }
}
