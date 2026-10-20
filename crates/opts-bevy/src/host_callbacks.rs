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
    /// The map's ticket system (`GetTicketName`, `GetTicketValue`); empty: the stub answers.
    pub tickets: Arc<opts_sim::passengers::TicketBook>,
    /// The values of the coins of the map's currency (`GiveChangeCoin` names one by index).
    pub coins: Arc<Vec<f32>>,
    /// The coins the scripts gave as change, by value, since the last read.
    pub change_given: Arc<RwLock<Vec<f32>>>,
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
                let idx = st.numbers.pop().unwrap_or(0.0).round() as i64;
                let name = usize::try_from(idx)
                    .ok()
                    .and_then(|i| ctx.tickets.tickets.get(i))
                    .map_or_else(
                        || "Normalfahrschein".to_string(),
                        |t| t.display.replace('@', " "),
                    );
                st.strings.push(name);
                true
            }

            // `ticketIdx (M.V.GetTicketValue)` -> price
            "getticketvalue" => {
                let idx = st.numbers.pop().unwrap_or(0.0).round() as i64;
                let price = usize::try_from(idx)
                    .ok()
                    .and_then(|i| ctx.tickets.price(i))
                    .unwrap_or(2.50);
                st.numbers.push(price);
                true
            }

            // `coinIdx (M.V.GiveChangeCoin)` -> void
            "givechangecoin" => {
                let idx = st.numbers.pop().unwrap_or(0.0).round() as i64;
                let value = usize::try_from(idx)
                    .ok()
                    .and_then(|i| ctx.coins.get(i))
                    .copied();
                if let (Some(v), Ok(mut log)) = (value, ctx.change_given.write()) {
                    if log.len() < 64 {
                        log.push(v);
                    }
                }
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
mod tests;
