use super::*;

#[test]
#[ignore = "census; prints"]
fn census_tram() {
    for rel in [
        "Vehicles/Tram_NF6D/NF6D_B_User.bus",
        "Vehicles/Tram_NF6D/NF6D_A.bus",
    ] {
        let Some(mut e) = load(rel) else {
            eprintln!("CENSUS {rel}: not available");
            continue;
        };
        run(&mut e, 1.0);
        let names = [
            "cp_batterietrennschalter_toggle",
            "kw_m_enginestart",
            "kw_s_1",
            "kw_s_2",
            "kw_s_R",
            "kw_s_N",
            "kw_s_plus",
            "kw_scheinwerfer_toggle",
            "bus_doorfront0",
            "horn",
            "parking_brake_toggle",
        ];
        let has: Vec<_> = names.iter().filter(|n| e.has_trigger(n)).collect();
        let before = e.variables();
        tap(&mut e, "cp_batterietrennschalter_toggle", 0.3);
        run(&mut e, 2.0);
        let after = e.variables();
        let changed: Vec<_> = after
            .iter()
            .filter(|(k, v)| {
                before
                    .iter()
                    .any(|(k2, v2)| k2 == k && (v2 - v).abs() > 1e-4)
            })
            .map(|(k, v)| format!("{k}={v:.2}"))
            .take(25)
            .collect();
        eprintln!("CENSUS {rel}: triggers {has:?}\nCENSUS battery key changed: {changed:?}");
    }
}
