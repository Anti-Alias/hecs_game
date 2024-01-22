use glam::{Vec3, Mat4, Vec4, Vec4Swizzles};
use derive_more::*;

/// A 3D shape that can be one of many.
/// Useful object to be used in frustum culling.
pub enum Shape {
    Sphere(Sphere),
    AABB(AABB)
}

/**
 * A simple sphere representation.
 */
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Sphere {

    pub const UNIT: Self = Sphere {
        center: Vec3::ZERO,
        radius: 1.0
    };

    pub fn transform(self, mat: Mat4) -> Self {
        let right = mat.col(0).xyz();
        let up = mat.col(1).xyz();
        let back = mat.col(2).xyz();
        let max_scale = max_len(right, up, back);
        Self {
            center: mat.transform_point3(self.center),
            radius: self.radius * max_scale * 0.5
        }
    }
}

pub fn max_len(a: Vec3, b: Vec3, c: Vec3) -> f32 {
    let a_len_sq = a.length_squared();
    let b_len_sq = b.length_squared();
    let c_len_sq = c.length_squared();
    if a_len_sq > b_len_sq {
        if a_len_sq > c_len_sq {
            a_len_sq.sqrt()
        }
        else {
            b_len_sq.sqrt()
        }
    }
    else {
        if b_len_sq > c_len_sq {
            b_len_sq.sqrt()
        }
        else {
            c_len_sq.sqrt()
        }
    }
}


impl Default for Sphere {
    fn default() -> Self { Self::UNIT }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct AABB {
    pub center: Vec3,
    pub extents: Vec3,
}

impl Default for AABB {
    fn default() -> Self { Self::UNIT }
}

impl AABB {

    pub const UNIT: Self = AABB {
        center: Vec3::ZERO,
        extents: Vec3::splat(0.5),
    };

    pub fn transform(self, mat: Mat4) -> Self {
        let right = mat.col(0).xyz() * self.extents.x;
        let up = mat.col(1).xyz() * self.extents.y;
        let forward = -mat.col(2).xyz() * self.extents.z;
        let scale_x =
            Vec3::X.dot(right).abs() +
            Vec3::X.dot(up).abs() +
            Vec3::X.dot(forward).abs();
        let scale_y =
            Vec3::Y.dot(right).abs() +
            Vec3::Y.dot(up).abs() +
            Vec3::Y.dot(forward).abs();
        let scale_z =
            Vec3::Z.dot(right).abs() +
            Vec3::Z.dot(up).abs() +
            Vec3::Z.dot(forward).abs();
        Self {
            center: mat.transform_point3(self.center),
            extents: Vec3::new(scale_x, scale_y, scale_z),
        }
    }
}

#[derive(Copy, Clone, PartialEq, From, Debug)]
pub enum Volume {
    Sphere(Sphere),
    AABB(AABB),
}

impl Volume {
    pub fn sphere(center: Vec3, radius: f32) -> Self {
        Self::Sphere(Sphere { center, radius })
    }
    pub fn aabb(center: Vec3, extents: Vec3) -> Self {
        Self::AABB(AABB { center, extents })
    }

    pub fn transform(self, mat: Mat4) -> Self {
        match self {
            Volume::Sphere(sphere) => Self::Sphere(sphere.transform(mat)),
            Volume::AABB(aabb) => Self::AABB(aabb.transform(mat)),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl Plane {

    pub fn from_vec4(vec: Vec4) -> Self {
        let abc = Vec3::new(vec.x, vec.y, vec.z);
        let mag = abc.length();
        Plane {
            normal: abc / mag,
            distance: -vec.w / mag,
        }
    }

    pub fn signed_distance(self, point: Vec3) -> f32 {
        self.normal.dot(point) - self.distance
    }

    pub fn projection_interval(self, aabb: AABB) -> f32 {
        aabb.extents.x * self.normal.x.abs() +
        aabb.extents.y * self.normal.y.abs() +
        aabb.extents.z * self.normal.z.abs()
    }
}

/**
 * 3D frustum, consisting of 6 planes.
 * Useful for culling objects offscreen during rendering.
 * 
 * Resources:
 * https://www.gamedevs.org/uploads/fast-extraction-viewing-frustum-planes-from-world-view-projection-matrix.pdf
 * https://gdbooks.gitbooks.io/3dcollisions/content/Chapter6/frustum.html
 */
#[derive(Clone, PartialEq, Debug)]
pub struct Frustum {
    pub left: Plane,
    pub right: Plane,
    pub bottom: Plane,
    pub top: Plane,
    pub near: Plane,
    pub far: Plane,
}

impl Frustum {

    pub fn contains_shape(&self, shape: Shape) -> bool {
        match shape {
            Shape::Sphere(sphere) => self.contains_sphere(sphere),
            Shape::AABB(aabb) => self.contains_aabb(aabb),
        }
    }

    /// Checks if point is inside frustum.
    /// False if point sits precisely on the plane.
    pub fn contains_point(&self, point: Vec3) -> bool {
        self.left.signed_distance(point) > 0.0 &&
        self.right.signed_distance(point) > 0.0 &&
        self.bottom.signed_distance(point) > 0.0 &&
        self.top.signed_distance(point) > 0.0 &&
        self.near.signed_distance(point) > 0.0 &&
        self.far.signed_distance(point) > 0.0
    }

    /// Checks if sphere is completely, or partially inside the frustum.
    /// False if outside of sphere sits precisely on the plane.
    pub fn contains_sphere(&self, sphere: Sphere) -> bool {
        let (point, radius) = (sphere.center, sphere.radius);
        self.left.signed_distance(point) > -radius &&
        self.right.signed_distance(point) > -radius &&
        self.bottom.signed_distance(point) > -radius &&
        self.top.signed_distance(point) > -radius &&
        self.near.signed_distance(point) > -radius &&
        self.far.signed_distance(point) > -radius
    }

    /// Checks if aabb is completely, or partially inside the frustum.
    /// False if outside of aabb sits precisely on the plane.
    pub fn contains_aabb(&self, aabb: AABB) -> bool {
        -self.left.projection_interval(aabb) < self.left.signed_distance(aabb.center) &&
        -self.right.projection_interval(aabb) < self.right.signed_distance(aabb.center) &&
        -self.bottom.projection_interval(aabb) < self.bottom.signed_distance(aabb.center) &&
        -self.top.projection_interval(aabb) < self.top.signed_distance(aabb.center) &&
        -self.near.projection_interval(aabb) < self.near.signed_distance(aabb.center) &&
        -self.far.projection_interval(aabb) < self.far.signed_distance(aabb.center)
    }

    /// Checks if volume is completely, or partially inside the frustum.
    /// False if edge of volume sits precisely on the plane.
    pub fn contains_volume(&self, volume: Volume) -> bool {
        match volume {
            Volume::Sphere(sphere) => self.contains_sphere(sphere),
            Volume::AABB(aabb) => self.contains_aabb(aabb),
        }
    }
}

impl From<Mat4> for Frustum {
    fn from(proj_view: Mat4) -> Self {
        let row1 = proj_view.row(0);
        let row2 = proj_view.row(1);
        let row3 = proj_view.row(2);
        let row4 = proj_view.row(3);
        Self {
            left: Plane::from_vec4(row4 + row1),
            right: Plane::from_vec4(row4 - row1),
            bottom: Plane::from_vec4(row4 + row2),
            top: Plane::from_vec4(row4 - row2),
            near: Plane::from_vec4(row3),
            far: Plane::from_vec4(row4 - row3),
        }
    }
}

#[cfg(test)]
mod test {

    use glam::{Mat4, Vec3};
    use crate::math::Frustum;

    #[test]
    fn signed_dist() {
        let proj = Mat4::orthographic_rh(-1.0, 1.0, -1.0, 1.0, 0.0, 1.0);
        let frustum = Frustum::from(proj);

        let center = Vec3::new(0.0, 0.0, -0.5);
        
        let expected_dist = 1.0;
        let actual_dist = frustum.left.signed_distance(center);
        assert_eq!(expected_dist, actual_dist);

        let expected_dist = 1.0;
        let actual_dist = frustum.right.signed_distance(center);
        assert_eq!(expected_dist, actual_dist);

        let expected_dist = 1.0;
        let actual_dist = frustum.bottom.signed_distance(center);
        assert_eq!(expected_dist, actual_dist);

        let expected_dist = 1.0;
        let actual_dist = frustum.top.signed_distance(center);
        assert_eq!(expected_dist, actual_dist);

        let expected_dist = 0.5;
        let actual_dist = frustum.near.signed_distance(center);
        assert_eq!(expected_dist, actual_dist);

        let expected_dist = 0.5;
        let actual_dist = frustum.far.signed_distance(center);
        assert_eq!(expected_dist, actual_dist);
    }
}