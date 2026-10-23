//! Passenger doors do not open while the vehicle moves.
//!
//! The vehicle scripts have interlocks of their own (their limits range from 2 to 10 km/h and
//! some buses have none that a click gets through), so the host holds the door triggers back
//! above a few km/h: the door keys and hot-spots do nothing until the vehicle has nearly stopped.
//! Triggers that are not about opening a passenger door (the stop request, the lock switches, the
//! driver's door) are not held back.

/// Above this road speed (km/h, either direction) the passenger doors cannot be operated.
pub const DOORS_LOCKED_ABOVE_KMH: f32 = 3.0;

/// Whether trigger `name` (a key or hot-spot trigger, with its `_drag` or `_off` suffix) operates a
/// passenger door.
#[must_use]
pub fn operates_passenger_door(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    let n = n
        .strip_suffix("_drag")
        .or_else(|| n.strip_suffix("_off"))
        .unwrap_or(&n);
    if n.contains("lock") || n.contains("wunsch") || n.contains("fahrer") || n.contains("halt") {
        return false;
    }
    // `door0`, `door12`, `bus_doorfront0`, `bus_doormiddle1`, `bus_doorrear0`, `cp_bus_door_all`,
    // `door_all`.
    let rest = n
        .strip_prefix("cp_bus_door")
        .or_else(|| n.strip_prefix("bus_door"))
        .or_else(|| n.strip_prefix("door"));
    let Some(rest) = rest else {
        return false;
    };
    let rest = rest.trim_start_matches('_');
    rest == "all"
        || ["front", "middle", "rear", "mid", "back"]
            .iter()
            .any(|p| rest.starts_with(p))
        || (!rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
}

/// Whether the door trigger `name` must be dropped at road speed `speed_kmh`.
#[must_use]
pub fn held_back(name: &str, speed_kmh: f32) -> bool {
    speed_kmh.abs() > DOORS_LOCKED_ABOVE_KMH && operates_passenger_door(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_door_triggers_of_the_vehicles_are_recognised_and_the_others_are_not() {
        for yes in [
            "door0",
            "door1_drag",
            "door3_off",
            "bus_doorfront0",
            "bus_doormiddle1",
            "bus_doorrear0",
            "cp_bus_door_all",
            "door_all",
        ] {
            assert!(operates_passenger_door(yes), "{yes}");
        }
        for no in [
            "bus_doorfront0_lock",
            "door_haltewunsch_1",
            "door_aussenoeffner",
            "cp_Fahrertuer",
            "door_halt_inw",
            "kw_s_N",
            "ev_doortriggeropen_0",
            "automatic_D",
        ] {
            assert!(!operates_passenger_door(no), "{no}");
        }
    }

    #[test]
    fn only_a_moving_vehicle_holds_them_back() {
        assert!(held_back("door0", 20.0));
        assert!(held_back("bus_doorfront0", -5.0));
        assert!(!held_back("door0", 1.0));
        assert!(!held_back("door_haltewunsch_1", 50.0));
    }
}
