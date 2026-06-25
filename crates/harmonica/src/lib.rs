//! Physics-based spring animation.
//!
//! This crate is the Rust port of [`charmbracelet/harmonica`]. It implements a
//! damped-harmonic-oscillator [`Spring`] that advances a value and its velocity
//! toward a target each frame, producing natural-feeling motion. The progress widget
//! uses it to animate its fill.
//!
//! [`charmbracelet/harmonica`]: https://github.com/charmbracelet/harmonica

#![warn(missing_docs)]

mod projectile;
mod spring;

pub use projectile::{GRAVITY, Point, Projectile, TERMINAL_GRAVITY, Vector, new_projectile};
pub use spring::{Spring, fps, new_spring};
