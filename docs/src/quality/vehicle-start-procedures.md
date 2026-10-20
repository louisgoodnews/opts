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

## Not yet looked at ❓

Trams (Tram_NF6D, Stadtbahn TW6000): electric, no engine start; their procedure (master switch, direction lever, power) is for the next slice.

### Citybus 530: the other controls (checked 2026-10-20, engine running, through `vehicle.click` / `vehicle.key`)

| Control | Operation | Result in the script variables |
| :--- | :--- | :--- |
| Parking brake `parking_brake_mouse` | drag the lever: `bremse_feststell_sw` moves by `mouse_y / 200` per frame; above 0.6 the brake is set, below 0.4 released (a drag of -160 px total releases it) | `bremse_feststell` 1 → 0, the dashboard lamp `cockpit_light_feststellbremse` follows |
| Doors: front `bus_doorfront0` (key `door_1`: opens `door_0` and `door_1`), rear (keys `door_2`/`door_3`: `door_2` and `door_3`; `door_4` closes them) | the keys;  click; the door opens only when the door air circuit `bremse_p_tank04` is above 600 000 Pa, about 30 s after the engine started (the circuit starts at 390 000 Pa and fills behind the main circuits) and the busbar is live | `door_0` 0 → 1 |
| Driver's light `cp_fahrerlicht_toggle`, interior lights `cp_licht_unterdeck_toggle` / `cp_licht_untenrechts_toggle` | click | `lights_fahrerlicht`, `lights_beleuchtung_unterdeck` switch, battery current rises with the load |
| Hazard lights `blinker_warn_toggle`, indicators (`indicator_left`, `indicator_off`) | click / key | `lights_blinker_l/r` pulse with `lights_blinkgeber`, bulbs and `cockpit_light_blinker*` follow |
| High beam (key `high_beam`) | key | `lights_fern`, `cockpit_light_fern` 1 |
| Key `lights` (`kw_scheinwerfer_toggle`) | key | switches the light key between off and dipped beam: `lights_abbl` 1 with `lights_stand`, `lights_rueck` (tail lights), `lights_terminus` and the dashboard lamp; at night the tail lights and the destination sign glow (screenshot checked) |
| Wipers `cp_wiper_turnswitch` | drag the rotary: `mouse_y / -50` per frame, positions at 0.17 / 0.5 / 0.83 (interval, off, slow, fast) | drag -40 px: mode 3 (fast), `wiperrunning` 1, `wiperpos` moves |
| Heating `cp_heizregler_temp` | drag: `mouse_y / -150` per frame | `cockpit_heizregler_temp` 0.4 after -60 px |
| Horn `horn` | click | event only (the sound is M-0102's) |

A plain click without a drag on a rotary or slider does nothing visible (the value moves in the drag trigger): not an error of OPTS. A control that reads `elec_busbar_main` (stalk `cp_lus_ster`) does nothing while the electrics are off.

### The mouse path

Hot-spots are picked by a ray from the cursor through the camera onto the vehicle's own meshes (`MouseTrigger`, nearest first); in the cockpit view the camera looks at the street, the dashboard comes into view by looking down (right mouse button). The control under the cursor shows its trigger name in a label at the bottom centre and as `hovered` in `vehicle.hotspots`; `input.mouse.position {x, y}` places the cursor for tests (checked: with the view turned down, the label showed `door0` under the cursor).
