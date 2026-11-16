//! Unit tests of `setvars_discovery`.

mod known;
mod stepping;

use super::*;

fn reps(lines: &[(&str, &str, &str)]) -> VehicleRepaints {
    let mut r = VehicleRepaints::default();
    for (repaint, var, value) in lines {
        r.setvars
            .entry((*repaint).to_string())
            .or_default()
            .push(((*var).to_string(), (*value).to_string()));
    }
    r
}

fn item<'a>(items: &'a [SetvarItem], name: &str) -> &'a SetvarItem {
    items.iter().find(|i| i.name == name).expect("item")
}
