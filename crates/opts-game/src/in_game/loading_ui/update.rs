//! Per-frame update of the loading screen: bar, stage, manifest and outcome box.

use super::*;

/// What the loading screen shows.
#[derive(bevy::ecs::system::SystemParam)]
pub struct LoadingData<'w> {
    state: Res<'w, SimulationLoadState>,
    report: Res<'w, LoadReport>,
    session: Res<'w, InGameSessionInfo>,
    setup: Res<'w, NewGameSetup>,
    weather: Res<'w, opts_bevy::WeatherCatalogue>,
    loc: Res<'w, Localization>,
}

/// The loading screen's nodes that [`update_loading_ui`] updates.
#[derive(bevy::ecs::system::SystemParam)]
pub struct LoadingNodes<'w, 's> {
    fill: Query<'w, 's, &'static mut Node, With<ProgressBarFill>>,
    percent: Query<'w, 's, &'static mut Text, (With<PercentText>, Without<ManifestText>)>,
    stage: Query<'w, 's, (&'static mut L10nKey, &'static mut L10nArgs), With<StageText>>,
    manifest: Query<'w, 's, (&'static ManifestText, &'static mut Text), Without<PercentText>>,
    outcome_box: Query<'w, 's, Entity, With<OutcomeBox>>,
}

/// Keeps bar, percentage, stage sentence, manifest and outcome current.
pub fn update_loading_ui(
    data: LoadingData,
    nodes: LoadingNodes,
    mut shown: Local<Option<OutcomeSig>>,
    mut commands: Commands,
) {
    let LoadingData {
        state,
        report,
        session,
        setup,
        weather,
        loc,
    } = data;
    let LoadingNodes {
        mut fill,
        mut percent,
        mut stage,
        mut manifest,
        outcome_box,
    } = nodes;
    let pct = (state.progress * 100.0).clamp(0.0, 100.0);
    for mut n in &mut fill {
        let w = Val::Percent(pct.max(2.0));
        if n.width != w {
            n.width = w;
        }
    }
    for mut t in &mut percent {
        let s = format!("{pct:.0}%");
        if t.0 != s {
            t.0 = s;
        }
    }
    let (key, args) = stage_message(&state);
    for (mut k, mut a) in &mut stage {
        if k.0 != key {
            k.0 = key;
        }
        if *a != args {
            *a = args.clone();
        }
    }
    let lines = manifest_lines(&loc, &weather, &session, &setup);
    for (m, mut t) in &mut manifest {
        let new = &lines
            .iter()
            .find(|(tile, _)| *tile == m.0)
            .map(|(_, l)| l[usize::from(m.1)].clone());
        if let Some(s) = new {
            if t.0 != *s {
                t.0.clone_from(s);
            }
        }
    }
    let sig = OutcomeSig {
        outcome: report.outcome,
        problems: report.blocking_count(),
        details: report.details_open,
    };
    if shown.as_ref() != Some(&sig) {
        if let Ok(b) = outcome_box.single() {
            commands.entity(b).despawn_related::<Children>();
            build_outcome(&mut commands, b, &report);
        }
        *shown = Some(sig);
    }
}
