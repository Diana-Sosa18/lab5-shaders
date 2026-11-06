mod math;
mod mesh;
mod noise;
mod raster;
mod shaders;

use math::*;
use mesh::Mesh;
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use raster::{raster_mesh, Camera};
use rayon::prelude::*;
use shaders::{Body, Frag, Globals, Object};
use std::time::Instant;

const W: usize = 960;
const H: usize = 720;

#[derive(Clone, Copy, PartialEq)]
enum View {
    System,
    Single(Body),
}

impl View {
    fn name(&self) -> &'static str {
        match self {
            View::System => "sistema",
            View::Single(Body::Sun) => "sol",
            View::Single(Body::Terra) => "terra",
            View::Single(Body::Zephyria) => "zephyria",
            View::Single(Body::Ignis) => "ignis",
            View::Single(Body::Glacies) => "glacies",
            View::Single(Body::Aurelia) => "aurelia",
            View::Single(Body::Moon) => "luna",
            View::Single(Body::Ring) => "anillos",
        }
    }
}

struct Scene {
    cam: Camera,
    light: Vec3,
    objects: Vec<Object>,
    rings: Vec<Object>,
    sun: Option<Object>,
}

fn tilt_of(b: Body) -> f32 {
    match b {
        Body::Terra => 0.41,
        Body::Zephyria => 0.45,
        Body::Glacies => 0.15,
        Body::Aurelia => 0.3,
        _ => 0.12,
    }
}

fn body(b: Body, pos: Vec3, scale: f32, spin: f32) -> Object {
    Object { body: b, pos, scale, spin, tilt: tilt_of(b) }
}

fn ring_of(o: &Object) -> Object {
    Object { body: Body::Ring, spin: 0.0, ..*o }
}

fn build(view: View, t: f32) -> Scene {
    match view {
        View::System => {
            let orbit = |r: f32, phase: f32| {
                let a = phase + t * 1.6 / r.powf(1.5);
                v3(a.cos() * r, 0.0, a.sin() * r)
            };
            let sun = body(Body::Sun, ZERO, 1.6, t * 0.05);
            let terra_pos = orbit(5.2, 1.0);
            let ma = t * 1.4;
            let mut objects = vec![
                sun,
                body(Body::Ignis, orbit(3.2, 0.3), 0.35, t * 0.5),
                body(Body::Terra, terra_pos, 0.5, t * 0.6),
                body(Body::Moon, terra_pos + v3(ma.cos() * 0.95, ma.sin() * 0.15, ma.sin() * 0.95), 0.14, t * 0.2),
                body(Body::Aurelia, orbit(7.2, 4.0), 0.55, t * 0.4),
                body(Body::Glacies, orbit(13.0, 5.4), 0.45, t * 0.35),
            ];
            let gas = body(Body::Zephyria, orbit(9.8, 2.4), 0.85, t * 0.9);
            objects.push(gas);
            Scene {
                cam: Camera::new(v3(0.0, 9.0, 17.5), v3(0.0, -1.0, 0.0), 58.0, W, H),
                light: ZERO,
                rings: vec![ring_of(&gas)],
                objects,
                sun: Some(sun),
            }
        }
        View::Single(b) => {
            let spin = t * 0.25;
            let main = body(b, ZERO, 1.0, spin);
            let mut objects = vec![main];
            let mut rings = Vec::new();
            let dist = match b {
                Body::Zephyria => 6.4,
                Body::Terra => 5.0,
                _ => 3.6,
            };
            if b == Body::Zephyria {
                rings.push(ring_of(&main));
            }
            if b == Body::Terra {
                let a = t * 0.5 + 2.2;
                let mp = rot_z(v3(a.cos() * 2.0, 0.0, a.sin() * 2.0), 0.2);
                objects.push(body(Body::Moon, mp, 0.27, t * 0.1));
            }
            Scene {
                cam: Camera::new(v3(0.0, 0.9, dist), ZERO, 45.0, W, H),
                light: v3(-9.0, 3.0, 6.0),
                objects,
                rings,
                sun: if b == Body::Sun { Some(main) } else { None },
            }
        }
    }
}

#[derive(Clone, Copy)]
struct GPix {
    depth: f32,
    id: i32,
    frag: Frag,
}

fn background() -> Vec<Vec3> {
    (0..W * H)
        .into_par_iter()
        .map(|i| {
            let (x, y) = ((i % W) as f32, (i / W) as f32);
            let nx = x / W as f32 * 3.0;
            let ny = y / H as f32 * 2.2;
            let neb = noise::fbm(v3(nx, ny, 0.5), 5);
            let neb2 = noise::fbm(v3(nx * 2.0 + 5.0, ny * 2.0, 1.5), 4);
            let mut col = v3(0.12, 0.03, 0.18) * smoothstep(0.45, 0.8, neb)
                + v3(0.02, 0.08, 0.18) * smoothstep(0.5, 0.85, neb2);
            let h = noise::hash((i % W) as i32, (i / W) as i32, 7);
            if h > 0.9975 {
                let b = (h - 0.9975) / 0.0025;
                let tint = noise::hash((i % W) as i32, (i / W) as i32, 13);
                col += mix(v3(0.7, 0.8, 1.0), v3(1.0, 0.85, 0.7), tint) * (0.3 + 0.7 * b);
            }
            col
        })
        .collect()
}

fn corona(x: f32, y: f32, cx: f32, cy: f32, r: f32, t: f32) -> Vec3 {
    let dx = x - cx;
    let dy = y - cy;
    let d = (dx * dx + dy * dy).sqrt() / r;
    if d < 1.0 {
        return ZERO;
    }
    let ang = dy.atan2(dx);
    let flick = noise::fbm(v3(ang.cos() * 3.0, ang.sin() * 3.0, t * 0.3), 4);
    let rays = noise::noise(v3(ang * 9.0, t * 0.2, 0.0)).powf(3.0);
    let glow = (-(d - 1.0) * 2.2).exp() * 0.8 + (-(d - 1.0) * 8.0).exp() * 0.9;
    v3(1.0, 0.55, 0.15) * (glow * (0.5 + 0.9 * flick) + rays * (-(d - 1.0) * 1.2).exp() * 0.5)
}

fn render(scene: &Scene, t: f32, bg: &[Vec3], sphere: &Mesh, ring: &Mesh) -> Vec<u32> {
    let g = Globals { time: t, light: scene.light, eye: scene.cam.eye };
    let mut gbuf = vec![GPix { depth: f32::INFINITY, id: -1, frag: Frag::default() }; W * H];
    for (id, o) in scene.objects.iter().enumerate() {
        raster_mesh(sphere, o, &scene.cam, |x, y, z, frag| {
            let px = &mut gbuf[y * W + x];
            if z < px.depth {
                *px = GPix { depth: z, id: id as i32, frag };
            }
        });
    }
    let sun_screen = scene.sun.map(|s| {
        let (cx, cy, _) = scene.cam.project(s.pos);
        let (ex, ey, _) = scene.cam.project(s.pos + scene.cam.up * s.scale);
        (cx, cy, ((ex - cx).powi(2) + (ey - cy).powi(2)).sqrt())
    });
    let mut color: Vec<Vec3> = gbuf
        .par_iter()
        .enumerate()
        .map(|(i, px)| {
            if px.id >= 0 {
                shaders::shade(&scene.objects[px.id as usize], &px.frag, &g)
            } else {
                let mut c = bg[i];
                if let Some((cx, cy, r)) = sun_screen {
                    c += corona((i % W) as f32 + 0.5, (i / W) as f32 + 0.5, cx, cy, r, t);
                }
                c
            }
        })
        .collect();
    for o in &scene.rings {
        raster_mesh(ring, o, &scene.cam, |x, y, z, frag| {
            let i = y * W + x;
            if z < gbuf[i].depth {
                let (c, a) = shaders::ring_shade(o, &frag, &g);
                color[i] = mix(color[i], c, a);
            }
        });
    }
    color
        .iter()
        .map(|c| {
            let f = |v: f32| (v.clamp(0.0, 1.0) * 255.0) as u32;
            (f(c.x) << 16) | (f(c.y) << 8) | f(c.z)
        })
        .collect()
}

fn save_png(path: &str, buf: &[u32]) {
    let file = std::fs::File::create(path).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), W as u32, H as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().unwrap();
    let bytes: Vec<u8> = buf
        .iter()
        .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8])
        .collect();
    writer.write_image_data(&bytes).unwrap();
}

const VIEWS: [(Key, View); 8] = [
    (Key::Key1, View::Single(Body::Sun)),
    (Key::Key2, View::Single(Body::Terra)),
    (Key::Key3, View::Single(Body::Zephyria)),
    (Key::Key4, View::Single(Body::Ignis)),
    (Key::Key5, View::Single(Body::Glacies)),
    (Key::Key6, View::Single(Body::Aurelia)),
    (Key::Key7, View::Single(Body::Moon)),
    (Key::Key0, View::System),
];

fn main() {
    let sphere = mesh::sphere(64, 128);
    let ring = mesh::annulus(shaders::RING_IN, shaders::RING_OUT, 6, 192);
    let bg = background();

    if std::env::args().any(|a| a == "--capture") {
        std::fs::create_dir_all("screenshots").unwrap();
        for (_, v) in VIEWS {
            let t = 14.0;
            let buf = render(&build(v, t), t, &bg, &sphere, &ring);
            save_png(&format!("screenshots/{}.png", v.name()), &buf);
        }
        return;
    }

    let mut window = Window::new("Lab 5 - Shaders", W, H, WindowOptions::default()).unwrap();
    let mut view = View::System;
    let mut t = 0.0f32;
    let mut speed = 1.0f32;
    let mut paused = false;
    let mut last = Instant::now();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();
        if !paused {
            t += dt * speed;
        }
        for (k, v) in VIEWS {
            if window.is_key_pressed(k, KeyRepeat::No) {
                view = v;
            }
        }
        if window.is_key_pressed(Key::Space, KeyRepeat::No) {
            paused = !paused;
        }
        if window.is_key_pressed(Key::Up, KeyRepeat::No) {
            speed *= 2.0;
        }
        if window.is_key_pressed(Key::Down, KeyRepeat::No) {
            speed *= 0.5;
        }
        let buf = render(&build(view, t), t, &bg, &sphere, &ring);
        window.set_title(&format!("Lab 5 - Shaders | {} | {:.0} fps", view.name(), 1.0 / dt.max(1e-4)));
        window.update_with_buffer(&buf, W, H).unwrap();
    }
}
