use geo::convexhull::ConvexHull;
use geo::{line_string, polygon};
use geo::{Coordinate, Line, LineString, Polygon};
use glam::{vec2, vec3, Mat2, Vec2, Vec3};
use quad_rand as qrand;

const SHADOW_SIZE: f32 = 5f32;

/// sample points and find it's convex hull
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

// TODO. Not currently working. debug
/// Calculates segment to be used for shadow drawing
pub fn shadow_segment(polygon: &Polygon<f32>, position: Vec2, light: Vec2) -> (Vec2, Vec2) {
    let exterior: Vec<_> = polygon
        .exterior()
        .points_iter()
        .map(|p| vec2(p.x() - light.x(), p.y() - light.y()))
        .collect();
    let (mut p1, mut p2) = shadow_segment_from_origin(exterior, position);
    p1 += light;
    p2 += light;
    (p1, p2)
}

// TODO replace with shadow segment
pub fn shadow_segments(polygon: &Polygon<f32>) -> Vec<(Vec2, Vec2)> {
    let ex: Vec<_> = polygon.exterior().points_iter().collect();
    let mut res = vec![];
    for i in 0..ex.len() {
        for j in 0..ex.len() {
            if i == j {
                continue;
            }
            let a = ex[i];
            let b = ex[j];
            res.push((
                vec2(a.x(), a.y()), 
                vec2(b.x(), b.y())
            ));
        }
    }
    res
}

// TODO replace with shadow segment
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

/// angle from positive x axis
fn polar_angle(vec: Vec2) -> f32 {
    let p = vec.y().atan2(vec.x());
    if p < 0. {
        p + 2. * std::f32::consts::PI
    } else {
        p
    }
    // vec.y().atan2(vec.x())
}

fn shortest_angle(a: Vec2, b: Vec2) -> f32 {
    let a = (polar_angle(a) - polar_angle(b)).abs();
    if a > std::f32::consts::PI  {
        2. * std::f32::consts::PI - a
    } else { 
        a
    }
}

/// Given that light is in (0, 0) position
/// calculate the current shadow blocking segment of the polygon
pub fn shadow_segment_from_origin(points: Vec<Vec2>, position: Vec2) -> (Vec2, Vec2) {
    let rotation = Mat2::from_angle(polar_angle(points[0] + position));
    let mut point1 = points[0];
    let mut angle1 = polar_angle(rotation * (point1 + position));
    let mut point2 = points[0];
    let mut angle2 = polar_angle(rotation * (point2 + position));
    for point in points.iter() {
        let cur = rotation * (*point + position);
        let angle = polar_angle(cur);
        if angle < angle1 {
            angle1 = angle;
            point1 = *point;
        };
        if angle > angle2 {
            angle2 = angle;
            point2 = *point;
        }
    }
    (point1, point2)
}

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

/// multiply uv on homogeneous coordinate
/// http://reedbeta.com/blog/quadrilateral-interpolation-part-1/
pub fn projective_textures(shape: &[Vec2; 4], uv: &[Vec2; 4]) -> [Vec3; 4] {
    let diagonal1 = MyLine::from_segment(shape[0], shape[2]);
    let diagonal2 = MyLine::from_segment(shape[1], shape[3]);
    // TODO handle error properly
    let center = intersect(diagonal1, diagonal2).unwrap_or(vec2(0., 0.)); // TODO
    let mut distances = vec![];
    for point in shape {
        distances.push((center - *point).length());
    }
    let mut uvq = [Default::default(); 4];
    for i in 0..4 {
        let adj = (i + 1) % 3;
        // TODO devision zero check
        let homogeneous = (distances[i] + distances[adj]) / distances[adj];
        uvq[i] = vec3(
            homogeneous * uv[i].x(),
            homogeneous * uv[i].y(),
            homogeneous,
        );
    }
    uvq
}

// ---------------------------------------------------

// no intersections in geo.
// Other deps are too heavy -- just write intersections of line manually

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
