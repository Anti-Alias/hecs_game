use glam::Vec3;
use crate::{Color, g3d::Mesh};

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
        normals.extend([Vec3::NEG_X; 4]);

        // RIGHT
        positions.extend([rbn, rbf, rtf, rtn]);
        normals.extend([Vec3::X; 4]);

        // BOTTOM
        positions.extend([lbf, rbf, rbn, lbn]);
        normals.extend([Vec3::NEG_Y; 4]);

        // TOP
        positions.extend([ltn, rtn, rtf, ltf]);
        normals.extend([Vec3::Y; 4]);

        // NEAR
        positions.extend([lbn, rbn, rtn, ltn]);
        normals.extend([Vec3::Z; 4]);

        // FAR
        positions.extend([rbf, lbf, ltf, rtf]);
        normals.extend([Vec3::NEG_Z; 4]);

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