//! Damped harmonic oscillator spring solver.

/// Machine epsilon used for damping-category comparisons.
const EPSILON: f64 = f64::EPSILON;

/// Returns a time delta for `frames_per_second`.
pub fn fps(frames_per_second: u32) -> f64 {
    1.0 / f64::from(frames_per_second)
}

/// A damped harmonic oscillator that drives a value toward a target over time.
///
/// Construct with [`new_spring`] once, then call [`Spring::update`] each frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring {
    pos_pos_coef: f64,
    pos_vel_coef: f64,
    vel_pos_coef: f64,
    vel_vel_coef: f64,
}

/// Creates a spring from frame delta, angular frequency, and damping ratio.
pub fn new_spring(delta_time: f64, angular_frequency: f64, damping_ratio: f64) -> Spring {
    let angular_frequency = angular_frequency.max(0.0);
    let damping_ratio = damping_ratio.max(0.0);

    if angular_frequency < EPSILON {
        return Spring {
            pos_pos_coef: 1.0,
            pos_vel_coef: 0.0,
            vel_pos_coef: 0.0,
            vel_vel_coef: 1.0,
        };
    }

    if damping_ratio > 1.0 + EPSILON {
        // Over-damped.
        let za = -angular_frequency * damping_ratio;
        let zb = angular_frequency * (damping_ratio * damping_ratio - 1.0).sqrt();
        let z1 = za - zb;
        let z2 = za + zb;

        let e1 = (z1 * delta_time).exp();
        let e2 = (z2 * delta_time).exp();

        let inv_two_zb = 1.0 / (2.0 * zb);
        let e1_over_two_zb = e1 * inv_two_zb;
        let e2_over_two_zb = e2 * inv_two_zb;
        let z1e1_over_two_zb = z1 * e1_over_two_zb;
        let z2e2_over_two_zb = z2 * e2_over_two_zb;

        Spring {
            pos_pos_coef: e1_over_two_zb * z2 - z2e2_over_two_zb + e2,
            pos_vel_coef: -e1_over_two_zb + e2_over_two_zb,
            vel_pos_coef: (z1e1_over_two_zb - z2e2_over_two_zb + e2) * z2,
            vel_vel_coef: -z1e1_over_two_zb + z2e2_over_two_zb,
        }
    } else if damping_ratio < 1.0 - EPSILON {
        // Under-damped.
        let omega_zeta = angular_frequency * damping_ratio;
        let alpha = angular_frequency * (1.0 - damping_ratio * damping_ratio).sqrt();

        let exp_term = (-omega_zeta * delta_time).exp();
        let cos_term = (alpha * delta_time).cos();
        let sin_term = (alpha * delta_time).sin();

        let inv_alpha = 1.0 / alpha;
        let exp_sin = exp_term * sin_term;
        let exp_cos = exp_term * cos_term;
        let exp_omega_zeta_sin_over_alpha = exp_term * omega_zeta * sin_term * inv_alpha;

        Spring {
            pos_pos_coef: exp_cos + exp_omega_zeta_sin_over_alpha,
            pos_vel_coef: exp_sin * inv_alpha,
            vel_pos_coef: -exp_sin * alpha - omega_zeta * exp_omega_zeta_sin_over_alpha,
            vel_vel_coef: exp_cos - exp_omega_zeta_sin_over_alpha,
        }
    } else {
        // Critically damped.
        let exp_term = (-angular_frequency * delta_time).exp();
        let time_exp = delta_time * exp_term;
        let time_exp_freq = time_exp * angular_frequency;

        Spring {
            pos_pos_coef: time_exp_freq + exp_term,
            pos_vel_coef: time_exp,
            vel_pos_coef: -angular_frequency * time_exp_freq,
            vel_vel_coef: -time_exp_freq + exp_term,
        }
    }
}

impl Spring {
    /// Updates `pos` and `vel` toward `equilibrium_pos`.
    pub fn update(&self, pos: f64, vel: f64, equilibrium_pos: f64) -> (f64, f64) {
        let old_pos = pos - equilibrium_pos;
        let old_vel = vel;

        let new_pos = old_pos * self.pos_pos_coef + old_vel * self.pos_vel_coef + equilibrium_pos;
        let new_vel = old_pos * self.vel_pos_coef + old_vel * self.vel_vel_coef;

        (new_pos, new_vel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_step_moves_toward_target() {
        let spring = new_spring(fps(60), 6.0, 0.2);
        let (pos, vel) = spring.update(0.0, 0.0, 100.0);
        assert!(pos > 0.0);
        assert!(pos < 100.0);
        assert!(vel > 0.0);
    }

    #[test]
    fn zero_frequency_is_identity() {
        let spring = new_spring(fps(60), 0.0, 0.5);
        let (pos, vel) = spring.update(5.0, 2.0, 100.0);
        assert_eq!(pos, 5.0);
        assert_eq!(vel, 2.0);
    }

    #[test]
    fn converges_near_equilibrium() {
        let spring = new_spring(fps(60), 8.0, 1.0);
        let mut pos = 0.0;
        let mut vel = 0.0;
        for _ in 0..120 {
            (pos, vel) = spring.update(pos, vel, 50.0);
        }
        assert!((pos - 50.0).abs() < 0.01);
        assert!(vel.abs() < 0.01);
    }

    #[test]
    fn fps_matches_period() {
        let dt = fps(60);
        assert!((dt - 1.0 / 60.0).abs() < 1e-9);
    }
}
