//! The light program a placed traffic-light object runs.

use super::*;

impl WorldLoadJob {
    /// The light program behind `TrafficLightPhase` of `obj`: its `[varparent]` junction's
    /// program, else its own first program.
    pub(super) fn signal_for(
        &mut self,
        obj: &MapObjectInstance,
        sco: &SceneryObject,
    ) -> Option<SignalProgram> {
        let program_of = |s: &SceneryObject, i: usize| {
            let (g, p) = s.traffic_light(i)?;
            let phases = p
                .phases
                .iter()
                .map(|ph| (ph.state, f64::from(ph.duration_sec)))
                .collect();
            SignalProgram::in_period(
                phases,
                f64::from(g.period_sec()),
                f64::from(p.approach_distance_m.unwrap_or(0.0)),
            )
        };
        let Some(&(parent, index)) = self.scenery.links.get(&obj.id_code) else {
            return program_of(sco, 0);
        };
        let linked = self
            .scenery
            .objects
            .get(&parent)
            .cloned()
            .and_then(|parent_sco| self.sco(&parent_sco))
            .and_then(|(_, s)| program_of(&s, index));
        if linked.is_some() {
            self.scenery.stats.signals_linked += 1;
        } else {
            self.scenery.stats.signals_unlinked += 1;
            tracing::debug!(
                object = obj.id_code,
                parent,
                index,
                "signal head: junction program not found"
            );
        }
        linked
    }
}

