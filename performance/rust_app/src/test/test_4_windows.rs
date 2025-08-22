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
8     - 0.000006
32     - 0.000021
128     - 0.000097
512     - 0.000516
2048     - 0.001548
8192     - 0.006780
32768     - 0.034149
131072     - 0.159685
524288     - 0.703147
2097152     - 3.182362
8388608     - 12.058687

// multithreading off
8     - 0.000005
32     - 0.000021
128     - 0.000099
512     - 0.000541
2048     - 0.001745
8192     - 0.007748
32768     - 0.038207
131072     - 0.190786
524288     - 0.862933
2097152     - 3.832362
8388608     - 15.595299

geom multithreading off

8     - 0.000006
32     - 0.000021
128     - 0.000080
512     - 0.000330
2048     - 0.001413
8192     - 0.006229
32768     - 0.030391
131072     - 0.146480
524288     - 0.632447
2097152     - 2.674580
8388608     - 11.443802

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