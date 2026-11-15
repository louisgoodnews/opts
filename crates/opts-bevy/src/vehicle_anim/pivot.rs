//! The pivot of a mesh animation: the pivot point, and the Y/Z swap of the OMSI basis.

use super::*;

/// Rows of the y/z swap `[1,0,0|0,0,1|0,1,0]` (the wheel meshes of the MAN New Lion's City).
pub const SWAP_YZ_ROWS: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]];

impl MeshPivot {
    /// A pivot with an identity matrix rotation.
    #[must_use]
    pub fn plain(origin: Vec3) -> Self {
        Self {
            origin,
            rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            scale: 1.0,
        }
    }

    pub(super) fn is_identity(&self) -> bool {
        (0..3).all(|i| (0..3).all(|j| (self.rows[i][j] - f32::from(u8::from(i == j))).abs() < 0.02))
    }

    pub(super) fn is_swap_yz(&self) -> bool {
        (0..3).all(|i| (0..3).all(|j| (self.rows[i][j] - SWAP_YZ_ROWS[i][j]).abs() < 0.02))
    }

    /// The matrix rotation in vehicle axes, `P = C·Mᵀ·C` with `C` the y/z swap between o3d and
    /// vehicle axes: an axis `a` of the origin rotation becomes the object's axis `P·a`.
    pub(super) fn vehicle_rotation(&self) -> Mat4 {
        let r = self.rows;
        // M has the rows r; Mᵀ has them as columns; conjugation with C swaps indices 1 and 2.
        let c = [0usize, 2, 1];
        // P[i][j] = Mᵀ[c(i)][c(j)] = M[c(j)][c(i)] = r[c(j)][c(i)]; column j of P:
        let column = |j: usize| Vec3::new(r[c[j]][c[0]], r[c[j]][c[1]], r[c[j]][c[2]]);
        Mat4::from_cols(
            column(0).extend(0.0),
            column(1).extend(0.0),
            column(2).extend(0.0),
            Vec4::W,
        )
    }
}

/// The y/z swap as a matrix in OMSI axes.
pub(super) fn swap_yz() -> Mat4 {
    Mat4::from_cols(
        Vec3::X.extend(0.0),
        Vec3::Z.extend(0.0),
        Vec3::Y.extend(0.0),
        Vec4::W,
    )
}
