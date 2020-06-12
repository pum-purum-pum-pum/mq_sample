use geo::convexhull::ConvexHull;
use geo::{LineString, Polygon};
use glam::{vec2, vec3, Vec2, Vec3};
use quad_rand as qrand;

const SHADOW_SIZE: f32 = 20f32;

/// Sample points and find it's convex hull
pub fn generate_convex_polygon(samples_num: usize, size: f32) -> Polygon<f32> {
    let mut points = vec![];
    for _ in 0..samples_num {
        let x = qrand::gen_range(-size, size);
        // sample from circle
        let chord = (size * size - x * x).sqrt();
        let y = qrand::gen_range(-chord, chord);
        points.push(vec2(x, y));
    }
    let points: Vec<(f32, f32)> = points.iter().map(|p| (p.x(), p.y())).collect();
    let polygon = Polygon::new(LineString::from(points), vec![]);
    polygon.convex_hull()
}

/// Full graph on vertices of polygon
pub fn _shadow_segments(polygon: &Polygon<f32>) -> Vec<(Vec2, Vec2)> {
    let ex: Vec<_> = polygon.exterior().points_iter().collect();
    let mut res = vec![];
    for i in 0..ex.len() {
        for j in 0..ex.len() {
            if i == j {
                continue;
            }
            let a = ex[i];
            let b = ex[j];
            res.push((vec2(a.x(), a.y()), vec2(b.x(), b.y())));
        }
    }
    res
}

// Polygons are small for this demo.
//O(n^2)
/// Find ligh blocking segment for polygon mesh from light source
pub fn brute_shadow_segment(polygon: &Polygon<f32>, position: Vec2, light: Vec2) -> (Vec2, Vec2) {
    let ex: Vec<_> = polygon.exterior().points_iter().collect();
    let mut res = (vec2(0., 0.), vec2(0., 0.));
    let mut max_angle = 0.;
    for i in 0..ex.len() {
        for j in 0..ex.len() {
            if i == j {
                continue;
            }
            let origin_a = vec2(ex[i].x(), ex[i].y());
            let origin_b = vec2(ex[j].x(), ex[j].y());
            let a = position + origin_a - light;
            let b = position + origin_b - light;
            let cur_angle = shortest_angle(a, b);
            if cur_angle > max_angle {
                max_angle = cur_angle;
                res = (origin_a, origin_b);
            }
        }
    }
    res
}

/// Angle from positive x axis
fn polar_angle(vec: Vec2) -> f32 {
    let p = vec.y().atan2(vec.x());
    if p < 0. {
        p + 2. * std::f32::consts::PI
    } else {
        p
    }
}

/// Shortest angle between two radius vectors
fn shortest_angle(a: Vec2, b: Vec2) -> f32 {
    let a = (polar_angle(a) - polar_angle(b)).abs();
    if a > std::f32::consts::PI {
        2. * std::f32::consts::PI - a
    } else {
        a
    }
}

/// Construct shadow quadrangle from ligh blocking segment and light position
/// position -- is the position of segment in the world
/// segment -- model coords
pub fn shadow_shape(segment: (Vec2, Vec2), light: Vec2, position: Vec2) -> [Vec2; 4] {
    let dir0 = (segment.0 + position - light).normalize();
    let dir1 = (segment.1 + position - light).normalize();
    [
        position + segment.0 + dir0 * SHADOW_SIZE,
        position + segment.0,
        position + segment.1,
        position + segment.1 + dir1 * SHADOW_SIZE,
    ]
}

/// Multiply uv on homogeneous coordinate to achive smooth texture interpolation
/// http://reedbeta.com/blog/quadrilateral-interpolation-part-1/
pub fn projective_textures(shape: &[Vec2; 4], uv: &[Vec2; 4]) -> [Vec3; 4] {
    let diagonal1 = MyLine::from_segment(shape[0], shape[2]);
    let diagonal2 = MyLine::from_segment(shape[1], shape[3]);
    let intersection = intersect(diagonal1, diagonal2);
    if let Some(center) = intersection {
        let mut distances = vec![];
        for point in shape {
            distances.push((center - *point).length());
        }
        let mut uvq = [Default::default(); 4];
        for i in 0..4 {
            let adj = (i + 2) % 3;
            let homogeneous = if distances[adj] > 0. {
                (distances[i] + distances[adj]) / distances[adj]
            } else {
                1.
            };
            uvq[i] = vec3(
                homogeneous * uv[i].x(),
                homogeneous * uv[i].y(),
                homogeneous,
            );
        }
        uvq
    } else {
        let mut res = [vec3(0., 0., 1.); 4];
        for (i, v) in uv.iter().enumerate() {
            res[i] = vec3(v.x(), v.y(), 1.);
        }
        res
    }
}

// ---------------------------------------------------

// No intersection point in geo for lines.
// Other deps are too heavy(while miniquad compiles in 5 sec) -- just write lines intersection manually

const EPS: f32 = 1E-9;

pub struct MyLine {
    a: f32,
    b: f32,
    c: f32,
}

impl MyLine {
    pub fn from_segment(p: Vec2, q: Vec2) -> Self {
        let a = p.y() - q.y();
        let b = q.x() - p.x();
        MyLine {
            a,
            b,
            c: -a * p.x() - b * p.y(),
        }
    }
}

pub fn det(a: f32, b: f32, c: f32, d: f32) -> f32 {
    a * d - b * c
}

/// Kramer's lines intersection
pub fn intersect(line1: MyLine, line2: MyLine) -> Option<Vec2> {
    let divisor = det(line1.a, line1.b, line2.a, line2.b);
    // either parallel or equivalent
    if divisor.abs() < EPS {
        return None;
    }
    let res = vec2(
        -det(line1.c, line1.b, line2.c, line2.b) / divisor,
        -det(line1.a, line1.c, line2.a, line2.c) / divisor,
    );
    Some(res)
}

#[test]
fn check_convex() {
    assert!(generate_convex_polygon(10, 1.).is_convex());
}

#[test]
fn projective_textures_identity() {
    let shape = [vec2(0., 0.), vec2(0., 1.), vec2(1., 1.), vec2(1., 0.)];
    let uv = [vec2(0., 0.), vec2(0., 1.), vec2(1., 1.), vec2(1., 0.)];
    let new_uvs = projective_textures(&shape, &uv);
    let mut shader_pass = [vec2(0., 0.); 4];
    for i in 0..4 {
        let tex = new_uvs[i];
        shader_pass[i] = vec2(tex.x() / tex.z(), tex.y() / tex.z());
    }
    assert_eq!(shader_pass, uv);
}

#[test]
fn angle90() {
    assert!((polar_angle(vec2(0., 1.)) - std::f32::consts::PI / 2.).abs() < EPS)
}

#[test]
fn lines_intersection() {
    let horizontal = MyLine::from_segment(vec2(0., 0.), vec2(1., 0.));
    let vertical = MyLine::from_segment(vec2(0., 0.), vec2(0., 1.));
    assert_eq!(Some(vec2(0., 0.)), intersect(horizontal, vertical));
}

#[test]
fn lines_equal() {
    let horizontal1 = MyLine::from_segment(vec2(0., 0.), vec2(1., 0.));
    let horizontal2 = MyLine::from_segment(vec2(0., 0.), vec2(1., 0.));
    assert_eq!(None, intersect(horizontal1, horizontal2));
}

#[test]
fn lines_parallel() {
    let horizontal1 = MyLine::from_segment(vec2(0., 0.), vec2(1., 0.));
    let horizontal2 = MyLine::from_segment(vec2(0., 1.), vec2(1., 1.));
    assert_eq!(None, intersect(horizontal1, horizontal2));
}
