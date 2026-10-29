//! The callbacks OMSI offers its vehicle scripts (`(M.V.name)`).
//!
//! Scripts call the host for data the scripts cannot know: random numbers fixed per vehicle,
//! operational depot data (`.hof`), timetable state, tickets, passengers, and ground elevation.
//! Specification: `docs/src/formats/text/vehicles/callbacks.md` (M-0093).

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use opts_core::formats::oft::OftFont;
pub(crate) use opts_core::formats::script_texture::{CpuScriptTexture, ScriptTextureCanvas};
use opts_sim::script::{ExternalHandler, Stacks};

mod route_callbacks;
mod script_texture;
mod services;

use route_callbacks::{handle_route_callback, handle_timetable_callback};
use script_texture::handle_script_texture_callback;
use services::handle_service_callback;

/// How many people stand on each place and walk on each path link of a vehicle (what the scripts'
/// `GetHumanCountOnSeat` and `GetHumanCountOnPathLink` ask), kept by the passenger system.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PassengerCounts {
    /// People on place `i` (the n-th `[passpos]` of the cabin).
    pub on_place: Vec<u32>,
    /// People walking along link `i` of the paths file.
    pub on_link: Vec<u32>,
}

/// The counts shared between the passenger system and the vehicle's callbacks.
pub type SharedPassengerCounts = Arc<RwLock<PassengerCounts>>;

/// The shared counts on the vehicle's root (the passenger system writes them).
#[derive(bevy::prelude::Component, Debug, Clone, Default)]
pub struct PassengerCountsHandle(pub SharedPassengerCounts);

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
    /// The bitmaps of those fonts, by the same index (`STTextOut` draws their glyphs).
    pub font_bitmaps: crate::script_text::FontBitmapSource,
    /// Base directory for resolving relative texture paths in `STLoadTex`.
    pub texture_base_dir: Option<PathBuf>,
    /// The map's ticket system (`GetTicketName`, `GetTicketValue`); empty: the stub answers.
    pub tickets: Arc<opts_sim::passengers::TicketBook>,
    /// The values of the coins of the map's currency (`GiveChangeCoin` names one by index).
    pub coins: Arc<Vec<f32>>,
    /// People per place and link (`GetHumanCountOn*`).
    pub passengers: SharedPassengerCounts,
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
        if let Some(handled) = handle_script_texture_callback(&ctx, name, st) {
            return handled;
        }
        if let Some(handled) = handle_route_callback(&ctx, name, st) {
            return handled;
        }
        if let Some(handled) = handle_timetable_callback(&ctx, name, st) {
            return handled;
        }
        if let Some(handled) = handle_service_callback(&ctx, name, st) {
            return handled;
        }

        if name == "nrspecrandom" {
            let n = st.numbers.pop().unwrap_or(0.0);
            let bits = mix(ctx.seed ^ mix(u64::from(n.round() as i64 as u64 as u32)));
            st.numbers.push((bits >> 40) as f32 / 16_777_216.0);
            true
        } else {
            false
        }
    })
}

#[cfg(test)]
mod tests;
