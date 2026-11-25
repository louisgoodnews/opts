//! Component swaps (ISS-052, M-0169): a vehicle's display or on-board computer is changed as one
//! named part. A part is a vendor group of the setvar catalogue, and its module is the value of
//! the one Choice option of that group. The swap then goes through the same validation, apply and
//! bus-configuration path as an equipment option: all or none, kept on the session record or the
//! wizard's choice, and stored with a bus configuration.
//!
//! The scripts and triggers that run after a swap are the vehicle's own: they read the option.
//! The ticket printer is not a part here; it waits for M-0156 (the ticket model).

use opts_bevy::control::state::Outcome;
use opts_cli::protocol::RpcError;
use opts_core::setvars::{SetvarCatalogue, SetvarEntry, SetvarKind};
use serde_json::{json, Map, Value};

use super::describe::value_of;
use super::set::set;
use super::source::Source;

/// The vendor group of a vehicle's display options.
// TODO(owner): the group name is assumed, not read from the vehicle files; confirm it.
const DISPLAY_GROUP: Option<&str> = Some("display");

/// The vendor group of a vehicle's on-board computer options.
// TODO(owner): the group name is not known; until it is set, this swap is refused.
const ON_BOARD_COMPUTER_GROUP: Option<&str> = None;

/// A part of the vehicle that can be swapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Part {
    Display,
    OnBoardComputer,
}

impl Part {
    /// The part a client names (`display`, `on_board_computer`).
    fn parse(name: &str) -> Result<Self, RpcError> {
        match name.trim() {
            "display" => Ok(Self::Display),
            "on_board_computer" => Ok(Self::OnBoardComputer),
            "ticket_printer" => Err(RpcError::invalid_params(
                "the ticket printer swap waits for M-0156 (the ticket model); not available",
            )),
            other => Err(RpcError::invalid_params(format!(
                "`{other}` is not a swappable part: use display or on_board_computer"
            ))),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Display => "display",
            Self::OnBoardComputer => "on_board_computer",
        }
    }

    /// The vendor group of the part's options; `None` while the group is not known.
    fn group(self) -> Option<&'static str> {
        match self {
            Self::Display => DISPLAY_GROUP,
            Self::OnBoardComputer => ON_BOARD_COMPUTER_GROUP,
        }
    }
}

/// The one Choice option of the part's group, the module selector. `None` when the vehicle has
/// none or more than one: the vehicle does not support the swap.
fn module_option(catalogue: &SetvarCatalogue, part: Part) -> Option<&SetvarEntry> {
    let group = part.group()?;
    let mut found = catalogue
        .entries
        .iter()
        .filter(|e| e.kind == SetvarKind::Choice && e.group.as_deref() == Some(group));
    let first = found.next()?;
    found.next().is_none().then_some(first)
}

/// `vehicle.setvars` with `swap: {part, module}`: swaps one part of the vehicle to `module`.
pub(super) fn request(
    world: &mut World,
    src: &Source,
    catalogue: &SetvarCatalogue,
    swap: &Value,
) -> Outcome {
    let part = swap.get("part").and_then(Value::as_str);
    let module = swap
        .get("module")
        .and_then(Value::as_i64)
        .and_then(|m| i32::try_from(m).ok());
    let (Some(part), Some(module)) = (part, module) else {
        return Outcome::err(RpcError::invalid_params(
            "parameter `swap` needs `part` (text) and `module` (a whole number)",
        ));
    };
    match Part::parse(part) {
        Ok(part) => swap_part(world, src, catalogue, part, module),
        Err(e) => Outcome::err(e),
    }
}

/// Sets the part's module option through the setvar path and logs the swap.
fn swap_part(
    world: &mut World,
    src: &Source,
    catalogue: &SetvarCatalogue,
    part: Part,
    module: i32,
) -> Outcome {
    let Some(entry) = module_option(catalogue, part) else {
        tracing::warn!(
            vehicle = %src.vehicle,
            part = part.name(),
            "component swap not supported: the vehicle has no single module option for the part"
        );
        return Outcome::err(RpcError::invalid_params(format!(
            "this vehicle does not support the {} swap",
            part.name()
        )));
    };
    let old = value_of(src, entry).round() as i32;
    let option = entry.name.clone();
    let values = Map::from_iter([(option.clone(), json!(module))]);
    let out = set(world, src, catalogue, &values, false);
    match &out {
        Outcome::Done(Ok(_)) => tracing::debug!(
            vehicle = %src.vehicle,
            part = part.name(),
            option = %option,
            old,
            new = module,
            "component swapped"
        ),
        _ => tracing::warn!(
            vehicle = %src.vehicle,
            part = part.name(),
            option = %option,
            new = module,
            "component swap refused by the vehicle's options"
        ),
    }
    out
}
