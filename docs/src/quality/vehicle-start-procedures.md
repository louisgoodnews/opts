# Vehicle Start Procedures (M-0141)

*Facts read from the vehicles' own scripts (OSC read as prose, no text copied) and confirmed in the running game through `vehicle.click`, `vehicle.hotspots` and `vehicle.script_vars`. Each procedure lists what the player does, which variables change and what is still unverified ❓. A vehicle is added when its procedure has been run.*

## How controls are operated

| Control | In the game | By command |
| :--- | :--- | :--- |
| a key of `keyboard.cfg` | the bound key (rebindable, `controls.vehicle.*`) | `vehicle.key action=<name>` |
| a cockpit control (`[mouseevent] name`) | left click in the cockpit or passenger view: `name`, then `name_drag` on **every frame in which the mouse moves** while the button is held, `name_off` on release; `mouse_x`/`mouse_y` carry the movement of the frame in pixels | `vehicle.click name=… drag_x=… drag_y=… frames=… hold_ms=…`; the controls of a vehicle: `vehicle.hotspots` |

Since M-0141 the drag trigger fires on every moving frame (before, once): controls that accumulate the mouse movement in a position (the ignition key, rotary switches, sliders) now move with the mouse. Not every key of `keyboard.cfg` is handled by every vehicle: `vehicle.key action=engine_start` raises `kw_m_enginestart`, which the Citybus 530's scripts do not answer; its engine starts through the ignition key hot-spot.

## Citybus 530 by Kajosoft (`01a_o530_e2_2.bus`, checked 2026-10-20 on Ahlheim 5)

The vehicle has 288 hot-spots. The engine start is the ignition key `cp_kluczyk_mov` (the position is `cp_kluczyk_rot`, 0 to 1; the script adds `mouse_y / -75` per drag call):

| Position (`cp_kluczyk_rot`) | `cp_kluczyk_rot_mode` | What the script does |
| :--- | ---: | :--- |
| below 0.25 | 0 | everything off (`elec_busbar_main_sw` 0) |
| 0.25 to 0.5 | 1 | electrics on (`elec_busbar_main_sw` 1, the busbar voltage `elec_busbar_main` rises to 0.9 and on) |
| 0.5 to 0.75 | 2 | drive position, injection stays |
| above 0.75 | 3 | starter: when the engine is off, `cp_taster_motorabstellung` is not pressed, the gear selector is in neutral (`antrieb_getr_gangwahl == 1`) and the busbar is above 0.3, `engine_starter` and `engine_injection_on` become 1 and the starter event is raised; releasing the key (`cp_kluczyk_mov_off`) springs it back to the drive position (0.625) and clears the starter |

**The first attempt after loading does not start the engine**: `engine_starter_antirepeat` is 1 from `{init}` and only the key's release (`_off`) clears it. So the procedure is: drag the key to the drive position, release, then drag past 0.75 and hold:

1. `vehicle.click name=cp_kluczyk_mov drag_y=-50 frames=60 hold_ms=200` (key to the drive position; the electrics come on).
2. wait for the busbar (about 2 s), then `vehicle.click name=cp_kluczyk_mov drag_y=-20 frames=30 hold_ms=4000` (to the starter, held).

Result in the game: `engine_on` 1, `engine_n` about 598 rpm (idle) a second after the start; the engine's idle loop is heard (level 0.085 at 9.6 m on the Vehicles bus, one voice with a sink). A single drag of 80 px in 10 frames does not start it: the busbar needs about a second to rise, and the starter branch runs once at the moment the key passes 0.75 (with the busbar still low it sets `engine_starter_antirepeat` and waits for the release).

❓ Open: the gear selector's states for driving (`antrieb_getr_gangwahl`), the parking brake interlock, the retarder; the same procedure for the other reference vehicles (MAN NL, SD200, NG272, C2, a tram).
