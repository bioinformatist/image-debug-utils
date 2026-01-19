//! `image-debug-utils` is a collection of niche but highly practical utilities designed to complement
//! the [`imageproc`] crate. It focuses on easing the debugging and visualization of common computer
//! vision tasks.
//!
//! To ensure a familiar developer experience, the modules and functions are organized to mirror
//! the structure of [`imageproc`] as closely as possible.
//!
//! # Main Modules
//!
//! *   [`contours`]: Utilities for working with contours found by `imageproc::contours`, including
//!     filtering by aspect ratio and hierarchical sorting.
//! *   [`rect`]: Tools for geometric primitives, such as converting rotated rectangle vertices
//!     into axis-aligned bounding boxes (useful for `imageproc::geometry::min_area_rect`).
//! *   [`region_labelling`]: Helpers for visualizing results from `imageproc::region_labelling`,
//!     such as coloring principal connected components.
//!
//! # Example: Filtering and Sorting Contours
//!
//! ```rust
//! use imageproc::contours::Contour;
//! use imageproc::contours::BorderType;
//! use image_debug_utils::contours::{remove_hypotenuse_in_place, sort_by_perimeters_owned};
//! let mut contours: Vec<Contour<i32>> = Vec::new(); // Dummy data
//! // 1. Remove thin, "hypotenuse-like" artifacts from contours
//! remove_hypotenuse_in_place(&mut contours, 5.0, None);
//!
//! // 2. Sort remaining contours by their perimeter (descending)
//! let sorted = sort_by_perimeters_owned(contours);
//! ```
//!
//! # Example: Visualizing Connected Components
//!
//! ```rust
//! use image_debug_utils::region_labelling::draw_principal_connected_components;
//! use image::{Rgba, ImageBuffer, Luma};
//! let labelled_image = ImageBuffer::<Luma<u32>, Vec<u32>>::new(10, 10); // Dummy data
//! // Draw the top 5 largest connected components with contrasting colors
//! let colored_image = draw_principal_connected_components(&labelled_image, 5, Rgba([0, 0, 0, 255]));
//! ```

mod colors;
pub mod contours;
pub mod rect;
pub mod region_labelling;
