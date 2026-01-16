use iced::widget::{
    Space, button, column, container, image as iced_image, pick_list, row, scrollable, slider, text,
};
use iced::{Element, Length, Task, Theme};
use image::{DynamicImage, Luma, Rgb, Rgba};
use image_debug_utils::{
    contours::remove_hypotenuse_in_place, rect::to_axis_aligned_bounding_box,
    region_labelling::draw_principal_connected_components,
};
use imageproc::{
    contours::{Contour, find_contours},
    drawing::{draw_hollow_rect_mut, draw_line_segment_mut},
    geometry::min_area_rect,
    region_labelling::connected_components,
};

use lucide_icons::{LUCIDE_FONT_BYTES, iced::icon_github};

#[cfg(target_arch = "wasm32")]
use web_sys::window;

pub fn main() -> iced::Result {
    iced::application(Visualizer::new, Visualizer::update, Visualizer::view)
        .title(|_state: &Visualizer| "Contour Visualizer".to_string())
        .theme(|_state: &Visualizer| Theme::Dark)
        .font(LUCIDE_FONT_BYTES)
        .run()
}

impl Visualizer {
    fn new() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }
}

struct Visualizer {
    original_image: Option<DynamicImage>,

    // The "clean" processed image (without selection highlights)
    base_processed_image: Option<DynamicImage>,

    // The currently displayed image handle (may include highlights)
    processed_handle: Option<iced_image::Handle>,
    original_handle: Option<iced_image::Handle>,

    current_instance: VisualizerInstance,
    status: String,

    // Data State
    contours_cache: Vec<Contour<i32>>,   // Preserves hierarchy
    perimeters_cache: Vec<f64>,          // indexed by master contour index
    child_counts_cache: Vec<usize>,      // indexed by master contour index
    sorted_indices: Vec<usize>,          // The filtered and sorted list of indices to display
    selected_contour_idx: Option<usize>, // Index into contours_cache
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum VisualizerInstance {
    FilterContours { max_aspect_ratio: f32 },
    SortPerimeter { min_perimeter: f32 },
    SortChildren,
    BoundingBox,
    ConnectedComponents { n: usize },
}

impl Default for VisualizerInstance {
    fn default() -> Self {
        Self::FilterContours {
            max_aspect_ratio: 5.0,
        }
    }
}

impl VisualizerInstance {
    fn mode(&self) -> VisualizerMode {
        match self {
            VisualizerInstance::FilterContours { .. } => VisualizerMode::FilterContours,
            VisualizerInstance::SortPerimeter { .. } => VisualizerMode::SortPerimeter,
            VisualizerInstance::SortChildren => VisualizerMode::SortChildren,
            VisualizerInstance::BoundingBox => VisualizerMode::BoundingBox,
            VisualizerInstance::ConnectedComponents { .. } => VisualizerMode::ConnectedComponents,
        }
    }
}

impl std::fmt::Display for VisualizerInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VisualizerInstance::FilterContours { .. } => {
                write!(f, "Filter Contours (Aspect Ratio)")
            }
            VisualizerInstance::SortPerimeter { .. } => write!(f, "Sort by Perimeter"),
            VisualizerInstance::SortChildren => write!(f, "Sort by Children Count"),
            VisualizerInstance::BoundingBox => write!(f, "Bounding Boxes (OBB vs AABB)"),
            VisualizerInstance::ConnectedComponents { .. } => write!(f, "Connected Components"),
        }
    }
}
impl Eq for VisualizerInstance {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisualizerMode {
    FilterContours,
    SortPerimeter,
    SortChildren,
    BoundingBox,
    ConnectedComponents,
}

impl VisualizerMode {
    const ALL: [VisualizerMode; 5] = [
        VisualizerMode::FilterContours,
        VisualizerMode::SortPerimeter,
        VisualizerMode::SortChildren,
        VisualizerMode::BoundingBox,
        VisualizerMode::ConnectedComponents,
    ];

    fn default_instance(&self) -> VisualizerInstance {
        match self {
            VisualizerMode::FilterContours => VisualizerInstance::FilterContours {
                max_aspect_ratio: 5.0,
            },
            VisualizerMode::SortPerimeter => VisualizerInstance::SortPerimeter {
                min_perimeter: 100.0,
            },
            VisualizerMode::SortChildren => VisualizerInstance::SortChildren,
            VisualizerMode::BoundingBox => VisualizerInstance::BoundingBox,
            VisualizerMode::ConnectedComponents => VisualizerInstance::ConnectedComponents { n: 5 },
        }
    }
}

impl std::fmt::Display for VisualizerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VisualizerMode::FilterContours => write!(f, "Filter Contours"),
            VisualizerMode::SortPerimeter => write!(f, "Sort by Perimeter"),
            VisualizerMode::SortChildren => write!(f, "Sort by Children"),
            VisualizerMode::BoundingBox => write!(f, "Bounding Boxes"),
            VisualizerMode::ConnectedComponents => write!(f, "Connected Components"),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    LoadRandomImage,
    ModeSelected(VisualizerMode),
    ImageDownloaded(Result<DynamicImage, String>),
    SliderChanged(f32),
    ContourSelected(usize),
    OpenGithub,
}

impl Default for Visualizer {
    fn default() -> Self {
        Self {
            original_image: None,
            base_processed_image: None,
            original_handle: None,
            processed_handle: None,
            current_instance: VisualizerInstance::default(),
            status: "Click Random Image to start".to_string(),
            contours_cache: Vec::new(),
            perimeters_cache: Vec::new(),
            child_counts_cache: Vec::new(),
            sorted_indices: Vec::new(),
            selected_contour_idx: None,
        }
    }
}

impl Visualizer {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LoadRandomImage => {
                self.status = "Downloading Random Image...".to_string();
                let seed = rand::random::<u32>();
                let url = format!("https://picsum.photos/seed/{}/600/400", seed);
                Task::perform(download_image(url), Message::ImageDownloaded)
            }
            Message::ModeSelected(mode) => {
                self.current_instance = mode.default_instance();
                self.status = format!("Switched to: {}", mode);
                self.selected_contour_idx = None;
                if self.original_image.is_some() {
                    self.process_and_update(true);
                }
                Task::none()
            }
            Message::ImageDownloaded(result) => {
                match result {
                    Ok(img) => {
                        self.status = "Image loaded. Processing...".to_string();
                        self.original_image = Some(img.clone());
                        // Create basic handle for original
                        let rgba = img.to_rgba8();
                        self.original_handle = Some(iced_image::Handle::from_rgba(
                            rgba.width(),
                            rgba.height(),
                            rgba.into_raw(),
                        ));

                        self.process_and_update(true);
                        self.status = "Ready.".to_string();
                    }
                    Err(err) => {
                        self.status = format!("Error: {}", err);
                    }
                }
                Task::none()
            }
            Message::SliderChanged(value) => {
                match &mut self.current_instance {
                    VisualizerInstance::FilterContours { max_aspect_ratio } => {
                        *max_aspect_ratio = value;
                    }
                    VisualizerInstance::SortPerimeter { min_perimeter } => {
                        *min_perimeter = value;
                    }
                    VisualizerInstance::ConnectedComponents { n } => {
                        *n = value as usize;
                    }
                    _ => {}
                }

                if self.original_image.is_some() {
                    self.process_and_update(true);
                }
                Task::none()
            }
            Message::ContourSelected(idx) => {
                if Some(idx) != self.selected_contour_idx {
                    self.selected_contour_idx = Some(idx);
                    // Just re-render highlights on the existing base image
                    self.process_and_update(false);
                }
                Task::none()
            }
            Message::OpenGithub => {
                #[cfg(target_arch = "wasm32")]
                if let Some(window) = window() {
                    let _ = window.open_with_url_and_target(
                        "https://github.com/bioinformatist/image-debug-utils",
                        "_blank",
                    );
                }
                #[cfg(not(target_arch = "wasm32"))]
                println!("Open GitHub: https://github.com/bioinformatist/image-debug-utils");

                Task::none()
            }
        }
    }

    /// `full_process`: If true, re-runs the heavy image processing (contours, etc.).
    /// If false, just redraws the highlights on the existing `base_processed_image`.
    fn process_and_update(&mut self, full_process: bool) {
        if full_process {
            if let Some(img) = &self.original_image {
                let res = process_image(img, self.current_instance);
                self.base_processed_image = Some(res.image);
                self.contours_cache = res.contours;
                self.perimeters_cache = res.perimeters;
                self.child_counts_cache = res.child_counts;
                self.sorted_indices = res.sorted_indices;
                self.selected_contour_idx = None;
            }
        }

        // Now generate the display handle, potentially burning in selection
        if let Some(base) = &self.base_processed_image {
            let mut display_img = base.to_rgb8(); // Work with RGB for drawing

            let green = Rgb([0, 255, 0]);
            let yellow = Rgb([255, 255, 0]);

            // Burn in selection logic
            if let Some(idx) = self.selected_contour_idx {
                match self.current_instance {
                    VisualizerInstance::SortPerimeter { .. } | VisualizerInstance::SortChildren => {
                        if let Some(contour) = self.contours_cache.get(idx) {
                            // Draw Selected Contour (Green)
                            if contour.points.len() > 1 {
                                for i in 0..contour.points.len() {
                                    let p1 = contour.points[i];
                                    let p2 = contour.points[(i + 1) % contour.points.len()];
                                    draw_line_segment_mut(
                                        &mut display_img,
                                        (p1.x as f32, p1.y as f32),
                                        (p2.x as f32, p2.y as f32),
                                        green,
                                    );
                                }
                            }

                            // If SortChildren, highlight children too (Yellow)
                            if let VisualizerInstance::SortChildren = self.current_instance {
                                // This is O(N) but N is usually small enough for simple debug utils (or 2000 capped)
                                // If needed we could build an adjacency list in process_image
                                for child in &self.contours_cache {
                                    if child.parent == Some(idx) && child.points.len() > 1 {
                                        for i in 0..child.points.len() {
                                            let p1 = child.points[i];
                                            let p2 = child.points[(i + 1) % child.points.len()];
                                            draw_line_segment_mut(
                                                &mut display_img,
                                                (p1.x as f32, p1.y as f32),
                                                (p2.x as f32, p2.y as f32),
                                                yellow,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Convert to Handle
            let width = display_img.width();
            let height = display_img.height();
            // Extend to RGBA for Iced
            let rgba = DynamicImage::ImageRgb8(display_img).to_rgba8();

            self.processed_handle = Some(iced_image::Handle::from_rgba(
                width,
                height,
                rgba.into_raw(),
            ));
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let mut controls = row![
            button("Random Image").on_press(Message::LoadRandomImage),
            pick_list(
                &VisualizerMode::ALL[..],
                Some(self.current_instance.mode()),
                Message::ModeSelected
            ),
        ]
        .spacing(20)
        .align_y(iced::Alignment::Center);

        // Dynamic controls based on instance
        match self.current_instance {
            VisualizerInstance::FilterContours { max_aspect_ratio } => {
                controls =
                    controls.push(text(format!("Max Aspect Ratio: {:.1}", max_aspect_ratio)));
                controls = controls.push(
                    slider(1.0..=20.0, max_aspect_ratio, Message::SliderChanged)
                        .step(0.1)
                        .width(Length::Fixed(200.0)),
                );
            }
            VisualizerInstance::ConnectedComponents { n } => {
                controls = controls.push(text(format!("Top N Components: {}", n)));
                controls = controls.push(
                    slider(1.0..=20.0, n as f32, Message::SliderChanged)
                        .step(1.0)
                        .width(Length::Fixed(200.0)),
                );
            }
            VisualizerInstance::SortPerimeter { min_perimeter } => {
                controls = controls.push(text(format!("Min Perimeter: {:.1}", min_perimeter)));
                controls = controls.push(
                    slider(0.0..=500.0, min_perimeter, Message::SliderChanged)
                        .step(10.0)
                        .width(Length::Fixed(200.0)),
                );
            }
            _ => {}
        }

        let status_bar = text(&self.status).size(14);

        let header = row![
            text("Image Debug Utils Visualizer").size(24),
            Space::new().width(Length::Fill),
            button(
                container(icon_github().size(16))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .style(|theme| {
                        container::Style::default().border(iced::Border {
                            color: theme.palette().text,
                            width: 1.0,
                            radius: 6.0.into(),
                        })
                    })
                    .padding(0)
            )
            .on_press(Message::OpenGithub)
            .padding(0)
            .width(Length::Fixed(24.0))
            .height(Length::Fixed(24.0))
            .style(button::text)
        ]
        .align_y(iced::Alignment::Center)
        .width(Length::Fill);

        let content = column![header, controls, status_bar]
            .spacing(20)
            .padding(20);

        let main_view: Element<'_, Message> = match self.current_instance {
            VisualizerInstance::SortPerimeter { .. } | VisualizerInstance::SortChildren => {
                self.view_list_layout()
            }
            _ => {
                // Default Layout
                row![
                    column![
                        text("Original").size(16),
                        image_display(&self.original_handle)
                    ]
                    .spacing(10)
                    .width(Length::FillPortion(1)),
                    column![
                        text("Processed").size(16),
                        image_display(&self.processed_handle)
                    ]
                    .spacing(10)
                    .width(Length::FillPortion(1))
                ]
                .spacing(20)
                .height(Length::Fill)
                .into()
            }
        };

        container(
            content.push(
                container(main_view)
                    .width(Length::Fill)
                    .height(Length::Fill),
            ),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_list_layout(&self) -> Element<'_, Message> {
        // Image Area
        let image_area: Element<'_, Message> = if let Some(handle) = &self.processed_handle {
            iced_image::viewer(handle.clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            container(text("No image processed"))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        };

        let list_area = {
            let title = match self.current_instance {
                VisualizerInstance::SortChildren => "Child Counts",
                _ => "Perimeters",
            };

            let mut col = column![text(title).size(16)].spacing(5);

            // Use filtered indices
            for &real_idx in &self.sorted_indices {
                // Determine label
                let label = match self.current_instance {
                    VisualizerInstance::SortChildren => {
                        let count = self.child_counts_cache.get(real_idx).unwrap_or(&0);
                        format!("{} children", count)
                    }
                    _ => {
                        let p = self.perimeters_cache.get(real_idx).unwrap_or(&0.0);
                        format!("{:.1}", p)
                    }
                };

                let is_selected = Some(real_idx) == self.selected_contour_idx;
                let btn = button(text(label).size(14))
                    .on_press(Message::ContourSelected(real_idx))
                    .width(Length::Fill)
                    .padding(5)
                    .style(move |t: &Theme, status| {
                        if is_selected {
                            button::primary(t, status)
                        } else {
                            button::secondary(t, status)
                        }
                    });
                col = col.push(btn);
            }
            scrollable(col).height(Length::Fill).width(Length::Fill)
        };

        row![
            container(image_area)
                .width(Length::FillPortion(4))
                .height(Length::Fill)
                .style(|_| container::Style::default()),
            container(list_area)
                .width(Length::FillPortion(1))
                .height(Length::Fill)
                .padding(5)
                .style(|_| container::Style::default().border(iced::Border {
                    color: iced::Color::from_rgb(0.3, 0.3, 0.3),
                    width: 1.0,
                    radius: 0.0.into()
                }))
        ]
        .spacing(10)
        .padding(10)
        .into()
    }
}

fn image_display(handle: &Option<iced_image::Handle>) -> Element<'_, Message> {
    if let Some(h) = handle {
        iced_image::viewer(h.clone())
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        container(text("No Image").size(20))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_| container::Style::default().background(iced::Color::from_rgb(0.1, 0.1, 0.1)))
            .into()
    }
}

async fn download_image(url: String) -> Result<DynamicImage, String> {
    let client = reqwest::Client::builder()
        .user_agent("image-debug-utils-visualizer/0.1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP Error: {}", resp.status()));
    }

    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    image::load_from_memory(&bytes).map_err(|e| e.to_string())
}

struct ProcessedResult {
    image: DynamicImage,
    contours: Vec<Contour<i32>>, // Now holds full contours
    perimeters: Vec<f64>,
    child_counts: Vec<usize>,
    sorted_indices: Vec<usize>,
}

fn process_image(img: &DynamicImage, instance: VisualizerInstance) -> ProcessedResult {
    let gray = img.to_luma8();

    // Pre-processing: Blur -> Otsu Threshold
    // 1. Gaussian Blur (sigma 1.0) to reduce noise
    let blurred = imageproc::filter::gaussian_blur_f32(&gray, 1.0);

    // 2. Otsu's Method for adaptive thresholding
    let threshold_level = imageproc::contrast::otsu_level(&blurred);

    // 3. Apply threshold (Standard: objects are dark on light background, so we invert logic effectively)
    // "If pixel value is LESS than threshold (dark), make it WHITE (active)."

    let mut binary = blurred.clone();
    for p in binary.pixels_mut() {
        if p.0[0] < threshold_level {
            *p = Luma([255]);
        } else {
            *p = Luma([0]);
        }
    }

    match instance {
        VisualizerInstance::FilterContours { max_aspect_ratio } => {
            let contours = find_contours::<i32>(&binary);
            let mut filtered_contours = contours.clone();
            remove_hypotenuse_in_place(&mut filtered_contours, max_aspect_ratio, None);

            let mut canvas = img.to_rgb8();
            let color = Rgb([0, 255, 0]); // Green
            for c in &filtered_contours {
                for i in 0..c.points.len() {
                    let p1 = c.points[i];
                    let p2 = c.points[(i + 1) % c.points.len()];
                    draw_line_segment_mut(
                        &mut canvas,
                        (p1.x as f32, p1.y as f32),
                        (p2.x as f32, p2.y as f32),
                        color,
                    );
                }
            }
            // For filter contours, we don't return partial data
            ProcessedResult {
                image: DynamicImage::ImageRgb8(canvas),
                contours: Vec::new(),
                perimeters: Vec::new(),
                child_counts: Vec::new(),
                sorted_indices: Vec::new(),
            }
        }
        VisualizerInstance::ConnectedComponents { n } => {
            let labels = connected_components(
                &binary,
                imageproc::region_labelling::Connectivity::Eight,
                Luma([0]),
            );
            // Background color black, labels colored
            let colored = draw_principal_connected_components(&labels, n, Rgba([0, 0, 0, 255]));
            ProcessedResult {
                image: DynamicImage::ImageRgba8(colored),
                contours: Vec::new(),
                perimeters: Vec::new(),
                child_counts: Vec::new(),
                sorted_indices: Vec::new(),
            }
        }
        VisualizerInstance::BoundingBox => {
            let contours = find_contours::<i32>(&binary);

            let mut canvas = img.to_rgb8();
            let red = Rgb([255, 0, 0]);
            // CHANGED: Blue -> Green
            let green = Rgb([0, 255, 0]);

            // Sort by size (descending)
            let width = img.width() as f64;
            let height = img.height() as f64;
            let total_area = width * height;

            // Use largest contour that covers < 90% of image area
            let selected_contour = contours
                .iter()
                .filter(|c| {
                    if c.points.len() < 10 {
                        return false;
                    }
                    let rect_points = min_area_rect(&c.points);
                    let aabb = to_axis_aligned_bounding_box(&rect_points);
                    let area = (aabb.width as f64) * (aabb.height as f64);
                    area < total_area * 0.90
                })
                .max_by_key(|c| c.points.len())
                .map(|c| {
                    let rect_points = min_area_rect(&c.points);
                    (c, rect_points)
                });

            if let Some((c, rect_points)) = selected_contour {
                // Draw the contour itself (Blue)
                let blue = Rgb([0, 0, 255]);
                for i in 0..c.points.len() {
                    let p1 = c.points[i];
                    let p2 = c.points[(i + 1) % c.points.len()];
                    draw_line_segment_mut(
                        &mut canvas,
                        (p1.x as f32, p1.y as f32),
                        (p2.x as f32, p2.y as f32),
                        blue,
                    );
                }

                // Draw OBB (Green)
                for i in 0..4 {
                    let p1 = rect_points[i];
                    let p2 = rect_points[(i + 1) % 4];
                    draw_line_segment_mut(
                        &mut canvas,
                        (p1.x as f32, p1.y as f32),
                        (p2.x as f32, p2.y as f32),
                        green,
                    );
                }

                // Draw AABB (Red)
                let aabb = to_axis_aligned_bounding_box(&rect_points);
                let rect_struct = imageproc::rect::Rect::at(aabb.x as i32, aabb.y as i32)
                    .of_size(aabb.width, aabb.height);
                draw_hollow_rect_mut(&mut canvas, rect_struct, red);
            }

            ProcessedResult {
                image: DynamicImage::ImageRgb8(canvas),
                contours: Vec::new(),
                perimeters: Vec::new(),
                child_counts: Vec::new(),
                sorted_indices: Vec::new(),
            }
        }
        VisualizerInstance::SortPerimeter { min_perimeter } => {
            let contours = find_contours::<i32>(&binary);

            // Calculate perimeters for ALL
            let perimeters: Vec<f64> = contours
                .iter()
                .map(|c| {
                    if c.points.len() < 2 {
                        return 0.0;
                    }
                    let mut p = 0.0;
                    for i in 0..c.points.len() {
                        let p1 = c.points[i];
                        let p2 = c.points[(i + 1) % c.points.len()];
                        let dx = (p1.x - p2.x) as f64;
                        let dy = (p1.y - p2.y) as f64;
                        p += dx.hypot(dy);
                    }
                    p
                })
                .collect();

            // Create sorted indices
            let mut indices: Vec<usize> = (0..contours.len()).collect();
            // Filter
            indices.retain(|&i| perimeters[i] >= min_perimeter as f64);
            // Sort
            indices.sort_by(|&a, &b| perimeters[b].partial_cmp(&perimeters[a]).unwrap());

            let mut canvas = img.to_rgb8();
            // Dim base drawing for filtered ones?
            let base_color = Rgb([50, 50, 80]);

            for &idx in &indices {
                if let Some(c) = contours.get(idx) {
                    for i in 0..c.points.len() {
                        let p1 = c.points[i];
                        let p2 = c.points[(i + 1) % c.points.len()];
                        draw_line_segment_mut(
                            &mut canvas,
                            (p1.x as f32, p1.y as f32),
                            (p2.x as f32, p2.y as f32),
                            base_color,
                        );
                    }
                }
            }

            ProcessedResult {
                image: DynamicImage::ImageRgb8(canvas),
                contours,   // Full contours
                perimeters, // Full perimeters
                child_counts: Vec::new(),
                sorted_indices: indices,
            }
        }
        VisualizerInstance::SortChildren => {
            let contours = find_contours::<i32>(&binary);

            // Calculate child counts
            let mut counts = vec![0; contours.len()];
            for c in &contours {
                if let Some(parent) = c.parent {
                    if let Some(cnt) = counts.get_mut(parent) {
                        *cnt += 1;
                    }
                }
            }

            // Sorted indices
            let mut indices: Vec<usize> = (0..contours.len()).collect();
            // Sort by count descending
            indices.sort_by(|&a, &b| counts[b].cmp(&counts[a]));
            // Let's filtered out 0 children to keep list clean
            indices.retain(|&i| counts[i] > 0);

            let mut canvas = img.to_rgb8();
            // Dim color for base layer (Same as SortPerimeter)
            let base_color = Rgb([50, 50, 80]);

            // Filtered indices loop (draw all parents)
            for &idx in &indices {
                if let Some(c) = contours.get(idx) {
                    for i in 0..c.points.len() {
                        let p1 = c.points[i];
                        let p2 = c.points[(i + 1) % c.points.len()];
                        draw_line_segment_mut(
                            &mut canvas,
                            (p1.x as f32, p1.y as f32),
                            (p2.x as f32, p2.y as f32),
                            base_color,
                        );
                    }
                }
            }

            ProcessedResult {
                image: DynamicImage::ImageRgb8(canvas),
                contours,
                perimeters: Vec::new(),
                child_counts: counts,
                sorted_indices: indices,
            }
        }
    }
}
