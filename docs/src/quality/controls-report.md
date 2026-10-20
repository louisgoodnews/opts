# Controls Report (M-0141)

*What can be operated and what cannot, for the reference vehicles; produced 2026-10-20 by `census_controls_report` in `crates/opts-game/tests/vehicle_start.rs` (run with `OMSI_PATH` and `--ignored --nocapture`). A hot-spot counts as handled when the vehicle's scripts have a trigger `<name>`, `<name>_drag` or `<name>_off`; a keyboard trigger counts as handled when the scripts have that trigger. The third direction, triggers the scripts handle that neither a key nor a hot-spot raises, is not computed ❓.*

| Vehicle | Mouse controls (hot-spots) | Keys of `keyboard.cfg` (`[vehicles]`, 91 triggers) |
| :--- | :--- | :--- |
| Citybus 530 | 289 hot-spots, 0 unhandled | 91 keyboard.cfg vehicle triggers, 36 unhandled |
| MAN NL | 111 hot-spots, 0 unhandled | 91 keyboard.cfg vehicle triggers, 35 unhandled |
| MAN SD200 | 94 hot-spots, 0 unhandled | 91 keyboard.cfg vehicle triggers, 26 unhandled |
| MAN NG272 | 112 hot-spots, 0 unhandled | 91 keyboard.cfg vehicle triggers, 35 unhandled |
| BRT C2 | 228 hot-spots, 3 unhandled | 91 keyboard.cfg vehicle triggers, 58 unhandled |
| Tram NF6D | 56 hot-spots, 0 unhandled | 91 keyboard.cfg vehicle triggers, 53 unhandled |

## Hot-spots without a handler

Only the BRT C2 has any: `AFR4_BT_Einstellung_10, AFR4_BT_Einstellung_6, AFR4_BT_Einstellung_8` (the buttons of a display setting, `AFR4`; decision: stub, the display is [M-0100](../roadmap/phase-4/M-0100.md)'s). Every other hot-spot of the five other vehicles has a script behind it.

## Keyboard triggers no script handles

The list per vehicle is long because `keyboard.cfg` names every vehicle key of OMSI. The decision per kind of trigger:

| Triggers | Why unhandled | Decision |
| :--- | :--- | :--- |
| `throttle`, `throttle_amplify`, `brake`, `clutch`, `steering_left`, `steering_right`, `steering_neutral`, `kw_s_*` (traction stages) | Not script triggers: the game's own drive input and rail dynamics read the pedals and the stage keys (W, S, the arrow keys) | **Host input**, nothing to handle; rebinding is [M-0142](../roadmap/phase-7/M-0142.md)'s next slice |
| `bus_ziel_*`, `bus_linie_*`, `bus_rollband_*` | The destination blinds and line numbers of buses that have them; the vehicle's own blinds are driven through the IBIS | **Out of scope** here, [M-0100](../roadmap/phase-4/M-0100.md) |
| `change_give`, `change_take`, `ticket_give`, `cashdesk_changer_*` | The cash desk of vehicles that sell tickets | **Out of scope**, [M-0101](../roadmap/phase-4/M-0101.md) |
| `bus_doorfront4`, `bus_doorfront5` | Fifth and sixth door; no reference vehicle has them | **Stub**: the bindings stay, the vehicles ignore them |
| `cp_licht_oberdeck_toggle`, `cp_licht_untenrechts_toggle`, `cp_licht_unterdeck_toggle`, `cp_microphone`, `cp_wischer_*`, `cp_schalter_kinderwagen`, `taster_nebelschluss`, `automatic_*`, `kw_standlicht_toggle`, `kw_m_enginestart`, `cp_batterietrennschalter_toggle` | Each vehicle implements only the functions it has (a tram has no upper deck; the C2 has no battery key but an ignition key) | **Per vehicle**: the vehicle keys page lists all, the vehicle ignores what it does not have; the procedures in [Vehicle Start Procedures](vehicle-start-procedures.md) say what each vehicle uses |

## The unhandled triggers per vehicle

**Citybus 530**: `brake, bus_doorfront4, bus_doorfront5, bus_linie_minus, bus_linie_plus, bus_rollband_dn, bus_rollband_dn_step, bus_rollband_setL1, bus_rollband_setL2, bus_rollband_setL3, bus_rollband_setT, bus_rollband_up, bus_rollband_up_step, bus_ziel_minus, bus_ziel_plus, change_give, change_take, clutch, cp_licht_oberdeck_toggle, cp_microphone, kw_s_1, kw_s_2, kw_s_3, kw_s_4, kw_s_5, kw_s_6, kw_s_N, kw_s_R, kw_s_minus, kw_s_plus, steering_left, steering_neutral, steering_right, throttle, throttle_amplify, ticket_give`

**MAN NL**: `brake, bus_doorfront4, bus_doorfront5, bus_linie_minus, bus_linie_plus, bus_rollband_dn, bus_rollband_dn_step, bus_rollband_setL1, bus_rollband_setL2, bus_rollband_setL3, bus_rollband_setT, bus_rollband_up, bus_rollband_up_step, bus_ziel_minus, bus_ziel_plus, change_give, change_take, clutch, cp_licht_oberdeck_toggle, kw_s_1, kw_s_2, kw_s_3, kw_s_4, kw_s_5, kw_s_6, kw_s_N, kw_s_R, kw_s_minus, kw_s_plus, steering_left, steering_neutral, steering_right, throttle, throttle_amplify, ticket_give`

**MAN SD200**: `brake, bus_doorfront4, bus_doorfront5, bus_linie_minus, bus_linie_plus, bus_ziel_minus, bus_ziel_plus, change_give, change_take, clutch, kw_s_1, kw_s_2, kw_s_3, kw_s_4, kw_s_5, kw_s_6, kw_s_N, kw_s_R, kw_s_minus, kw_s_plus, steering_left, steering_neutral, steering_right, throttle, throttle_amplify, ticket_give`

**MAN NG272**: `brake, bus_doorfront4, bus_doorfront5, bus_linie_minus, bus_linie_plus, bus_rollband_dn, bus_rollband_dn_step, bus_rollband_setL1, bus_rollband_setL2, bus_rollband_setL3, bus_rollband_setT, bus_rollband_up, bus_rollband_up_step, bus_ziel_minus, bus_ziel_plus, change_give, change_take, clutch, cp_licht_oberdeck_toggle, kw_s_1, kw_s_2, kw_s_3, kw_s_4, kw_s_5, kw_s_6, kw_s_N, kw_s_R, kw_s_minus, kw_s_plus, steering_left, steering_neutral, steering_right, throttle, throttle_amplify, ticket_give`

**BRT C2**: `IBIS_0, IBIS_1, IBIS_2, IBIS_3, IBIS_4, IBIS_5, IBIS_6, IBIS_7, IBIS_8, IBIS_9, IBIS_eingabe, IBIS_loeschen, IBIS_setmode_linie_kurs, IBIS_setmode_route, IBIS_setmode_ziel, automatic_1, automatic_2, brake, bus_20h-switch, bus_doorfront4, bus_doorfront5, bus_linie_minus, bus_linie_plus, bus_rollband_dn, bus_rollband_dn_step, bus_rollband_setL1, bus_rollband_setL2, bus_rollband_setL3, bus_rollband_setT, bus_rollband_up, bus_rollband_up_step, bus_ziel_minus, bus_ziel_plus, cashdesk_changer_15_00, cashdesk_changer_5_00, change_give, change_take, clutch, cp_schalter_kinderwagen, cp_wischer_intervall_toggle, kw_s_1, kw_s_2, kw_s_3, kw_s_4, kw_s_5, kw_s_6, kw_s_N, kw_s_R, kw_s_minus, kw_s_plus, kw_standlicht_toggle, steering_left, steering_neutral, steering_right, taster_nebelschluss, throttle, throttle_amplify, ticket_give`

**Tram NF6D**: `automatic_1, automatic_2, automatic_N, automatic_R, brake, bus_20h-switch, bus_doorfront4, bus_doorfront5, bus_linie_minus, bus_linie_plus, bus_rollband_dn, bus_rollband_dn_step, bus_rollband_setL1, bus_rollband_setL2, bus_rollband_setL3, bus_rollband_setT, bus_rollband_up, bus_rollband_up_step, bus_ziel_minus, bus_ziel_plus, cashdesk_changer_0_05, cashdesk_changer_0_10, cashdesk_changer_0_15, cashdesk_changer_0_30, cashdesk_changer_0_50, cashdesk_changer_15_00, cashdesk_changer_1_00, cashdesk_changer_1_50, cashdesk_changer_2_00, cashdesk_changer_4_00, cashdesk_changer_5_00, cashdesk_changer_8_00, change_give, change_take, clutch, cp_batterietrennschalter_toggle, cp_licht_oberdeck_toggle, cp_licht_untenrechts_toggle, cp_licht_unterdeck_toggle, cp_schalter_kinderwagen, cp_wischer_intervall_toggle, cp_wischer_wascher_button, kw_m_enginestart, kw_s_6, kw_s_minus, kw_scheinwerfer_toggle, kw_standlicht_toggle, steering_left, steering_neutral, steering_right, taster_nebelschluss, throttle, throttle_amplify`
