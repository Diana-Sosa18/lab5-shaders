use crate::math::*;
use crate::noise::*;

pub const RING_IN: f32 = 1.35;
pub const RING_OUT: f32 = 2.45;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Body {
    Sun,
    Terra,
    Zephyria,
    Ignis,
    Glacies,
    Aurelia,
    Moon,
    Ring,
}

#[derive(Clone, Copy, Default)]
pub struct Frag {
    pub obj: Vec3,
    pub n: Vec3,
    pub w: Vec3,
}

#[derive(Clone, Copy)]
pub struct Globals {
    pub time: f32,
    pub light: Vec3,
    pub eye: Vec3,
}

#[derive(Clone, Copy)]
pub struct Object {
    pub body: Body,
    pub pos: Vec3,
    pub scale: f32,
    pub spin: f32,
    pub tilt: f32,
}

impl Object {
    pub fn ring_normal(&self) -> Vec3 {
        rot_z(v3(0.0, 1.0, 0.0), self.tilt)
    }
}

struct Ctx {
    p: Vec3,
    n: Vec3,
    l: Vec3,
    v: Vec3,
    ndl: f32,
}

fn ctx(f: &Frag, g: &Globals) -> Ctx {
    let n = f.n.normalize();
    let l = (g.light - f.w).normalize();
    Ctx {
        p: f.obj.normalize(),
        n,
        l,
        v: (g.eye - f.w).normalize(),
        ndl: n.dot(l),
    }
}

fn ramp(stops: &[(f32, Vec3)], t: f32) -> Vec3 {
    if t <= stops[0].0 {
        return stops[0].1;
    }
    for w in stops.windows(2) {
        if t <= w[1].0 {
            let k = (t - w[0].0) / (w[1].0 - w[0].0);
            return mix(w[0].1, w[1].1, k);
        }
    }
    stops[stops.len() - 1].1
}

fn fresnel(c: &Ctx, power: f32) -> f32 {
    (1.0 - c.n.dot(c.v).max(0.0)).powf(power)
}

fn splat(x: f32) -> Vec3 {
    v3(x, x, x)
}

fn sphere_hit(ro: Vec3, rd: Vec3, center: Vec3, r: f32) -> bool {
    let oc = ro - center;
    let b = oc.dot(rd);
    let cc = oc.dot(oc) - r * r;
    let h = b * b - cc;
    h > 0.0 && (-b - h.sqrt()) > 0.0
}

pub fn ring_density(t: f32) -> f32 {
    if !(0.0..=1.0).contains(&t) {
        return 0.0;
    }
    let fine = noise(v3(t * 90.0, 0.5, 0.5));
    let mid = noise(v3(t * 23.0, 3.1, 1.7));
    let mut d = 0.3 + 0.4 * mid + 0.3 * fine;
    d *= 0.75 + 0.25 * (t * 140.0).sin();
    d *= 0.35 + 0.65 * smoothstep(0.1, 0.25, t);
    d *= 1.0 - 0.95 * smoothstep(0.035, 0.0, (t - 0.6).abs());
    d *= 1.0 - 0.9 * smoothstep(0.012, 0.0, (t - 0.87).abs());
    d *= smoothstep(0.0, 0.08, t) * smoothstep(1.0, 0.94, t);
    d.clamp(0.0, 1.0)
}

fn ring_shadow(o: &Object, w: Vec3, l: Vec3) -> f32 {
    let nrm = o.ring_normal();
    let denom = nrm.dot(l);
    if denom.abs() < 1e-4 {
        return 1.0;
    }
    let t = (o.pos - w).dot(nrm) / denom;
    if t <= 0.0 {
        return 1.0;
    }
    let hit = w + l * t;
    let r = (hit - o.pos).length() / o.scale;
    1.0 - ring_density((r - RING_IN) / (RING_OUT - RING_IN)) * 0.75
}

pub fn ring_shade(o: &Object, f: &Frag, g: &Globals) -> (Vec3, f32) {
    let r = (f.obj.x * f.obj.x + f.obj.z * f.obj.z).sqrt();
    let t = (r - RING_IN) / (RING_OUT - RING_IN);
    let d = ring_density(t);
    if d <= 0.001 {
        return (ZERO, 0.0);
    }
    let tint = ramp(
        &[
            (0.0, v3(0.5, 0.47, 0.44)),
            (0.3, v3(0.85, 0.74, 0.58)),
            (0.6, v3(0.92, 0.84, 0.7)),
            (0.8, v3(0.78, 0.66, 0.55)),
            (1.0, v3(0.7, 0.7, 0.72)),
        ],
        t,
    );
    let grain = 0.85 + 0.3 * noise(v3(t * 320.0, 1.0, 2.0));
    let l = (g.light - f.w).normalize();
    let v = (g.eye - f.w).normalize();
    let lit = 0.3 + 0.7 * o.ring_normal().dot(l).abs();
    let shadow = if sphere_hit(f.w, l, o.pos, o.scale) { 0.1 } else { 1.0 };
    let scatter = 1.0 + 0.8 * (-v.dot(l)).max(0.0).powf(6.0);
    (tint * (grain * lit * shadow * scatter), (d * 1.1).min(0.95))
}

fn sun(f: &Frag, g: &Globals) -> Vec3 {
    let c = ctx(f, g);
    let p = c.p;
    let t = g.time;
    let w = fbm(p * 3.0 + v3(t * 0.08, 0.0, t * 0.05), 4);
    let plasma = fbm(p * 5.0 + splat(w) * 2.5 - v3(0.0, t * 0.06, 0.0), 5);
    let (c1, c2, _) = voronoi(p * 45.0 + v3(0.0, t * 0.15, 0.0));
    let gran = smoothstep(0.0, 0.35, c2 - c1);
    let mut heat = plasma * 0.85 + gran * 0.18;
    let spots = fbm(p * 1.6 + v3(50.0, 0.0, t * 0.01), 4);
    let umbra = smoothstep(0.71, 0.75, spots);
    let penumbra = smoothstep(0.66, 0.71, spots);
    heat *= 1.0 - penumbra * 0.35 - umbra * 0.55;
    let fac = fbm(p * 7.0 + v3(0.0, -t * 0.1, 3.0), 3);
    heat += smoothstep(0.62, 0.75, fac) * 0.25 * (1.0 - umbra);
    let col = ramp(
        &[
            (0.0, v3(0.35, 0.03, 0.0)),
            (0.35, v3(0.9, 0.25, 0.02)),
            (0.55, v3(1.0, 0.55, 0.08)),
            (0.75, v3(1.0, 0.85, 0.35)),
            (1.0, v3(1.0, 1.0, 0.85)),
        ],
        heat,
    );
    let mu = c.n.dot(c.v).max(0.0);
    let limb = 0.35 + 0.65 * mu.powf(0.45);
    col * (limb * 1.15) + v3(1.0, 0.4, 0.05) * (fresnel(&c, 4.0) * 0.8)
}

fn terra(f: &Frag, g: &Globals) -> Vec3 {
    let c = ctx(f, g);
    let p = c.p;
    let warp = fbm(p * 1.7 + v3(4.0, 1.0, 7.0), 4);
    let h = fbm(p * 2.3 + splat(warp) * 1.1, 7);
    let lat = p.y.abs();
    let sea = 0.5;
    let detail = fbm(p * 18.0, 3);
    let ocean = h < sea;
    let mut col;
    if ocean {
        let depth = (sea - h) / sea;
        col = mix(v3(0.08, 0.42, 0.55), v3(0.01, 0.06, 0.22), smoothstep(0.0, 0.3, depth));
        col = col * (0.9 + 0.2 * detail);
    } else {
        let e = (h - sea) / (1.0 - sea);
        let moist = fbm(p * 3.1 + v3(20.0, 5.0, 11.0), 4);
        let dry = smoothstep(0.55, 0.38, moist) * (1.0 - smoothstep(0.1, 0.35, (lat - 0.3).abs()));
        let green = mix(v3(0.16, 0.4, 0.1), v3(0.04, 0.2, 0.05), smoothstep(0.4, 0.7, moist));
        let mut low = mix(green, v3(0.8, 0.63, 0.38), dry);
        low = mix(low, v3(0.45, 0.45, 0.36), smoothstep(0.55, 0.75, lat));
        col = mix(v3(0.84, 0.78, 0.56), low, smoothstep(0.0, 0.04, e));
        col = mix(col, v3(0.38, 0.32, 0.27), smoothstep(0.25, 0.45, e));
        col = mix(col, v3(0.95, 0.95, 0.97), smoothstep(0.55, 0.68, e + lat * 0.25));
        col = col * (0.85 + 0.3 * detail);
    }
    let ice = smoothstep(0.8, 0.86, lat + (fbm(p * 6.0, 3) - 0.5) * 0.15);
    col = mix(col, v3(0.92, 0.95, 1.0), ice);
    let diff = (c.ndl * 0.9 + 0.1).max(0.0);
    let mut lit = col * (0.03 + diff);
    if ocean && c.ndl > 0.0 {
        let r = reflect(-c.l, c.n);
        lit += v3(1.0, 0.95, 0.8) * (r.dot(c.v).max(0.0).powf(45.0) * 0.7 * (1.0 - ice));
    }
    let night = smoothstep(0.05, -0.2, c.ndl);
    if !ocean {
        let c1 = noise(p * 55.0);
        let c2 = fbm(p * 9.0 + splat(3.0), 3);
        let city = smoothstep(0.7, 0.9, c1) * smoothstep(0.45, 0.6, c2) * (1.0 - ice);
        lit += v3(1.0, 0.7, 0.3) * (city * night * 1.5);
    }
    let cp = rot_y(p, g.time * 0.04);
    let cw = fbm(cp * 2.0 + v3(0.0, g.time * 0.01, 0.0), 3);
    let cl = fbm(cp * 4.0 + splat(cw) * 1.5, 5);
    let cloud = smoothstep(0.5, 0.72, cl) * 0.95;
    lit = mix(lit, splat(0.04 + diff * 1.05), cloud);
    lit + v3(0.3, 0.55, 1.0) * (fresnel(&c, 3.0) * (0.12 + diff * 1.2))
}

fn zephyria(o: &Object, f: &Frag, g: &Globals) -> Vec3 {
    let c = ctx(f, g);
    let p = c.p;
    let s = (p.y * 9.0).sin() * 0.06 + 0.02;
    let pp = rot_y(p, g.time * s);
    let turb = fbm(v3(pp.x * 3.0, pp.y * 12.0, pp.z * 3.0), 5);
    let fine = fbm(pp * 9.0 + splat(turb) * 2.0, 4);
    let y = p.y + (turb - 0.5) * 0.16 + (fine - 0.5) * 0.04;
    let b1 = (y * 20.0).sin() * 0.5 + 0.5;
    let b2 = (y * 43.0 + 1.3).sin() * 0.5 + 0.5;
    let b3 = (y * 7.0 + 0.4).sin() * 0.5 + 0.5;
    let cream = v3(0.95, 0.88, 0.74);
    let mut col = mix(cream, v3(0.84, 0.6, 0.38), b1);
    col = mix(col, v3(0.52, 0.32, 0.2), smoothstep(0.6, 1.0, b2) * 0.6);
    col = mix(col, v3(0.72, 0.38, 0.22), smoothstep(0.7, 1.0, b3) * 0.35);
    col = col * (0.9 + 0.2 * fine);
    let sp = rot_y(p, g.time * 0.025);
    let lon = sp.z.atan2(sp.x);
    let lat = sp.y.clamp(-1.0, 1.0).asin();
    let dl = wrap_pi(lon - 0.8);
    let dy = lat + 0.38;
    let e = ((dl / 0.38).powi(2) + (dy / 0.16).powi(2)).sqrt();
    if e < 1.3 {
        let ang = dy.atan2(dl * 0.42);
        let sw = (e * 14.0 - ang * 2.0 - g.time * 1.5 + fine * 4.0).sin() * 0.5 + 0.5;
        let sc = mix(v3(0.75, 0.28, 0.15), v3(0.93, 0.55, 0.35), sw);
        let core = mix(sc, v3(0.6, 0.18, 0.1), smoothstep(0.5, 0.0, e));
        col = mix(col, core, smoothstep(1.0, 0.75, e));
        let rim = smoothstep(0.75, 1.0, e) * smoothstep(1.3, 1.0, e);
        col = mix(col, cream * 1.05, rim * 0.6);
    }
    let polar = smoothstep(0.65, 0.95, p.y.abs());
    col = mix(col, v3(0.55, 0.58, 0.65) * (0.8 + 0.3 * fine), polar * 0.7);
    let diff = c.ndl.max(0.0).powf(0.8);
    let sh = if c.ndl > 0.0 { ring_shadow(o, f.w, c.l) } else { 1.0 };
    col * (0.02 + diff * sh) + v3(0.95, 0.75, 0.5) * (fresnel(&c, 2.5) * diff * 0.35)
}

fn ignis(f: &Frag, g: &Globals) -> Vec3 {
    let c = ctx(f, g);
    let p = c.p;
    let t = g.time;
    let wp = p * 3.2 + v3(fbm(p * 2.0, 3), fbm(p * 2.0 + splat(7.0), 3), 0.0) * 0.8;
    let (f1, f2, _) = voronoi(wp);
    let crack = smoothstep(0.12, 0.0, f2 - f1);
    let (g1, g2, _) = voronoi(wp * 3.0);
    let crack2 = smoothstep(0.06, 0.0, g2 - g1) * 0.6;
    let rock = fbm(p * 10.0, 4);
    let crust = mix(v3(0.06, 0.05, 0.05), v3(0.3, 0.23, 0.2), rock) * (0.6 + 0.6 * f1);
    let flow = fbm(p * 2.5 + v3(t * 0.05, -t * 0.03, t * 0.04), 5);
    let sea = smoothstep(0.52, 0.62, flow);
    let pulse = 0.75 + 0.25 * (t * 2.0 + f1 * 12.0).sin();
    let churn = 0.7 + 0.3 * fbm(p * 8.0 + v3(0.0, t * 0.2, 0.0), 3);
    let heat = (crack.max(crack2) * pulse).max(sea * churn);
    let lava = ramp(
        &[
            (0.0, ZERO),
            (0.3, v3(0.5, 0.02, 0.0)),
            (0.6, v3(1.0, 0.25, 0.0)),
            (0.85, v3(1.0, 0.65, 0.1)),
            (1.0, v3(1.0, 0.95, 0.6)),
        ],
        heat,
    );
    let diff = c.ndl.max(0.0);
    let mut col = crust * ((1.0 - sea) * (0.03 + diff)) + lava * 1.3;
    let a = fbm(rot_y(p, t * 0.06) * 3.0, 4);
    let ash = smoothstep(0.55, 0.75, a) * 0.7;
    col = mix(col, v3(0.15, 0.12, 0.11) * (0.05 + diff) + lava * 0.15, ash);
    col + v3(1.0, 0.3, 0.05) * (fresnel(&c, 2.0) * 0.5)
}

fn glacies(f: &Frag, g: &Globals) -> Vec3 {
    let c = ctx(f, g);
    let p = c.p;
    let mut col = mix(v3(0.72, 0.84, 0.95), v3(0.95, 0.97, 1.0), fbm(p * 3.0, 5));
    let blue = fbm(p * 1.5 + v3(9.0, 1.0, 4.0), 4);
    col = mix(col, v3(0.45, 0.65, 0.9), smoothstep(0.55, 0.75, blue) * 0.6);
    let ch = fbm(p * 12.0 + splat(2.0), 3);
    let chaos = smoothstep(0.6, 0.7, fbm(p * 1.2 + v3(30.0, 0.0, 0.0), 3));
    col = mix(col, mix(v3(0.6, 0.48, 0.42), v3(0.86, 0.8, 0.74), ch), chaos * 0.7);
    let r1 = 1.0 - (fbm(p * 2.5 + v3(1.0, 2.0, 3.0), 5) * 2.0 - 1.0).abs();
    let r2 = 1.0 - (fbm(p * 6.0 + v3(5.0, 1.0, 8.0), 4) * 2.0 - 1.0).abs();
    col = mix(col, v3(0.55, 0.28, 0.18), smoothstep(0.93, 0.99, r1) * 0.85);
    col = mix(col, v3(0.2, 0.42, 0.75), smoothstep(0.95, 0.995, r2) * 0.6);
    let diff = c.ndl.max(0.0);
    let mut lit = col * (0.03 + diff);
    if c.ndl > 0.0 {
        let s = reflect(-c.l, c.n).dot(c.v).max(0.0);
        lit += splat(s.powf(30.0) * 0.5);
        let sparkle = smoothstep(0.9, 0.97, noise(p * 400.0));
        lit += splat(sparkle * s.powf(4.0) * 1.5);
    }
    let sss = smoothstep(-0.25, 0.05, c.ndl) * smoothstep(0.3, 0.0, c.ndl);
    lit += v3(0.2, 0.5, 0.9) * (sss * 0.35);
    lit + v3(0.5, 0.85, 1.0) * (fresnel(&c, 3.0) * (0.1 + diff * 0.8))
}

fn aurelia(f: &Frag, g: &Globals) -> Vec3 {
    let c = ctx(f, g);
    let p = c.p;
    let t = g.time;
    let q = v3(
        fbm(p * 2.0 + v3(0.0, t * 0.03, 0.0), 4),
        fbm(p * 2.0 + v3(5.2, 1.3, t * 0.02), 4),
        fbm(p * 2.0 + v3(2.0, 8.0, 1.0), 4),
    );
    let sw = fbm(p * 3.0 + q * 2.5, 5);
    let mut col = ramp(
        &[
            (0.0, v3(0.05, 0.02, 0.15)),
            (0.4, v3(0.2, 0.05, 0.35)),
            (0.6, v3(0.05, 0.35, 0.45)),
            (0.8, v3(0.4, 0.75, 0.7)),
            (1.0, v3(0.9, 0.95, 0.9)),
        ],
        sw,
    );
    let land = smoothstep(0.64, 0.66, fbm(p * 4.0 + v3(40.0, 0.0, 0.0), 5));
    col = mix(col, mix(v3(0.85, 0.55, 0.2), v3(0.9, 0.3, 0.45), fbm(p * 20.0, 2)), land);
    let diff = c.ndl.max(0.0);
    let mut lit = col * (0.04 + diff);
    if c.ndl > 0.0 {
        let s = reflect(-c.l, c.n).dot(c.v).max(0.0);
        lit += v3(1.0, 0.8, 1.0) * (s.powf(35.0) * 0.6 * (1.0 - land));
    }
    let night = smoothstep(0.1, -0.2, c.ndl);
    let ridge = 1.0 - ((sw - 0.5) * 2.0).abs();
    let glow = smoothstep(0.85, 1.0, ridge) * (0.6 + 0.4 * (t * 3.0 + sw * 20.0).sin());
    lit += v3(0.1, 1.0, 0.8) * (glow * night * (1.0 - land) * 1.2);
    let lat = p.y.abs();
    let lon = p.z.atan2(p.x);
    let curtain = ((lon * 6.0 + t * 0.8).sin() + (lon * 13.0 - t * 1.3).sin() * 0.5) * 0.03;
    let band = smoothstep(0.06, 0.0, (lat - 0.8 - curtain).abs());
    let flick = 0.6 + 0.4 * noise(v3(lon * 8.0, t * 2.0, lat * 20.0));
    let auc = mix(v3(0.2, 1.0, 0.5), v3(1.0, 0.3, 0.8), smoothstep(0.75, 0.86, lat));
    lit += auc * (band * flick * (0.4 + night * 0.9));
    lit + v3(0.9, 0.4, 1.0) * (fresnel(&c, 2.5) * (0.15 + diff * 0.8))
}

fn moon(f: &Frag, g: &Globals) -> Vec3 {
    let c = ctx(f, g);
    let p = c.p;
    let maria = smoothstep(0.42, 0.5, fbm(p * 1.8 + v3(11.0, 3.0, 7.0), 4));
    let mut col = mix(v3(0.68, 0.66, 0.63), v3(0.3, 0.29, 0.3), maria);
    col = col * (0.85 + 0.3 * fbm(p * 14.0, 4));
    for (scale, strength) in [(4.0f32, 1.0f32), (9.0, 0.75), (20.0, 0.5)] {
        let (f1, _, id) = voronoi(p * scale + v3(scale, scale * 0.3, 0.0));
        if id < 0.55 {
            let d = f1 / (0.25 + 0.4 * id);
            let bowl = smoothstep(0.8, 0.4, d);
            let rim = smoothstep(0.65, 0.85, d) * smoothstep(1.05, 0.85, d);
            col = col * (1.0 - bowl * 0.35 * strength) + splat(rim * 0.15 * strength);
        }
    }
    col * (0.02 + c.ndl.max(0.0))
}

pub fn shade(o: &Object, f: &Frag, g: &Globals) -> Vec3 {
    match o.body {
        Body::Sun => sun(f, g),
        Body::Terra => terra(f, g),
        Body::Zephyria => zephyria(o, f, g),
        Body::Ignis => ignis(f, g),
        Body::Glacies => glacies(f, g),
        Body::Aurelia => aurelia(f, g),
        Body::Moon => moon(f, g),
        Body::Ring => ZERO,
    }
}
