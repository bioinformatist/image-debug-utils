use image::{DynamicImage, Luma};
use image_debug_utils::contours::remove_hypotenuse_in_place;
use imageproc::{
    contours::{Contour, find_contours},
    drawing::draw_line_segment_mut,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load Local Image
    let img_path = "assets/sample.jpg";
    println!("Loading image from {}...", img_path);
    let img = image::open(img_path)
        .expect("Failed to open assets/sample.jpg. Please place a sample image there.")
        .to_luma8();

    // 2. Pre-processing & Binarize
    // Use Gaussian Blur to reduce noise
    let blurred = imageproc::filter::gaussian_blur_f32(&img, 1.0);
    // Use Otsu's method for adaptive thresholding
    let threshold = imageproc::contrast::otsu_level(&blurred);
    println!("Computed Otsu threshold: {}", threshold);

    // Invert the image (assume object is dark on light background)
    let mut binary_img = blurred.clone();
    for pixel in binary_img.pixels_mut() {
        if pixel.0[0] < threshold {
            *pixel = Luma([255]); // Object
        } else {
            *pixel = Luma([0]); // Background
        }
    }

    // 3. Find Contours
    let contours_before = find_contours(&binary_img);
    println!("Contours before: {}", contours_before.len());

    // 4. Visualize Before
    let mut canvas_before = DynamicImage::ImageLuma8(img.clone()).to_rgb8();
    draw_contours_on_canvas(
        &mut canvas_before,
        &contours_before,
        image::Rgb([255, 0, 0]),
    );
    canvas_before.save("assets/readme_before.png")?;
    println!("Saved assets/readme_before.png");

    // 5. Process: Remove Hypotenuse
    // Filter out artifacts with high aspect ratio (e.g. thin lines, noise)
    // Using a lower threshold (2.0) makes it more sensitive/aggressive.
    let mut contours_after = contours_before.clone();
    remove_hypotenuse_in_place(&mut contours_after, 2.0, None);
    println!("Contours after: {}", contours_after.len());

    // 6. Visualize After
    let mut canvas_after = DynamicImage::ImageLuma8(img.clone()).to_rgb8();
    draw_contours_on_canvas(&mut canvas_after, &contours_after, image::Rgb([0, 255, 0]));
    canvas_after.save("assets/readme_after.png")?;
    println!("Saved assets/readme_after.png");

    // 7. Visualize Bounding Boxes (OBB vs AABB)
    let mut canvas_rects = DynamicImage::ImageLuma8(img.clone()).to_rgb8();

    // Efficiently find the largest contour (main object) to avoid sorting everything
    if let Some(c) = contours_before.iter().max_by_key(|c| c.points.len()) {
        // OBB (Green)
        let rect_points = imageproc::geometry::min_area_rect(&c.points);
        for i in 0..4 {
            let p1 = rect_points[i];
            let p2 = rect_points[(i + 1) % 4];
            draw_line_segment_mut(
                &mut canvas_rects,
                (p1.x as f32, p1.y as f32),
                (p2.x as f32, p2.y as f32),
                image::Rgb([0, 255, 0]),
            );
        }

        // AABB (Red)
        let aabb = image_debug_utils::rect::to_axis_aligned_bounding_box(&rect_points);
        let rect_struct = imageproc::rect::Rect::at(aabb.x as i32, aabb.y as i32)
            .of_size(aabb.width, aabb.height);
        imageproc::drawing::draw_hollow_rect_mut(
            &mut canvas_rects,
            rect_struct,
            image::Rgb([255, 0, 0]),
        );
    }
    canvas_rects.save("assets/readme_rects.png")?;
    println!("Saved assets/readme_rects.png");

    // 8. Visualize Connected Components
    let labels = imageproc::region_labelling::connected_components(
        &binary_img,
        imageproc::region_labelling::Connectivity::Eight,
        Luma([0]),
    );
    let colored = image_debug_utils::region_labelling::draw_principal_connected_components(
        &labels,
        5,
        image::Rgba([0, 0, 0, 255]),
    );
    colored.save("assets/readme_components.png")?;
    println!("Saved assets/readme_components.png");

    Ok(())
}

fn draw_contours_on_canvas(
    canvas: &mut image::RgbImage,
    contours: &[Contour<i32>],
    color: image::Rgb<u8>,
) {
    for c in contours {
        for i in 0..c.points.len() {
            let p1 = c.points[i];
            let p2 = c.points[(i + 1) % c.points.len()];
            draw_line_segment_mut(
                canvas,
                (p1.x as f32, p1.y as f32),
                (p2.x as f32, p2.y as f32),
                color,
            );
        }
    }
}
