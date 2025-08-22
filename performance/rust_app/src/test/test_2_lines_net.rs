use std::time::Instant;
use x_overlay::core::fill_rule::FillRule;
use x_overlay::core::overlay_rule::OverlayRule;
use x_overlay::i_float::int::point::IntPoint;
use x_overlay::i_shape::int::path::IntPath;
use x_overlay::ortho::overlay::OrthoOverlay;

pub(crate) struct LinesNetTest;

/*

// 2
// Intersection:

multithreading on

4     - 0.000004
8     - 0.000014
16     - 0.000050
32     - 0.000196
64     - 0.001016
128     - 0.003970
256     - 0.020870
512     - 0.096745
1024     - 0.397470
2048     - 1.537385
4096     - 7.696920

multithreading off

4     - 0.000005
8     - 0.000015
16     - 0.000053
32     - 0.000216
64     - 0.001114
128     - 0.004175
256     - 0.018462
512     - 0.086984
1024     - 0.404892
2048     - 1.694361
4096     - 7.508013

geom multithreading off

4     - 0.000006
8     - 0.000016
16     - 0.000050
32     - 0.000196
64     - 0.001032
128     - 0.003914
256     - 0.018113
512     - 0.088561
1024     - 0.371023
2048     - 1.676831
4096     - 7.055219

// geom swipe line

4     - 0.000005
8     - 0.000014
16     - 0.000050
32     - 0.000191
64     - 0.000852
128     - 0.003730
256     - 0.017368
512     - 0.082651
1024     - 0.379062
2048     - 1.638863
4096     - 6.566427

*/

// A grid is formed by the intersection of a set of vertical and horizontal lines.
impl LinesNetTest {
    pub(crate) fn run(n: usize, rule: OverlayRule, scale: f64, multithreading: bool) { // 500
        let subj_paths = Self::many_lines_x(20, n);
        let clip_paths = Self::many_lines_y(20, n);

        let it_count = ((scale / (n as f64)) as usize).max(1);
        let sq_it_count= it_count * it_count;

        let start = Instant::now();

        let mut overlay = OrthoOverlay::default();
        overlay.solver.multithreading = multithreading;
        
        for _i in 0..sq_it_count {
            overlay.init_with_ortho_contours(&subj_paths, &clip_paths).expect("valid");
            overlay.overlay(rule, FillRule::NonZero);
        }

        let duration = start.elapsed();
        let time = duration.as_secs_f64() / sq_it_count as f64;

        let polygons_count = 2 * n;

        println!("{}     - {:.6}", polygons_count, time);
    }

    fn many_lines_x(a: i32, n: usize) -> Vec<IntPath> {
        let w = a / 2;
        let s = a * (n as i32) / 2;
        let mut x = -s + w / 2;
        let mut result = Vec::with_capacity(n);
        for _ in 0..n {
            let path: IntPath = vec![
                IntPoint::new(x, -s),
                IntPoint::new(x, s),
                IntPoint::new(x + w, s),
                IntPoint::new(x + w, -s),
            ];
            result.push(path);
            x += a;
        }

        result
    }

    fn many_lines_y(a: i32, n: usize) -> Vec<IntPath> {
        let h = a / 2;
        let s = a * (n as i32) / 2;
        let mut y = -s + h / 2;
        let mut result = Vec::with_capacity(n);
        for _ in 0..n {
            let path: IntPath = vec![
                IntPoint::new(-s, y),
                IntPoint::new(s, y),
                IntPoint::new(s, y - h),
                IntPoint::new(-s, y - h),
            ];
            result.push(path);
            y += a;
        }

        result
    }
}