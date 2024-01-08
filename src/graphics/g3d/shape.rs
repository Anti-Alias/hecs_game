use glam::Vec3;
use crate::{Color, Mesh};

const NORM_LEFT: Vec3 = Vec3::new(-1.0, 0.0, 0.0);
const NORM_RIGHT: Vec3 = Vec3::new(1.0, 0.0, 0.0);
const NORM_UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);
const NORM_DOWN: Vec3 = Vec3::new(0.0, -1.0, 0.0);
const NORM_NEAR: Vec3 = Vec3::new(0.0, 0.0, 1.0);
const NORM_FAR: Vec3 = Vec3::new(0.0, 0.0, -1.0);

/**
 * A simple cuboid shape.
 */
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct Cuboid {
    pub center: Vec3,
    pub half_extents: Vec3,
    pub color: Color,
}

impl From<Cuboid> for Mesh {
    fn from(cuboid: Cuboid) -> Self {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let center = cuboid.center;
        let half = cuboid.half_extents;

        // 8 points on a cube
        let lbf = center + Vec3::new(-half.x,   -half.y,    -half.z);
        let rbf = center + Vec3::new( half.x,   -half.y,    -half.z);
        let ltf = center + Vec3::new(-half.x,    half.y,    -half.z);
        let rtf = center + Vec3::new( half.x,    half.y,    -half.z);
        let lbn = center + Vec3::new(-half.x,   -half.y,     half.z);
        let rbn = center + Vec3::new( half.x,   -half.y,     half.z);
        let ltn = center + Vec3::new(-half.x,    half.y,     half.z);
        let rtn = center + Vec3::new( half.x,    half.y,     half.z);

        // LEFT
        positions.extend([lbf, lbn, ltn, ltf]);
        normals.extend([NORM_LEFT; 4]);

        // RIGHT
        positions.extend([rbn, rbf, rtf, rtn]);
        normals.extend([NORM_RIGHT; 4]);

        // BOTTOM
        positions.extend([lbf, rbf, rbn, lbn]);
        normals.extend([NORM_DOWN; 4]);

        // TOP
        positions.extend([ltn, rtn, rtf, ltf]);
        normals.extend([NORM_UP; 4]);

        // NEAR
        positions.extend([lbn, rbn, rtn, ltn]);
        normals.extend([NORM_NEAR; 4]);

        // FAR
        positions.extend([rbf, lbf, ltf, rtf]);
        normals.extend([NORM_FAR; 4]);

        Mesh {
            positions,
            colors: Some(vec![cuboid.color; 24]),
            normals: Some(normals),
            indices: vec![
                0,1,2,2,3,0,
                4,5,6,6,7,4,
                8,9,10,10,11,8,
                12,13,14,14,15,12,
                16,17,18,18,19,16,
                20,21,22,22,23,20,
            ],
            ..Default::default()
        }
    }
}