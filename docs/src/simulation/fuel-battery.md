# Fuel and Battery

| Metadata | Value |
| :--- | :--- |
| **Status** | 🚧 In progress (2026-11-16): the model, the setting, the save section, the control command and the HUD readout (on-screen check open) are in; the bar gauge, the VM variable, a key or menu refuel action and the audit of the stock scripts are open |
| **Code** | `opts_sim::vehicle::fuel` (`FuelState`, `FuelMode`, the step and the torque cut in `fuel/sim.rs`), `opts-game` `vehicle_fuel_settings.rs`, `control_fuel.rs`, `save/sections/fuel.rs` |
| **Milestone** | [M-0123](../roadmap/phase-12/M-0123.md) |
| **Settings** | `vehicle.fuel_mode` (see the [settings reference](../reference/settings.md)) |

## 1. The model

Each session vehicle may carry a `FuelState`: a store of `capacity` units with `content` left, a `kind` (diesel or electric) and the player's `mode`. It lives on the vehicle's simulation (`VehicleSimulation::fuel`, `None` for a vehicle without the model; saves from before it load with `None`).

- **Consumption.** The store drains by `rate_per_s(throttle)`: the idle rate at throttle 0 and the full-throttle rate at throttle 1, linear in between. Every step of the drive takes the throttle pedal (not `engine_n` or the torque; the design's "throttle times engine speed" is kept for later).
- **Modes.** `Off` never changes the level. `Realistic` uses the rates once. `Accelerated` multiplies them by 10.
- **Torque cut.** At empty a diesel keeps no drive torque (`torque_factor` 0): the step multiplies the tractive force and the script's wheel torque by it, so the vehicle coasts. An empty battery keeps 20 % (limp mode). `Off` always keeps 100 %.
- **Low level.** `is_low` is true at 10 % of the capacity or less.
- **Refuel.** `VehicleSimulation::refuel` fills the store. A level can be set (`set_level`, 0 to 1).

### Values are OPTS assumptions

The OMSI files of the corpus do not state these numbers in a form the model reads, so they are model assumptions, not OMSI values:

| Constant | Value | Meaning |
| :--- | :--- | :--- |
| `DIESEL_IDLE_L_PER_H` | 3 | diesel burn at idle (litres per hour) |
| `DIESEL_FULL_L_PER_H` | 60 | diesel burn at full throttle |
| `ELECTRIC_IDLE_UNITS_PER_H` | 2 | electric draw at idle (capacity units per hour) |
| `ELECTRIC_FULL_UNITS_PER_H` | 120 | electric draw at full throttle |
| `DEFAULT_CAPACITY` | 250 | tank (litres) or store used when the vehicle data names none |
| `ACCELERATED_FACTOR` | 10 | multiplier of *Accelerated* |
| `LOW_LEVEL` | 0.10 | low-level warning threshold |
| `EMPTY_ELECTRIC_TORQUE_FRACTION` | 0.2 | torque an empty battery keeps |

The milestone's ❓ items stay open: the unit of the scripts' `engine_tank_content` and the tank constant, the variable that carries an electric vehicle's state of charge (none is in the predefined-variable list), and whether `veh_tank` is raised by the vehicle or expected from the host. Until they are settled, the host model is the authority for the torque cut and the scripts keep their own `engine_tank_content`.

## 2. Setting, start and the player's vehicle

`vehicle.fuel_mode` (*Fuel and battery*: `off`, `realistic`, `accelerated`, default `realistic`, a proposal for the project lead to confirm). Every frame `sync_fuel_mode` gives each session vehicle a model when it has none, at the start share of the *Fuel* setting (`vehicle.fuel`, default 0.6), and sets the mode. The model is diesel with `DEFAULT_CAPACITY`; electric vehicles are not told apart yet.

## 3. Control protocol

`vehicle.fuel` (debug) answers the player's vehicle's model: `{model, kind, capacity, content, level, mode, low, torque_factor}`, or `{model: false}`. With `level` (0 to 1) it sets the charge, with `refuel: true` it fills the store first. A level outside 0 to 1 is `-32602`; `1001` without a player vehicle.

## 4. Save

The `fuel` section (version 1, `Vehicles` stage, after the vehicles exist) holds the model of each live session vehicle. Restore keeps the saved content, capacity and kind, and the mode of the live setting. A file without the section never reaches the apply: its vehicles keep the fresh models at the start share instead of a full tank, which the milestone asked for and is not met yet.

## 5. Not yet done

- **HUD readout, on-screen check open.** Built (`hud/fuel_hud.rs`, `update/fuel.rs`): the widget `fuel` shows the level in percent and `fuel-low` shows `LOW` or `OK` (a word, not only a colour) at or below 10 %, in the bottom-left instrument row of the bus, articulated bus and trolleybus profiles. Both are hidden when the setting is *Off* (no readout). A bar gauge is not drawn; a person still has to look at it on screen.
- **VM variable.** `tank_percent` has no consumer. The stock scripts write it themselves from `engine_tank_content` (`engine_GN.osc:275`), so a host write would fight the script; the authority per vehicle family needs the audit first.
- **Refuel action** on a key or in a menu when stopped. Only the control command exists.
- **Stations.** Refuelling at `[petrolstation]` objects is a stretch decision for a follow-up milestone.
- **Audit.** The table of which vehicles use `engine_tank_content`, `tank_percent`, `veh_tank` and which electric vehicles expose a charge.
