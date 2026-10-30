use super::*;

#[test]
fn the_doors_open_on_arrival_close_after_the_passengers_and_reset_on_leaving() {
    let (d, v) = next_doors(Doors::Away, false, false);
    assert_eq!((d, v), (Doors::Away, None));
    let (d, v) = next_doors(Doors::Away, true, true);
    assert_eq!((d, v), (Doors::Open, Some(1.0)));
    let (d, v) = next_doors(Doors::Open, true, true);
    assert_eq!((d, v), (Doors::Open, None));
    let (d, v) = next_doors(Doors::Open, true, false);
    assert_eq!((d, v), (Doors::Closing, Some(-1.0)));
    let (d, v) = next_doors(Doors::Closing, true, false);
    assert_eq!((d, v), (Doors::Closing, None));
    let (d, v) = next_doors(Doors::Closing, false, false);
    assert_eq!((d, v), (Doors::Away, Some(0.0)));
}
