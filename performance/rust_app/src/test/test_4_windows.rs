use std::time::Instant;
use x_overlay::core::fill_rule::FillRule;
use x_overlay::core::overlay_rule::OverlayRule;
use x_overlay::i_float::int::point::IntPoint;
use x_overlay::i_shape::int::path::IntPath;
use x_overlay::ortho::overlay::OrthoOverlay;

pub(crate) struct WindowsTest;
/*
// 4
// Difference:

// multithreading on
8     - 0.000005
32     - 0.000023
128     - 0.000155
512     - 0.000383
2048     - 0.001534
8192     - 0.007696
32768     - 0.029771
131072     - 0.123545
524288     - 0.517019
2097152     - 2.088639
8388608     - 8.471537

// multithreading off

8     - 0.000005
32     - 0.000022
128     - 0.000129
512     - 0.000529
2048     - 0.002150
8192     - 0.010820
32768     - 0.045010
131072     - 0.191146
524288     - 0.879353
2097152     - 3.634808
8388608     - 15.136664

*/

// A grid of square frames, each with a smaller square cutout in the center.
impl WindowsTest {
    pub(crate) fn run(n: usize, rule: OverlayRule, scale: f64, multithreading: bool) { // 500
        let offset = 30;
        let x = (n as i32) * offset / 2;
        let origin = IntPoint::new(-x, -x);
        let (subj_paths, clip_paths) = Self::many_windows(origin, 20, 10, offset, n);

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

        let polygons_count = 2 * n * n;

        println!("{}     - {:.6}", polygons_count, time);
    }

    fn many_windows(start: IntPoint, a: i32, b: i32, offset: i32, n: usize) -> (Vec<IntPath>, Vec<IntPath>) {
        let mut boundaries = Vec::with_capacity(n * n);
        let mut holes = Vec::with_capacity(n * n);
        let mut y = start.y;
        let c = (a - b) / 2;
        let d = b + c;
        for _ in 0..n {
            let mut x = start.x;
            for _ in 0..n {
                let boundary: IntPath = vec![
                    IntPoint::new(x, y),
                    IntPoint::new(x, y + a),
                    IntPoint::new(x + a, y + a),
                    IntPoint::new(x + a, y),
                ];
                boundaries.push(boundary);

                let hole: IntPath = vec![
                    IntPoint::new(x + c, y + c),
                    IntPoint::new(x + c, y + d),
                    IntPoint::new(x + d, y + d),
                    IntPoint::new(x + d, y + c),
                ];
                holes.push(hole);

                x += offset;
            }
            y += offset;
        }

        (boundaries, holes)
    }
}