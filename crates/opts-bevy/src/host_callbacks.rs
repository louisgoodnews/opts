//! The callbacks OMSI offers its vehicle scripts (`(M.V.name)`).
//!
//! Scripts call the host for data the scripts cannot know: random numbers fixed per vehicle,
//! operational depot data (`.hof`), timetable state, tickets, passengers, and ground elevation.
//! Specification: `docs/src/formats/text/vehicles/callbacks.md` (M-0093).

use std::sync::Arc;

use opts_core::formats::hof::HofFile;
use opts_sim::script::{ExternalHandler, Stacks};

/// Context for vehicle script callbacks (`(M.V.name)`).
#[derive(Clone, Default)]
pub struct VehicleHostContext {
    /// Deterministic seed identifying this vehicle instance.
    pub seed: u64,
    /// Active depot (`.hof`) operational data, if loaded.
    pub hof: Option<Arc<HofFile>>,
    /// Active route index in the `.hof` file, if scheduled.
    pub active_route_index: Option<usize>,
    /// Current bus stop index along the active route.
    pub current_stop_index: usize,
    /// Timetable delay in seconds (positive = late, negative = early).
    pub delay_seconds: f32,
    /// Line string shown on line displays (e.g. "33", "109").
    pub line_string: String,
}

/// SplitMix64 finaliser: a well-mixed 64-bit value from `x`.
fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Creates a callback handler for a vehicle identified by `seed`, without depot or timetable data.
#[must_use]
pub fn vehicle_callbacks(seed: u64) -> ExternalHandler {
    vehicle_callbacks_with_context(VehicleHostContext {
        seed,
        ..Default::default()
    })
}

/// Creates a callback handler for a vehicle with depot and timetable context.
#[must_use]
pub fn vehicle_callbacks_with_context(ctx: VehicleHostContext) -> ExternalHandler {
    Box::new(move |name, st: &mut Stacks| {
        match name {
            // 1. Randomness
            // `n (M.V.NrSpecRandom)` -> r in [0, 1) fixed per vehicle and n.
            "nrspecrandom" => {
                let n = st.numbers.pop().unwrap_or(0.0);
                let bits = mix(ctx.seed ^ mix(u64::from(n.round() as i64 as u64 as u32)));
                st.numbers.push((bits >> 40) as f32 / 16_777_216.0);
                true
            }

            // 2. Terminus Callbacks (.hof)
            // `code (M.V.GetTerminusIndex)` -> idx (0-based row index in terminus table, or -1)
            "getterminusindex" => {
                let code = st.numbers.pop().unwrap_or(0.0).round() as i32;
                let idx = ctx
                    .hof
                    .as_ref()
                    .and_then(|h| h.find_terminus_by_code(code))
                    .map(|(i, _)| i as f32)
                    .unwrap_or(-1.0);
                st.numbers.push(idx);
                true
            }

            // `termIdx col (M.V.GetTerminusString)` -> string on string stack
            "getterminusstring" => {
                let col = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let term_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let text = ctx
                    .hof
                    .as_ref()
                    .and_then(|h| h.terminus_string(term_idx, col))
                    .unwrap_or("");
                st.strings.push(text.to_string());
                true
            }

            // `termIdx (M.V.GetTerminusCode)` -> code
            "getterminuscode" => {
                let term_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let code = ctx
                    .hof
                    .as_ref()
                    .and_then(|h| h.terminus(term_idx))
                    .map(|t| t.code as f32)
                    .unwrap_or(-1.0);
                st.numbers.push(code);
                true
            }

            // 3. Route Callbacks (.hof)
            // `routeCode (M.V.GetRouteIndex)` -> routeIdx (or -1)
            "getrouteindex" => {
                let route_code = st.numbers.pop().unwrap_or(0.0).round() as i32;
                let idx = ctx
                    .hof
                    .as_ref()
                    .and_then(|h| h.find_route_by_code(route_code))
                    .map(|(i, _)| i as f32)
                    .unwrap_or(-1.0);
                st.numbers.push(idx);
                true
            }

            // `routeIdx stopNo (M.V.GetRouteBusstopIdent)` -> ident string
            "getroutebusstopident" => {
                let stop_no = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let route_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let ident = ctx
                    .hof
                    .as_ref()
                    .and_then(|h| h.route(route_idx))
                    .and_then(|r| r.busstops.get(stop_no))
                    .map(String::as_str)
                    .unwrap_or("");
                st.strings.push(ident.to_string());
                true
            }

            // `routeIdx (M.V.GetRouteTerminusIndex)` -> terminus row index of the route
            "getrouteterminusindex" => {
                let route_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let term_idx = ctx
                    .hof
                    .as_ref()
                    .and_then(|h| h.route(route_idx))
                    .and_then(|r| {
                        ctx.hof
                            .as_ref()
                            .and_then(|h| h.find_terminus_by_code(r.terminus_code))
                    })
                    .map(|(i, _)| i as f32)
                    .unwrap_or(-1.0);
                st.numbers.push(term_idx);
                true
            }

            // `routeIdx (M.V.GetBusstopCount)` -> stop count of route
            "getbusstopcount" => {
                let route_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let count = ctx
                    .hof
                    .as_ref()
                    .and_then(|h| h.route(route_idx))
                    .map(|r| r.busstops.len() as f32)
                    .unwrap_or(0.0);
                st.numbers.push(count);
                true
            }

            // 4. Bus Stop Callbacks (.hof)
            // `ident:str (M.V.GetBusstopIndex)` -> busstopIdx (or -1)
            "getbusstopindex" => {
                let ident = st.strings.pop().unwrap_or_default();
                let idx = ctx
                    .hof
                    .as_ref()
                    .and_then(|h| h.find_busstop_by_ident(&ident))
                    .map(|(i, _)| i as f32)
                    .unwrap_or(-1.0);
                st.numbers.push(idx);
                true
            }

            // `busstopIdx col (M.V.GetBusstopString)` -> string
            "getbusstopstring" => {
                let col = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let stop_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let text = ctx
                    .hof
                    .as_ref()
                    .and_then(|h| h.busstop_string(stop_idx, col))
                    .unwrap_or("");
                st.strings.push(text.to_string());
                true
            }

            // 5. Depot Global Strings
            // `slot (M.V.GetDepotStringGlobal)` -> string
            "getdepotstringglobal" => {
                let slot = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let text = ctx
                    .hof
                    .as_ref()
                    .and_then(|h| h.global_string(slot))
                    .unwrap_or("");
                st.strings.push(text.to_string());
                true
            }

            // 6. Timetable Callbacks
            // `(M.V.GetTTLineString)` -> string
            "getttlinestring" => {
                let line_str = if !ctx.line_string.is_empty() {
                    ctx.line_string.clone()
                } else if let Some(r) = ctx
                    .active_route_index
                    .and_then(|idx| ctx.hof.as_ref()?.route(idx))
                {
                    r.line_string.clone()
                } else {
                    String::new()
                };
                st.strings.push(line_str);
                true
            }

            // `(M.V.GetTTTerminusIndex)` -> terminus index
            "getttterminusindex" => {
                let term_idx = ctx
                    .active_route_index
                    .and_then(|idx| ctx.hof.as_ref()?.route(idx))
                    .and_then(|r| ctx.hof.as_ref()?.find_terminus_by_code(r.terminus_code))
                    .map(|(i, _)| i as f32)
                    .unwrap_or(-1.0);
                st.numbers.push(term_idx);
                true
            }

            // `(M.V.GetTTBusstopCount)` -> count of stops on active schedule
            "getttbusstopcount" => {
                let count = ctx
                    .active_route_index
                    .and_then(|idx| ctx.hof.as_ref()?.route(idx))
                    .map(|r| r.busstops.len() as f32)
                    .unwrap_or(0.0);
                st.numbers.push(count);
                true
            }

            // `(M.V.GetTTBusstopIndex)` -> current stop index (0-based)
            "getttbusstopindex" => {
                st.numbers.push(ctx.current_stop_index as f32);
                true
            }

            // `stopNo (M.V.GetTTBusstopName)` -> stop name/ident string
            "getttbusstopname" => {
                let stop_no = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let name = ctx
                    .active_route_index
                    .and_then(|idx| ctx.hof.as_ref()?.route(idx))
                    .and_then(|r| r.busstops.get(stop_no))
                    .map(String::as_str)
                    .unwrap_or("");
                st.strings.push(name.to_string());
                true
            }

            // `(M.V.GetTTDelay)` -> delay in seconds
            "getttdelay" => {
                st.numbers.push(ctx.delay_seconds);
                true
            }

            // `stopNo (M.V.GetTTBusstopDep)` -> departure time in seconds of day
            "getttbusstopdep" => {
                let _stop_no = st.numbers.pop();
                st.numbers.push(0.0);
                true
            }

            // `stopNo (M.V.GetTTBusstopArr)` -> arrival time in seconds of day
            "getttbusstoparr" => {
                let _stop_no = st.numbers.pop();
                st.numbers.push(0.0);
                true
            }

            // Fonts & Metrics
            //  -> font handle/index
            "getfontindex" => {
                let _name = st.strings.pop();
                st.numbers.push(1.0);
                true
            }

            //  -> text width in pixels
            "textlength" => {
                let _font = st.numbers.pop().unwrap_or(0.0);
                let t = st.strings.pop().unwrap_or_default();
                st.numbers.push((t.chars().count() * 8) as f32);
                true
            }

            // 7. Tickets & Money
            // `ticketIdx (M.V.GetTicketName)` -> string
            "getticketname" => {
                let _idx = st.numbers.pop();
                st.strings.push("Normalfahrschein".to_string());
                true
            }

            // `ticketIdx (M.V.GetTicketValue)` -> price
            "getticketvalue" => {
                let _idx = st.numbers.pop();
                st.numbers.push(2.50);
                true
            }

            // `coinIdx (M.V.GiveChangeCoin)` -> void
            "givechangecoin" => {
                let _coin_idx = st.numbers.pop();
                true
            }

            // 8. Passengers & Geometry
            // `seatIdx (M.V.GetHumanCountOnSeat)` -> count
            "gethumancountonseat" => {
                let _seat = st.numbers.pop();
                st.numbers.push(0.0);
                true
            }

            // `linkIdx (M.V.GetHumanCountOnPathLink)` -> count
            "gethumancountonpathlink" => {
                let _link = st.numbers.pop();
                st.numbers.push(0.0);
                true
            }

            // `x y z (M.V.GetHeightAbovePoint)` -> ground/kerb height
            "getheightabovepoint" => {
                let _z = st.numbers.pop();
                let _y = st.numbers.pop();
                let _x = st.numbers.pop();
                st.numbers.push(0.0);
                true
            }

            _ => false,
        }
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn aachen_line_33_destination_and_cabin_monitor_resolution() {
        let aachen_path = std::path::Path::new(
            "/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2/Vehicles/MAN_NL_NG/Aachen.hof",
        );
        if !aachen_path.exists() {
            return;
        }
        let bytes = std::fs::read(aachen_path).expect("read Aachen.hof");
        let hof = Arc::new(HofFile::parse_bytes(&bytes).expect("parse Aachen.hof"));
        assert_eq!(hof.name, "Aachen");

        // Route 3301: Line 33 Route 01 (FUCHSERDE >> VAALS)
        let (route_idx, route) = hof.find_route_by_code(3301).expect("find route 3301");
        assert_eq!(route.terminus_code, 11);
        assert!(!route.busstops.is_empty());

        let ctx = VehicleHostContext {
            seed: 1234,
            hof: Some(hof.clone()),
            active_route_index: Some(route_idx),
            current_stop_index: 0,
            delay_seconds: 0.0,
            line_string: "33".to_string(),
        };
        let mut h = vehicle_callbacks_with_context(ctx);

        let mut st = Stacks::default();

        // 1. Matrix display queries active timetable line string
        assert!(h("getttlinestring", &mut st));
        assert_eq!(st.strings.pop(), Some("33".to_string()));

        // 2. Matrix display queries timetable destination terminus index
        assert!(h("getttterminusindex", &mut st));
        let term_idx = st.numbers.pop().expect("term_idx");
        assert!(term_idx >= 0.0);

        // 3. Front matrix queries destination lines
        // string 1: Front-ANNAX upper line
        st.numbers.push(term_idx);
        st.numbers.push(1.0);
        assert!(h("getterminusstring", &mut st));
        assert_eq!(st.strings.pop(), Some("V A A L S*B".to_string()));

        // string 2: Front-ANNAX lower line
        st.numbers.push(term_idx);
        st.numbers.push(2.0);
        assert!(h("getterminusstring", &mut st));
        assert_eq!(st.strings.pop(), Some("FLATS".to_string()));

        // 4. Cabin monitor queries stop names
        assert!(h("getttbusstopcount", &mut st));
        let stop_count = st.numbers.pop().expect("stop_count");
        assert!(stop_count > 0.0);

        st.numbers.push(0.0); // stop 0
        assert!(h("getttbusstopname", &mut st));
        let first_stop = st.strings.pop().expect("first stop name");
        assert!(!first_stop.is_empty());
    }

    #[test]
    fn nr_spec_random_is_stable_per_vehicle_and_number() {
        let mut a = vehicle_callbacks(1);
        let mut b = vehicle_callbacks(2);
        let call = |h: &mut ExternalHandler, n: f32| {
            let mut st = Stacks::default();
            st.numbers.push(n);
            assert!(h("nrspecrandom", &mut st));
            st.numbers[0]
        };
        let (x, y) = (call(&mut a, 301.0), call(&mut a, 302.0));
        assert_eq!(x, call(&mut a, 301.0));
        assert_ne!(x, y);
        assert_ne!(x, call(&mut b, 301.0));
        assert!((0.0..1.0).contains(&x));
        assert!(!a("unknown", &mut Stacks::default()));
    }

    use super::*;

    const DEMO_HOF: &str = r#"
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

    #[test]
    fn host_callbacks_resolve_hof_data() {
        let hof = Arc::new(HofFile::parse_str(DEMO_HOF).expect("parse demo hof"));
        let ctx = VehicleHostContext {
            seed: 42,
            hof: Some(hof),
            active_route_index: Some(0),
            current_stop_index: 1,
            delay_seconds: 45.0,
            line_string: "33".to_string(),
        };
        let mut h = vehicle_callbacks_with_context(ctx);

        // Terminus callbacks
        let mut st = Stacks::default();
        st.numbers.push(11.0);
        assert!(h("getterminusindex", &mut st));
        assert_eq!(st.numbers.pop(), Some(0.0));

        st.numbers.push(0.0); // termIdx
        st.numbers.push(1.0); // col 1
        assert!(h("getterminusstring", &mut st));
        assert_eq!(st.strings.pop(), Some("Vaals Flats".to_string()));

        st.numbers.push(0.0);
        assert!(h("getterminuscode", &mut st));
        assert_eq!(st.numbers.pop(), Some(11.0));

        // Route callbacks
        st.numbers.push(3301.0);
        assert!(h("getrouteindex", &mut st));
        assert_eq!(st.numbers.pop(), Some(0.0));

        st.numbers.push(0.0); // routeIdx
        st.numbers.push(1.0); // stopNo 1
        assert!(h("getroutebusstopident", &mut st));
        assert_eq!(st.strings.pop(), Some("Ponttor".to_string()));

        st.numbers.push(0.0); // routeIdx
        assert!(h("getrouteterminusindex", &mut st));
        assert_eq!(st.numbers.pop(), Some(0.0)); // terminus index for code 11

        st.numbers.push(0.0);
        assert!(h("getbusstopcount", &mut st));
        assert_eq!(st.numbers.pop(), Some(2.0));

        // Bus stop callbacks
        st.strings.push("ponttor".to_string());
        assert!(h("getbusstopindex", &mut st));
        assert_eq!(st.numbers.pop(), Some(0.0));

        st.numbers.push(0.0); // stopIdx
        st.numbers.push(1.0); // col 1
        assert!(h("getbusstopstring", &mut st));
        assert_eq!(st.strings.pop(), Some("Ponttor".to_string()));

        // Depot global strings
        st.numbers.push(0.0);
        assert!(h("getdepotstringglobal", &mut st));
        assert_eq!(st.strings.pop(), Some("Aachen_Announcements".to_string()));

        // Timetable callbacks
        assert!(h("getttlinestring", &mut st));
        assert_eq!(st.strings.pop(), Some("33".to_string()));

        assert!(h("getttterminusindex", &mut st));
        assert_eq!(st.numbers.pop(), Some(0.0));

        assert!(h("getttbusstopcount", &mut st));
        assert_eq!(st.numbers.pop(), Some(2.0));

        assert!(h("getttbusstopindex", &mut st));
        assert_eq!(st.numbers.pop(), Some(1.0));

        st.numbers.push(0.0);
        assert!(h("getttbusstopname", &mut st));
        assert_eq!(st.strings.pop(), Some("Bushof".to_string()));

        assert!(h("getttdelay", &mut st));
        assert_eq!(st.numbers.pop(), Some(45.0));

        // Tickets & Passengers & Geometry
        st.numbers.push(0.0);
        assert!(h("getticketvalue", &mut st));
        assert_eq!(st.numbers.pop(), Some(2.50));

        st.numbers.push(0.0);
        assert!(h("getticketname", &mut st));
        assert_eq!(st.strings.pop(), Some("Normalfahrschein".to_string()));

        st.numbers.push(1.0);
        assert!(h("givechangecoin", &mut st));

        st.numbers.push(3.0);
        assert!(h("gethumancountonseat", &mut st));
        assert_eq!(st.numbers.pop(), Some(0.0));

        st.numbers.push(1.0);
        st.numbers.push(2.0);
        st.numbers.push(3.0);
        assert!(h("getheightabovepoint", &mut st));
        assert_eq!(st.numbers.pop(), Some(0.0));
    }
}
