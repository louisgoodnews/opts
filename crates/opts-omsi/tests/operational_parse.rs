//! Integration and robustness tests for operational data parsers (M-0014).
//!
//! Validates:
//! - `.org` vehicle registration lists (`opts_omsi::formats::org`)
//! - `Money/<pack>/*.cfg` money systems (`opts_omsi::formats::money`)
//! - `paths.cfg` passenger paths networks (`opts_omsi::formats::vehicles::paths`)
//! - `.bus` vehicle manifests (`opts_omsi::formats::vehicles::bus`)

use opts_omsi::formats::money::{parse_money, MoneySystem};
use opts_omsi::formats::org::{parse_org, RegistrationPool};
use opts_omsi::formats::vehicles::bus::{parse_bus, BusManifest, RegistrationMode};
use opts_omsi::formats::vehicles::paths::{parse_paths, PassengerPaths};

#[test]
fn test_org_pool_weighting_distribution() {
    let org_text = r#"
B-V 1000
B-V 1000
B-V 1000
B-V 2000
B-V 3000
B-V 3000
"#;
    let pool = parse_org(org_text).expect("org should parse");
    assert_eq!(pool.total_count(), 6);
    assert_eq!(pool.unique_count(), 3);
    assert_eq!(pool.weight("B-V 1000"), 3);
    assert_eq!(pool.weight("B-V 2000"), 1);
    assert_eq!(pool.weight("B-V 3000"), 2);
    assert_eq!(pool.select_weighted(0), Some("B-V 1000"));
    assert_eq!(pool.select_weighted(3), Some("B-V 2000"));
    assert_eq!(pool.select_weighted(4), Some("B-V 3000"));
    assert_eq!(pool.select_weighted(6), None);
}

#[test]
fn test_money_system_parsing() {
    let money_text = r#"
[currency]
Euro
2

[coin]
Euro_005.o3d
0.05

[coin]
Euro_010.o3d
0.10

[coin]
Euro_020.o3d
0.20

[coin]
Euro_050.o3d
0.50

[coin]
Euro_100.o3d
1.00

[coin]
Euro_200.o3d
2.00

[bill]
Euro_5.o3d
5.00

[bill]
Euro_10.o3d
10.00
"#;
    let money = parse_money(money_text).expect("money system should parse");
    assert_eq!(money.currency.name, "Euro");
    assert_eq!(money.currency.decimals, 2);
    assert_eq!(money.coins.len(), 6);
    assert_eq!(money.bills.len(), 2);
    assert_eq!(money.coins[0].value, 0.05);
    assert_eq!(money.coins[5].value, 2.0);
    assert_eq!(money.bills[0].value, 5.0);
    assert_eq!(money.bills[1].value, 10.0);
}

#[test]
fn test_passenger_paths_network() {
    let paths_text = r#"
[stepsoundpack]
3
Step_01.wav
Step_02.wav
Step_03.wav

0 (Front Door)
[pathpnt]
0.850
4.120
0.350

1 (Cash Desk)
[pathpnt]
0.200
3.500
0.400

2 (Rear Door)
[pathpnt]
0.850
-2.500
0.350

[pathlink]
0
1

[pathlink]
1
2

[entry]
0
0

[exit]
2
1

[next_stepsound]
0

[next_roomheight]
2.20
"#;
    let paths = parse_paths(paths_text).expect("paths should parse");
    assert_eq!(paths.sound_packs.len(), 1);
    assert_eq!(paths.sound_packs[0].sounds.len(), 3);
    assert_eq!(paths.point_count(), 3);
    assert_eq!(paths.link_count(), 2);
    assert_eq!(paths.points[0].position.x, 0.85);
    assert_eq!(paths.points[1].position.y, 3.50);
    assert_eq!(paths.points[2].position.z, 0.35);
    assert_eq!(paths.links[0].from, 0);
    assert_eq!(paths.links[0].to, 1);
    assert_eq!(paths.links[1].from, 1);
    assert_eq!(paths.links[1].to, 2);
    assert_eq!(paths.entries.len(), 1);
    assert_eq!(paths.entries[0].pathpoint, 0);
    assert_eq!(paths.exits.len(), 1);
    assert_eq!(paths.exits[0].pathpoint, 2);
    assert_eq!(paths.next_stepsound, Some(0));
    assert_eq!(paths.next_roomheight, Some(2.20));
}

#[test]
fn test_bus_manifest_articulated_and_solo() {
    let main_bus = r#"
[friendlyname]
MAN
GN92
BVG

[description]
Gelenkbus Frontwagen
[end]

[number]
Regs_GN92.org

[registration_list]
Regs_Plates.org

[sound]
sound\sound_GN92.cfg

[paths]
model\paths_GN92_main.cfg

[passengercabin]
model\passengercabin_GN92_main.cfg

[model]
model\model_GN92_main.cfg

[varnamelist]
1
script\main_vars.txt

[script]
1
script\main.osc

[constfile]
1
script\main_const.txt

[mass]
13.8

[coupling_back]
-4.331

[newachse]
achse_long
3.238
achse_maxwidth
2.25
achse_antrieb
0
"#;

    let trailer_bus = r#"
[friendlyname]
MAN
GN92 Trail
BVG

[scriptshare]

[coupling_front]
4.169

[newachse]
achse_long
-1.500
achse_maxwidth
2.25
achse_antrieb
1
"#;

    let main = parse_bus(main_bus).expect("main bus should parse");
    assert_eq!(main.friendly_name.as_ref().unwrap().model, "GN92");
    assert_eq!(
        main.registration_mode,
        RegistrationMode::List("Regs_Plates.org".to_string())
    );
    assert_eq!(main.coupling_back_y, Some(-4.331));
    assert_eq!(main.coupling_front_y, None);
    assert!(!main.script_share);
    assert_eq!(main.mass_tonnes, Some(13.8));
    assert_eq!(main.axles.len(), 1);
    assert!(!main.axles[0].is_driven);

    let trail = parse_bus(trailer_bus).expect("trailer bus should parse");
    assert_eq!(trail.friendly_name.as_ref().unwrap().model, "GN92 Trail");
    assert!(trail.script_share);
    assert_eq!(trail.coupling_front_y, Some(4.169));
    assert_eq!(trail.coupling_back_y, None);
    assert_eq!(trail.axles.len(), 1);
    assert!(trail.axles[0].is_driven);
}

#[test]
fn test_cross_format_linkage() {
    let bus_content = r#"
[friendlyname]
Solaris
Urbino 12
Standard

[number]
regs.org

[paths]
paths.cfg
"#;
    let org_content = "1201\n1202\n1203\n";
    let paths_content = r#"
[pathpnt]
0.0
0.0
0.0
"#;

    let bus = parse_bus(bus_content).expect("bus parses");
    assert_eq!(bus.number_file.as_deref(), Some("regs.org"));
    assert_eq!(bus.paths_cfg.as_deref(), Some("paths.cfg"));

    let org = parse_org(org_content).expect("org parses");
    assert_eq!(org.total_count(), 3);

    let paths = parse_paths(paths_content).expect("paths parses");
    assert_eq!(paths.point_count(), 1);
}

#[test]
fn test_robustness_empty_and_short_inputs() {
    // 0-byte
    let empty = b"";
    assert!(RegistrationPool::parse_bytes(empty).is_err());
    assert!(MoneySystem::parse_bytes(empty).is_err());
    // Empty paths & bus produce valid defaults without panicking
    let p = PassengerPaths::parse_bytes(empty).expect("empty paths gives empty struct");
    assert_eq!(p.point_count(), 0);
    let b = BusManifest::parse_bytes(empty).expect("empty bus gives empty manifest");
    assert_eq!(b.axles.len(), 0);

    // 1-byte
    let single = b"\n";
    assert!(RegistrationPool::parse_bytes(single).is_err());
    assert!(MoneySystem::parse_bytes(single).is_err());
    let _ = PassengerPaths::parse_bytes(single);
    let _ = BusManifest::parse_bytes(single);
}

#[test]
fn test_robustness_pseudorandom_fuzzing() {
    let mut state: u64 = 0xDEADBEEFCAFE;
    let mut next_byte = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (state >> 33) as u8
    };

    for _ in 0..1000 {
        let len = (next_byte() as usize % 256) + 1;
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push(next_byte());
        }

        // None of these parsers should panic on random fuzzed input
        let _ = RegistrationPool::parse_bytes(&buf);
        let _ = MoneySystem::parse_bytes(&buf);
        let _ = PassengerPaths::parse_bytes(&buf);
        let _ = BusManifest::parse_bytes(&buf);
    }
}
