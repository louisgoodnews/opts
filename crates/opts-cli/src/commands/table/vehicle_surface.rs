//! The surface of the player's vehicle: dirt and wetness (M-0120, M-0121).

use crate::commands::{c, CommandSpec};

pub(super) const PART: &[CommandSpec] = &[
    c(
        "vehicle.surface",
        "drive",
        "The dirt and wetness of the player's vehicle and the road's snow; with `dirt` or `wet` set them, with `wiped` wash the dirt off",
        "dirt:f? wet:f? wiped:b?",
    )
    .since("1.34")
    .result("`{dirt, wet, wiped_at, snow}`; `wiped_at` is the session clock's second of the last wash (`null` before one)")
    .doc("`dirt` and `wet` are shares from 0 to 1 (`-32602` outside it). `dirt` is the script variable `Dirt_Norm`, which the stock scripts leave at 0: the host's level is fed to the player's vehicle every frame, so the dirt overlays show it (M-0120). `wiped: true` applies last and sets the dirt to 0, as a wash does. `wet` is kept for the vehicle's wet paint (M-0121) and is not drawn yet. `snow` is the snow cover of the road the weather gives and is read only. `1001` without a player vehicle.")
    .debug(),
];
