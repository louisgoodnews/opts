//! Unit tests of `hof.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;

const DEMO_HOF: &str = r#"
[name]
Demo Depot

[servicetrip]
Betriebsfahrt

[global_strings]
3
Demo_Announcements
Demo_Rollband
Demo_Sidesign

stringcount_terminus
4

[addterminus_list]
	1	Depot	DEPOT		DEPOT	DEPOT
	{ALLEX}	2	Endstation	END		END	END
[end]

stringcount_busstop
4

[addbusstop_list]
Hbf	HBF	Hauptbahnhof		Hauptbahnhof
Markt	MARKT	Marktplatz		Marktplatz
[end]

[infosystem_trip]
1001
D-H
1
10

[infosystem_busstop_list]
2
Hbf
Markt
"#;

#[test]
fn parses_demo_hof() {
    let hof = HofFile::parse_str(DEMO_HOF).expect("parse demo hof");
    assert_eq!(hof.name, "Demo Depot");
    assert_eq!(hof.service_trip, "Betriebsfahrt");
    assert_eq!(hof.stringcount_terminus, 4);
    assert_eq!(hof.stringcount_busstop, 4);
    assert_eq!(hof.global_strings.len(), 3);
    assert_eq!(hof.global_string(0), Some("Demo_Announcements"));
    assert_eq!(hof.global_string(1), Some("Demo_Rollband"));

    assert_eq!(hof.termini.len(), 2);
    assert_eq!(hof.termini[0].code, 1);
    assert_eq!(hof.termini[0].ident, "Depot");
    assert_eq!(hof.termini[0].strings.len(), 4);
    assert_eq!(hof.termini[0].strings[0], "DEPOT");
    assert!(!hof.termini[0].all_exit);

    assert_eq!(hof.termini[1].code, 2);
    assert_eq!(hof.termini[1].ident, "Endstation");
    assert!(hof.termini[1].all_exit);

    assert_eq!(hof.busstops.len(), 2);
    assert_eq!(hof.busstops[0].ident, "Hbf");
    assert_eq!(hof.busstops[0].strings.len(), 4);
    assert_eq!(hof.busstops[0].strings[0], "HBF");
    assert_eq!(hof.busstops[0].strings[1], "Hauptbahnhof");

    assert_eq!(hof.routes.len(), 1);
    assert_eq!(hof.routes[0].route_code, 1001);
    assert_eq!(hof.routes[0].route_string, "D-H");
    assert_eq!(hof.routes[0].terminus_code, 1);
    assert_eq!(hof.routes[0].line_string, "10");
    assert_eq!(hof.routes[0].busstops, vec!["Hbf", "Markt"]);

    // Lookup functions
    assert_eq!(hof.find_terminus_by_code(1).map(|(i, _)| i), Some(0));
    assert_eq!(hof.terminus_string(0, 0), Some("DEPOT"));
    assert_eq!(hof.find_busstop_by_ident("hbf").map(|(i, _)| i), Some(0));
    assert_eq!(hof.busstop_string(0, 1), Some("Hauptbahnhof"));
    assert_eq!(hof.find_route_by_code(1001).map(|(i, _)| i), Some(0));
}

#[test]
fn parses_legacy_single_entries() {
    let text = r#"
stringcount_terminus
3
stringcount_busstop
2

[addterminus]
10
Zentrum
ZENTRUM
Zentrum
Zent.
................

[addterminus_allexit]
20
Betriebshof
BETRIEBSHOF
Betriebshof
Btrh.
................

[addbusstop]
Haltestelle1
H1
Haltestelle Eins
................
"#;
    let hof = HofFile::parse_str(text).expect("parse legacy entries");
    assert_eq!(hof.termini.len(), 2);
    assert_eq!(hof.termini[0].code, 10);
    assert_eq!(hof.termini[0].ident, "Zentrum");
    assert_eq!(hof.termini[0].strings, vec!["ZENTRUM", "Zentrum", "Zent."]);
    assert!(!hof.termini[0].all_exit);

    assert_eq!(hof.termini[1].code, 20);
    assert_eq!(hof.termini[1].ident, "Betriebshof");
    assert!(hof.termini[1].all_exit);

    assert_eq!(hof.busstops.len(), 1);
    assert_eq!(hof.busstops[0].ident, "Haltestelle1");
    assert_eq!(hof.busstops[0].strings, vec!["H1", "Haltestelle Eins"]);
}
