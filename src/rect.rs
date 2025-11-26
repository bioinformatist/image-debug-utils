use image::math::Rect;
use imageproc::point::Point;
use num_traits::{Num, ToPrimitive};

/// Calculates the axis-aligned bounding box of a rotated rectangle's vertices.
///
/// This function is designed to work with the output of `imageproc::geometry::min_area_rect`,
/// which is an array of four `Point<T>`. It iterates through the points to find the
/// minimum and maximum x and y coordinates, then constructs an `image::math::Rect`
/// that encloses all four points.
///
/// This version is generic over numeric types that implement `PartialOrd`, making it
/// suitable for both integer and floating-point coordinates.
///
/// # Arguments
///
/// * `vertices` - An array of 4 `Point<T>` representing the corners of a rectangle.
///   `T` must be a numeric type that supports partial ordering and arithmetic operations.
///
/// # Returns
///
/// An `image::math::Rect` representing the smallest possible axis-aligned rectangle
/// that contains all the input vertices.
///
/// # Panics
///
/// This function assumes the input array is not empty, which is guaranteed by its
/// type `&[Point<T>; 4]`.
///
/// # Examples
///
/// ```use image::math::Rect;
/// use imageproc::point::Point;
/// use image_debug_utils::rect::to_axis_aligned_bounding_box;
///
/// let rotated_rect_vertices = [
///     Point { x: 50.0, y: 10.0 },
///     Point { x: 90.0, y: 50.0 },
///     Point { x: 50.0, y: 90.0 },
///     Point { x: 10.0, y: 50.0 },
/// ];
///
/// let bounding_box = to_axis_aligned_bounding_box(&rotated_rect_vertices);
///
/// assert_eq!(bounding_box.x, 10);
/// assert_eq!(bounding_box.y, 10);
/// assert_eq!(bounding_box.width, 80);
/// assert_eq!(bounding_box.height, 80);
/// ```
pub fn to_axis_aligned_bounding_box<T>(vertices: &[Point<T>; 4]) -> Rect
where
    T: Copy + PartialOrd + Num + ToPrimitive,
{
    let p0 = vertices[0];
    let mut min_x = p0.x;
    let mut max_x = p0.x;
    let mut min_y = p0.y;
    let mut max_y = p0.y;

    // Iterate over the remaining 3 points.
    // Manual comparison is used here because `T` only has a `PartialOrd`.
    // This is required to support floating-point types, which do not implement `Ord`.
    for p in &vertices[1..] {
        if p.x < min_x {
            min_x = p.x;
        }
        if p.x > max_x {
            max_x = p.x;
        }
        if p.y < min_y {
            min_y = p.y;
        }
        if p.y > max_y {
            max_y = p.y;
        }
    }

    let x = min_x.to_u32().unwrap_or(0);
    let y = min_y.to_u32().unwrap_or(0);

    let width = max_x.to_u32().unwrap_or(0).saturating_sub(x);
    let height = max_y.to_u32().unwrap_or(0).saturating_sub(y);

    Rect {
        x,
        y,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use imageproc::point::Point;

    #[test]
    fn test_bounding_box_for_rotated_rect() {
        // A diamond shape, which is a rotated square.
        // min_x=10, max_x=90, min_y=10, max_y=90
        let vertices = [
            Point { x: 50, y: 10 },
            Point { x: 90, y: 50 },
            Point { x: 50, y: 90 },
            Point { x: 10, y: 50 },
        ];
        let expected = Rect {
            x: 10,
            y: 10,
            width: 80,
            height: 80,
        };
        assert_eq!(to_axis_aligned_bounding_box(&vertices), expected);
    }

    #[test]
    fn test_bounding_box_for_axis_aligned_rect() {
        // An already axis-aligned rectangle.
        let vertices = [
            Point { x: 20, y: 30 },
            Point { x: 120, y: 30 },
            Point { x: 120, y: 80 },
            Point { x: 20, y: 80 },
        ];
        let expected = Rect {
            x: 20,
            y: 30,
            width: 100,
            height: 50,
        };
        // The order of points doesn't matter. Let's shuffle them.
        let shuffled_vertices = [vertices[2], vertices[0], vertices[3], vertices[1]];
        assert_eq!(to_axis_aligned_bounding_box(&vertices), expected);
        assert_eq!(to_axis_aligned_bounding_box(&shuffled_vertices), expected);
    }

    #[test]
    fn test_bounding_box_with_negative_coordinates() {
        // This test now compiles and passes because the function signature
        // uses `PartialOrd`, which is implemented for f64.
        let vertices = [
            Point { x: -10.0, y: -20.0 },
            Point { x: 50.0, y: 30.0 },
            Point { x: 50.0, y: -20.0 },
            Point { x: -10.0, y: 30.0 },
        ];

        // After conversion to u32, negative values become 0.
        let expected = Rect {
            x: 0,       // min_x of -10.0 becomes 0
            y: 0,       // min_y of -20.0 becomes 0
            width: 50,  // max_x of 50.0 -> 50. 50.saturating_sub(0) = 50
            height: 30, // max_y of 30.0 -> 30. 30.saturating_sub(0) = 30
        };
        assert_eq!(to_axis_aligned_bounding_box(&vertices), expected);
    }

    #[test]
    fn test_single_point_rect() {
        // A degenerate rectangle where all points are the same.
        let vertices = [
            Point { x: 100, y: 100 },
            Point { x: 100, y: 100 },
            Point { x: 100, y: 100 },
            Point { x: 100, y: 100 },
        ];
        let expected = Rect {
            x: 100,
            y: 100,
            width: 0,
            height: 0,
        };
        assert_eq!(to_axis_aligned_bounding_box(&vertices), expected);
    }
}
