//! Reading, checking and applying setvar values.


use super::*;


pub(super) fn get(src: &Source, catalogue: &SetvarCatalogue) -> Outcome {
    let options: Vec<Value> = catalogue.entries.iter().map(|e| describe(src, e)).collect();
    Outcome::ok(json!({
        "context": src.context.name(),
        "repaint": repaint_of(src),
        "family": catalogue.family,
        "count": options.len(),
        "options": options,
        "not_offered": catalogue.undeclared,
    }))
}

/// What a value must be for `e`, in words.
pub(super) fn expectation(e: &SetvarEntry) -> String {
    match e.kind {
        SetvarKind::Switch => "0 or 1".into(),
        SetvarKind::Choice => format!("one of {:?}", e.values),
        SetvarKind::Number => format!("a whole number from {} to {}", e.min, e.max),
        SetvarKind::Fraction => "not settable here: only the repaint sets decimal options".into(),
    }
}

/// Validates the batch against the catalogue; every refusal is listed, nothing is applied.
pub(super) fn validate(
    catalogue: &SetvarCatalogue,
    values: &Map<String, Value>,
    allow_unlisted: bool,
) -> Result<Vec<(String, i32)>, Vec<Value>> {
    let (mut ok, mut bad) = (Vec::new(), Vec::new());
    for (name, v) in values {
        let refuse =
            |why: String| json!({ "key": name, "code": code::INVALID_PARAMS, "message": why });
        let number = match v {
            Value::Number(n) => n.as_f64(),
            Value::Bool(b) => Some(f64::from(u8::from(*b))),
            Value::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        };
        let Some(number) = number.filter(|n| n.is_finite() && n.fract() == 0.0) else {
            bad.push(refuse(format!("{name}: expected a whole number")));
            continue;
        };
        match catalogue.get(name) {
            Some(e) if e.accepts(number) => ok.push((e.name.clone(), number as i32)),
            Some(e) => bad.push(refuse(format!("{}: expected {}", e.name, expectation(e)))),
            None if allow_unlisted => ok.push((name.trim().to_string(), number as i32)),
            None => bad.push(refuse(format!(
                "{name}: not an option of this vehicle (allow_unlisted: true sets it anyway; the vendors warn that unlisted variables can cause faults)"
            ))),
        }
    }
    if bad.is_empty() {
        Ok(ok)
    } else {
        Err(bad)
    }
}

/// A change of the driven vehicle: set on its scripts at once, kept on its session record (the
/// save, a saved configuration and the editor read it there) and, while the driven vehicle is
/// the one the drive started with, on the load job too.
pub fn apply_live(world: &mut World, name: &str, value: i32, default: i32) {
    let active = world.get_resource::<SessionVehicles>().and_then(|r| {
        r.active()
            .filter(|r| r.is_alive())
            .map(|r| (r.id, r.bus_file.clone()))
    });
    if let Some((id, _)) = &active {
        world
            .resource_mut::<SessionVehicles>()
            .set_setvar(*id, name, value, default);
    }
    if let Some(mut job) = world.get_resource_mut::<WorldLoadJob>() {
        let first = match &active {
            Some((_, bus)) => job
                .vehicle_file()
                .is_some_and(|f| f == std::path::Path::new(bus)),
            None => true,
        };
        if first {
            job.set_vehicle_setvar(name, value);
        }
    }
    let mut q =
        world.query_filtered::<&opts_bevy::VehicleScripts, With<opts_bevy::PlayerVehicle>>();
    if let Some(scripts) = q.iter(world).next() {
        if let Ok(mut engine) = scripts.engine.lock() {
            engine.set(name, value as f32);
        }
    }
}
