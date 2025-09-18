use std::hint::black_box;
use std::time::Instant;
use x_overlay::core::fill_rule::FillRule;
use x_overlay::core::overlay::Overlay;
use x_overlay::core::overlay_rule::OverlayRule;
use x_overlay::core::solver::Solver;
use x_overlay::i_float::int::point::IntPoint;
use x_overlay::i_shape::int::path::IntPath;

pub(crate) struct CheckerboardTest;

/*
// test 0
// Xor:

multithreading on

2(5 0.7)     - 0.000041(-4.4)
4(25 1.4)     - 0.000059(-4.2)
8(113 2.1)     - 0.000123(-3.9)
16(481 2.7)     - 0.000292(-3.5)
32(1985 3.3)     - 0.001101(-3.0)
64(8065 3.9)     - 0.005974(-2.2)
128(32513 4.5)     - 0.017719(-1.8)
256(130561 5.1)     - 0.093034(-1.0)
512(523265 5.7)     - 0.585155(-0.2)
1024(2095105 6.3)     - 2.783931(0.4)
2048(8384513 6.9)     - 15.064510(1.2)

multithreading off

2(5 0.7)     - 0.000007(-5.2)
4(25 1.4)     - 0.000027(-4.6)
8(113 2.1)     - 0.000154(-3.8)
16(481 2.7)     - 0.000937(-3.0)
32(1985 3.3)     - 0.003197(-2.5)
64(8065 3.9)     - 0.012069(-1.9)
128(32513 4.5)     - 0.042825(-1.4)
256(130561 5.1)     - 0.198824(-0.7)
512(523265 5.7)     - 1.152738(0.1)
1024(2095105 6.3)     - 5.477457(0.7)
2048(8384513 6.9)     - 29.447586(1.5)

 */

// A grid of overlapping squares forming a simple checkerboard pattern.
impl CheckerboardTest {
    pub(crate) fn run(n: usize, rule: OverlayRule, scale: f64, multithreading: bool) { // 1000
        let subj_paths = Self::many_squares(IntPoint::new(0, 0), 20, 30, n);
        let clip_paths = Self::many_squares(IntPoint::new(15, 15), 20, 30, n - 1);

        let it_count = ((scale / (n as f64)) as usize).max(1);
        let sq_it_count= it_count * it_count;

        let start = Instant::now();

        for _i in 0..sq_it_count {
            let mut overlay = black_box(Overlay::with_contours_custom(&subj_paths, &clip_paths, Default::default(), Solver::new(multithreading)).expect("valid"));
            black_box(overlay.overlay(FillRule::NonZero, rule));
        }

        let duration = start.elapsed();
        let time = duration.as_secs_f64() / sq_it_count as f64;

        let polygons_count = n * n + (n - 1) * (n - 1);
        let count_log = (polygons_count as f64).log10();
        let time_log = time.log10();

        println!("{}({} {:.1})     - {:.6}({:.1})", n, polygons_count, count_log, time, time_log);
    }

    fn many_squares(start: IntPoint, size: i32, offset: i32, n: usize) -> Vec<IntPath> {
        let mut result = Vec::with_capacity(n * n);
        let mut y = start.y;
        for _ in 0..n {
            let mut x = start.x;
            for _ in 0..n {
                let path: IntPath = vec![
                    IntPoint::new(x, y),
                    IntPoint::new(x, y + size),
                    IntPoint::new(x + size, y + size),
                    IntPoint::new(x + size, y),
                ];
                result.push(path);
                x += offset;
            }
            y += offset;
        }

        result
    }
}