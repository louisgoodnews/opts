use super::*;

const OTP: &str = "\tDefiniert ein System\n\n\t[ticketpack]\t\tprose that is not a parameter\n\n[voicepath]\nTicketPacks\\HB_76_Bremen\\\n\n***************\nTicketpack für Bremen\n***************\n\n[ticketpack]\n0.2\n0.1\n1.7\n0.3\n\n\n[ticket_2]\nKurzstrecke\nShort trip\n0\n15\n200\n1.10\nKurzstrecke@\n0\n1\n---------------------\n[ticket_2]\nTageskarte\nDay ticket\n0\n0\n200\n6.50\nTages@karte\n1\n0.4\n---------------------\n[ticket]\nKind\nChild\n4\n6\n14\n0.90\nKind@\n";

#[test]
fn behaviour_tickets_and_voices_are_read() {
    let p = TicketPack::parse(OTP);
    assert_eq!(p.behaviour.ticketbuy_prop, 0.1);
    assert_eq!(p.behaviour.chattiness, 1.7);
    assert_eq!(p.voice_path.as_deref(), Some("TicketPacks\\HB_76_Bremen\\"));
    assert_eq!(p.tickets.len(), 3);
    assert_eq!(p.tickets[0].name_english, "Short trip");
    assert_eq!(p.tickets[0].value, 1.10);
    assert!(!p.tickets[0].day_ticket);
    assert!(p.tickets[1].day_ticket);
    assert_eq!(p.tickets[1].probability, 0.4);
    assert_eq!((p.tickets[2].max_stations, p.tickets[2].age_max), (4, 14));
    assert_eq!(p.tickets[2].probability, 1.0, "`[ticket]` has no weight");
    assert_eq!(p.price(1), Some(6.50));
    assert_eq!(p.price(9), None);
}

#[test]
fn eligibility_follows_age_and_stops() {
    let p = TicketPack::parse(OTP);
    let names = |age, stops| p.eligible(age, stops).map(|(i, _)| i).collect::<Vec<_>>();
    assert_eq!(names(30, 3), vec![0, 1], "an adult: not the child ticket");
    assert_eq!(names(10, 3), vec![1, 2], "a child: day and child ticket");
    assert_eq!(names(10, 9), vec![1], "the child ticket covers four stops");
    assert_eq!(names(3, 1), vec![1], "below 15 only the day ticket here");
}

#[test]
fn an_empty_file_gives_the_default_behaviour_and_no_tickets() {
    let p = TicketPack::parse("");
    assert!(p.tickets.is_empty());
    assert_eq!(p.behaviour, Behaviour::default());
}

/// Every installed `.otp` parses with tickets and plausible prices (`OMSI_PATH`, `--ignored`).
#[test]
#[ignore = "needs an OMSI installation"]
fn every_installed_pack_parses() {
    let Ok(root) = std::env::var("OMSI_PATH") else {
        return;
    };
    let mut files = 0;
    let mut tickets = 0;
    for pack in std::fs::read_dir(std::path::Path::new(&root).join("TicketPacks"))
        .expect("TicketPacks")
        .flatten()
    {
        for f in std::fs::read_dir(pack.path())
            .into_iter()
            .flatten()
            .flatten()
        {
            if f.path()
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("otp"))
            {
                let p = TicketPack::parse_bytes(&std::fs::read(f.path()).expect("read"));
                assert!(
                    p.behaviour.ticketbuy_prop == 0.0 || !p.tickets.is_empty(),
                    "{:?} sells nothing though passengers buy",
                    f.path()
                );
                assert!(
                    p.tickets.iter().all(|t| t.value >= 0.0 && t.value < 500.0),
                    "{:?}: implausible price",
                    f.path()
                );
                assert!(p.behaviour.ticketbuy_prop <= 1.0);
                files += 1;
                tickets += p.tickets.len();
            }
        }
    }
    eprintln!("{files} packs, {tickets} tickets");
    assert!(files > 20);
}

#[test]
fn indented_keywords_count_and_a_price_may_be_zero() {
    let p = TicketPack::parse("[ticketpack]\n0.8\n0.2\n0.7\n0.1\n\n[ticket]\nFare cash\nFare cash\n0\n0\n200\n0\nFare cash\n\n\t[ticket]\n\tReduced\n\tReduced\n\t0\n\t0\n\t200\n\t1.00\n\tReduced\n");
    assert_eq!(p.tickets.len(), 2);
    assert_eq!(p.tickets[0].value, 0.0);
    assert_eq!(p.tickets[1].name, "Reduced");
    assert_eq!(p.tickets[1].value, 1.0);
}

#[test]
fn the_fallback_sells_to_adults_and_children() {
    let p = TicketPack::fallback();
    assert!(p.eligible(30, 3).count() >= 3);
    assert!(p.eligible(10, 3).any(|(_, t)| t.name_english == "Child"));
    assert!(p.tickets.iter().all(|t| t.value > 0.0));
}
