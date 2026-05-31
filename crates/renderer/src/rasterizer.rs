pub struct Rasterizer;
use std::{
    cmp::{max, min},
    fmt::Debug,
};

use glam::{IVec2, UVec2, Vec3};
use itertools::iproduct;
use wide::{f32x4, i32x4};

use crate::{
    datatypes::{FragmentInput, Triangle}, framebuffer::Tile, lerp::Lerp
};
impl Rasterizer {
    pub fn rasterize<T: Lerp + Copy + Debug + Send + Sync, F:FnMut(FragmentInput<T>)>
      (
        triangle: &Triangle<T>,
        width: usize,
        height: usize,
        // tile: &Tile,
        mut callback: F,
    ) {
        let IVec2 { x: ax, y: ay } = Self::to_screen_space(triangle.position[0], width, height);
        let IVec2 { x: bx, y: by } = Self::to_screen_space(triangle.position[1], width, height);
        let IVec2 { x: cx, y: cy } = Self::to_screen_space(triangle.position[2], width, height);

        let bbminx = min(min(ax, bx), cx).max(0); // bounding box for the triangle
        let bbminy = min(min(ay, by), cy).max(0); // defined by its top left and bottom right corners
        let bbmaxx = max(max(ax, bx), cx).min(width as i32 - 1);
        let bbmaxy = max(max(ay, by), cy).min(height as i32 - 1);

        let total_area = Self::signed_triangle_area(ax, ay, bx, by, cx, cy);

        if total_area < 1.0 {
            return;
        }
        // let mut output = Vec::new();
        let mut y = bbminy;
        while y <= bbmaxy {
            let mut x = bbminx;
            while x <= bbmaxx {
                let alpha = Self::signed_triangle_area(x, y, bx, by, cx, cy) / total_area;
                let beta = Self::signed_triangle_area(x, y, cx, cy, ax, ay) / total_area;
                let gamma = Self::signed_triangle_area(x, y, ax, ay, bx, by) / total_area;
                if alpha < 0.0 || beta < 0.0 || gamma < 0.0 {
                    x += 1;
                    continue;
                } // negative barycentric coordinate => the pixel is outside the triangle
                // if x >= width as i32 || y >= height as i32 {
                //     continue;
                // }
                let data =
                    triangle.data[0] * alpha + triangle.data[1] * beta + triangle.data[2] * gamma;
                let depth = triangle.position[0].z * alpha + triangle.position[1].z * beta + triangle.position[2].z * gamma;
                // output.push(FragmentInput::new(UVec2::new(x as u32, y as u32), data));
                (callback)(FragmentInput::new(IVec2::new(x, y), depth, data));
                x += 1;
            }
            y += 1;
        }
        // output
    }

    #[inline(always)]
    pub(crate) fn to_screen_space(position: Vec3, width: usize, height: usize) -> IVec2 {
        let x = (position.x + 1.0) * 0.5 * width as f32;
        let y = (1.0 - (position.y + 1.0) * 0.5) * height as f32;
        IVec2::new(unsafe { x.to_int_unchecked() }, unsafe {
            y.to_int_unchecked()
        })
    }

    #[inline(always)]
    fn signed_triangle_area(ax: i32, ay: i32, bx: i32, by: i32, cx: i32, cy: i32) -> f32 {
        return 0.5
            * ((by - ay) * (bx + ax) + (cy - by) * (cx + bx) + (ay - cy) * (ax + cx)) as f32;
    }

    #[inline(always)]
    fn signed_triangle_area_dot(a: IVec2, b: IVec2, c: IVec2) -> f32 {
        let ab = b - a;
        let ac = c - a;

        0.5 * (ab.x * ac.y - ab.y * ac.x) as f32
    }

    #[inline(always)]
    fn signed_triangle_area_bulk(
        ax: i32x4,
        ay: i32x4,
        bx: i32x4,
        by: i32x4,
        cx: i32x4,
        cy: i32x4,
    ) -> f32x4 {
        let multiplier = f32x4::splat(0.5);
        return multiplier
            * f32x4::from_i32x4(
                (by - ay) * (bx + ax) + (cy - by) * (cx + bx) + (ay - cy) * (ax + cx),
            );
    }

    #[inline(always)]
    fn edge_function(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32) -> f32 {
        (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)
    }
}
