//! Weather alerts (M-0117): a warning in the alert slot when the road has little grip or the
//! visibility falls below the hint threshold. Both follow the values the HUD samples each frame.

use super::*;
use opts_sim::vehicle::ROAD_FRICTION;
use opts_traffic::weather::is_low_visibility;

/// Grip factor (friction over dry asphalt) below which the road counts as slippery.
pub const LOW_GRIP_FACTOR: f32 = 0.8;

/// Whether the road friction `mu` (the vehicle's `road_friction`) is low.
#[must_use]
pub fn grip_is_low(mu: f32) -> bool {
    mu / ROAD_FRICTION < LOW_GRIP_FACTOR
}

fn toggle(
    id: &'static str,
    key: &'static str,
    on: bool,
    raise: &mut MessageWriter<RaiseAlert>,
    clear: &mut MessageWriter<ClearAlert>,
) {
    if on {
        raise.write(RaiseAlert {
            id,
            severity: AlertSeverity::Warning,
            key,
            args: Vec::new(),
        });
    } else {
        clear.write(ClearAlert(id));
    }
}

/// Raises or clears the low-grip and low-visibility alerts when their state changes.
pub fn weather_alerts(
    values: Res<HudValues>,
    mut raise: MessageWriter<RaiseAlert>,
    mut clear: MessageWriter<ClearAlert>,
    mut last: Local<(bool, bool)>,
) {
    let grip = values.has_vehicle && grip_is_low(values.road_friction);
    let visibility = values.has_vehicle && is_low_visibility(f64::from(values.visibility_m));
    if grip != last.0 {
        toggle(GRIP_ID, keys::ALERT_LOW_GRIP, grip, &mut raise, &mut clear);
        last.0 = grip;
    }
    if visibility != last.1 {
        toggle(VISIBILITY_ID, keys::ALERT_LOW_VISIBILITY, visibility, &mut raise, &mut clear);
        last.1 = visibility;
    }
}

/// Alert id of the low-grip warning.
pub const GRIP_ID: &str = "weather.grip";
/// Alert id of the low-visibility warning.
pub const VISIBILITY_ID: &str = "weather.visibility";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wet_and_snowy_roads_count_as_low_grip_and_dry_does_not() {
        assert!(!grip_is_low(ROAD_FRICTION));
        assert!(grip_is_low(0.55));
        assert!(grip_is_low(0.25));
    }

    fn visible(app: &App) -> Option<&'static str> {
        app.world()
            .resource::<AlertQueue>()
            .visible()
            .map(|a| a.id)
    }

    #[test]
    fn low_grip_and_low_visibility_raise_and_clear_their_alerts() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .add_message::<RaiseAlert>()
            .add_message::<ClearAlert>()
            .init_resource::<AlertQueue>()
            .insert_resource(HudValues {
                has_vehicle: true,
                ..HudValues::default()
            })
            .add_systems(Update, (weather_alerts, update_queue).chain());
        app.update();
        assert_eq!(visible(&app), None, "dry and clear: no alert");
        {
            let mut v = app.world_mut().resource_mut::<HudValues>();
            v.road_friction = 0.55;
        }
        app.update();
        assert_eq!(visible(&app), Some(GRIP_ID));
        {
            let mut v = app.world_mut().resource_mut::<HudValues>();
            v.road_friction = ROAD_FRICTION;
            v.visibility_m = 80.0;
        }
        app.update();
        assert_eq!(visible(&app), Some(VISIBILITY_ID));
        app.world_mut().resource_mut::<HudValues>().visibility_m = f32::INFINITY;
        app.update();
        assert_eq!(visible(&app), None);
    }
}
