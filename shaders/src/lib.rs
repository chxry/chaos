#![no_std]
use spirv_std::spirv;
use spirv_std::glam::{Vec3, Vec4, UVec3};
use shared::{TRAIL_LENGTH, Uniform};

#[spirv(compute(threads(1)))]
pub fn compute(
  #[spirv(global_invocation_id)] id: UVec3,
  #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] points: &mut [Vec3],
  #[spirv(uniform, descriptor_set = 1, binding = 0)] uniform: &Uniform,
) {
  let n = (id.x * TRAIL_LENGTH) as usize;
  for i in 1..TRAIL_LENGTH {
    points[n + (TRAIL_LENGTH - i) as usize] = points[n + (TRAIL_LENGTH - i - 1) as usize];
  }
  let pos = &mut points[n];
  let a = 10.0;
  let b = 28.0;
  let c = 8.0 / 3.0;
  *pos += uniform.delta_time
    * Vec3::new(
      a * (pos.y - pos.x),
      pos.x * (b - pos.z) - pos.y,
      pos.x * pos.y - c * pos.z,
    );
}

#[spirv(vertex)]
pub fn vert(
  #[spirv(vertex_index)] idx: u32,
  #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] points: &mut [Vec3],
  #[spirv(uniform, descriptor_set = 1, binding = 0)] uniform: &Uniform,
  #[spirv(point_size)] out_size: &mut f32,
  #[spirv(position)] out_pos: &mut Vec4,
  out_alpha: &mut f32,
) {
  *out_alpha = (idx % TRAIL_LENGTH) as f32 / TRAIL_LENGTH as f32;
  *out_size = 6.0 * *out_alpha;
  *out_pos = uniform.cam * points[idx as usize].extend(1.0);
}

#[spirv(fragment)]
pub fn frag(alpha: f32, out_color: &mut Vec4) {
  *out_color = Vec4::new(1.0, 1.0, 1.0, alpha);
}
