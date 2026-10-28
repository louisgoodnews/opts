//! Unit tests for `super` (`report.rs`).

use super::*;

fn rec(key: &str, verdict: Verdict, cats: &[&str]) -> Record {
    Record {
        sco: format!("Sceneryobjects/{key}.sco"),
        key: key.into(),
        verdict,
        categories: cats.iter().map(|c| (*c).to_string()).collect(),
        ..Record::default()
    }
}

#[test]
fn summaries_and_compare() {
    let dir = tempfile::tempdir().unwrap();
    let a = vec![
        rec("a", Verdict::Pass, &[]),
        rec("b", Verdict::Fail, &["F-TEX-MISSING"]),
    ];
    let b = vec![rec("a", Verdict::Pass, &[]), rec("b", Verdict::Pass, &[])];
    write_summaries(dir.path(), &a).unwrap();
    let cats: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("categories.json")).unwrap())
            .unwrap();
    assert_eq!(cats["F-TEX-MISSING"]["count"], 1);
    let diff = compare(&a, &b);
    assert!(diff.contains("1 changed, 1 unchanged"));
}

#[test]
fn truncated_lines_are_skipped_and_latest_wins() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("results.jsonl");
    let mut w = ResultsWriter::open(&p).unwrap();
    w.append(&rec("a", Verdict::Fail, &["F-CRASH"])).unwrap();
    w.append(&rec("a", Verdict::Pass, &[])).unwrap();
    std::fs::OpenOptions::new()
        .append(true)
        .open(&p)
        .unwrap()
        .write_all(b"{\"sco\":")
        .unwrap();
    let all = latest(read_results(&p));
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].verdict, Verdict::Pass);
}
