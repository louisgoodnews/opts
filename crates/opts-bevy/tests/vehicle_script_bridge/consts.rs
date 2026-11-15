//! The fixtures of the bridge test: the bus model, its script and the keyboard configuration.

pub(super) const MODEL: &str = "\
[mesh]
door.o3d

[mouseevent]
bus_doorfront0

[newanim]
origin_trans
0
0
0
anim_trans
door_0
0.8

[interiorlight]
lights_cab
0.7
255
255
255
0
3
2

[spotlight]
0
2
0.8
0
1
-0.3
233
233
255
500
30
70

[spotlight]
0
2
0.8
0
1
-0.3
255
255
233
100
30
70
";

pub(super) const SCRIPT: &str = "\
{init}
    -1 (S.L.Spot_Select)
{end}

{trigger:bus_doorfront0}
    1 (S.L.door_0)
    1 (S.L.lights_cab)
    1 (S.L.Spot_Select)
{end}

{trigger:horn_off}
    0 (S.L.door_0)
{end}
";

pub(super) const KEYBOARD: &[u8] = b"[vehicles]\n[entry]\nbus_doorfront0\n181\n0\n[entry]\nhorn\n35\n1\n";
