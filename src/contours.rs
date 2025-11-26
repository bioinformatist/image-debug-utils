use imageproc::{
    contours::{BorderType, Contour},
    geometry::min_area_rect,
    point::Point,
};
use num::{Num, NumCast};
use num_traits::AsPrimitive;

/// Calculates the perimeter of each contour and sorts them in descending order.
///
/// This function takes a vector of `Contour<T>` objects, computes the perimeter for each one,
/// and returns a new vector of tuples, where each tuple contains the original contour
/// and its calculated perimeter as an `f64`. The returned vector is sorted based on the
/// perimeter in descending order.
///
/// For performance, this function takes ownership of the input vector and uses an unstable sort.
/// The perimeter is calculated as the sum of Euclidean distances between consecutive points,
/// closing the loop by including the distance between the last and first point.
///
/// # Type Parameters
///
/// * `T`: The numeric type of the point coordinates within the contour. It must be a type
///   that can be losslessly converted to `f64` for distance calculations, such as `i32` or `u32`.
///
/// # Arguments
///
/// * `contours`: A `Vec<Contour<T>>` which will be consumed by the function.
///
/// # Returns
///
/// A `Vec<(Contour<T>, f64)>` sorted by the perimeter in descending order.
/// Contours with 0 or 1 point will have a perimeter of `0.0`.
///
pub fn sort_by_perimeters_owned<T>(contours: Vec<Contour<T>>) -> Vec<(Contour<T>, f64)>
where
    T: Num + NumCast + Copy + PartialEq + Eq + AsPrimitive<f64>,
{
    let mut contours_with_perimeters: Vec<(Contour<T>, f64)> = contours
        .into_iter()
        .map(|contour| {
            let perimeter: f64 = contour
                .points
                .iter()
                .zip(contour.points.iter().cycle().skip(1))
                .map(|(p1, p2)| {
                    let dx: f64 = p2.x.as_() - p1.x.as_();
                    let dy: f64 = p2.y.as_() - p1.y.as_();
                    dx.hypot(dy)
                })
                .sum();
            (contour, perimeter)
        })
        .collect();

    contours_with_perimeters.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));

    contours_with_perimeters
}

/// Filters a vector of contours in-place based on shape properties.
///
/// This function removes contours that do not meet the specified criteria.
/// The filtering is done based on two optional conditions:
///
/// 1.  **Border Type**: If `border_type` is `Some`, only contours with a matching
///     `border_type` are kept.
/// 2.  **Aspect Ratio**: Contours whose minimum area bounding rectangle has an
///     aspect ratio (long side / short side) greater than or equal to
///     `max_aspect_ratio` are removed.
///
/// # Arguments
///
/// * `contours`: A mutable reference to a `Vec<Contour>` to be filtered.
/// * `max_aspect_ratio`: The maximum allowed aspect ratio. Must be a positive value.
/// * `border_type`: An `Option<BorderType>` to filter contours by their border type.
///
/// # Panics
///
/// Panics if `max_aspect_ratio` is not a positive finite number.
pub fn remove_hypotenuse_in_place(
    contours: &mut Vec<Contour<i32>>,
    max_aspect_ratio: f32,
    border_type: Option<BorderType>,
) {
    assert!(
        max_aspect_ratio.is_finite() && max_aspect_ratio > 0.0,
        "max_aspect_ratio must be a positive finite number"
    );

    let distance_squared = |p1: Point<i32>, p2: Point<i32>| -> f32 {
        let dx = (p1.x - p2.x) as f32;
        let dy = (p1.y - p2.y) as f32;
        dx * dx + dy * dy
    };

    contours.retain(|contour| {
        if let Some(required_type) = border_type
            && contour.border_type != required_type
        {
            return false;
        }

        if contour.points.len() < 4 {
            return false;
        }

        let rect_points = min_area_rect(&contour.points);

        let side1_squared = distance_squared(rect_points[0], rect_points[1]);
        let side2_squared = distance_squared(rect_points[1], rect_points[2]);

        if side1_squared < 1e-6 || side2_squared < 1e-6 {
            return false;
        }

        let aspect_ratio = if side1_squared > side2_squared {
            (side1_squared / side2_squared).sqrt()
        } else {
            (side2_squared / side1_squared).sqrt()
        };

        aspect_ratio < max_aspect_ratio
    });
}

/// Counts the number of direct child contours for each contour by consuming the input vector,
/// and returns pairs of `(Contour, count)`, sorted by the count in descending order.
///
/// This function takes ownership of the `contours` vector. This is a highly performant
/// approach as it avoids cloning the `Contour` objects. Instead, it moves them from the
/// input vector into the result vector. This is ideal when the original `contours` vector
/// is no longer needed after this call.
///
/// The time complexity is O(N log N), dominated by the final sort, where N is the number
/// of contours. The memory overhead is minimal as no deep copies of contour data occur.
///
/// # Arguments
///
/// * `contours` - A `Vec<Contour<i32>>` which will be consumed by the function. The caller
///   loses ownership of this vector.
///
/// # Returns
///
/// A `Vec<(Contour<i32>, usize)>` where each tuple contains a contour moved from the input
/// and its direct child count. The vector is sorted in descending order based on the count.
///
pub fn sort_by_direct_children_count_owned(
    contours: Vec<Contour<i32>>,
) -> Vec<(Contour<i32>, usize)> {
    if contours.is_empty() {
        return Vec::new();
    }

    let mut child_counts = vec![0; contours.len()];

    for contour in &contours {
        if let Some(parent_index) = contour.parent
            && let Some(count) = child_counts.get_mut(parent_index)
        {
            *count += 1;
        }
    }

    let mut result: Vec<(Contour<i32>, usize)> = contours
        .into_iter()
        .enumerate()
        .map(|(i, contour)| (contour, child_counts[i]))
        .collect();

    result.sort_unstable_by(|a, b| b.1.cmp(&a.1));

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_float_eq(a: f64, b: f64) {
        assert!(
            (a - b).abs() < 1e-9,
            "Assertion failed: expected {}, got {}",
            b,
            a
        );
    }

    fn make_contour(parent: Option<usize>, points: Vec<Point<i32>>) -> Contour<i32> {
        Contour {
            points,
            parent,
            border_type: BorderType::Hole,
        }
    }

    fn make_simple_contour(parent: Option<usize>) -> Contour<i32> {
        make_contour(parent, Vec::new())
    }

    #[test]
    fn test_filter_contours() {
        // A "good" contour with a 1:1 aspect ratio.
        let good_contour = Contour {
            points: vec![
                Point::new(10, 10),
                Point::new(20, 10),
                Point::new(20, 20),
                Point::new(10, 20),
            ],
            border_type: BorderType::Outer,
            parent: None,
        };

        // A "bad" contour with a high 10:1 aspect ratio.
        let bad_contour_high_ratio = Contour {
            points: vec![
                Point::new(0, 0),
                Point::new(100, 0),
                Point::new(100, 10),
                Point::new(0, 10),
            ],
            border_type: BorderType::Outer,
            parent: None,
        };

        // A contour with a different border type (Hole).
        let hole_contour = Contour {
            points: vec![
                Point::new(30, 30),
                Point::new(40, 30),
                Point::new(40, 40),
                Point::new(30, 40),
            ],
            border_type: BorderType::Hole,
            parent: Some(0),
        };

        // A degenerate contour with too few points.
        let degenerate_contour = Contour {
            points: vec![Point::new(0, 0), Point::new(1, 1)],
            border_type: BorderType::Outer,
            parent: None,
        };

        // --- Test Case 1: Filter by aspect ratio only ---
        let mut contours1 = vec![
            good_contour.clone(),
            bad_contour_high_ratio.clone(),
            degenerate_contour.clone(),
        ];
        remove_hypotenuse_in_place(&mut contours1, 5.0, None);
        assert_eq!(contours1.len(), 1);
        assert_eq!(contours1[0].points, good_contour.points);

        // --- Test Case 2: Filter by BorderType only (using a high aspect ratio threshold) ---
        let mut contours2 = vec![
            good_contour.clone(),
            bad_contour_high_ratio.clone(),
            hole_contour.clone(),
        ];
        remove_hypotenuse_in_place(&mut contours2, 100.0, Some(BorderType::Outer));
        assert_eq!(contours2.len(), 2, "Should keep both 'Outer' contours");
        // Check that the hole contour is removed.
        assert!(!contours2.iter().any(|c| c.border_type == BorderType::Hole));

        // --- Test Case 3: Filter by both aspect ratio and BorderType ---
        let mut contours3 = vec![
            good_contour.clone(),
            bad_contour_high_ratio.clone(),
            hole_contour.clone(),
        ];
        remove_hypotenuse_in_place(&mut contours3, 5.0, Some(BorderType::Outer));
        assert_eq!(
            contours3.len(),
            1,
            "Should keep only the 'good' outer contour"
        );
        assert_eq!(contours3[0].points, good_contour.points);

        // --- Test Case 4: No contours removed ---
        let mut contours4 = vec![good_contour.clone(), hole_contour.clone()];
        remove_hypotenuse_in_place(&mut contours4, 10.0, None);
        assert_eq!(contours4.len(), 2, "Should not remove any contours");

        // --- Test Case 5: Filter everything out ---
        let mut contours5 = vec![bad_contour_high_ratio.clone()];
        remove_hypotenuse_in_place(&mut contours5, 5.0, None);
        assert!(
            contours5.is_empty(),
            "Should filter out the high-ratio contour"
        );
    }

    #[test]
    fn test_owned_nested_hierarchy_for_direct_children() {
        // Hierarchy:
        // 0 -> 1      (1 direct child)
        // 1 -> 2      (1 direct child)
        // 3 -> 4, 5   (2 direct children)
        let contours = vec![
            make_contour(None, vec![Point::new(1, 1)]), // 0, give unique points
            make_contour(Some(0), vec![Point::new(2, 2)]), // 1
            make_simple_contour(Some(1)),               // 2
            make_contour(None, vec![Point::new(10, 10)]), // 3
            make_simple_contour(Some(3)),               // 4
            make_simple_contour(Some(3)),               // 5
        ];
        let contours_clone = contours.clone();

        let result = sort_by_direct_children_count_owned(contours);

        assert_eq!(result.len(), 6);

        // First element must be contour 3 with 2 children.
        // MODIFICATION: Compare the `points` field.
        assert_eq!(result[0].0.points, contours_clone[3].points);
        assert_eq!(result[0].1, 2);

        // The next two elements must have 1 child each.
        assert_eq!(result[1].1, 1);
        assert_eq!(result[2].1, 1);
        let one_child_contours_points: Vec<_> =
            result[1..3].iter().map(|(c, _)| &c.points).collect();
        // MODIFICATION: Check if the points vectors are present.
        assert!(one_child_contours_points.contains(&&contours_clone[0].points));
        assert!(one_child_contours_points.contains(&&contours_clone[1].points));

        // The remaining must have 0 children.
        assert!(result[3..].iter().all(|(_, count)| *count == 0));
    }

    #[test]
    fn test_calculate_perimeters_and_sort_comprehensive() {
        // 1. Test with an empty vector
        let empty_contours: Vec<Contour<i32>> = vec![];
        let result = sort_by_perimeters_owned(empty_contours);
        assert!(
            result.is_empty(),
            "Should return an empty vec for empty input"
        );

        // 2. Test with a mix of contours, including edge cases
        // Using the updated struct literal syntax for Contour
        let square = Contour {
            // Perimeter: 10 + 10 + 10 + 10 = 40.0
            points: vec![
                Point::new(0, 0),
                Point::new(10, 0),
                Point::new(10, 10),
                Point::new(0, 10),
            ],
            border_type: BorderType::Outer,
            parent: None,
        };
        let line = Contour {
            // Perimeter: 10 (forward) + 10 (back) = 20.0
            points: vec![Point::new(0, 0), Point::new(10, 0)],
            border_type: BorderType::Outer,
            parent: None,
        };
        let triangle = Contour {
            // Perimeter: 3 + 4 + 5 = 12.0
            points: vec![Point::new(0, 0), Point::new(3, 0), Point::new(0, 4)],
            border_type: BorderType::Outer,
            parent: None,
        };
        let single_point = Contour {
            // Perimeter: 0.0
            points: vec![Point::new(100, 100)],
            border_type: BorderType::Hole,
            parent: Some(0),
        };
        let empty_points = Contour {
            // Perimeter: 0.0
            points: vec![],
            border_type: BorderType::Outer,
            parent: None,
        };

        let contours = vec![triangle, single_point, square, empty_points, line];
        let result = sort_by_perimeters_owned(contours);

        // 3. Verify the results
        assert_eq!(result.len(), 5, "Should return results for all 5 contours");

        // Check order and values
        // [0]: Square (40.0)
        assert_eq!(result[0].0.points.len(), 4);
        assert_float_eq(result[0].1, 40.0);

        // [1]: Line (20.0)
        assert_eq!(result[1].0.points.len(), 2);
        assert_float_eq(result[1].1, 20.0);

        // [2]: Triangle (12.0)
        assert_eq!(result[2].0.points.len(), 3);
        assert_float_eq(result[2].1, 12.0);

        // [3] & [4]: Single Point (0.0) and Empty Points (0.0).
        // Their relative order is not guaranteed due to unstable sort,
        // so we just check that the last two perimeters are 0.0.
        assert_float_eq(result[3].1, 0.0);
        assert_float_eq(result[4].1, 0.0);
        let last_two_point_counts: Vec<usize> =
            result.iter().skip(3).map(|c| c.0.points.len()).collect();
        assert!(last_two_point_counts.contains(&0));
        assert!(last_two_point_counts.contains(&1));
    }
}
