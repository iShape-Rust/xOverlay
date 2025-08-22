use std::time::Instant;
use x_overlay::core::fill_rule::FillRule;
use x_overlay::core::overlay_rule::OverlayRule;
use x_overlay::i_float::int::point::IntPoint;
use x_overlay::i_shape::int::path::IntPath;
use x_overlay::ortho::overlay::OrthoOverlay;

pub(crate) struct CrossTest;

/*

// 5
// Union:

// multithreading on
4     - 0.000009
8     - 0.000017
16     - 0.000034
32     - 0.000081
64     - 0.000217
128     - 0.000608
256     - 0.002016
512     - 0.002641
1024     - 0.005925
2048     - 0.018777
4096     - 0.044756
8192     - 0.165539
16384     - 0.331655
32768     - 1.148905
65536     - 2.197493
131072     - 8.194153
262144     - 15.285741

// multithreading off
4     - 0.000009
8     - 0.000018
16     - 0.000039
32     - 0.000089
64     - 0.000219
128     - 0.000638
256     - 0.002004
512     - 0.002683
1024     - 0.008269
2048     - 0.028743
4096     - 0.058336
8192     - 0.237410
16384     - 0.460193
32768     - 1.816680
65536     - 4.008331
131072     - 15.923762
262144     - 31.619456


geom multithreading off

4     - 0.000009
8     - 0.000017
16     - 0.000033
32     - 0.000079
64     - 0.000141
128     - 0.000322
256     - 0.000728
512     - 0.001900
1024     - 0.002963
2048     - 0.006745
4096     - 0.014768
8192     - 0.032764
16384     - 0.073155
32768     - 0.179328
65536     - 0.419386
131072     - 1.044835
262144     - 2.463408
524288     - 6.518206

geom map

4     - 0.000009
8     - 0.000019
16     - 0.000044
32     - 0.000089
64     - 0.000167
128     - 0.000387
256     - 0.000791
512     - 0.001944
1024     - 0.003577
2048     - 0.006597
4096     - 0.014917
8192     - 0.031029
16384     - 0.068286
32768     - 0.149406
65536     - 0.351961
131072     - 0.777655
262144     - 2.034850
524288     - 4.438061


*/

// A series of concentric squares, each progressively larger than the last.
impl CrossTest {
    pub(crate) fn run(n: usize, rule: OverlayRule, scale: f64, multithreading: bool) { // 500
        let (subj_paths, clip_paths) = Self::concentric_squares(4, n);

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