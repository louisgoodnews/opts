//! `Chrono.cfg` fields and display names of `entry`.

use super::super::*;

/// The grammar of `Chrono.cfg`: dates, name, description up to `[end]` (with blank lines),
/// deactivated lines, repeated ticket packs, money system.
#[test]
fn chrono_cfg_fields() {
    let text = "\r\n\r\n[name]\r\nFahrplanwechsel 23.5.1993\r\n\r\n[description]\r\n* Neue KI-Linie\r\n\r\nzweite Zeile\r\n[end]\r\n\r\n[startdate]\r\n19930523\r\n\r\n[enddate]\r\n19940528\r\n[deactivate_lines]\r\n2\r\nRelais\r\nE\r\n[ticketpack]\r\nTicketpacks\\a.otp\r\n[ticketpack]\r\nTicketpacks\\b.otp\r\n\r\n[moneysystem]\r\nMoney\\DM\\DM_93.cfg";
    let mut e = ChronoEntry::default();
    e.read_cfg(text.as_bytes());
    assert_eq!(e.name.as_deref(), Some("Fahrplanwechsel 23.5.1993"));
    assert_eq!(
        e.description.as_deref(),
        Some("* Neue KI-Linie\n\nzweite Zeile")
    );
    assert_eq!(e.start, Some(ChronoDate::from_ymd(1993, 5, 23)));
    assert_eq!(e.end, Some(ChronoDate::from_ymd(1994, 5, 28)));
    assert_eq!(e.deactivate_lines, vec!["Relais", "E"]);
    assert_eq!(
        e.ticketpacks,
        vec!["Ticketpacks\\a.otp", "Ticketpacks\\b.otp"]
    );
    assert_eq!(e.moneysystem.as_deref(), Some("Money\\DM\\DM_93.cfg"));
    // cp1252 is decoded.
    let mut e = ChronoEntry::default();
    e.read_cfg(b"[name]\r\nGr\xfcn\r\n");
    assert_eq!(e.name.as_deref(), Some("Grün"));
}

#[test]
fn display_names_prefer_the_language() {
    let e = ChronoEntry {
        rel: "grp/0200_Grenzoeffnung".into(),
        localized_names: vec![
            ("DEU".into(), "Grenzöffnung".into()),
            ("ENG".into(), "Fall of the Wall".into()),
        ],
        ..ChronoEntry::default()
    };
    assert_eq!(e.display_name("DEU"), "Grenzöffnung");
    assert_eq!(e.display_name("FRA"), "Fall of the Wall");
    let bare = ChronoEntry {
        rel: "grp/0200_Grenzoeffnung".into(),
        ..ChronoEntry::default()
    };
    assert_eq!(bare.display_name("ENG"), "0200_Grenzoeffnung");
}
