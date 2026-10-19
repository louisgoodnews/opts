//! The callbacks OMSI offers its vehicle scripts (`(M.V.name)`).
//!
//! Scripts call the host for data the scripts cannot know: random numbers fixed per vehicle,
//! operational depot data (`.hof`), timetable state, tickets, passengers, and ground elevation.
//! Specification: `docs/src/formats/text/vehicles/callbacks.md` (M-0093).

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use opts_core::formats::oft::OftFont;
use opts_core::formats::script_texture::{CpuScriptTexture, ScriptTextureCanvas};
use opts_sim::script::{ExternalHandler, Stacks};

/// Context for vehicle script callbacks (`(M.V.name)`).
#[derive(Clone, Default)]
pub struct VehicleHostContext {
    /// Deterministic seed identifying this vehicle instance.
    pub seed: u64,
    /// The vehicle's depot and the driver's selection (`.hof`), shared with the protocol.
    pub route: crate::route::RouteHandle,
    /// Script textures for this vehicle (`[scripttexture]`).
    pub script_textures: Arc<RwLock<Vec<CpuScriptTexture>>>,
    /// Loaded fonts for this vehicle / game session.
    pub fonts: Arc<RwLock<Vec<OftFont>>>,
    /// Base directory for resolving relative texture paths in `STLoadTex`.
    pub texture_base_dir: Option<PathBuf>,
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
                    .route
                    .with(|r| r.hof.as_ref()?.find_terminus_by_code(code).map(|(i, _)| i));
                st.numbers.push(idx.map_or(-1.0, |i| i as f32));
                true
            }

            // `termIdx col (M.V.GetTerminusString)` -> string on string stack
            "getterminusstring" => {
                let col = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let term_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let text = ctx.route.with(|r| {
                    r.hof
                        .as_ref()
                        .and_then(|h| h.terminus_string(term_idx, col))
                        .unwrap_or("")
                        .to_string()
                });
                st.strings.push(text);
                true
            }

            // `termIdx (M.V.GetTerminusCode)` -> code
            "getterminuscode" => {
                let term_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let code = ctx
                    .route
                    .with(|r| r.hof.as_ref()?.terminus(term_idx).map(|t| t.code));
                st.numbers.push(code.map_or(-1.0, |c| c as f32));
                true
            }

            // 3. Route Callbacks (.hof)
            // `routeCode (M.V.GetRouteIndex)` -> routeIdx (or -1)
            "getrouteindex" => {
                let route_code = st.numbers.pop().unwrap_or(0.0).round() as i32;
                let idx = ctx.route.with(|r| {
                    r.hof
                        .as_ref()?
                        .find_route_by_code(route_code)
                        .map(|(i, _)| i)
                });
                st.numbers.push(idx.map_or(-1.0, |i| i as f32));
                true
            }

            // `routeIdx stopNo (M.V.GetRouteBusstopIdent)` -> ident string
            "getroutebusstopident" => {
                let stop_no = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let route_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let ident = ctx.route.with(|r| {
                    r.hof
                        .as_ref()
                        .and_then(|h| h.route(route_idx))
                        .and_then(|rt| rt.busstops.get(stop_no))
                        .cloned()
                        .unwrap_or_default()
                });
                st.strings.push(ident);
                true
            }

            // `routeIdx (M.V.GetRouteTerminusIndex)` -> terminus row index of the route
            "getrouteterminusindex" => {
                let route_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let term_idx = ctx.route.with(|r| {
                    let h = r.hof.as_ref()?;
                    let route = h.route(route_idx)?;
                    h.find_terminus_by_code(route.terminus_code).map(|(i, _)| i)
                });
                st.numbers.push(term_idx.map_or(-1.0, |i| i as f32));
                true
            }

            // `routeIdx (M.V.GetBusstopCount)` -> stop count of route
            "getbusstopcount" => {
                let route_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let count = ctx.route.with(|r| {
                    r.hof
                        .as_ref()
                        .and_then(|h| h.route(route_idx))
                        .map_or(0, |rt| rt.busstops.len())
                });
                st.numbers.push(count as f32);
                true
            }

            // 4. Bus Stop Callbacks (.hof)
            // `ident:str (M.V.GetBusstopIndex)` -> busstopIdx (or -1)
            "getbusstopindex" => {
                let ident = st.strings.pop().unwrap_or_default();
                let idx = ctx.route.with(|r| {
                    r.hof
                        .as_ref()?
                        .find_busstop_by_ident(&ident)
                        .map(|(i, _)| i)
                });
                st.numbers.push(idx.map_or(-1.0, |i| i as f32));
                true
            }

            // `busstopIdx col (M.V.GetBusstopString)` -> string
            "getbusstopstring" => {
                let col = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let stop_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let text = ctx.route.with(|r| {
                    r.hof
                        .as_ref()
                        .and_then(|h| h.busstop_string(stop_idx, col))
                        .unwrap_or("")
                        .to_string()
                });
                st.strings.push(text);
                true
            }

            // 5. Depot Global Strings
            // `slot (M.V.GetDepotStringGlobal)` -> string
            "getdepotstringglobal" => {
                let slot = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                let text = ctx.route.with(|r| {
                    r.hof
                        .as_ref()
                        .and_then(|h| h.global_string(slot))
                        .unwrap_or("")
                        .to_string()
                });
                st.strings.push(text);
                true
            }

            // 6. Timetable Callbacks: the active timetable is the selected route; free driving
            // (nothing selected) answers with neutral values, never garbage.
            // `(M.V.GetTTLineString)` -> string
            "getttlinestring" => {
                st.strings.push(ctx.route.with(|r| r.tt_line_text()));
                true
            }

            // `(M.V.GetTTTerminusIndex)` -> terminus index
            "getttterminusindex" => {
                let term_idx = ctx.route.with(|r| r.tt_terminus_index());
                st.numbers.push(term_idx.map_or(-1.0, |i| i as f32));
                true
            }

            // `(M.V.GetTTBusstopCount)` -> count of stops on active schedule
            "getttbusstopcount" => {
                st.numbers
                    .push(ctx.route.with(|r| r.tt_stop_count()) as f32);
                true
            }

            // `(M.V.GetTTBusstopIndex)` -> current stop index (0-based)
            "getttbusstopindex" => {
                st.numbers.push(ctx.route.with(|r| r.stop_index) as f32);
                true
            }

            // `stopNo (M.V.GetTTBusstopName)` -> stop name/ident string
            "getttbusstopname" => {
                let stop_no = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                st.strings.push(ctx.route.with(|r| r.tt_stop_name(stop_no)));
                true
            }

            // `(M.V.GetTTDelay)` -> delay in seconds
            "getttdelay" => {
                st.numbers.push(ctx.route.with(|r| r.delay_s));
                true
            }

            // `stopNo (M.V.GetTTBusstopDep)` -> departure time in seconds of day
            "getttbusstopdep" => {
                let stop_no = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                st.numbers
                    .push(ctx.route.with(|r| r.tt_departure_s(stop_no)));
                true
            }

            // `stopNo (M.V.GetTTBusstopArr)` -> arrival time in seconds of day
            "getttbusstoparr" => {
                let stop_no = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
                st.numbers.push(ctx.route.with(|r| r.tt_arrival_s(stop_no)));
                true
            }

            // Fonts & Metrics
            // `fontName (M.V.GetFontIndex)` -> font handle/index
            "getfontindex" => {
                let name = st.strings.pop().unwrap_or_default();
                let name_trim = name.trim();
                let fonts = ctx.fonts.read().unwrap();
                let mut found = None;
                for (i, f) in fonts.iter().enumerate() {
                    if f.name.eq_ignore_ascii_case(name_trim) {
                        found = Some(i as f32);
                        break;
                    }
                }
                let idx = found
                    .or_else(|| {
                        if name_trim.eq_ignore_ascii_case("default") && !fonts.is_empty() {
                            Some(0.0)
                        } else if !fonts.is_empty() {
                            fonts
                                .iter()
                                .position(|f| {
                                    f.name
                                        .to_ascii_lowercase()
                                        .starts_with(&name_trim.to_ascii_lowercase())
                                })
                                .map(|i| i as f32)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0.0);
                st.numbers.push(idx);
                true
            }

            // `font text (M.V.TextLength)` -> text width in pixels
            "textlength" => {
                let font_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
                let t = st.strings.pop().unwrap_or_default();
                let fonts = ctx.fonts.read().unwrap();
                let width = if font_idx >= 0 && (font_idx as usize) < fonts.len() {
                    let f = &fonts[font_idx as usize];
                    crate::text_texture::text_width(f, &t) as f32
                } else {
                    (t.chars().count() * 8) as f32
                };
                st.numbers.push(width);
                true
            }

            // Script Textures (ST*)
            // `tex (M.V.STNewTex)` -> void
            "stnewtex" => {
                let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
                if tex_idx >= 0 {
                    let mut texs = ctx.script_textures.write().unwrap();
                    if let Some(tex) = texs.get_mut(tex_idx as usize) {
                        tex.new_tex();
                    }
                }
                true
            }

            // `tex (M.V.STLock)` -> void
            "stlock" => {
                let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
                if tex_idx >= 0 {
                    let mut texs = ctx.script_textures.write().unwrap();
                    if let Some(tex) = texs.get_mut(tex_idx as usize) {
                        tex.lock();
                    }
                }
                true
            }

            // `tex (M.V.STUnlock)` -> void
            "stunlock" => {
                let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
                if tex_idx >= 0 {
                    let mut texs = ctx.script_textures.write().unwrap();
                    if let Some(tex) = texs.get_mut(tex_idx as usize) {
                        tex.unlock();
                    }
                }
                true
            }

            // `tex A R G B (M.V.STSetColor)` -> void
            "stsetcolor" => {
                let b = st.numbers.pop().unwrap_or(0.0) as u8;
                let g = st.numbers.pop().unwrap_or(0.0) as u8;
                let r = st.numbers.pop().unwrap_or(0.0) as u8;
                let a = st.numbers.pop().unwrap_or(255.0) as u8;
                let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
                if tex_idx >= 0 {
                    let mut texs = ctx.script_textures.write().unwrap();
                    if let Some(tex) = texs.get_mut(tex_idx as usize) {
                        tex.set_color(a, r, g, b);
                    }
                }
                true
            }

            // `tex x1 y1 x2 y2 (M.V.STDrawRect)` -> void
            "stdrawrect" => {
                let y2 = st.numbers.pop().unwrap_or(0.0) as i32;
                let x2 = st.numbers.pop().unwrap_or(0.0) as i32;
                let y1 = st.numbers.pop().unwrap_or(0.0) as i32;
                let x1 = st.numbers.pop().unwrap_or(0.0) as i32;
                let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
                if tex_idx >= 0 {
                    let mut texs = ctx.script_textures.write().unwrap();
                    if let Some(tex) = texs.get_mut(tex_idx as usize) {
                        tex.draw_rect(x1, y1, x2, y2);
                    }
                }
                true
            }

            // `tex x y (M.V.STDrawPixel)` -> void
            "stdrawpixel" => {
                let y = st.numbers.pop().unwrap_or(0.0) as i32;
                let x = st.numbers.pop().unwrap_or(0.0) as i32;
                let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
                if tex_idx >= 0 {
                    let mut texs = ctx.script_textures.write().unwrap();
                    if let Some(tex) = texs.get_mut(tex_idx as usize) {
                        tex.draw_pixel(x, y);
                    }
                }
                true
            }

            // `tex x y (M.V.STReadPixel)` -> void
            "streadpixel" => {
                let y = st.numbers.pop().unwrap_or(0.0) as i32;
                let x = st.numbers.pop().unwrap_or(0.0) as i32;
                let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
                if tex_idx >= 0 {
                    let mut texs = ctx.script_textures.write().unwrap();
                    if let Some(tex) = texs.get_mut(tex_idx as usize) {
                        tex.read_pixel(x, y);
                    }
                }
                true
            }

            // `tex (M.V.STGetR)` -> R
            "stgetr" => {
                let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
                let val = if tex_idx >= 0 {
                    let texs = ctx.script_textures.read().unwrap();
                    texs.get(tex_idx as usize).map(|t| t.get_r()).unwrap_or(0.0)
                } else {
                    0.0
                };
                st.numbers.push(val);
                true
            }

            // `tex (M.V.STGetG)` -> G
            "stgetg" => {
                let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
                let val = if tex_idx >= 0 {
                    let texs = ctx.script_textures.read().unwrap();
                    texs.get(tex_idx as usize).map(|t| t.get_g()).unwrap_or(0.0)
                } else {
                    0.0
                };
                st.numbers.push(val);
                true
            }

            // `tex (M.V.STGetB)` -> B
            "stgetb" => {
                let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
                let val = if tex_idx >= 0 {
                    let texs = ctx.script_textures.read().unwrap();
                    texs.get(tex_idx as usize).map(|t| t.get_b()).unwrap_or(0.0)
                } else {
                    0.0
                };
                st.numbers.push(val);
                true
            }

            // `tex (M.V.STGetA)` -> A
            "stgeta" => {
                let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
                let val = if tex_idx >= 0 {
                    let texs = ctx.script_textures.read().unwrap();
                    texs.get(tex_idx as usize).map(|t| t.get_a()).unwrap_or(0.0)
                } else {
                    0.0
                };
                st.numbers.push(val);
                true
            }

            // `tex (M.V.STFilter)` -> void
            "stfilter" => {
                let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
                if tex_idx >= 0 {
                    let mut texs = ctx.script_textures.write().unwrap();
                    if let Some(tex) = texs.get_mut(tex_idx as usize) {
                        tex.filter();
                    }
                }
                true
            }

            // `file tex (M.V.STLoadTex)` -> msg
            "stloadtex" => {
                let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
                let file = st.strings.pop().unwrap_or_default();
                if tex_idx >= 0 && !file.is_empty() {
                    let mut path = std::path::PathBuf::from(&file);
                    if path.is_relative() {
                        if let Some(base) = &ctx.texture_base_dir {
                            path = base.join(&path);
                        }
                    }
                    let mut texs = ctx.script_textures.write().unwrap();
                    if let Some(tex) = texs.get_mut(tex_idx as usize) {
                        let _ = tex.load_tex(&path);
                    }
                }
                st.strings.push(String::new());
                true
            }

            // `tex x y font a b (text) (M.V.STTextOut)` -> void
            "sttextout" => {
                let text = st.strings.pop().unwrap_or_default();
                let spacing = st.numbers.pop().unwrap_or(0.0) as i32;
                let alignment = st.numbers.pop().unwrap_or(0.0) as u32;
                let font_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
                let y = st.numbers.pop().unwrap_or(0.0) as i32;
                let x = st.numbers.pop().unwrap_or(0.0) as i32;
                let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;

                if tex_idx >= 0 && font_idx >= 0 {
                    let fonts = ctx.fonts.read().unwrap();
                    if let Some(font) = fonts.get(font_idx as usize) {
                        let mut texs = ctx.script_textures.write().unwrap();
                        if let Some(tex) = texs.get_mut(tex_idx as usize) {
                            tex.text_out(x, y, font, &text, alignment, spacing);
                        }
                    }
                }
                true
            }

            // `tex (M.V.STCopyColor)` -> void
            "stcopycolor" => {
                let _tex = st.numbers.pop();
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
    use opts_core::formats::hof::HofFile;

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
        let (_, route) = hof.find_route_by_code(3301).expect("find route 3301");
        assert_eq!(route.terminus_code, 11);
        assert!(!route.busstops.is_empty());

        let route = crate::route::RouteHandle::new(Some(hof.clone()));
        route.select(3301).expect("select 3301");
        let ctx = VehicleHostContext {
            seed: 1234,
            route,
            ..Default::default()
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
        let route = crate::route::RouteHandle::new(Some(hof));
        route.select(3301).expect("select 3301");
        route.set_stop(1);
        route.set_delay(45.0);
        let ctx = VehicleHostContext {
            seed: 42,
            route,
            ..Default::default()
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
    #[test]
    fn host_callbacks_script_textures_lifecycle_and_drawing() {
        let canvas = CpuScriptTexture::new(64, 32);
        let textures = Arc::new(RwLock::new(vec![canvas]));
        let ctx = VehicleHostContext {
            script_textures: textures.clone(),
            ..Default::default()
        };
        let mut h = vehicle_callbacks_with_context(ctx);
        let mut st = Stacks::default();

        // 1. Lock
        st.numbers.push(0.0);
        assert!(h("stlock", &mut st));
        assert!(textures.read().unwrap()[0].is_locked());

        // 2. Set color: A=255, R=255, G=128, B=64
        st.numbers.push(0.0); // tex
        st.numbers.push(255.0); // A
        st.numbers.push(255.0); // R
        st.numbers.push(128.0); // G
        st.numbers.push(64.0); // B
        assert!(h("stsetcolor", &mut st));

        // 3. Draw rect
        st.numbers.push(0.0); // tex
        st.numbers.push(2.0); // x1
        st.numbers.push(2.0); // y1
        st.numbers.push(10.0); // x2
        st.numbers.push(10.0); // y2
        assert!(h("stdrawrect", &mut st));

        // 4. Read pixel
        st.numbers.push(0.0); // tex
        st.numbers.push(5.0); // x
        st.numbers.push(5.0); // y
        assert!(h("streadpixel", &mut st));

        // 5. Get R, G, B, A
        st.numbers.push(0.0);
        assert!(h("stgetr", &mut st));
        assert_eq!(st.numbers.pop(), Some(255.0));

        st.numbers.push(0.0);
        assert!(h("stgetg", &mut st));
        assert_eq!(st.numbers.pop(), Some(128.0));

        st.numbers.push(0.0);
        assert!(h("stgetb", &mut st));
        assert_eq!(st.numbers.pop(), Some(64.0));

        st.numbers.push(0.0);
        assert!(h("stgeta", &mut st));
        assert_eq!(st.numbers.pop(), Some(255.0));

        // 6. Unlock
        st.numbers.push(0.0);
        assert!(h("stunlock", &mut st));
        assert!(!textures.read().unwrap()[0].is_locked());
        assert!(textures.read().unwrap()[0].is_dirty());
    }
}
