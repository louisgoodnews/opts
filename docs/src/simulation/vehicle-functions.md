# Vehicle Functions in OMSI

| Metadata | Value |
| :--- | :--- |
| **Basis** | Corpus survey of 5 script sets (7 vehicle parts), 2026-10-16, read-only |
| **Status** | 🟡 Draft |
| **Companion** | [OMSI scripting guide](../formats/text/vehicles/osc-guide.md) (the language), [OSC runtime design](../architecture/osc-runtime.md) (how OPTS should run it) |
| **Demand numbers** | From the [bus script integration research](../meta/bus-script-integration-2026-10-14.md) (1,089 player buses), cited as "corpus-wide" |

## 📖 Overview

This catalogue lists what buses *do* through their scripts — the behavioural capabilities a player meets — and for each one which scripts implement it, which variables and callbacks it runs on, and what the host must provide. It is the demand side of the runtime design: a function whose host requirement is missing cannot work, whatever the VM does.

**How it was built.** The script sets of five vehicles were read completely (every `.osc`, varlist, string varlist and constant file named by the `.bus`): the **MAN GN92** (`MAN_NL_NG/MAN_GN92_main.bus`, the reference bus, articulated, 26 scripts), the **MAN SD83** (`MAN_SD200/MAN_SD83.bus`, double-decker, 24), the **Mercedes Citaro** (`EMT_Citaro/EMT_Solo.bus`, 23), the **Düwag NF6D tram** (`Tram_NF6D/NF6D_A.bus` plus the B and C parts, 6 + 3 + 3) and the **Solaris Urbino II** (`Urbino_II/SU_12.bus`, 17). For each set, every variable, macro, trigger, event and callback name was tallied per file; the functions below were identified from those names and from reading the scripts that carry them. Where a line is cited it is `vehicle folder/file:line` as installed. The GN92, SD83 and Citaro share one script library (the Citaro's main file still carries the SD85 header), so agreement among those three is weaker evidence than agreement with the tram and the Urbino.

**Vocabulary.** *Buses observed* counts the five sets; the tram counts once although three parts were read. *Host requirements* names the engine-side service without which the function stays inert: physics contract, inputs (environment, time), events (sound), canvas (script textures), hof (route data), timetable, tickets, passengers, ground query, consist state. The variable families (`door_n`, `axle_brakeforce_n_s`, …) are the predefined names of [`predefined-variables.md`](../formats/text/vehicles/predefined-variables.md); callbacks are specified in [`callbacks.md`](../formats/text/vehicles/callbacks.md).

**Per-function format.**

```
### Name
- Buses observed: … (n/5)
- Scripts: files that implement it (GN92 names first)
- Variables: the ones that matter, inputs ⟵, outputs ⟶
- Callbacks: (M.V.…) used
- Host requirements: services
- Notes
```

## 🔩 Drivetrain

### Engine start and stop

- **Buses observed:** GN92, SD83, Citaro, Urbino, NF6D (5/5; the tram "starts" by closing the battery and raising the pantograph)
- **Scripts:** `engine_GN.osc` (triggers `kw_m_enginestart`, `kw_m_engine_startbutton`, `kw_m_engineshutdown`), `elec.osc`, `cockpit.osc` (`cp_taster_anlasser`); Urbino `Urbino_II_Engi.osc`; tram `NF6D_Cockpit_A.osc`
- **Variables:** `engine_starter`, `engine_injection_on`, `engine_on`, `engine_n` ⟶; `elec_busbar_main` ⟵ (electrics); AI path writes `engine_on` directly from `AI_Engine` (`MAN_GN92_main.osc:106-125`)
- **Callbacks:** none
- **Host requirements:** key triggers, events (`ev_engine_starter` family), electrics
- **Notes:** The starter torque is `exp(−n/n½)·starterM·busbar` added to `engine_M` (`engine_GN.osc:385`); `engine_n` is integrated from torque and inertia (`engine_GN.osc:404-411`). A cold-started vehicle in the rig therefore needs the battery and starter triggers before anything else moves (today `script_startup_system` emulates them).

### Engine model: torque, idle, temperature, consumption

- **Buses observed:** GN92, SD83, Citaro, Urbino (4/5)
- **Scripts:** `engine_GN.osc` macros `engine_moment`, `engine_acceleration`, `engine_consumption`, `engine_speedcontrol`, `engine_temperature`, `engine_traegheit`; Urbino `Urbino_II_Engi.osc`
- **Variables:** `engine_n`, `engine_M`, `engine_temperature`, `engine_tank_content`, `tank_percent` ⟶ (`engine_GN.osc:275`); `Throttle`, `Weather_Temperature`, `Wheel_RotationSpeed_*` ⟵; curves `engine_M_maxThrottle`, `engine_M_minThrottle`, `engine_turbo_boost` (constant files)
- **Callbacks:** none
- **Host requirements:** constant files with curves, environment temperature, trigger `veh_tank` (engine-raised: refuelling, `engine_GN.osc:129`)
- **Notes:** Torque is a blend of two curves by throttle plus turbo boost (`engine_GN.osc:353`); everything downstream (gearbox, exhaust, sound pitch) reads `engine_n`.

### Automatic gearbox with torque converter, kickdown and shift logic

- **Buses observed:** GN92, SD83, Citaro, Urbino (4/5; the tram has a fixed ratio `GetriebeUe`)
- **Scripts:** `antrieb_D854_GN.osc` (macros `antrieb_frame`, `antrieb_getr_chkNxtGear`, `antrieb_getr_chkCh`, `antrieb_getr_schaltmoment`); SD83 `antrieb.osc`; Citaro `antrieb.osc`; Urbino `Urbino_II_D854.osc`
- **Variables:** `antrieb_getr_gangvorwahl` (selector), `antrieb_getr_gangwahl`, `antrieb_getr_aktugang` (gear), `antrieb_n_kardanwelle`, `antrieb_kickdown`, `antrieb_throttle_stellglied`, `antrieb_beschleunigung`, `antrieb_wandler_*`; `M_Wheel` ⟶; `Throttle`, `Velocity`, `Wheel_RotationSpeed_2_L/R`, `n_Wheel` ⟵; constants `antrieb_getr_ratio1..`, `antrieb_i_achse`, `antrieb_getr_autoSwUp*Spd*`, `gear_efficiency`
- **Callbacks:** none
- **Host requirements:** physics contract (wheel speeds in, `M_Wheel` out)
- **Notes:** The output shaft speed is derived from the driven axle's wheel speeds (`antrieb_D854_GN.osc:52`), so the wheel speeds the host feeds decide every shift. `M_Wheel` is written in several places and later *added to* by the retarder (`:181, 288, 495, 501`): the host must read it after the whole frame.

### Retarder

- **Buses observed:** GN92, SD83, Citaro, Urbino (4/5)
- **Scripts:** `antrieb_D854_GN.osc` (retarder section), `cockpit.osc` (lever), `engine_GN.osc`
- **Variables:** `antrieb_retarder`, `antrieb_retarder_armed`, `antrieb_retardermoment`, `antrieb_retarder_time_elapsed`; curves `retarder_stufe1`, `retarder_fadein`; `M_Wheel` ⟶ (negative contribution)
- **Callbacks:** none
- **Host requirements:** physics contract (a negative `M_Wheel` must brake)
- **Notes:** The retarder torque is proportional to shaft speed and fades in over time (`antrieb_D854_GN.osc:495`).

### Tram traction

- **Buses observed:** NF6D (1/5)
- **Scripts:** `NF6D_A_Kopf.osc` (`A_Kopf_Frame`)
- **Variables:** `SLR_A_Kopf_Gas` / `SLR_B_Kopf_Gas` (the active cab's controller through a control cable), `WinkelV`, `GetriebeUe`, `Anfahr_Mult`, `M_Wheel_Soll`; `M_Wheel` ⟶
- **Callbacks:** none
- **Host requirements:** physics contract, consist state (cables)
- **Notes:** `M_Wheel = 210000 · gas / WinkelV · ratio · startMult`, then divided by 30 or 17 in some regimes (`NF6D_A_Kopf.osc:955-1001`); the sign flips for the rear cab. This is the expression the research report proposes for calibrating the `M_Wheel` unit (Q-BSI-1).

### Gear display and selector

- **Buses observed:** GN92, SD83, Citaro, Urbino (4/5)
- **Scripts:** `cockpit.osc` (triggers `automatic_D/N/R`, selector buttons), `antrieb_*.osc`
- **Variables:** `antrieb_getr_gangvorwahl`, `cockpit_light_*`
- **Host requirements:** key triggers, lamps (`model.cfg` reads the variables)

### Over-rev and start interlocks

- **Buses observed:** GN92, SD83, Citaro, Urbino (4/5)
- **Scripts:** `antrieb_D854_GN.osc:100-110` (no reverse above 5 km/h; neutral removes the gear), `engine_GN.osc` (`engine_starter_antirepeat`)
- **Variables:** `antrieb_getr_gangvorwahl`, `Velocity`, `engine_starter_antirepeat`
- **Host requirements:** none beyond inputs

## 🛑 Brakes

### Service brake with air system

- **Buses observed:** GN92, SD83, Citaro, Urbino (4/5; the tram writes a single `Brakeforce`)
- **Scripts:** `bremse_GN92.osc` (1,715 lines: tanks, valves, cylinders, `bremse_druckausgleich`), SD83 `bremse.osc`, Citaro `bremse_solo.osc`, Urbino `Urbino_II_brea.osc`
- **Variables:** `bremse_p_tank01..04`, `bremse_v_tank*`, `bremse_p_balg_*`, `bremse_kompressor*`; `Axle_Brakeforce_n_L/R`, `Brakeforce` (SD83 and Urbino also) ⟶; `Brake`, `Velocity_Ground`, `Axle_Suspension_*`, `Wheel_RotationSpeed_*` ⟵
- **Callbacks:** none
- **Host requirements:** physics contract (per-wheel brake force), events (`ev_bremse_*`, compressor), initial random tank pressure from `random`
- **Notes:** The compressor fills four circuits over about two minutes from a random start pressure (`script-vm.md`, store-keeps-value section); doors consume air from tank 4 (`door-3.osc:1709`). Brake pressure gauges read `bremse_p_tank*` through `model.cfg`.

### Parking brake (spring brake)

- **Buses observed:** all (5/5)
- **Scripts:** `bremse_GN92.osc`, `cockpit.osc` (trigger `parking_brake_toggle`, mouse `parking_brake_mouse_*`); tram `NF6D_A_Kopf.osc` (`federspeicher_a`)
- **Variables:** `bremse_feststell`, `bremse_feststell_sw`, `bremse_feststell_klinke`; Citaro adds an emergency release `bremse_feststell_notloese*`
- **Host requirements:** physics (a held vehicle must not roll: research report §5 item 1), events `ev_bremse_feststell_ein/aus` (1,008 buses corpus-wide)

### Stop brake (Haltestellenbremse)

- **Buses observed:** GN92, SD83, Citaro, Urbino (4/5)
- **Scripts:** `bremse_GN92.osc`, `door-3.osc` (interlock: doors need the stop brake, `door-3.osc:74-76`), `lights.osc` (no brake light while the stop brake holds, `lights.osc:184`)
- **Variables:** `bremse_halte`, `bremse_halte_sw`, constants `door_needs_stopbrake*`
- **Host requirements:** physics, events `ev_haltebremse_*`

### ABS and ASR

- **Buses observed:** GN92, SD83, Citaro, Urbino (4/5)
- **Scripts:** `bremse_GN92.osc` (`bremse_abs_eingriff_0l..2r`), `antrieb_*.osc` (ASR), Urbino `abs_vl..hr`, `cp_asr`
- **Variables:** per-wheel `Wheel_RotationSpeed_n_s` ⟵ against `Velocity`; `Axle_Brakeforce_*` ⟶ modulated
- **Host requirements:** physics contract with **real per-wheel speeds** (slip): with wheel speeds derived from chassis speed, as today, ABS never engages

### Retarder and engine brake as brakes

See Drivetrain; the retarder lever is in `cockpit.osc` and its effect enters `M_Wheel`.

### Tram brakes: rail brake, sanding, PZB

- **Buses observed:** NF6D (1/5)
- **Scripts:** `NF6D_A_Kopf.osc` (comment block lines 5-20: manual and PZB states of `SchienenbremseSand`), `NF6D_Global_A.osc`, `NF6D_Cockpit_A.osc`
- **Variables:** `pzb_bremsung_a/b`, `slr_pzb_a/b`, `sand_mult`, `cp_sand_a`, `brakeforce` ⟶; events `ev_sand_an/aus`
- **Host requirements:** physics (brake force), consist state, rail signalling for PZB ❓ (no input exists today)

## 🚪 Doors

### Door open and close with interlocks

- **Buses observed:** all (5/5)
- **Scripts:** `door-3.osc` (1,837 lines: triggers `bus_doorfront0/1`, `bus_dooraft`, macros `Door0_Calc`…`Door5_Calc`, `trg_bus_*`); SD83 `door.osc`; Citaro `door_solo.osc` (2,766 lines); tram `NF6D_Tueren_A/B/C.osc`; Urbino `Urbino_II_tuer.osc`
- **Variables:** `doorTarget_n`, `door_n` (position 0..1), `doorSpeed_n`, `doorMaxSpeed_n`, `doorAcc_n`, `door_n_warten`; curves `door_n_opn_speed`, `door_n_cls_speed`; interlocks `bremse_halte`, `elec_busbar_main`, `Velocity`; tram `tuer_n`, `tuer_richtung_n`, `slr_tuersteuerung_a/b`
- **Callbacks:** none
- **Host requirements:** key and mouse triggers (`door_n_drag`), events (`ev_doortriggeropen_n`, `ev_fahrertuer_auf/zu`: 1,007 buses corpus-wide), air system
- **Notes:** The trigger toggles a target; the frame integrates the position with acceleration and speed limits (`door-3.osc:974-1010`). "Thinking doors" wait for the neighbouring wing (`:991-997`). The tram's rear parts run their own door scripts and receive the door command through control cables (`NF6D_B.bus` `[control_cable_front]`).

### Passenger door clearance, stop request and outside opener

- **Buses observed:** all (5/5)
- **Scripts:** `door-3.osc` (triggers `door_haltewunsch`, `door_aussenoeffner`, `door_kinderwagenwunsch`; `:147-195, 319, 383-388`), Citaro `door_solo.osc`, tram `NF6D_Tueren_A.osc`, Urbino `Urbino_II_tuer.osc`, `Urbino_II_Ianz.osc`
- **Variables:** `PAX_Entry0..n_Req`, `PAX_Exit0..n_Req` ⟵ (passenger wants in/out); `PAX_Entry*_Open`, `PAX_Exit*_Open` ⟶ (derived from `door_n > 0.9`, `door-3.osc:383-385`); `haltewunsch`, `door_freigabe`, `AI_Scheduled_AtStation` ⟶
- **Callbacks:** `GetHumanCountOnPathLink` (GN92, Citaro, Urbino: is someone standing in the door?), `GetHumanCountOnSeat` (tram, 40 uses in the A part and 51 in C)
- **Host requirements:** **passengers** (request flags in, open flags out, counts on links and seats), the stop-request lamp and chime (events)
- **Notes:** This is the contract between the scripts and the passenger system: nobody boards unless the script says the door is open, and the script does not close on someone the count says is in the door.

### Wheelchair ramp and kneeling

- **Buses observed:** GN92, Citaro, Urbino (3/5)
- **Scripts:** `ramplift.osc` (GN92 lift), `bremse_GN92.osc` (`bremse_kneeling`: air suspension), Citaro `door_solo.osc`/`vdv_cockpit.osc` (`cp_kneeling_*`, `cp_autokneel_sw`), Urbino `Urbino_II_tuer.osc` (`cp_ramp_toggle`, `cti_rampe`)
- **Variables:** `Axle_Springfactor_n_s` ⟶ (kneeling lowers the suspension), `cockpit_light_kneeling*`, ramp animation variables; `cti_rampe` ⟵ (repaint `[setvar]`: equipment option)
- **Callbacks:** `GetHeightAbovePoint` (GN92 3 uses, Citaro 1, Urbino 1: where does the ramp meet the kerb?)
- **Host requirements:** ground query, suspension (`Axle_Springfactor` consumed by physics: nothing consumes it today), repaint `[setvar]` before `{init}`, events `ev_warning_rampe`

## 💡 Lighting

### Headlights, position and high beam

- **Buses observed:** all (5/5)
- **Scripts:** `lights.osc` (triggers `kw_standlicht_toggle`, `kw_scheinwerfer_toggle`, `kw_fernlicht_toggle`), `cockpit.osc` (switch and lever); Urbino `Urbino_II_Elec.osc` (`lights_abblend`, `vdv_lichtschalter_*`); tram `NF6D_Global_A.osc` (`scheinwerfer_a`, `fernlicht_a`)
- **Variables:** `lights_sw_*`, `lights_stand`, `lights_abblend`, `lights_fern`; `Spot_Select`, `AI_Light` ⟶; `elec_busbar_main` ⟵
- **Host requirements:** light rendering from the variables (exists: `vehicle_lights.rs`), key triggers, `Envir_Brightness` for automatic light

### Indicators and hazard lights

- **Buses observed:** all (5/5)
- **Scripts:** `lights.osc` (`lights_runblinkgeber`, `lights_calc_geberfaktor`; triggers `blinker_left_move`, `blinker_right_move`, `blinker_warn_toggle`), `cockpit.osc`; tram `NF6D_Global_A.osc` with control cables `SLR_BlinkerL_A`/`SLW_BlinkerL_A` to the other parts
- **Variables:** `lights_sw_blinker` (0/1/2), `lights_sw_warnblinker`, `lights_blinkgeber`; `lights_blinker_l/r`, `AI_Blinker_L/R` ⟶ (`lights.osc:190-203`)
- **Host requirements:** events `ev_lights_blinker_on/off` (1,015 buses corpus-wide: the relay click), lamp rendering, AI traffic reads `AI_Blinker_*`

### Brake and reversing lights

- **Buses observed:** all (5/5)
- **Scripts:** `lights.osc:184-188` (`lights_brems` from `Brake > 0.1` or the stop brake; `lights_rueckfahr` from reverse gear)
- **Variables:** `Brake` ⟵, `antrieb_getr_gangwahl` ⟵, `lights_brems`, `lights_rueckfahr` ⟶; Urbino `AI_Brakelight`
- **Host requirements:** lamp rendering

### Interior, dashboard and reading lights

- **Buses observed:** all (5/5)
- **Scripts:** `lights.osc` (`AI_Interiorlight`), `cockpit.osc`; Citaro `cp_innenlicht_swl/swu`; tram `NF6D_Cockpit_A.osc` (`slw_lichtfahrgastraum`, emergency light `slw_lichtfahrgastraum_not` over cables)
- **Variables:** `AI_Interiorlight` ⟶ (1,032 buses corpus-wide), `lights_innen*`, dashboard brightness
- **Host requirements:** interior light rendering, consist state (tram)

### Bulb failures and repair

- **Buses observed:** GN92, SD83 (2/5)
- **Scripts:** `lights.osc` (`lights_repair`, `lights_repair_timecalc`, `lights_malfunction_*`), `MAN_GN92_main.osc:145-160` (`malfunction_reset`, `malfunction_gettime`)
- **Variables:** `wearlifespan` ⟵ (805 buses corpus-wide), per-bulb state
- **Callbacks:** `NrSpecRandom`
- **Host requirements:** engine-raised triggers `malfunction_reset` and `malfunction_gettime`; the latter leaves a repair time on the stack for the engine ❓ (Q-OSC-G3)

## 🧾 IBIS

### Line and course entry, route selection

- **Buses observed:** all (5/5)
- **Scripts:** `IBIS-2.osc` (triggers `IBIS_0..9`, `IBIS_eingabe`, `IBIS_setmode_*`, `IBIS_loeschen`; macros `IBIS_SetRoute`, `IBIS_Zahlentasten`); SD83 `IBIS.osc`; tram `NF6D_IBIS_A.osc` (95 route/IBIS names); Urbino `Urbino_II_IBIS.osc`; Citaro: the IBIS lives in `ticketprinter.osc` (its `IBIS_init` is commented out in the main file)
- **Variables:** `IBIS_mode`, `IBIS_input`, `IBIS_LinieKurs`, `IBIS_Route`, `IBIS_RouteIndex`, `IBIS_TerminusCode/Index`; strings `IBIS`, `IBIS_cabindisplay`, `Matrix_Nr`; `target_index_int` ⟶ (1,057 buses corpus-wide); `AI_Target_Index`, `SetLineTo`, `schedule_active` ⟵
- **Callbacks:** `GetRouteIndex`, `GetRouteTerminusIndex`, `GetTerminusIndex`, `GetTerminusCode`, `GetTerminusString`, `GetDepotStringGlobal` (all 5 sets)
- **Host requirements:** **hof** (the depot file: termini, stops, routes), key triggers, events `ev_IBIS_piep`/`pieperror`
- **Notes:** Invalid input is signalled by `-1` from `GetRouteIndex`/`GetTerminusIndex` and an error mode with a blinking display (`IBIS-2.osc:147-157, 183-192`). The engine-raised trigger `ai_scheduled_settarget` sets line and terminus for a scheduled bus (`VMatrix.osc:38-60`).

### Current stop, next stop, stop list

- **Buses observed:** all (5/5)
- **Scripts:** `IBIS-2.osc` (`IBIS_RefreshBusstop`, `IBIS_vor`, `IBIS_rueck`, `IBIS_LCD-refresh`), tram `NF6D_IBIS_A.osc`, Urbino `Urbino_II_IBIS.osc`, `Urbino_II_Ianz.osc` (interior display)
- **Variables:** `IBIS_busstop` (position along the route), `IBIS_busstop_index` (global stop row); strings `act_busstop` ⟶ (the engine reads it: passengers, announcements), `IBIS_busstop_name`, `Haltestelle`
- **Callbacks:** `GetBusstopCount`, `GetRouteBusstopIdent`, `GetBusstopIndex`, `GetBusstopString` (all 5 sets); Urbino: 87 `GetBusstopString` and 73 `GetRouteBusstopIdent` uses (it renders the whole list)
- **Host requirements:** hof; `Refresh_Strings` output tells the engine that strings changed (1,014 buses corpus-wide)
- **Notes:** `act_busstop` is the one string the engine must read back: `IBIS-2.osc:652-655` sets it "for the OMSI-internal calculation and the derived global stop index".

### Timetable: delay, departure times, schedule state

- **Buses observed:** GN92, Citaro, NF6D, Urbino (4/5)
- **Scripts:** `IBIS-2.osc:540` (`GetTTDelay`), Citaro `ticketprinter.osc` (14 `GetTTDelay` uses), tram `NF6D_IBIS_A.osc`, Urbino `Urbino_II_fare.osc:435-442` (next three departures as `hh:mm`), `Urbino_II_Ianz.osc`
- **Variables:** `schedule_active` ⟵ (494 buses corpus-wide); display strings
- **Callbacks:** `GetTTDelay`, `GetTTBusstopIndex`, `GetTTBusstopName`, `GetTTBusstopCount`, `GetTTBusstopDep`, `GetTTBusstopArr`, `GetTTLineString`, `GetTTTerminusIndex`
- **Host requirements:** **timetable** (the active course's stops and times against the simulation clock); today OPTS answers these with static spawn-time values
- **Notes:** Delay is positive when late; the scripts choose "too early" below −30 s and "too late" above 240 s (`callbacks.md`).

## 🎟️ Ticketing

### Ticket selection and printing

- **Buses observed:** GN92, Citaro, Urbino (3/5; the SD83 has a cash desk only, the tram a `ticket_give` trigger only)
- **Scripts:** `ticketprinter.osc` (triggers `ticketprinter_button_ticket_0..4`, `_enter`, `_cancel`, `ticketprinter_getticket`), Citaro `ticketprinter.osc` (1,321 lines, with a menu and the IBIS), Urbino `Urbino_II_fare.osc` (1,268 lines, "Faremaster"), `Urbino_II_Alme.osc`
- **Variables:** `ticketprinter_ticket_preselection/selection`, `ticketprinter_druckt`, `ticketprinter_ticket_pos`; `GivenTicket` ⟶ (938 buses corpus-wide; `-1` = none, `ticketprinter.osc:251, 347`)
- **Callbacks:** `GetTicketName`, `GetTicketValue` (Citaro 17 and 28 uses)
- **Host requirements:** **tickets** (the vehicle's ticket pack `.otp`: names and prices), **passengers** (a passenger asks for a ticket and takes `GivenTicket`), events `ev_printer_*`, `ev_ticketsale_giveticket`
- **Notes:** Printing is a timed animation (`ticketprinter_ticket_pos` integrated over one second, `:340-349`); the ticket is handed over when it completes.

### Cash desk and change

- **Buses observed:** GN92, SD83, Citaro, Urbino (4/5)
- **Scripts:** `cashdesk.osc` (one trigger per coin slot: `cashdesk_changer_0_05` … `2_00`, upper and lower rows), Citaro `cashdesk_EUR.osc`, Urbino `Urbino_II_cash.osc`
- **Variables:** `cashdesk_changer_sw_*`, `cashdesk_changetimer_*`
- **Callbacks:** `GiveChangeCoin` (26, 26, 25, 25 uses)
- **Host requirements:** **money** (coin and bill table of the vehicle's currency), passengers (who receives the coin), events `ev_ticketsale_givechange*`
- **Notes:** The coin index runs over the currency file's coins then bills (`callbacks.md`); the same index is reused per slot across triggers.

### Fare computation and stop-dependent prices

- **Buses observed:** Urbino (1/5), Citaro partly
- **Scripts:** `Urbino_II_fare.osc` (`Faremaster_BusStop`, `Faremaster_BusStopCount`), Citaro `ticketprinter.osc` (zone logic ❓ not read in depth)
- **Callbacks:** `GetTTBusstopDep`, `GetTTBusstopArr`, `GetTicketValue`, `GetBusstopString`
- **Host requirements:** timetable, tickets, hof

## 🌡️ HVAC

### Heating, ventilation, air conditioning

- **Buses observed:** GN92, SD83, Citaro, Urbino (4/5)
- **Scripts:** `heizung.osc` (542 lines, 137 names in the GN92; `cabinair_a_*`, `auxheat_*`), Citaro `heizung.osc` (771 lines, `c2klima_*` with driver AC and recirculation), Urbino `Urbino_II_heat.osc`
- **Variables:** `Cabinair_Temp` ⟶ (986 buses corpus-wide), `Cabinair_AbsHum`; `Weather_Temperature`, `Weather_AbsHum`, `humans_count` ⟵ (passengers warm the cabin), `Envir_Brightness` (Citaro); heater and blower switches in `cockpit.osc`
- **Callbacks:** none
- **Host requirements:** environment (temperature, humidity), passenger count, engine temperature; the engine consumes `Cabinair_Temp` for window fogging ❓ (`rain.osc`, `beschlag_wiper` in the Citaro)
- **Notes:** The tram has no HVAC script.

### Window fogging and defrost

- **Buses observed:** Citaro, GN92 (via `rain.osc`) (2/5)
- **Scripts:** Citaro `wiper.osc`/`heizung.osc` (`beschlag_wiper`), GN92 `rain.osc`
- **Host requirements:** environment, `Cabinair_*` consumed by rendering

## 🪟 Wipers and Washers

### Wiper modes, interval, washer

- **Buses observed:** all (5/5)
- **Scripts:** `wiper.osc` (`wiper_frame`), `cockpit.osc` (lever `ev_wischerhebel`, 11 events), Citaro `vdv_cockpit.osc` (rotary switch), tram `NF6D_Cockpit_A.osc` (`cp_wischer1..3`), Urbino `Urbino_II_misc.osc` (`cp_wipermode_drag`)
- **Variables:** `wiper_pos`, `wiper_mode`, `cockpit_light_wischer_*`; `PrecipRate` ⟵ (`wiper.osc`, `rain.osc`: 1,051 buses corpus-wide)
- **Host requirements:** environment (`PrecipRate`, `PrecipType` on the tram), key triggers (`kw_wipermode_up`), events `wiper_start` (998 buses corpus-wide), wiper animation from the position variable
- **Notes:** No rain sensor in the five sets.

### Rain, dirt and spray visuals

- **Buses observed:** all (5/5)
- **Scripts:** `rain.osc`, `dirt.osc`, `spray.osc`; tram `NF6D_Global_A.osc` (`regen`, `dreck_strassenbahn`); Urbino `Urbino_II_misc.osc`
- **Variables:** `rain_window_*_wetness`, `dirt_wiped`, `dirt_wiper_pos`; `PrecipRate`, `DirtRate`, `StreetCond` ⟵
- **Host requirements:** environment inputs, rendering of wetness and dirt from the variables (window materials)

## 📻 Information Systems

### Stop announcements

- **Buses observed:** GN92, Citaro, NF6D, Urbino (4/5)
- **Scripts:** `IBIS-2.osc:668-682` (`IBIS_PlayAnnouncement`), Citaro `ticketprinter.osc` (`ev_ansage`), tram `NF6D_IBIS_A.osc`, Urbino `Urbino_II_IBIS.osc`, `Urbino_II_fare.osc` (`faremaster_playannouncement`)
- **Variables:** `act_busstop` (string), `IBIS_busstop`
- **Callbacks:** `GetDepotStringGlobal` (slot 0: announcement folder), `GetBusstopCount`
- **Host requirements:** **audio with a payload**: the `(T.F.ev_IBIS_Ansagen)` event carries the `.wav` path on the string stack (`Vehicles/Announcements/<folder>/<ident>[_#terminus].wav`); hof
- **Notes:** The path is also written with `$msg` for debugging and then used (`IBIS-2.osc:673`), see the guide's pitfall on `$msg`.

### Interior displays (LCD, next-stop display)

- **Buses observed:** GN92, Citaro, NF6D, Urbino (4/5)
- **Scripts:** `IBIS-2.osc` (`IBIS_LCD-refresh`, `IBIS_cabindisplay`), tram `NF6D_IBIS_A.osc` (`ibis_innenanzeige`), Urbino `Urbino_II_Ianz.osc` (370 lines, `cti_innenanzeige` option)
- **Variables:** strings `IBIS_cabindisplay`, `IBIS_busstop_name`; `IBIS_LCD-zeile`, `IBIS_LCD-zeilentimer`
- **Host requirements:** `[texttexture]` (exists) or canvas; hof

### Microphone, radio, outside volume

- **Buses observed:** all (5/5)
- **Scripts:** `sound_volume.osc`, `cockpit.osc`
- **Variables:** `Snd_Microphone`, `Snd_Radio`, `Snd_OutsideVol` ⟶ (1,015 buses corpus-wide write `Snd_OutsideVol`)
- **Host requirements:** audio mixer reading the `Snd_*` variables (doors open → outside louder)

## 🎨 Displays and Indicators

### Destination display (matrix, roller blind, plug-in sign)

- **Buses observed:** all (5/5)
- **Scripts:** `VMatrix.osc` (746 lines: `Matrix_frame`, `matrix_refreshIntIndex`, triggers `matrix_seitenschildklemme_drag`, `bus_matrix_change_steckschild`, engine trigger `ai_scheduled_settarget`), SD83 `Matrix.osc` (roller blind through `[texttexture]`), tram `NF6D_Matrix_A.osc` (968 lines), Urbino `Urbino_II_Matr.osc` (1,831 lines, `cti_matrixtype` chooses the display type per repaint)
- **Variables:** `Matrix_Nr`, `Matrix_TerminusIndex`, `Matrix_Xpos`, `Matrix_RefreshCursor` (pixels are transferred over several frames, `VMatrix.osc:468-479`), `matrix_fehlerpixel` (dead pixels by age, `NrSpecRandom`); `target_index_int` ⟶
- **Callbacks:** `GetFontIndex` (every frame, `VMatrix.osc:157-164`), `TextLength`, `STNewTex`, `STLock`, `STUnlock`, `STSetColor`, `STTextOut`, `STDrawRect`, `STReadPixel`, `STGetR`, `STFilter`, `STLoadTex`; `GetTerminusString` (column 7 names a bitmap, `:326`)
- **Host requirements:** **canvas** (script textures with fonts), hof, repaint `[setvar]` (`cti_matrixtype`), bitmap files under `Vehicles/Anzeigen/`
- **Notes:** Four of five sets draw with the `ST*` API; the SD83 only sets strings for `[texttexture]`. The matrix is the first-order display demand (936 buses corpus-wide).

### Dashboard gauges and warning lamps

- **Buses observed:** all (5/5)
- **Scripts:** `cockpit.osc` (2,103 lines, 118 triggers: `tacho_frame`, `cockpit_tachowinkel`, `pressure_warning_needle_frame`, `cockpit_light_*` lamps), Citaro `vdv_cockpit.osc` (4,151 lines: VDV standard cockpit with display pages), Urbino `Urbino_II_dash.osc` (1,693 lines), tram `NF6D_Global_A.osc` (`tachowinkel_a`, `lm_b_stoerung`)
- **Variables:** `Velocity`, `kmcounter_km/m` ⟵ (998 buses corpus-wide); needle angles and lamp states ⟶ read by `model.cfg` animations; `cockpit_light_speedwarning`
- **Host requirements:** inputs, animation from variables (exists)
- **Notes:** The Citaro's VDV cockpit is the largest single script of the survey; its display pages are drawn with `[texttexture]` strings.

### Script textures in general

See the destination display; `[scripttexture]` canvases per part: GN92 2, Citaro 2, tram 2, Urbino 2 (research report §4.9 and `model.cfg` blocks).

## 🚨 Warnings and Alarms

### Speed warning, pressure warning, door warning

- **Buses observed:** GN92, SD83, Citaro, tram (4/5)
- **Scripts:** `cockpit.osc` (`cockpit_light_speedwarning`, `pressure_warning_needle_frame`), `door-3.osc` (door open while moving), tram `NF6D_Global_A.osc` (`warnglocke`, `lm_b_stoerung`)
- **Host requirements:** events (buzzers), lamps

### Collision and damage, malfunctions

- **Buses observed:** GN92, SD83, Citaro (3/5)
- **Scripts:** `collision.osc` (`Collision_Trigger`, `Collision_Malfunction_Reset`, `Collision_Malfunction_TimeCalc`), `MAN_GN92_main.osc:136-160`, `elec.osc` (`Elec_Malfunction_*`)
- **Variables:** `coll_energy`, `coll_pos_y/z` ⟵ (961 buses corpus-wide read `coll_energy`); thresholds `collision_energy_*_threshold`; `wearlifespan`
- **Host requirements:** engine-raised trigger `collision` with the collision inputs set first; `malfunction_reset` and `malfunction_gettime` from the repair menu ❓
- **Notes:** Damage effects: electrics fail above a threshold, brakes on the Citaro (`collision_energy_bremse_threshold`).

### Horn and bell

- **Buses observed:** all (5/5)
- **Scripts:** `cockpit.osc` (`hupe_frame`, triggers `horn`/`horn_off`), Citaro three horn buttons, tram `warnglocke` (bell) and `hupe_auto_a`, Urbino `lichthupe`
- **Host requirements:** events `ev_hupe_an/aus` (980-997 buses corpus-wide), key triggers

## 🧬 Vehicle-Specific Extensions

### Articulation (bending protection)

- **Buses observed:** GN92, Urbino (2/5; the Citaro solo only carries the names for its articulated sibling)
- **Scripts:** `articulation.osc` (77 lines: `articulation_frame`), `engine_GN.osc` (bending protection cuts torque), Urbino `Urbino_II_Gele.osc`
- **Variables:** `articulation_0_alpha/beta` ⟵ (602 buses corpus-wide: engine-written joint angles), `articulation_0_alpha_speed`, `articulation_knickschutz_warning`, `engine_knickschutz_*`
- **Host requirements:** consist physics writing the joint angles each step (exists for the head; the rear part has no engine of its own today)

### Multiple-unit control (control cables)

- **Buses observed:** NF6D (1/5)
- **Scripts:** all six `NF6D_*_A.osc` and the B/C parts; 85 `SL*` names in the A part
- **Variables:** `SLR_*` (read from the cable), `SLW_*`/`SLA_*` (written to it): blinkers, battery, pantograph, door control, cab selection `slr_a_kopf_inaktiv`, PZB; declared as 102 `[control_cable_front]` entries in `NF6D_A.bus` and matching `[control_cable_*]` in B and C
- **Host requirements:** **consist state** with a cable bridge between the parts' variable stores (strategy D1)
- **Notes:** The one surveyed vehicle whose parts run *different* scripts (the GN92 trailer shares the head's set through `[scriptshare]`).

### Pantograph and overhead line

- **Buses observed:** NF6D (1/5)
- **Scripts:** `NF6D_Global_A.osc`, `NF6D_Cockpit_A.osc` (`timer_panto`, `panto_anim_*`, `cti_ol_erkennung` in part C)
- **Host requirements:** consist state; an overhead-line input does not exist ❓

### Kneeling and ramp

See Doors.

### Repaint-dependent equipment

- **Buses observed:** Urbino, GN92 (2/5 visibly; 872 buses corpus-wide)
- **Variables:** `cti_matrixtype`, `cti_innenanzeige`, `cti_rampe`, `cti_ol_erkennung` ⟵ from the `.cti` `[setvar]` of the chosen repaint
- **Host requirements:** apply `[setvar]` before `{init}` (exists)

### AI-driven operation

- **Buses observed:** all (5/5)
- **Scripts:** `man_GN92_main_AI.osc` (`{frame_ai}`, reaches 7 scripts), SD83 `man_sd85_main_AI.osc`, Citaro `AI.osc` (`ai_frame` called from `{frame}`), tram and Urbino handle `AI` inside their frames
- **Variables:** `AI`, `AI_Engine`, `AI_Target_Index`, `AI_Scheduled_AtStation` ⟵/⟶, `AI_Blinker_*`, `AI_Light`, `AI_Interiorlight` ⟶; the AI path writes `Throttle`/`Brake` into `M_Wheel`/`Brakeforce` directly (`man_GN92_main_AI.osc`)
- **Host requirements:** the traffic planner's pedals and target, `{frame_ai}` dispatch (no caller today: research report §2)

### Force feedback

- **Buses observed:** GN92, SD83, Citaro, Urbino (4/5)
- **Scripts:** `forcefeedback.osc`
- **Variables:** `FF_Vib_Amp`, `FF_Vib_Period` ⟶ from `engine_n` and `Throttle`
- **Host requirements:** input device (nothing consumes them today)

## 📊 Coverage Table

Universal = all five sets; common = three or four; rare = one or two; deferred = the host service it needs is planned in a later phase of the [strategy](../architecture/bus-script-integration.md) (P5 passengers and tickets, P6 sound, P7 AI) or has no milestone.

| Function | Universal | Common | Rare | Deferred | Host service (strategy phase) |
| :--- | :---: | :---: | :---: | :---: | :--- |
| Engine start and stop | ✓ | | | | triggers, events (P0, P6) |
| Engine model | | ✓ | | | inputs, constants (P0, P1) |
| Automatic gearbox, converter | | ✓ | | | physics contract (P1) |
| Retarder | | ✓ | | | physics contract (P1) |
| Tram traction | | | ✓ | | physics, consist (P1, P2) |
| Service brake, air system | | ✓ | | | physics contract (P1) |
| Parking brake | ✓ | | | | physics (P1) |
| Stop brake | | ✓ | | | physics (P1) |
| ABS/ASR | | ✓ | | | per-wheel physics (P1) |
| Tram rail brake, sanding, PZB | | | ✓ | ✓ | no signalling input exists |
| Doors with interlocks | ✓ | | | | triggers, events, consist for parts (P0, P2, P6) |
| Passenger clearance, stop request | ✓ | | | ✓ | passengers (P5) |
| Ramp and kneeling | | ✓ | | | ground query (done), springfactor (P1) |
| Headlights | ✓ | | | | lights (done) |
| Indicators, hazard | ✓ | | | | events (P6), lights (done) |
| Brake and reversing lights | ✓ | | | | lights (done) |
| Interior lights | ✓ | | | | lights (done), consist (P2) |
| Bulb failures | | | ✓ | | engine-raised triggers (E7) |
| IBIS line and route | ✓ | | | | hof (done for 27 callbacks), triggers |
| Current and next stop | ✓ | | | | hof, `act_busstop` read-back, `Refresh_Strings` (P3) |
| Timetable delay, times | | ✓ | | | live timetable (P3) |
| Ticket printing | | ✓ | | ✓ | tickets (done), passengers (P5) |
| Cash desk | | ✓ | | ✓ | money (P5) |
| Fare computation | | | ✓ | ✓ | timetable, tickets (P3, P5) |
| HVAC | | ✓ | | | environment inputs (P0), `Cabinair_Temp` consumer |
| Fogging and defrost | | | ✓ | | rendering |
| Wipers | ✓ | | | | `PrecipRate` (P0), events (P6) |
| Rain, dirt, spray | ✓ | | | | inputs (P0), rendering |
| Stop announcements | | ✓ | | ✓ | audio with payload (P6) |
| Interior displays | | ✓ | | | `[texttexture]` (done), hof |
| Microphone, radio, outside volume | ✓ | | | ✓ | audio mixer (P6) |
| Destination display | ✓ | | | | canvas and fonts (P4), hof |
| Gauges and lamps | ✓ | | | | inputs (done) |
| Warnings | | ✓ | | | events (P6) |
| Collision and malfunctions | | ✓ | | | engine-raised triggers (P0, E7) |
| Horn and bell | ✓ | | | ✓ | events (P6) |
| Articulation | | | ✓ | | consist physics (P2) |
| Control cables | | | ✓ | | consist state (P2) |
| Pantograph | | | ✓ | ✓ | no overhead-line input |
| Repaint equipment | | ✓ | | | `[setvar]` before init (done) |
| AI operation | ✓ | | | ✓ | planner pedals, `{frame_ai}` (P7) |
| Force feedback | | ✓ | | ✓ | input device (no milestone) |

**Reading the table for the design.** The universal functions that are *not* yet served fall into four services: the physics contract (brakes, parking brake, gearbox), the live route and timetable layer (`act_busstop` read-back, `Refresh_Strings`, timetable), the canvas (destination display), and events (every state change raises one). These are the first-order demands the [design sheet](../architecture/osc-runtime.md) addresses; passengers, money, audio playback and AI are named there and deferred to their milestones.

## 🔗 Related Documents

- [OMSI scripting guide](../formats/text/vehicles/osc-guide.md) — the language and its idioms (the door, gearbox, IBIS and matrix patterns are quoted there)
- [OSC runtime design](../architecture/osc-runtime.md) — the runtime that serves these functions
- [Bus script integration strategy](../architecture/bus-script-integration.md) and [research report](../meta/bus-script-integration-2026-10-14.md) — corpus-wide demand numbers
- [Callbacks](../formats/text/vehicles/callbacks.md), [Predefined variables](../formats/text/vehicles/predefined-variables.md), [Script sets](../formats/text/vehicles/script-sets.md)

---

**Clean-room attestation (ADR-0002):** no OMSI binaries inspected, no GPL source read, no OMSI asset committed; variable and file names are facts about the installed files.
