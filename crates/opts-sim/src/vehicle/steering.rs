//! Steering response of the driven vehicle (ISS-057): the normalised wheel input passes a curve
//! before it becomes the central road wheel angle, and each steering decision is traced.

use tracing::trace;

/// Central road wheel angle at full wheel input (rad, about 47 degrees): the lock of the steered
/// axles, the clamp of the central angle in [`super::axle::AxleConfig`].
pub(super) const CENTRAL_LOCK_RAD: f32 = 0.82;

/// Share of the cubic term in the steering curve (ISS-057). The curve is
/// `f(x) = (1 - k) * x + k * x^3`: at full input it is exactly 1, so the full lock and the steady
/// turn at lock are unchanged; near centre its slope is `1 - k`, so a small input steers less.
pub(super) const STEER_CUBIC_SHARE: f32 = 0.5;

/// Maps the normalised wheel input `[-1, 1]` to the normalised central angle `[-1, 1]`.
pub(super) fn steering_curve(input: f32) -> f32 {
    let x = input.clamp(-1.0, 1.0);
    (1.0 - STEER_CUBIC_SHARE) * x + STEER_CUBIC_SHARE * x * x * x
}

/// Traces one steering decision at trace level (per frame, target `opts_sim::steering`): the
/// central angle that the axles get, the lock it is clamped to, and the lateral acceleration.
pub(super) fn trace_steering(input: f32, central_rad: f32, speed_m_s: f32, yaw_rate_rad_s: f32) {
    let clamped = central_rad.abs() >= CENTRAL_LOCK_RAD;
    let lat_accel_m_s2 = speed_m_s * yaw_rate_rad_s;
    trace!(
        target: "opts_sim::steering",
        input,
        central_rad,
        lock_rad = CENTRAL_LOCK_RAD,
        clamped,
        speed_m_s,
        yaw_rate_rad_s,
        lat_accel_m_s2,
        "steering decision"
    );
}
