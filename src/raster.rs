use crate::math::*;
use crate::mesh::Mesh;
use crate::shaders::{Frag, Object};

pub struct Camera {
    pub eye: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub fwd: Vec3,
    pub f: f32,
    pub w: f32,
    pub h: f32,
}

impl Camera {
    pub fn new(eye: Vec3, target: Vec3, fov_deg: f32, w: usize, h: usize) -> Camera {
        let fwd = (target - eye).normalize();
        let right = fwd.cross(v3(0.0, 1.0, 0.0)).normalize();
        let up = right.cross(fwd);
        Camera {
            eye,
            right,
            up,
            fwd,
            f: 1.0 / (fov_deg.to_radians() * 0.5).tan(),
            w: w as f32,
            h: h as f32,
        }
    }

    pub fn project(&self, p: Vec3) -> (f32, f32, f32) {
        let d = p - self.eye;
        let x = d.dot(self.right);
        let y = d.dot(self.up);
        let z = d.dot(self.fwd);
        let aspect = self.w / self.h;
        let sx = (x / z * self.f / aspect * 0.5 + 0.5) * self.w;
        let sy = (0.5 - y / z * self.f * 0.5) * self.h;
        (sx, sy, z)
    }
}

fn edge(ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32) -> f32 {
    (px - ax) * (by - ay) - (py - ay) * (bx - ax)
}

pub fn raster_mesh<F: FnMut(usize, usize, f32, Frag)>(mesh: &Mesh, o: &Object, cam: &Camera, mut emit: F) {
    let tv: Vec<(f32, f32, f32, Vec3, Vec3, Vec3)> = mesh
        .verts
        .iter()
        .map(|vx| {
            let wp = o.pos + rot_z(rot_y(vx.p * o.scale, o.spin), o.tilt);
            let wn = rot_z(rot_y(vx.n, o.spin), o.tilt);
            let (sx, sy, z) = cam.project(wp);
            (sx, sy, z, vx.p, wn, wp)
        })
        .collect();
    let wmax = cam.w - 1.0;
    let hmax = cam.h - 1.0;
    for tri in &mesh.tris {
        let a = &tv[tri[0] as usize];
        let b = &tv[tri[1] as usize];
        let c = &tv[tri[2] as usize];
        if a.2 < 0.05 || b.2 < 0.05 || c.2 < 0.05 {
            continue;
        }
        let area = edge(a.0, a.1, b.0, b.1, c.0, c.1);
        if area.abs() < 1e-9 {
            continue;
        }
        let minx = a.0.min(b.0).min(c.0).floor().max(0.0);
        let maxx = a.0.max(b.0).max(c.0).ceil().min(wmax);
        let miny = a.1.min(b.1).min(c.1).floor().max(0.0);
        let maxy = a.1.max(b.1).max(c.1).ceil().min(hmax);
        if minx > maxx || miny > maxy {
            continue;
        }
        for y in miny as i32..=maxy as i32 {
            let py = y as f32 + 0.5;
            for x in minx as i32..=maxx as i32 {
                let px = x as f32 + 0.5;
                let w0 = edge(b.0, b.1, c.0, c.1, px, py) / area;
                let w1 = edge(c.0, c.1, a.0, a.1, px, py) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let i0 = w0 / a.2;
                let i1 = w1 / b.2;
                let i2 = w2 / c.2;
                let z = 1.0 / (i0 + i1 + i2);
                let (k0, k1, k2) = (i0 * z, i1 * z, i2 * z);
                let frag = Frag {
                    obj: a.3 * k0 + b.3 * k1 + c.3 * k2,
                    n: (a.4 * k0 + b.4 * k1 + c.4 * k2).normalize(),
                    w: a.5 * k0 + b.5 * k1 + c.5 * k2,
                };
                emit(x as usize, y as usize, z, frag);
            }
        }
    }
}
