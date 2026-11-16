//! The self-written scenery object and its script.

pub(super) const SCO: &str = "\
[mesh]
pole.o3d

[LOD]
0.2

[mesh]
lamp_glass.o3d

[visible]
lamp_on
1

[mesh]
hand.o3d

[newanim]
origin_trans
0
0
2
anim_rot
hand_deg
1

[light_enh_2]
0
-0.2
2
0
-1
0
0
0
1
1
0
255
220
150
0.4
180
180
lamp_on
1
0
0
0
0

[LOD]
0

[mesh]
stub.o3d
";

/// Lit between 18:00 and 06:00; the hand turns 10 degrees per simulated second.
pub(super) const SCRIPT: &str = "\
{frame}
    (L.S.Time) 64800 >= (L.S.Time) 21600 < || (S.L.lamp_on)
    (L.S.Timegap) 10 * (L.L.hand_deg) + (S.L.hand_deg)
{end}
";
