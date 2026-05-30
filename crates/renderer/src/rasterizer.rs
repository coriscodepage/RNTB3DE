pub struct Rasterizer;
use std::{
    cmp::{max, min},
    fmt::Debug,
};

use bumpalo::Bump;
use glam::{IVec2, UVec2, Vec3};

use crate::{
    datatypes::{FragmentInput, Triangle},
    lerp::Lerp,
};
impl Rasterizer {
    pub fn rasterize<T: Lerp + Debug>(
        triangle: &Triangle<T>,
        width: usize,
        height: usize,
    ) -> Vec<FragmentInput<T>> {

        let IVec2 { x: ax, y: ay } = Self::to_screen_space(triangle.position[0], width, height);
        let IVec2 { x: bx, y: by } = Self::to_screen_space(triangle.position[1], width, height);
        let IVec2 { x: cx, y: cy } = Self::to_screen_space(triangle.position[2], width, height);

        let bbminx = min(min(ax, bx), cx); // bounding box for the triangle
        let bbminy = min(min(ay, by), cy); // defined by its top left and bottom right corners
        let bbmaxx = max(max(ax, bx), cx);
        let bbmaxy = max(max(ay, by), cy);
        let total_area = Self::signed_triangle_area(ax, ay, bx, by, cx, cy);
        let mut output = Vec::new();
        for x in bbminx..=bbmaxx {
            for y in bbminy..=bbmaxy {
                let alpha = Self::signed_triangle_area(x, y, bx, by, cx, cy) / total_area;
                let beta = Self::signed_triangle_area(x, y, cx, cy, ax, ay) / total_area;
                let gamma = Self::signed_triangle_area(x, y, ax, ay, bx, by) / total_area;
                if alpha < 0.0 || beta < 0.0 || gamma < 0.0 {
                    continue;
                } // negative barycentric coordinate => the pixel is outside the triangle
                output.push(FragmentInput::new(
                    UVec2::new(x as u32, y as u32),
                    triangle.data[0] * alpha + triangle.data[1] * beta + triangle.data[2] * gamma,
                ));
            }
        }
        // panic!("{:?}", output);
        // panic!();
        output
    }

    #[inline]
    fn to_screen_space(position: Vec3, width: usize, height: usize) -> IVec2 {
        let x = ((position.x + 1.0) * 0.5 * width as f32).round();
        let y = ((1.0 - (position.y + 1.0) * 0.5) * height as f32).round();
        IVec2::new(x as i32, y as i32)
    }

    #[inline]
    fn signed_triangle_area(ax: i32, ay: i32, bx: i32, by: i32, cx: i32, cy: i32) -> f32 {
        return 0.5
            * ((by - ay) * (bx + ax) + (cy - by) * (cx + bx) + (ay - cy) * (ax + cx)) as f32;
    }
}
