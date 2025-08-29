use std::time::Instant;
use x_overlay::core::fill_rule::FillRule;
use x_overlay::core::overlay::Overlay;
use x_overlay::core::overlay_rule::OverlayRule;
use x_overlay::core::solver::Solver;
use x_overlay::i_float::int::point::IntPoint;
use x_overlay::i_shape::int::path::IntPath;

pub(crate) struct CrossTest;

/*

// 5
// Union:

// multithreading on

4     - 0.000006
8     - 0.000013
16     - 0.000028
32     - 0.000091
64     - 0.000212
128     - 0.000423
256     - 0.000722
512     - 0.001449
1024     - 0.003937
2048     - 0.007425
4096     - 0.023274
8192     - 0.050322
16384     - 0.174423
32768     - 0.358752
65536     - 1.404767
131072     - 2.896030
262144     - 12.049359
524288     - 26.293638

// multithreading off


*/

// A series of concentric squares, each progressively larger than the last.
impl CrossTest {
    pub(crate) fn run(n: usize, rule: OverlayRule, scale: f64, multithreading: bool) { // 500
        let (subj_paths, clip_paths) = Self::concentric_squares(4, n);

        let it_count = ((scale / (n as f64)) as usize).max(1);
        let sq_it_count= it_count * it_count;

        let start = Instant::now();

        for _i in 0..sq_it_count {
            let mut overlay = Overlay::with_contours_custom(&subj_paths, &clip_paths, Default::default(), Solver::new(multithreading)).expect("valid");
            overlay.overlay(FillRule::NonZero, rule);
        }

        let duration = start.elapsed();
        let time = duration.as_secs_f64() / sq_it_count as f64;

        let polygons_count = 2 * n;

        println!("{}     - {:.6}", polygons_count, time);
    }

    fn concentric_squares(a: i32, n: usize) -> (Vec<IntPath>, Vec<IntPath>) {
        let mut vert = Vec::with_capacity(2 * n);
        let mut horz = Vec::with_capacity(2 * n);
        let s = 2 * a;
        let mut r = s;
        for _ in 0..n {
            let hz_top: IntPath = vec![
                IntPoint::new(-r, r - a),
                IntPoint::new(-r, r),
                IntPoint::new(r, r),
                IntPoint::new(r, r - a),
            ];
            let hz_bot: IntPath = vec![
                IntPoint::new(-r, -r),
                IntPoint::new(-r, -r + a),
                IntPoint::new(r, -r + a),
                IntPoint::new(r, -r),
            ];
            horz.push(hz_top);
            horz.push(hz_bot);

            let vt_left: IntPath = vec![
                IntPoint::new(-r, -r),
                IntPoint::new(-r, r),
                IntPoint::new(-r + a, r),
                IntPoint::new(-r + a, -r),
            ];
            let vt_right: IntPath = vec![
                IntPoint::new(r - a, -r),
                IntPoint::new(r - a, r),
                IntPoint::new(r, r),
                IntPoint::new(r, -r),
            ];
            vert.push(vt_left);
            vert.push(vt_right);

            r += s;
        }

        (vert, horz)
    }
}