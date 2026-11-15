//! The scene the script-effect tests build: the boxes, the scripted objects and the broken ones.

use std::path::Path;

use crate::common::{add_anchor, add_box, add_sco, add_script};

const HEAD: &str = "[groups]\n1\nTest\n\n[friendlyname]\nt\n\n";

/// The anchor and the boxes the scripted objects are built from.
fn add_boxes(root: &Path) {
    add_anchor(root);
    add_box(
        root,
        "base",
        [-4.0, 0.0, -4.0],
        [4.0, 3.0, 4.0],
        [150, 150, 150],
    );
    add_box(
        root,
        "flag",
        [-1.0, 3.0, -1.0],
        [1.0, 6.0, 1.0],
        [220, 40, 40],
    );
    add_box(
        root,
        "pole",
        [-0.5, 0.0, -3.0],
        [0.5, 8.0, -2.0],
        [90, 90, 90],
    );
    add_box(
        root,
        "red",
        [-0.8, 6.0, -1.0],
        [0.8, 7.5, 1.0],
        [255, 30, 30],
    );
    add_box(
        root,
        "yellow",
        [-0.8, 4.0, -1.0],
        [0.8, 5.5, 1.0],
        [250, 220, 30],
    );
    add_box(
        root,
        "green",
        [-0.8, 2.0, -1.0],
        [0.8, 3.5, 1.0],
        [30, 220, 60],
    );
}

/// The toggle and slider objects with their scripts.
fn add_toggle_and_slider(root: &Path) {
    add_sco(
        root,
        "toggle",
        &format!(
            "{HEAD}[mesh]\nbase.x\n\n[mesh]\nflag.x\n\n[visible]\nshow\n1\n\n[script]\n1\nscript\\toggle.osc\n"
        ),
    );
    add_script(
        root,
        "toggle.osc",
        "{frame}\n (L.S.Time) 64800 >= (S.L.show)\n{end}\n",
    );
    add_sco(
        root,
        "slider",
        &format!(
            "{HEAD}[mesh]\nbase.x\n\n[mesh]\nflag.x\n\n[newanim]\norigin_trans\n0\n0\n0\nanim_trans\nslide\n4\n\n[script]\n1\nscript\\slide.osc\n"
        ),
    );
    add_script(
        root,
        "slide.osc",
        "{frame}\n (L.S.Month) 3 < (S.L.slide)\n{end}\n",
    );
}

/// The traffic head and the lamp with their scripts.
fn add_head_and_lamp(root: &Path) {
    add_sco(
        root,
        "head",
        &format!(
            "{HEAD}[trafficlight]\n\n[mesh]\npole.x\n\n[mesh]\nred.x\n\n[visible]\nred\n1\n\n\
             [mesh]\nyellow.x\n\n[visible]\nyellow\n1\n\n[mesh]\ngreen.x\n\n[visible]\ngreen\n1\n\n\
             [script]\n1\nscript\\head.osc\n"
        ),
    );
    add_script(
        root,
        "head.osc",
        "{frame}\n (L.L.TrafficLightPhase) 0 = (S.L.red)\n (L.L.TrafficLightPhase) 3 = (S.L.yellow)\n \
         (L.L.TrafficLightPhase) 6 = (S.L.green)\n{end}\n",
    );
    add_sco(
        root,
        "lamp",
        &format!(
            "{HEAD}[mesh]\nbase.x\n\n[light_enh]\n0\n0\n6\n255\n200\n100\n2\nlamp_on\n0\n0\n0\n0\n\n\
             [script]\n1\nscript\\lamp.osc\n"
        ),
    );
    add_script(
        root,
        "lamp.osc",
        "{frame}\n (L.S.Time) 64800 >= (L.S.Time) 21600 < || (S.L.lamp_on)\n{end}\n",
    );
}

/// The objects without a working script: a missing script file and an inert one.
fn add_broken_scripts(root: &Path) {
    add_sco(
        root,
        "missing",
        &format!("{HEAD}[mesh]\nbase.x\n\n[script]\n1\nscript\\nothere.osc\n"),
    );
    add_sco(
        root,
        "inert",
        &format!(
            "{HEAD}[mesh]\nbase.x\n\n[mesh]\nflag.x\n\n[visible]\nnever\n1\n\n[script]\n1\nscript\\idle.osc\n"
        ),
    );
    add_script(root, "idle.osc", "{frame}\n 0 (S.L.other)\n{end}\n");
}

pub(crate) fn build_root(root: &Path) {
    add_boxes(root);
    add_toggle_and_slider(root);
    add_head_and_lamp(root);
    add_broken_scripts(root);
}
