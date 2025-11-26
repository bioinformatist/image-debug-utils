use image::Rgba;
use palette::{FromColor, Hsl, Srgb};

/// Generates `n` visually distinct, contrasting RGBA colors.
pub(crate) fn generate_contrasting_colors(n: usize, alpha: u8) -> Vec<Rgba<u8>> {
    let mut colors = Vec::with_capacity(n);

    for i in 0..n {
        let hue = (i as f32 * 360.0) / n as f32;

        let saturation = 0.9;
        let lightness = 0.5;

        let hsl_color = Hsl::new(hue, saturation, lightness);
        let srgb_linear = Srgb::from_color(hsl_color);
        let srgb_u8: Srgb<u8> = srgb_linear.into_format();

        colors.push(Rgba([srgb_u8.red, srgb_u8.green, srgb_u8.blue, alpha]));
    }

    colors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_contrasting_colors_works() {
        assert!(generate_contrasting_colors(0, 255).is_empty());
        assert_eq!(
            generate_contrasting_colors(1, 255),
            vec![Rgba([242, 13, 13, 255])]
        );
        assert_eq!(
            generate_contrasting_colors(2, 255),
            vec![Rgba([242, 13, 13, 255]), Rgba([13, 242, 242, 255])]
        );
        assert_eq!(
            generate_contrasting_colors(3, 255),
            vec![
                Rgba([242, 13, 13, 255]),
                Rgba([13, 242, 13, 255]),
                Rgba([13, 13, 242, 255])
            ]
        );
    }
}
