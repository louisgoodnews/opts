use super::*;

/// Path of the generated page, relative to this crate.
const PAGE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/src/reference/hardware-fields.md"
);
use serde_json::Value;

fn leaves(prefix: &str, v: &Value, out: &mut Vec<String>) {
    let Value::Object(m) = v else { return };
    let is_fact = m.len() == 1 && (m.contains_key("known") || m.contains_key("unknown"));
    if is_fact {
        out.push(prefix.to_owned());
        return;
    }
    for (k, v) in m {
        let p = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        leaves(&p, v, out);
    }
}

#[test]
fn field_table_matches_the_serialised_profile() {
    let v = serde_json::to_value(crate::HardwareProfile::default()).unwrap();
    let mut real = Vec::new();
    leaves("", &v, &mut real);
    real.sort();
    let mut listed: Vec<String> = FIELDS.iter().map(|x| x.path.to_owned()).collect();
    listed.sort();
    assert_eq!(real, listed, "FIELDS and the profile structs disagree");
}

#[test]
fn reference_page_is_up_to_date() {
    let want = render();
    if std::env::var_os("UPDATE_HARDWARE_DOC").is_some() {
        std::fs::write(PAGE, &want).unwrap();
        return;
    }
    // A Windows checkout (`core.autocrlf`) has CRLF line ends; the page is compared by content.
    let have = std::fs::read_to_string(PAGE)
        .unwrap_or_else(|e| panic!("read the reference page: {e}"))
        .replace("\r\n", "\n");
    assert_eq!(
        have, want,
        "run: UPDATE_HARDWARE_DOC=1 cargo test -p opts-hardware reference"
    );
}
