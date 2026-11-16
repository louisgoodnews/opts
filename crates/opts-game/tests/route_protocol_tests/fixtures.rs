//! The depot timetable and keypad fixtures of the route tests.

pub(super) const HOF: &str = "
[name]
Demo

[servicetrip]
Betriebsfahrt

[global_strings]
2
a
b

stringcount_terminus
2

[addterminus_list]
	11	Vaals	VAALS
	22	Bushof	BUSHOF
[end]

stringcount_busstop
2

[addbusstop_list]
Ponttor	PONTTOR
Bushof	BUSHOF
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

[infosystem_trip]
3302
VAALS - FUCHSERDE
22
33

[infosystem_busstop_list]
1
Ponttor
";

/// A keypad that records what is pressed: the digits typed since the last mode key, the route
/// entered with `ibis_eingabe` in mode 1 (line and suffix), 2 (course) and 3 (destination).
pub(super) const IBIS: &str = "
{trigger:ibis_setmode_linie_kurs}\n1 (S.L.mode)\n0 (S.L.input)\n{end}
{trigger:ibis_setmode_route}\n2 (S.L.mode)\n0 (S.L.input)\n{end}
{trigger:ibis_setmode_ziel}\n3 (S.L.mode)\n0 (S.L.input)\n{end}
{trigger:ibis_eingabe}\n(L.L.mode) 1 =\n{if}\n(L.L.input) (S.L.line_complex)\n{endif}\n(L.L.mode) 2 =\n{if}\n(L.L.input) (S.L.course)\n{endif}\n(L.L.mode) 3 =\n{if}\n(L.L.input) (S.L.terminus)\n{endif}\n{end}
{trigger:ibis_0}\n(L.L.input) 10 * 0 + (S.L.input)\n{end}
{trigger:ibis_1}\n(L.L.input) 10 * 1 + (S.L.input)\n{end}
{trigger:ibis_2}\n(L.L.input) 10 * 2 + (S.L.input)\n{end}
{trigger:ibis_3}\n(L.L.input) 10 * 3 + (S.L.input)\n{end}
{trigger:ibis_4}\n(L.L.input) 10 * 4 + (S.L.input)\n{end}
{trigger:ibis_5}\n(L.L.input) 10 * 5 + (S.L.input)\n{end}
{trigger:ibis_6}\n(L.L.input) 10 * 6 + (S.L.input)\n{end}
{trigger:ibis_7}\n(L.L.input) 10 * 7 + (S.L.input)\n{end}
{trigger:ibis_8}\n(L.L.input) 10 * 8 + (S.L.input)\n{end}
{trigger:ibis_9}\n(L.L.input) 10 * 9 + (S.L.input)\n{end}
";
