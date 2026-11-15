//! The HOF fixture of the callback tests: a small route with its terminus and stop strings.

pub(super) const DEMO_HOF: &str = r#"
[name]
Aachen Demo

[servicetrip]
Betriebsfahrt

[global_strings]
2
Aachen_Announcements
Aachen_Rollband

stringcount_terminus
4

[addterminus_list]
	11	Vaals	VAALS	Vaals Flats	VAALS	Vaals
	22	Bushof	BUSHOF	Aachen Bushof	BUSHOF	Bushof
[end]

stringcount_busstop
3

[addbusstop_list]
Ponttor	PONTTOR	Ponttor	Ponttor
Bushof	BUSHOF	Bushof	Bushof
[end]

[infosystem_trip]
3301
FUCHSERDE - VAALS
11
33

[infosystem_busstop_list]
2
Bushof
Ponttor
"#;
