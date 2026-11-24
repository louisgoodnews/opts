//! Keywords beyond mass and axles that shape how a vehicle drives or is driven by AI:
//! the [`VehicleExtras`] block and its [`ControlCable`] entries, and how each keyword is applied.

use super::blocks;
use crate::formats::text::{parse_float, OmsiBlock};

/// One `[control_cable_front]` / `[control_cable_back]` entry: a pair of script variables that
/// travel between coupled units (the tram's battery switch, indicators, doors, traction demand).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlCable {
    /// The cable sits at the unit's front coupling (else the rear one).
    pub front: bool,
    /// Plug kind: `C` (central, links to the other unit's `C` of the same number), `L` or `R`.
    pub kind: char,
    /// Number of the cable within its kind.
    pub index: u32,
    /// The variable that receives the value the cable carries (the scripts read it).
    pub receive: String,
    /// The variable the unit's scripts write to put a value on the cable.
    pub send: String,
    /// The link works only while this variable of the unit is above 0.5.
    pub coupling: String,
}

/// Keywords that shape how a vehicle drives or is driven by AI, beyond mass and axles.
///
/// Counts are files of the installation that declare the keyword (2,752 `.bus`/`.ovh` files).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VehicleExtras {
    /// `[ai_veh_type]`: `0` passenger car, `1` cab, `2` bus, `3` truck (the legend in the vehicle
    /// files themselves; 80 files).
    pub ai_veh_type: Option<u32>,
    /// `[ai_brakeperformance]`: the numbers of the block in file order, the first being the
    /// brake multiplier (141 files; the others are unexplained).
    pub ai_brake_performance: Vec<f32>,
    /// `[rowdy_factor]`: `(low, high)`, a per-vehicle random draw between the two at creation
    /// (539 files).
    pub rowdy_factor: Option<(f32, f32)>,
    /// `[ai_deltaheight]`: vertical offset when driven by AI, metres (1,485 files).
    pub ai_delta_height: Option<f32>,
    /// `[rollwiderstand]`: rolling resistance as a constant force, newtons (2,731 files).
    pub roll_resistance_n: Option<f32>,
    /// `[inv_min_turnradius]`: reciprocal of the smallest turning radius, 1/m (2,479 files).
    pub inv_min_turn_radius: Option<f32>,
    /// `[schwerpunkt]`: height of the centre of gravity, metres (1,908 files).
    pub centre_of_gravity_height: Option<f32>,
    /// `[rot_pnt_long]`: longitudinal coordinate of the rotation point, metres (2,731 files).
    pub rotation_point_long: Option<f32>,
    /// Moments of inertia about x (lateral), y (longitudinal) and z (vertical), t·m²: from
    /// `[momentofintertia]`, else from lines 2 to 4 of `[mass]` when those are not all zero.
    pub inertia_t_m2: Option<[f32; 3]>,
    /// `[control_cable_front]` and `[control_cable_back]` entries in file order.
    pub control_cables: Vec<ControlCable>,
}

impl VehicleExtras {
    pub(super) fn apply_block(&mut self, block: &OmsiBlock) -> bool {
        match block.keyword.as_str() {
            "momentofintertia" => {
                let v: Vec<f32> = block
                    .parameters
                    .iter()
                    .take(3)
                    .map_while(|p| parse_float(p).ok())
                    .collect();
                if let [x, y, z] = v[..] {
                    self.inertia_t_m2 = Some([x, y, z]);
                }
            }
            "ai_veh_type" => {
                self.ai_veh_type = block
                    .parameters
                    .first()
                    .and_then(|p| p.trim().parse::<u32>().ok());
            }
            "ai_brakeperformance" => {
                self.ai_brake_performance = block
                    .parameters
                    .iter()
                    .map_while(|p| parse_float(p).ok())
                    .collect();
            }
            "rowdy_factor" => {
                let v: Vec<f32> = block
                    .parameters
                    .iter()
                    .map_while(|p| parse_float(p).ok())
                    .take(2)
                    .collect();
                if let [lo, hi] = v[..] {
                    self.rowdy_factor = Some((lo.min(hi), lo.max(hi)));
                }
            }
            "ai_deltaheight" => self.ai_delta_height = blocks::first_float(&block.parameters),
            "rollwiderstand" => self.roll_resistance_n = blocks::first_float(&block.parameters),
            "inv_min_turnradius" => {
                self.inv_min_turn_radius = blocks::first_float(&block.parameters);
            }
            "schwerpunkt" => {
                self.centre_of_gravity_height = blocks::first_float(&block.parameters);
            }
            tag @ ("control_cable_front" | "control_cable_back") => {
                if let [kind, index, receive, send, coupling, ..] = &block.parameters[..] {
                    if let (Some(kind), Ok(index)) =
                        (kind.trim().chars().next(), index.trim().parse::<u32>())
                    {
                        self.control_cables.push(ControlCable {
                            front: tag == "control_cable_front",
                            kind: kind.to_ascii_uppercase(),
                            index,
                            receive: receive.trim().to_owned(),
                            send: send.trim().to_owned(),
                            coupling: coupling.trim().to_owned(),
                        });
                    }
                }
            }
            "rot_pnt_long" => self.rotation_point_long = blocks::first_float(&block.parameters),
            _ => return false,
        }
        true
    }
}
