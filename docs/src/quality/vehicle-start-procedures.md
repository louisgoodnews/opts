# Vehicle Start Procedures (M-0141)

*Facts read from the vehicles' own scripts (OSC read as prose, no text copied) and confirmed in the running game through `vehicle.click`, `vehicle.hotspots` and `vehicle.script_vars`. Each procedure lists what the player does, which variables change and what is still unverified ❓. A vehicle is added when its procedure has been run.*

## The set-up degree (settings, category *Vehicle*)

`vehicle.setup` (0 to 1, default 0.75), `vehicle.fuel` (share of the tank, default 0.6) and `vehicle.start_lights` (off, parking, dipped) say how a vehicle begins a drive. The host brings the vehicle there by pressing the keys of its procedure (the *recipe* of its family, `crates/opts-bevy/src/vehicle_setup.rs`); it writes nothing into the scripts' variables except the tank content, once.

| Degree | State |
| ---: | :--- |
| below 0.25 | cold: nothing started, parking brake set |
| from 0.25 | electrics on |
| from 0.5 | ignition in the drive position |
| from 0.75 | engine running (parking brake still set) |
| 1 | ready to drive: parking brake released, the gear selector follows the driver |

Recipes (recognised by a variable the vehicle's scripts own): *stepped ignition key* (Citybus 530: battery key three times, the last one held), *inserted ignition key* (BRT C2: battery key three times, then the start key held), *battery switch and start key* (MAN NL, SD200, NG272 and every vehicle not recognised). The fuel is set as a share of `engine_tank_capacity` (250 l when the vehicle's constants name none). Changes apply to the next vehicle; raising the degree also continues a vehicle that has not got that far. Verified live on the Citybus 530 for 0, 0.25, 0.5 and 1 (cold; electrics, lights and fuel; key in drive position; running with the brake released); the other families in the rig only (`crates/opts-bevy/src/vehicle_setup/tests.rs`, `crates/opts-game/tests/vehicle_start.rs`). Rail vehicles have no recipe yet ❓.

## How controls are operated

| Control | In the game | By command |
| :--- | :--- | :--- |
| a key of `keyboard.cfg` | the bound key (rebindable, `controls.vehicle.*`) | `vehicle.key action=<name>` |
| a cockpit control (`[mouseevent] name`) | left click in the cockpit or passenger view: `name`, then `name_drag` on **every frame in which the mouse moves** while the button is held, `name_off` on release; `mouse_x`/`mouse_y` carry the movement of the frame in pixels | `vehicle.click name=… drag_x=… drag_y=… frames=… hold_ms=…`; the controls of a vehicle: `vehicle.hotspots` |

Since M-0141 the drag trigger fires on every moving frame (before, once): controls that accumulate the mouse movement in a position (the ignition key, rotary switches, sliders) now move with the mouse. Not every vehicle handles the same keys the same way: `engine_start` (`kw_m_enginestart`) starts the engine of the MAN family, steps the Citybus 530's ignition key *back* and starts the C2 from the drive position; the `battery` key (`cp_batterietrennschalter_toggle`) is the battery switch of the MAN family and the ignition key's next step on the Citybus and the C2. The sections below say which. The rig tests of these procedures are `crates/opts-game/tests/vehicle_start.rs` (they run the vehicles' own scripts headless; they skip without `OMSI_PATH`).

## Citybus 530 by Kajosoft (`01a_o530_e2_2.bus`, checked 2026-10-20 on Ahlheim 5)

The vehicle has 288 hot-spots. The engine start is the ignition key `cp_kluczyk_mov` (the position is `cp_kluczyk_rot`, 0 to 1; the script adds `mouse_y / -75` per drag call):

| Position (`cp_kluczyk_rot`) | `cp_kluczyk_rot_mode` | What the script does |
| :--- | ---: | :--- |
| below 0.25 | 0 | everything off (`elec_busbar_main_sw` 0) |
| 0.25 to 0.5 | 1 | electrics on (`elec_busbar_main_sw` 1, the busbar voltage `elec_busbar_main` rises to 0.9 and on) |
| 0.5 to 0.75 | 2 | drive position, injection stays |
| above 0.75 | 3 | starter: when the engine is off, `cp_taster_motorabstellung` is not pressed, the gear selector is in neutral (`antrieb_getr_gangwahl == 1`) and the busbar is above 0.3, `engine_starter` and `engine_injection_on` become 1 and the starter event is raised; releasing the key (`cp_kluczyk_mov_off`) springs it back to the drive position (0.625) and clears the starter |

**With the mouse in the game, the first attempt after loading did not start the engine** (checked live; headless, in the rig, the same single pull does start it, so the cause is a state the game's start-up leaves behind, not the script): `engine_starter_antirepeat` is 1 from `{init}` and only the key's release (`_off`) clears it. So the procedure is: drag the key to the drive position, release, then drag past 0.75 and hold:

1. `vehicle.click name=cp_kluczyk_mov drag_y=-50 frames=60 hold_ms=200` (key to the drive position; the electrics come on).
2. wait for the busbar (about 2 s), then `vehicle.click name=cp_kluczyk_mov drag_y=-20 frames=30 hold_ms=4000` (to the starter, held).

Result in the game: `engine_on` 1, `engine_n` about 598 rpm (idle) a second after the start; the engine's idle loop is heard (level 0.085 at 9.6 m on the Vehicles bus, one voice with a sink). 

**With the keyboard** (no mouse, checked live and in the rig): the `battery` key (E) steps the key: 1st press electrics on (key 0.375), 2nd drive position (0.625), 3rd (held) the starter (1.0) — the engine starts and the key springs back to the drive position on release. The `engine_start` key (M) steps back: from the drive position it stops the engine (key 0.375), a second press switches the electrics off.

❓ Open: the gear selector's states for driving (`antrieb_getr_gangwahl`), the retarder.

## MAN NL, SD200 and NG272 (`MAN_EN92_main.bus`, `MAN_SD77.bus`, `MAN_GN92_main.bus`, rig 2026-10-20)

All three share one procedure: the `battery` key switches the electrics on (`elec_busbar_main` 0.9), then the `engine_start` key (M) **held** starts the engine (starter while held, `engine_on` 1 within about 2 s, idle 490 to 560 rpm); the same key pressed on a running engine shuts it down when the busbar is live and the door air circuit `bremse_p_tank04` is above 400 000 Pa (the shut-down actuator needs air). Interlocks the script checks: gear selector in neutral (`antrieb_getr_gangwahl` 1), busbar above 0.3, no antirepeat set. In the game the vehicle is **already running at spawn**: the game's own start-up helper (`script_startup_system`, `crates/opts-bevy/src/vehicle_script.rs`) presses the battery and start keys 0.5 s after loading; the battery key pressed again then switches the electrics off while the engine keeps running ❓ (a diesel would normally stop only with the shut-down valve; the script owns that).

## BRT C2 (`C2_2T.bus`, rig 2026-10-20)

Stepped key like the Citybus, with one more step: the `battery` key inserts the key (1st press, nothing else happens), turns it to the electrics (2nd, busbar 1), to the drive position (3rd); the `engine_start` key held then starts the engine from the drive position. The ignition key hot-spot is `C2_CP_Zuendschluessel` (drag).

## Tram NF6D (`NF6D_B_User.bus`, the driver's car; map *Tram*, checked 2026-10-20)

There is **no start procedure**: the driver's car has no battery or engine trigger (`NF6D_A.bus` answers the battery key with `sla_elec_bat`, the cab car does not), its scripts own no `engine_on`, and the tram's traction is the game's own rail dynamics (parking brake, gear `drive`, throttle). The start-up helper therefore skips the engine steps; at degree 1 the parking brake is released and the tram drives (throttle 0.6: 7 km/h after 3 s, 18 km/h after 9 s, live).

What the scripts do answer (keys, checked live through `vehicle.key`): `interior_driver` (`lichtinnenfahrer_b_cp` 1), `hazard` (`slb_blinkerl_b`/`slb_blinkerr_b`), `wiper` (`wischer1_cp_anim`), `door_3` (`tuerfreigabe_b`, `tuervorwahl_links_b`: the left doors are released and preselected); `door_1`/`door_2` are handled but change nothing visible in the scripts' variables ❓ (the doors of the other side); `high_beam` and `indicator_left` are handled and change no variable I looked at ❓; the `lights` key is not handled (the tram has no `kw_scheinwerfer_toggle`). The hot-spots (56) include the traction/brake stages `kw_s_1` to `kw_s_5`, `kw_s_N`, `kw_s_R`, `Notbremse` and its reset, `TurboTram`, the IBIS keys and the cab windows. How the driver's stages (`kw_s_*`) relate to the game's throttle is open ❓ (M-0098: rail on its scripts' torque).

## Loads with problems (broken tiles)

The map *Tram* lists 20 tiles and 6 files are missing: the loading screen waits for *Continue*. For scripts and the command line: `loading.continue` presses it (and lists the problems), `scene.load_map {accept_problems: true}` starts the drive by itself for that load, `--accept-load-problems` does so for every load. A load that failed outright still stops. The problems stay in the report and are written to the log as a warning.

## Not yet looked at ❓

Other trams and trains (Stadtbahn TW6000, ULF, the `.zug` consists with several driven cars), buses of other makers.