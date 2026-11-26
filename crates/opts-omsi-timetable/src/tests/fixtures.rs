//! Shared fixtures of the tests: the trip, line, stop and link texts, and a timetable.

use super::super::*;

pub(super) const TTP: &str = "-----------------------\r\nTime Table Trip File\r\n-----------------------\r\n\r\n[trip]\r\n\r\nHoengen I G A\r\n11\r\n\r\n.........................\r\n        Stations\r\n.........................\r\n\r\n[station_typ2]\r\n100\r\n\r\n[station_typ2]\r\n101\r\n\r\n[station_typ2]\r\n102\r\n\r\n[station_typ2]\r\n103\r\n\r\n[station_typ2]\r\n104\r\n\r\n[profile]\r\nstandard\r\n8.000\r\n\r\n[profile_man_arr_time]\r\n0\r\n0.000\r\n[profile_man_arr_time]\r\n2\r\n2.000\r\n[profile_otherstopping]\r\n2\r\n30\r\n[profile_man_dep_time]\r\n2\r\n3.000\r\n[profile_man_arr_time]\r\n4\r\n8.000\r\n[profile]\r\nBlockverkehr\r\n10.000\r\n";

pub(super) const TTL: &str = "------------------------\r\nTime Table Line File\r\n------------------------\r\n\r\n[userallowed]\r\n\r\n[priority]\r\n2\r\n------------------------------------\r\n\r\n[newtour]\r\nMOFR_NORD\r\nGelenk ASEAG\r\n799\r\n\r\n------------------------------------\r\n\r\n  Dep.: 4:54:0\r\n[addtrip]\r\n11_IGA\r\n1\r\n294.000\r\n\r\n  Dep.: 5:24:0\r\n[addtrip]\r\n11_IGA\r\n0\r\n324.000\r\n";

pub(super) const STOPS: &str = "---------------------------\r\nTime Table BusStopList File\r\n---------------------------\r\n\r\n[busstop]\r\nHeuvel\r\n79\r\n100\r\n5.0000000000\r\n0\r\n\r\n\r\n[busstop]\r\n\r\n58\r\n101\r\n2.0000000000\r\n0\r\n\r\n";

pub(super) fn timetables() -> Timetables {
    Timetables::from_parts(
        parse_busstops(STOPS.as_bytes()),
        vec![("11".into(), parse_line(TTL.as_bytes()))],
        vec![("11_IGA".into(), parse_trip(TTP.as_bytes()))],
    )
}

pub(super) const LINKS: &str = "---------------------------\r\nTime Table StnLinkList File\r\n---------------------------\r\n\r\nCreated with Omsi Version 2.2.032\r\nDate: 05.10.2015\r\n\r\nPonttor ==> Bushof\r\n\r\n[StnLink]\r\n600.0000000000\r\n100\r\n101\r\n1.1300000000\r\n4.0600000000\r\n87.0000000000\r\n64.8000000000\r\n0\r\n1\r\n\r\n   0:\r\n[StnLink_entry]\r\n17754\r\n0\r\n7\r\n400.0000000000\r\n-1\r\n0\r\n0\r\n\r\n   1:\r\n[StnLink_entry]\r\n17753\r\n0\r\n7\r\n200.0000000000\r\n-1\r\n0\r\n0\r\n\r\nBushof ==> Vaals\r\n\r\n[StnLink]\r\n1400.0\r\n101\r\n102\r\n4.06\r\n4.19\r\n64.8\r\n15.1\r\n0\r\n0\r\n\r\n   0:\r\n[StnLink_entry]\r\n161539\r\n7\r\n3\r\n1400.0\r\n-1\r\n0\r\n1\r\n";
