pub struct Rasterizer;
use std::{
    cmp::{max, min},
    fmt::Debug,
};

use glam::{IVec2, UVec2, Vec3};
use itertools::iproduct;
use wide::{CmpGe, bytemuck::Zeroable, f32x4, i32x4};

use crate::{
    datatypes::{FragmentInput, Triangle},
    framebuffer::Tile,
    lerp::Lerp,
};

impl Rasterizer {
    pub fn rasterize<T: Lerp + Copy + Debug + Send + Sync, F: FnMut(FragmentInput<T>)>(
        triangle: &Triangle<T>,
        tile: &Tile,
        mut callback: F,
    ) {
        let IVec2 { x: ax, y: ay } = triangle.position[0];
        let IVec2 { x: bx, y: by } = triangle.position[1];
        let IVec2 { x: cx, y: cy } = triangle.position[2];

        let depth_a_wide = wide::f32x4::splat(triangle.depth[0].x);
        let depth_b_wide = wide::f32x4::splat(triangle.depth[1].x);
        let depth_c_wide = wide::f32x4::splat(triangle.depth[2].x);

        let bbminx = min(min(ax, bx), cx); // bounding box for the triangle
        let bbminy = min(min(ay, by), cy); // defined by its top left and bottom right corners
        let bbmaxx = max(max(ax, bx), cx);
        let bbmaxy = max(max(ay, by), cy);

        let total_area = Self::edge_function(
            triangle.position[0],
            triangle.position[1],
            triangle.position[2],
        );
        let tile_min = tile.min();
        let tile_max = tile.max();
        let starting_pos = IVec2::new(bbminx.max(tile_min.x), bbminy.max(tile_min.y)); // Using a starting pos with an edge fuction and not recalculating the baro every iter
        let mut w0_row =
            Self::edge_function(triangle.position[1], triangle.position[2], starting_pos);
        let mut w1_row =
            Self::edge_function(triangle.position[2], triangle.position[0], starting_pos);
        let mut w2_row =
            Self::edge_function(triangle.position[0], triangle.position[1], starting_pos);

        let delta_0x = -(cy - by);
        let delta_0y = cx - bx;
        let delta_1x = -(ay - cy);
        let delta_1y = ax - cx;
        let delta_2x = -(by - ay);
        let delta_2y = bx - ax;

        let delta_0x_wide = wide::i32x4::splat(delta_0x * 4);
        let delta_1x_wide = wide::i32x4::splat(delta_1x * 4);
        let delta_2x_wide = wide::i32x4::splat(delta_2x * 4);

        let inv_area = 1.0 / total_area as f32;
        let inv_area_wide = wide::f32x4::splat(inv_area);

        let mut y = bbminy.max(tile_min.y);
        while y <= bbmaxy.min(tile_max.y) {

            let mut w0_wide = wide::i32x4::from([
                w0_row,
                w0_row + delta_0x,
                w0_row + delta_0x * 2,
                w0_row + delta_0x * 3,
            ]);
            let mut w1_wide = wide::i32x4::from([
                w1_row,
                w1_row + delta_1x,
                w1_row + delta_1x * 2,
                w1_row + delta_1x * 3,
            ]);
            let mut w2_wide = wide::i32x4::from([
                w2_row,
                w2_row + delta_2x,
                w2_row + delta_2x * 2,
                w2_row + delta_2x * 3,
            ]);
            let wide_zero = wide::f32x4::zeroed();
            let mut x = bbminx.max(tile_min.x);
            while x <= bbmaxx.min(tile_max.x) {

                let alpha_wide = w0_wide.round_float() * inv_area_wide;
                let beta_wide = w1_wide.round_float() * inv_area_wide;
                let gamma_wide = w2_wide.round_float() * inv_area_wide;

                let inside = alpha_wide.simd_ge(wide_zero)
                    & beta_wide.simd_ge(wide_zero)
                    & gamma_wide.simd_ge(wide_zero);

                let inside_bits = inside.to_bitmask();

                if inside_bits != 0 {
                    let depth_wide = depth_a_wide * alpha_wide + depth_b_wide * beta_wide + depth_c_wide * gamma_wide;
                    for lane in 0..4 {
                        let px = x + lane as i32;
                        if px > bbmaxx.min(tile_max.x) {
                            break;
                        } else if inside_bits & (1 << lane) == 0 {
                            continue;
                        }
                        let alpha = alpha_wide.as_array()[lane];
                        let beta = beta_wide.as_array()[lane];
                        let gamma = gamma_wide.as_array()[lane];
                        let data = triangle.data[0] * alpha
                            + triangle.data[1] * beta
                            + triangle.data[2] * gamma;
                        let depth = depth_wide.as_array()[lane];
                        (callback)(FragmentInput::new(IVec2::new(px, y), depth, data));
                    }
                }
                x += 4;
                w0_wide += delta_0x_wide;
                w1_wide += delta_1x_wide;
                w2_wide += delta_2x_wide;

            }
            y += 1;
            w0_row += delta_0y;
            w1_row += delta_1y;
            w2_row += delta_2y;
        }
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
    fn edge_function(a: IVec2, b: IVec2, c: IVec2) -> i32 {
        (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Vec2, Vec4, vec4};

    fn test_triangle() -> Triangle<Vec4> {
        Triangle {
            position: [IVec2::new(0, 0), IVec2::new(3, 0), IVec2::new(0, 3)],
            depth: [
                Vec2::new(0.2, 0.0),
                Vec2::new(0.4, 0.0),
                Vec2::new(0.6, 0.0),
            ],
            data: [
                vec4(1.0, 0.0, 0.0, 1.0),
                vec4(0.0, 1.0, 0.0, 1.0),
                vec4(0.0, 0.0, 1.0, 1.0),
            ],
        }
    }

    #[test]
    fn rasterize_emits_fragments_inside_tile() {
        let triangle = test_triangle();
        let tile = Tile {
            x: 0,
            y: 0,
            width: 4,
            height: 4,
        };

        let mut fragments = Vec::new();
        Rasterizer::rasterize(&triangle, &tile, |fragment| fragments.push(fragment));

        let top_left = fragments
            .iter()
            .find(|fragment| fragment.position == IVec2::new(0, 0))
            .expect("expected the triangle corner fragment to be rasterized");

        assert_eq!(top_left.data, vec4(1.0, 0.0, 0.0, 1.0));
        assert!((top_left.depth - 0.2).abs() < 1e-6);
        assert!(!fragments.is_empty());
        assert!(fragments.iter().all(|fragment| {
            fragment.position.x >= tile.min().x
                && fragment.position.x <= tile.max().x - 1
                && fragment.position.y >= tile.min().y
                && fragment.position.y <= tile.max().y - 1
        }));
    }

    #[test]
    fn rasterize_skips_tiles_that_do_not_overlap_the_triangle() {
        let triangle = test_triangle();
        let tile = Tile {
            x: 8,
            y: 8,
            width: 4,
            height: 4,
        };

        let mut fragments = Vec::new();
        Rasterizer::rasterize(&triangle, &tile, |fragment| fragments.push(fragment));

        assert!(fragments.is_empty());
    }
}
