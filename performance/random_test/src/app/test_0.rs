use i_overlay::i_float::int::point::IntPoint;
use i_overlay::i_shape::int::area::Area;
use i_overlay::i_shape::int::shape::IntContour;
use rand::Rng;
use crate::app::test_0_i_overlay::RandomTestI1;
use crate::app::test_0_x_overlay::RandomTestX1;
use crate::app::util::CircleCompare;

pub struct RandomTest0;

impl RandomTest0 {

    pub fn run_0(n: usize) {

        for _ in 0..n {
            let (subj, area) = Self::random_ccw_rects(2, 3);
            let ir = RandomTestI1::run(&subj);
            let xr = RandomTestX1::run(&subj);


            let ax = xr.area();
            let ai = ir.area();
            if ax != ai {
                assert!(false, "Not equal are");
            }

            if !xr.are_equal(&ir) {
                assert!(false, "Not equal shapes")
            }
        }
    }

    fn random_ccw_rects(n: usize, p: usize) -> (Vec<IntContour>, usize) {
        let w = 1i32 << p;
        let mut rng = rand::rng();
        let mut seen = vec![0u8; 1 << (2 * p)];
        let mut area = 0usize;

        let mut rects = Vec::with_capacity(n);

        for _ in 0..n {
            let rx: i32 = rng.random_range(0..w - 1);
            let ry: i32 = rng.random_range(0..w - 1);
            let ra = rng.random_range(1..w - rx);
            let rb = rng.random_range(1..w - ry);

            for x in rx..rx + ra {
                for y in ry..ry + rb {
                    let idx = ((y as usize) << p) | (x as usize);
                    area += (seen[idx] == 0) as usize;
                    seen[idx] = 1;
                }
            }

            rects.push(
                vec![
                    IntPoint::new(rx, ry),
                    IntPoint::new(rx + ra, ry),
                    IntPoint::new(rx + ra, ry + rb),
                    IntPoint::new(rx, ry + rb),
                ]
            )
        }


        (rects, area)
    }
}