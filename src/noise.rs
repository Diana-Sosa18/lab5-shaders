use crate::math::*;

pub fn hash(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(73856093)
        ^ (y as u32).wrapping_mul(19349663)
        ^ (z as u32).wrapping_mul(83492791);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    h = h.wrapping_mul(2246822519);
    h ^= h >> 13;
    (h & 0x00ff_ffff) as f32 / 16777215.0
}

pub fn noise(p: Vec3) -> f32 {
    let (xf, yf, zf) = (p.x.floor(), p.y.floor(), p.z.floor());
    let (fx, fy, fz) = (p.x - xf, p.y - yf, p.z - zf);
    let (x, y, z) = (xf as i32, yf as i32, zf as i32);
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);
    let uz = fz * fz * (3.0 - 2.0 * fz);
    let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let x00 = l(hash(x, y, z), hash(x + 1, y, z), ux);
    let x10 = l(hash(x, y + 1, z), hash(x + 1, y + 1, z), ux);
    let x01 = l(hash(x, y, z + 1), hash(x + 1, y, z + 1), ux);
    let x11 = l(hash(x, y + 1, z + 1), hash(x + 1, y + 1, z + 1), ux);
    l(l(x00, x10, uy), l(x01, x11, uy), uz)
}

pub fn fbm(mut p: Vec3, octaves: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut total = 0.0;
    for _ in 0..octaves {
        sum += noise(p) * amp;
        total += amp;
        amp *= 0.5;
        p = p * 2.03 + v3(1.7, 9.2, 3.1);
    }
    sum / total
}

pub fn voronoi(p: Vec3) -> (f32, f32, f32) {
    let (xf, yf, zf) = (p.x.floor(), p.y.floor(), p.z.floor());
    let (x, y, z) = (xf as i32, yf as i32, zf as i32);
    let mut f1 = 9.0f32;
    let mut f2 = 9.0f32;
    let mut id = 0.0;
    for k in -1..=1 {
        for j in -1..=1 {
            for i in -1..=1 {
                let (cx, cy, cz) = (x + i, y + j, z + k);
                let fp = v3(
                    cx as f32 + hash(cx, cy, cz),
                    cy as f32 + hash(cx + 31, cy + 17, cz + 7),
                    cz as f32 + hash(cx + 59, cy + 3, cz + 101),
                );
                let d = (fp - p).length();
                if d < f1 {
                    f2 = f1;
                    f1 = d;
                    id = hash(cx + 211, cy + 97, cz + 43);
                } else if d < f2 {
                    f2 = d;
                }
            }
        }
    }
    (f1, f2, id)
}
