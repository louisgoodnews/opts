//! The start and the end of a trailer chain: the head's state and what it receives.

use super::follower::ChainState;
use super::*;

/// The state with only the head in it.
pub(super) fn new_state(root: Entity, tractor: &BusManifest) -> ChainState {
    // Rail: the head's mass (at least 20 t), then the stated mass of every following unit.
    let head_kg = commands_mass(tractor);
    ChainState {
        cams: Vec::new(),
        driver_cams: Vec::new(),
        previous_kg: manifest_mass_kg(tractor).unwrap_or(HEAD_MASS_DEFAULT_KG),
        chain: Vec::new(),
        layout: Vec::new(),
        owner: root,
        consist: vec![opts_bevy::ConsistUnit {
            part: None,
            head: true,
            cables: super::super::scripts::cables_of(tractor),
            turned: false,
        }],
        axle_base: tractor.axles.len(),
        powered_kg: head_kg,
        total_kg: head_kg,
    }
}

/// Puts the chain's results on the head: its script consist, rail masses and composition.
pub(super) fn finish_chain(
    commands: &mut Commands,
    root: Entity,
    tractor: &BusManifest,
    rail: bool,
    st: &mut ChainState,
) {
    let consist = std::mem::take(&mut st.consist);
    let chain = std::mem::take(&mut st.chain);
    let layout = std::mem::take(&mut st.layout);
    if consist.iter().any(|u| u.part.is_some()) || consist.iter().any(|u| !u.cables.is_empty()) {
        commands
            .entity(root)
            .insert(opts_bevy::ScriptConsist { units: consist });
    }
    if rail {
        // The whole consist's masses for the dynamics (set once the head has its `RailPlayer`).
        let load = opts_sim::rail::ConsistLoad {
            total_kg: st.total_kg,
            powered_kg: st.powered_kg,
            braked_kg: st.total_kg,
        };
        commands.queue(move |world: &mut World| {
            if let Some(mut player) = world.get_mut::<super::super::super::rail::RailPlayer>(root) {
                player.set_consist(load);
            }
        });
    }
    if !chain.is_empty() {
        if !rail {
            commands
                .entity(root)
                .insert(opts_bevy::ArticulatedChain { segments: chain });
        }
        commands
            .entity(root)
            .insert((super::super::super::rail::VehicleComposition {
                head: head_part(root, tractor),
                followers: layout,
            },));
    }
}

/// The head's part of the composition.
fn head_part(root: Entity, tractor: &BusManifest) -> super::super::super::rail::Part {
    super::super::super::rail::Part {
        entity: root,
        name: tractor
            .friendly_name
            .as_ref()
            .map(|f| f.model.clone())
            .unwrap_or_default(),
        behind: 0.0,
        half_span: tractor.bogie_half_span.unwrap_or(0.0),
        reversed: false,
        meshes: 0,
        vertices: 0,
        body: tractor
            .bounding_box
            .map(|(size, centre)| super::super::super::BodyBox::from_bounding_box(size, centre)),
    }
}
