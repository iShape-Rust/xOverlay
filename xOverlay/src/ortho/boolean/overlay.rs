// use crate::core::fill_rule::FillRule;
// use crate::core::overlay_rule::OverlayRule;
// use i_shape::int::shape::IntShapes;
// use crate::ortho::overlay::Overlay;
//
// gear Overlay {
//     pub fn overlay(&mut self, overlay_rule: OverlayRule, fill_rule: FillRule) -> IntShapes {
//         self.build_custom_graph(fill_rule, overlay_rule);
//         if let Some(graph) = &mut self.graph {
//             graph.extract_shapes(overlay_rule)
//         } else {
//             vec![]
//         }
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     extern crate std;
//
//     use crate::core::fill_rule::FillRule;
//     use crate::core::overlay_rule::OverlayRule;
//     use crate::graph::boolean::winding_count::ShapeCountBoolean;
//     use i_float::int::point::IntPoint;
//     use i_shape::int::area::Area;
//     use i_shape::int::shape::IntContour;
//     use rand::{thread_rng, Rng};
//
//     #[test]
//     fn test_0() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//
//         let subj = [vec![
//             IntPoint::new(0, 0),
//             IntPoint::new(10, 0),
//             IntPoint::new(10, 10),
//             IntPoint::new(0, 10),
//         ]];
//
//         overlay.init_with_ortho_contours(&subj, &[]).expect("OK");
//         let result = overlay.overlay(OverlayRule::Subject, FillRule::EvenOdd);
//
//         assert_eq!(result.len(), 1);
//         let shape = &result[0];
//         assert_eq!(shape.len(), 1);
//         assert_eq!(shape[0].len(), 4);
//         assert_eq!(shape[0].area(), -100);
//     }
//
//     #[test]
//     fn test_1() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//
//         let subj = [
//             vec![
//                 IntPoint::new(0, 0),
//                 IntPoint::new(5, 0),
//                 IntPoint::new(5, 10),
//                 IntPoint::new(0, 10),
//             ],
//             vec![
//                 IntPoint::new(5, 0),
//                 IntPoint::new(10, 0),
//                 IntPoint::new(10, 10),
//                 IntPoint::new(5, 10),
//             ],
//         ];
//
//         overlay.init_with_ortho_contours(&subj, &[]).expect("OK");
//         let result = overlay.overlay(OverlayRule::Subject, FillRule::EvenOdd);
//
//         assert_eq!(result.len(), 1);
//         let shape = &result[0];
//         assert_eq!(shape.len(), 1);
//         assert_eq!(shape[0].len(), 4);
//         assert_eq!(shape[0].area(), -100);
//     }
//
//     #[test]
//     fn test_2() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//
//         let subj = [
//             vec![
//                 IntPoint::new(0, 0),
//                 IntPoint::new(5, 0),
//                 IntPoint::new(5, 5),
//                 IntPoint::new(0, 5),
//             ],
//             vec![
//                 IntPoint::new(0, 5),
//                 IntPoint::new(5, 5),
//                 IntPoint::new(5, 10),
//                 IntPoint::new(0, 10),
//             ],
//             vec![
//                 IntPoint::new(5, 0),
//                 IntPoint::new(10, 0),
//                 IntPoint::new(10, 5),
//                 IntPoint::new(5, 5),
//             ],
//             vec![
//                 IntPoint::new(5, 5),
//                 IntPoint::new(10, 5),
//                 IntPoint::new(10, 10),
//                 IntPoint::new(5, 10),
//             ],
//         ];
//
//         overlay.init_with_ortho_contours(&subj, &[]).expect("OK");
//         let result = overlay.overlay(OverlayRule::Subject, FillRule::EvenOdd);
//
//         assert_eq!(result.len(), 1);
//         let shape = &result[0];
//         assert_eq!(shape.len(), 1);
//         assert_eq!(shape[0].len(), 4);
//         assert_eq!(shape[0].area(), -100);
//     }
//
//     #[test]
//     fn test_3() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//
//         let subj = [
//             vec![
//                 IntPoint::new(1, 1),
//                 IntPoint::new(1, 3),
//                 IntPoint::new(3, 3),
//                 IntPoint::new(3, 1),
//             ],
//             vec![
//                 IntPoint::new(0, 0),
//                 IntPoint::new(4, 0),
//                 IntPoint::new(4, 4),
//                 IntPoint::new(0, 4),
//             ],
//         ];
//
//         overlay.init_with_ortho_contours(&subj, &[]).expect("OK");
//         let result = overlay.overlay(OverlayRule::Subject, FillRule::EvenOdd);
//
//         assert_eq!(result.len(), 1);
//         let shape = &result[0];
//         assert_eq!(shape.len(), 2);
//         assert_eq!(shape[0].len(), 4);
//         assert_eq!(shape[0].area(), -16);
//         assert_eq!(shape[1].len(), 4);
//         assert_eq!(shape[1].area(), 4);
//     }
//
//     #[test]
//     fn test_4() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//
//         let subj = [
//             vec![
//                 IntPoint::new(0, 5),
//                 IntPoint::new(3, 5),
//                 IntPoint::new(3, 7),
//                 IntPoint::new(0, 7),
//             ],
//             vec![
//                 IntPoint::new(0, 1),
//                 IntPoint::new(1, 1),
//                 IntPoint::new(1, 6),
//                 IntPoint::new(0, 6),
//             ],
//         ];
//
//         overlay.init_with_ortho_contours(&subj, &[]).expect("OK");
//         let result = overlay.overlay(OverlayRule::Subject, FillRule::NonZero);
//
//         assert_eq!(result.len(), 1);
//         let shape = &result[0];
//         assert_eq!(shape.len(), 1);
//         assert_eq!(shape[0].area(), -10);
//     }
//
//     #[test]
//     fn test_5() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//
//         let subj = [
//             vec![
//                 IntPoint::new(5, 5),
//                 IntPoint::new(6, 5),
//                 IntPoint::new(6, 7),
//                 IntPoint::new(5, 7),
//             ],
//             vec![
//                 IntPoint::new(3, 1),
//                 IntPoint::new(6, 1),
//                 IntPoint::new(6, 6),
//                 IntPoint::new(3, 6),
//             ],
//             vec![
//                 IntPoint::new(6, 6),
//                 IntPoint::new(7, 6),
//                 IntPoint::new(7, 7),
//                 IntPoint::new(6, 7),
//             ],
//         ];
//
//         overlay.init_with_ortho_contours(&subj, &[]).expect("OK");
//         let result = overlay.overlay(OverlayRule::Subject, FillRule::NonZero);
//
//         assert_eq!(result.len(), 1);
//         let shape = &result[0];
//         assert_eq!(shape.len(), 1);
//         assert_eq!(shape[0].area(), -17);
//     }
//
//     #[test]
//     fn test_6() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//
//         let subj = [
//             vec![
//                 IntPoint::new(4, 3),
//                 IntPoint::new(7, 3),
//                 IntPoint::new(7, 6),
//                 IntPoint::new(4, 6),
//             ],
//             vec![
//                 IntPoint::new(5, 5),
//                 IntPoint::new(6, 5),
//                 IntPoint::new(6, 7),
//                 IntPoint::new(5, 7),
//             ],
//         ];
//
//         overlay.init_with_ortho_contours(&subj, &[]).expect("OK");
//         let result = overlay.overlay(OverlayRule::Subject, FillRule::NonZero);
//
//         assert_eq!(result.len(), 1);
//         let shape = &result[0];
//         assert_eq!(shape.len(), 1);
//         assert_eq!(shape[0].area(), -10);
//     }
//
//     #[test]
//     fn test_7() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//         overlay.options.min_count_per_column_power = 2;
//
//         let subj = [
//             vec![
//                 IntPoint::new(0, 0),
//                 IntPoint::new(4, 0),
//                 IntPoint::new(4, 4),
//                 IntPoint::new(0, 4),
//             ],
//             vec![
//                 IntPoint::new(4, 1),
//                 IntPoint::new(6, 1),
//                 IntPoint::new(6, 3),
//                 IntPoint::new(4, 3),
//             ],
//         ];
//
//         overlay.init_with_ortho_contours(&subj, &[]).expect("OK");
//         let result = overlay.overlay(OverlayRule::Subject, FillRule::NonZero);
//
//         assert_eq!(result.len(), 1);
//         let shape = &result[0];
//         assert_eq!(shape.len(), 1);
//         assert_eq!(shape[0].area(), -20);
//     }
//
//     #[test]
//     fn test_8() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//         overlay.options.min_count_per_column_power = 2;
//
//         let subj = [
//             vec![
//                 IntPoint::new(0, 1),
//                 IntPoint::new(4, 1),
//                 IntPoint::new(4, 3),
//                 IntPoint::new(0, 3),
//             ],
//             vec![
//                 IntPoint::new(4, 0),
//                 IntPoint::new(6, 0),
//                 IntPoint::new(6, 4),
//                 IntPoint::new(4, 4),
//             ],
//         ];
//
//         overlay.init_with_ortho_contours(&subj, &[]).expect("OK");
//         let result = overlay.overlay(OverlayRule::Subject, FillRule::NonZero);
//
//         assert_eq!(result.len(), 1);
//         let shape = &result[0];
//         assert_eq!(shape.len(), 1);
//         assert_eq!(shape[0].area(), -16);
//     }
//
//     #[test]
//     fn test_9() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//         overlay.options.min_count_per_column_power = 2;
//
//         let subj = [
//             vec![
//                 IntPoint::new(5, 0),
//                 IntPoint::new(6, 0),
//                 IntPoint::new(6, 6),
//                 IntPoint::new(5, 6),
//             ],
//             vec![
//                 IntPoint::new(3, 4),
//                 IntPoint::new(6, 4),
//                 IntPoint::new(6, 5),
//                 IntPoint::new(3, 5),
//             ],
//         ];
//
//         overlay.init_with_ortho_contours(&subj, &[]).expect("OK");
//         let result = overlay.overlay(OverlayRule::Subject, FillRule::NonZero);
//
//         assert_eq!(result.len(), 1);
//         let shape = &result[0];
//         assert_eq!(shape.len(), 1);
//         assert_eq!(shape[0].area(), -8);
//     }
//
//     #[test]
//     fn test_10() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//         overlay.options.min_count_per_column_power = 2;
//
//         let subj = [
//             vec![
//                 IntPoint::new(0, 1),
//                 IntPoint::new(6, 1),
//                 IntPoint::new(6, 7),
//                 IntPoint::new(0, 7),
//             ],
//             vec![
//                 IntPoint::new(4, 0),
//                 IntPoint::new(6, 0),
//                 IntPoint::new(6, 1),
//                 IntPoint::new(4, 1),
//             ],
//         ];
//
//         overlay.init_with_ortho_contours(&subj, &[]).expect("OK");
//         let result = overlay.overlay(OverlayRule::Subject, FillRule::NonZero);
//
//         assert_eq!(result.len(), 1);
//         let shape = &result[0];
//         assert_eq!(shape.len(), 1);
//         assert_eq!(shape[0].area(), -38);
//     }
//
//     #[test]
//     fn test_11() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//         overlay.options.min_count_per_column_power = 2;
//
//         let subj = [
//             vec![
//                 IntPoint::new(0, 0),
//                 IntPoint::new(4, 0),
//                 IntPoint::new(4, 6),
//                 IntPoint::new(0, 6),
//             ],
//             vec![
//                 IntPoint::new(1, 5),
//                 IntPoint::new(4, 5),
//                 IntPoint::new(4, 6),
//                 IntPoint::new(1, 6),
//             ],
//         ];
//
//         overlay.init_with_ortho_contours(&subj, &[]).expect("OK");
//         let result = overlay.overlay(OverlayRule::Subject, FillRule::NonZero);
//
//         assert_eq!(result.len(), 1);
//         let shape = &result[0];
//         assert_eq!(shape.len(), 1);
//         assert_eq!(shape[0].area(), -24);
//     }
//
//     #[test]
//     fn test_12() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//         overlay.options.min_count_per_column_power = 2;
//
//         let subj = [
//             vec![
//                 IntPoint::new(6, 0),
//                 IntPoint::new(7, 0),
//                 IntPoint::new(7, 6),
//                 IntPoint::new(6, 6),
//             ],
//             vec![
//                 IntPoint::new(3, 2),
//                 IntPoint::new(6, 2),
//                 IntPoint::new(6, 4),
//                 IntPoint::new(3, 4),
//             ],
//             vec![
//                 IntPoint::new(0, 0),
//                 IntPoint::new(6, 0),
//                 IntPoint::new(6, 3),
//                 IntPoint::new(0, 3),
//             ],
//             vec![
//                 IntPoint::new(1, 0),
//                 IntPoint::new(3, 0),
//                 IntPoint::new(3, 2),
//                 IntPoint::new(1, 2),
//             ],
//         ];
//
//         overlay.init_with_ortho_contours(&subj, &[]).expect("OK");
//         let result = overlay.overlay(OverlayRule::Subject, FillRule::NonZero);
//
//         assert_eq!(result.len(), 1);
//         let shape = &result[0];
//         assert_eq!(shape.len(), 1);
//         assert_eq!(shape[0].area(), -27);
//     }
//
//     #[test]
//     fn test_random_0() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//         overlay.options.min_count_per_column_power = 2;
//         for _ in 0..20_000 {
//             let (rects, target) = random_ccw_rects(2, 3);
//             overlay.init_with_ortho_contours(&rects, &[]).expect("OK");
//             let shapes = overlay.overlay(OverlayRule::Subject, FillRule::NonZero);
//             let area = (-shapes.area()) as usize;
//
//             if area != target {
//                 assert_eq!(area, target);
//             }
//         }
//     }
//
//     #[test]
//     fn test_random_1() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//         overlay.options.min_count_per_column_power = 2;
//         for _ in 0..20_000 {
//             let (rects, target) = random_ccw_rects(3, 3);
//             overlay.init_with_ortho_contours(&rects, &[]).expect("OK");
//             let shapes = overlay.overlay(OverlayRule::Subject, FillRule::NonZero);
//             let area = (-shapes.area()) as usize;
//
//             if area != target {
//                 assert_eq!(area, target);
//             }
//         }
//     }
//
//     #[test]
//     fn test_random_2() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//         overlay.options.min_count_per_column_power = 2;
//         for _ in 0..20_000 {
//             let (rects, target) = random_ccw_rects(4, 3);
//             overlay.init_with_ortho_contours(&rects, &[]).expect("OK");
//             let shapes = overlay.overlay(OverlayRule::Subject, FillRule::NonZero);
//             let area = (-shapes.area()) as usize;
//
//             if area != target {
//                 assert_eq!(area, target);
//             }
//         }
//     }
//
//     #[test]
//     fn test_random_3() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//         overlay.options.min_count_per_column_power = 2;
//         for _ in 0..10_000 {
//             let (rects, target) = random_ccw_rects(16, 3);
//             overlay.init_with_ortho_contours(&rects, &[]).expect("OK");
//             let shapes = overlay.overlay(OverlayRule::Subject, FillRule::NonZero);
//             let area = (-shapes.area()) as usize;
//
//             if area != target {
//                 assert_eq!(area, target);
//             }
//         }
//     }
//
//     #[test]
//     fn test_random_4() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//         overlay.options.min_count_per_column_power = 2;
//         for _ in 0..10_000 {
//             let (rects, target) = random_ccw_rects(4, 4);
//             overlay.init_with_ortho_contours(&rects, &[]).expect("OK");
//             let shapes = overlay.overlay(OverlayRule::Subject, FillRule::NonZero);
//             let area = (-shapes.area()) as usize;
//
//             if area != target {
//                 assert_eq!(area, target);
//             }
//         }
//     }
//
//     #[test]
//     fn test_random_5() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//         overlay.options.min_count_per_column_power = 2;
//         for _ in 0..10_000 {
//             let (rects, target) = random_ccw_rects(16, 5);
//             overlay.init_with_ortho_contours(&rects, &[]).expect("OK");
//             let shapes = overlay.overlay(OverlayRule::Subject, FillRule::NonZero);
//             let area = (-shapes.area()) as usize;
//
//             if area != target {
//                 assert_eq!(area, target);
//             }
//         }
//     }
//
//     #[test]
//     fn test_random_6() {
//         let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();
//         overlay.options.min_count_per_column_power = 2;
//         for _ in 0..1_000 {
//             let (rects, target) = random_ccw_rects(64, 6);
//             overlay.init_with_ortho_contours(&rects, &[]).expect("OK");
//             let shapes = overlay.overlay(OverlayRule::Subject, FillRule::NonZero);
//             let area = (-shapes.area()) as usize;
//
//             if area != target {
//                 assert_eq!(area, target);
//             }
//         }
//     }
//
//     fn random_ccw_rects(n: usize, p: usize) -> (Vec<IntContour>, usize) {
//         let w = 1i32 << p;
//         let mut rng = thread_rng();
//         let mut seen = vec![0u8; 1 << (2 * p)];
//         let mut area = 0usize;
//
//         let mut rects = Vec::with_capacity(n);
//
//         for _ in 0..n {
//             let rx: i32 = rng.gen_range(0..w - 1);
//             let ry: i32 = rng.gen_range(0..w - 1);
//             let ra = rng.gen_range(1..w - rx);
//             let rb = rng.gen_range(1..w - ry);
//
//             for x in rx..rx + ra {
//                 for y in ry..ry + rb {
//                     let idx = ((y as usize) << p) | (x as usize);
//                     area += (seen[idx] == 0) as usize;
//                     seen[idx] = 1;
//                 }
//             }
//
//             rects.push(
//                 vec![
//                     IntPoint::new(rx, ry),
//                     IntPoint::new(rx + ra, ry),
//                     IntPoint::new(rx + ra, ry + rb),
//                     IntPoint::new(rx, ry + rb),
//                 ]
//             )
//         }
//
//
//         (rects, area)
//     }
// }
