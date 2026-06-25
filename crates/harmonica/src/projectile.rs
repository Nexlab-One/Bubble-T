//! Simple physics projectile motion.
//!
//! Port of [`charmbracelet/harmonica`]'s projectile solver. Construct once with
//! [`new_projectile`], then call [`Projectile::update`] each frame.
//!
//! [`charmbracelet/harmonica`]: https://github.com/charmbracelet/harmonica

/// A point in 2D/3D space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
    /// Z coordinate.
    pub z: f64,
}

impl Point {
    /// Creates a point at `(x, y, z)`.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

/// A vector (magnitude and direction from the origin).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector {
    /// X component.
    pub x: f64,
    /// Y component.
    pub y: f64,
    /// Z component.
    pub z: f64,
}

impl Vector {
    /// Creates a vector `(x, y, z)`.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

/// Standard gravity for coordinate planes with origin at bottom-left.
pub const GRAVITY: Vector = Vector {
    x: 0.0,
    y: -9.81,
    z: 0.0,
};

/// Gravity for terminal-style coordinates (origin at top-left).
pub const TERMINAL_GRAVITY: Vector = Vector {
    x: 0.0,
    y: 9.81,
    z: 0.0,
};

/// A projectile with position, velocity, and acceleration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projectile {
    pos: Point,
    vel: Vector,
    acc: Vector,
    delta_time: f64,
}

/// Creates a projectile from frame delta and initial kinematic state.
pub fn new_projectile(
    delta_time: f64,
    initial_position: Point,
    initial_velocity: Vector,
    initial_acceleration: Vector,
) -> Projectile {
    Projectile {
        pos: initial_position,
        vel: initial_velocity,
        acc: initial_acceleration,
        delta_time,
    }
}

impl Projectile {
    /// Advances one frame and returns the new position.
    pub fn update(&mut self) -> Point {
        self.pos.x += self.vel.x * self.delta_time;
        self.pos.y += self.vel.y * self.delta_time;
        self.pos.z += self.vel.z * self.delta_time;

        self.vel.x += self.acc.x * self.delta_time;
        self.vel.y += self.acc.y * self.delta_time;
        self.vel.z += self.acc.z * self.delta_time;

        self.pos
    }

    /// Returns the current position.
    pub fn position(self) -> Point {
        self.pos
    }

    /// Returns the current velocity.
    pub fn velocity(self) -> Vector {
        self.vel
    }

    /// Returns the current acceleration.
    pub fn acceleration(self) -> Vector {
        self.acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projectile_moves_under_gravity() {
        let mut p = new_projectile(
            crate::fps(60),
            Point::new(0.0, 100.0, 0.0),
            Vector::new(10.0, 0.0, 0.0),
            GRAVITY,
        );
        let start_y = p.position().y;
        p.update();
        p.update();
        let pos = p.position();
        assert!(pos.x > 0.0);
        assert!(pos.y < start_y);
    }

    #[test]
    fn velocity_increases_with_acceleration() {
        let mut p = new_projectile(
            1.0,
            Point::new(0.0, 0.0, 0.0),
            Vector::new(0.0, 0.0, 0.0),
            Vector::new(1.0, 0.0, 0.0),
        );
        p.update();
        assert_eq!(p.velocity().x, 1.0);
    }
}
