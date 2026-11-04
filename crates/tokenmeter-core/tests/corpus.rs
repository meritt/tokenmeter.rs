use std::path::PathBuf;

use tokenmeter_core::{count, encode};

struct Case {
    text: String,
    ids: Vec<u32>,
}

fn corpus() -> Vec<Case> {
    let path = std::env::var_os("TOKENMETER_CORPUS")
        .map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/o200k.tsv"), PathBuf::from);
    let data = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let cases: Vec<Case> = data.lines().map(case).collect();

    assert!(!cases.is_empty(), "{}: no texts", path.display());
    cases
}

fn case(line: &str) -> Case {
    let (ids, hex) = line.split_once('\t').expect("ids, a tab and the text in hex");
    let bytes: Vec<u8> =
        (0..hex.len()).step_by(2).map(|at| u8::from_str_radix(&hex[at..at + 2], 16).expect("hex")).collect();
    let ids = if ids.is_empty() { vec![] } else { ids.split(',').map(|id| id.parse().expect("an ID")).collect() };

    Case { text: String::from_utf8(bytes).expect("UTF-8"), ids }
}

#[test]
fn encodes_as_tiktoken() {
    for case in corpus() {
        assert_eq!(encode(&case.text), case.ids, "{:?}", case.text);
    }
}

#[test]
fn counts_as_tiktoken() {
    for case in corpus() {
        assert_eq!(count(&case.text), case.ids.len(), "{:?}", case.text);
    }
}
