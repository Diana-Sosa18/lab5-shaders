use crate::math::*;
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy)]
pub struct Vertex {
    pub p: Vec3,
    pub n: Vec3,
}

pub struct Mesh {
    pub verts: Vec<Vertex>,
    pub tris: Vec<[u32; 3]>,
}

fn grid(verts: Vec<Vertex>, rows: u32, slices: u32) -> Mesh {
    let mut tris = Vec::new();
    let row = slices + 1;
    for i in 0..rows {
        for j in 0..slices {
            let a = i * row + j;
            let b = a + row;
            tris.push([a, b, a + 1]);
            tris.push([a + 1, b, b + 1]);
        }
    }
    Mesh { verts, tris }
}

pub fn sphere(stacks: u32, slices: u32) -> Mesh {
    let mut verts = Vec::new();
    for i in 0..=stacks {
        let phi = PI * i as f32 / stacks as f32;
        for j in 0..=slices {
            let th = TAU * j as f32 / slices as f32;
            let p = v3(phi.sin() * th.cos(), phi.cos(), phi.sin() * th.sin());
            verts.push(Vertex { p, n: p });
        }
    }
    grid(verts, stacks, slices)
}

pub fn annulus(inner: f32, outer: f32, rings: u32, slices: u32) -> Mesh {
    let mut verts = Vec::new();
    for i in 0..=rings {
        let r = inner + (outer - inner) * i as f32 / rings as f32;
        for j in 0..=slices {
            let th = TAU * j as f32 / slices as f32;
            verts.push(Vertex {
                p: v3(r * th.cos(), 0.0, r * th.sin()),
                n: v3(0.0, 1.0, 0.0),
            });
        }
    }
    grid(verts, rings, slices)
}
