//! Scalar BattleCamera.Playing projection for damage callbacks, not rendering.
//! BP_ActionCommandBase starts an enabled ability camera before its damage
//! timeline and finishes it in post-action. The caller supplies that command's
//! imported UseAbilityCamera flag; overlapping Hired Help children share the
//! break camera here and do not enable ability cameras.

pub(super) struct HitCamera {
    break_busy_until: f64,
}

impl HitCamera {
    pub(super) fn new() -> Self {
        Self {
            break_busy_until: f64::NEG_INFINITY,
        }
    }

    pub(super) fn playing(&self, tick: f64, ability_camera: bool) -> bool {
        // Action events run before the appended camera sequence's OnStop at
        // the same tick. The retail Guards capture exercises this equality.
        ability_camera || tick <= self.break_busy_until
    }

    pub(super) fn on_break(&mut self, tick: f64, ability_camera: bool) {
        // PlayBreakCamera skips StartBreakCamera while Playing, but its pushed
        // continuation STILL schedules the separate break timer. The caller
        // owns that timer and must not gate it on this camera-start condition.
        if !self.playing(tick, ability_camera) {
            self.break_busy_until = tick + f64::from(crate::opening_crt::data().break_busy_ticks);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn break_camera_does_not_restart_and_is_busy_at_its_endpoint() {
        let mut camera = HitCamera::new();
        let duration = f64::from(crate::opening_crt::data().break_busy_ticks);
        assert!(!camera.playing(0.0, false));
        camera.on_break(100.0, false);
        camera.on_break(101.0, false);
        assert!(camera.playing(100.0 + duration, false));
        assert!(!camera.playing(101.0 + duration, false));
    }

    #[test]
    fn ability_camera_prevents_starting_a_nested_break_camera() {
        let mut camera = HitCamera::new();
        camera.on_break(100.0, true);
        assert!(camera.playing(100_000.0, true));
        // No break camera was started under the ability camera, so it cannot
        // leak into the next command after the ability camera finishes.
        assert!(!camera.playing(101.0, false));
    }
}
