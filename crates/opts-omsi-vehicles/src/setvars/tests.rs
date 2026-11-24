//! Unit tests of `setvars`.

use super::*;

mod catalogue;
mod kinds;
mod shipped;

fn sv(repaint: &str, var: &str, value: &str) -> CtiSetvar {
    CtiSetvar {
        repaint: repaint.into(),
        variable: var.into(),
        value: value.into(),
    }
}

fn declared(names: &[(&str, &str)]) -> BTreeMap<String, BTreeSet<String>> {
    let mut m: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (n, list) in names {
        m.entry(n.to_lowercase())
            .or_default()
            .insert((*list).into());
    }
    m
}

fn kind_of(values: &[&str]) -> SetvarEntry {
    let observed: Vec<CtiSetvar> = values
        .iter()
        .enumerate()
        .map(|(i, v)| sv(&format!("r{i}"), "opt", v))
        .collect();
    let c = SetvarCatalogue::build(&observed, &BTreeMap::new(), None);
    c.entries.into_iter().next().expect("one entry")
}
