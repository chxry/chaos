#![no_std]
use glam::Mat4;

pub const PARTICLES: u32 = 50;
pub const TRAIL_LENGTH: u32 = 500;

#[repr(C, align(16))]
#[derive(Default)]
pub struct Uniform {
  pub cam: Mat4,
  pub delta_time: f32,
}
