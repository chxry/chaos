#![no_std]
use spirv_std::spirv;
use spirv_std::glam::{Vec3, Vec4, UVec3};
use shared::{TRAIL_LENGTH, Uniform};

#[spirv(compute(threads(1)))]
pub fn compute(
  #[spirv(global_invocation_id)] id: UVec3,
  #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] points: &mut [Vec4],
  #[spirv(uniform, descriptor_set = 1, binding = 0)] uniform: &Uniform,
) {
  let n = id.x * TRAIL_LENGTH;
  for i in 0..TRAIL_LENGTH - 1 {
    points[(n + i) as usize] = points[(n + i + 1) as usize];
  }

  let pos = &mut points[(n + TRAIL_LENGTH - 1) as usize];
  let a = 10.0;
  let b = 28.0;
  let c = 8.0 / 3.0;
  let delta = Vec3::new(
    a * (pos.y - pos.x),
    pos.x * (b - pos.z) - pos.y,
    pos.x * pos.y - c * pos.z,
  );
  *pos = (pos.truncate() + uniform.delta_time * 0.1 * delta).extend(delta.length());
}

#[spirv(vertex)]
pub fn vert(
  #[spirv(vertex_index)] idx: u32,
  #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] points: &mut [Vec4],
  #[spirv(uniform, descriptor_set = 1, binding = 0)] uniform: &Uniform,
  #[spirv(point_size)] out_size: &mut f32,
  #[spirv(position)] out_pos: &mut Vec4,
  out_alpha: &mut f32,
  out_vel: &mut f32,
) {
  let local_pos = (idx % TRAIL_LENGTH) as f32 / TRAIL_LENGTH as f32;
  *out_size = 5.0 * local_pos;
  *out_pos = uniform.cam * points[idx as usize].truncate().extend(1.0);
  *out_alpha = local_pos;
  *out_vel = points[idx as usize].w;
}

#[spirv(fragment)]
pub fn frag(alpha: f32, vel: f32, out_color: &mut Vec4) {
  *out_color = Vec4::new(1.0, vel / 150.0, 1.0, alpha);
}
