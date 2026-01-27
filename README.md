# image-debug-utils

[![Crates.io](https://img.shields.io/crates/v/image-debug-utils.svg)](https://crates.io/crates/image-debug-utils)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/bioinformatist/image-debug-utils)
[![Rust CI](https://github.com/bioinformatist/image-debug-utils/actions/workflows/ci.yml/badge.svg)](https://github.com/bioinformatist/image-debug-utils/actions/workflows/ci.yml)
[![Build and Deploy to Pages](https://github.com/bioinformatist/image-debug-utils/actions/workflows/pages.yml/badge.svg)](https://github.com/bioinformatist/image-debug-utils/actions/workflows/pages.yml)
[![codecov](https://codecov.io/gh/bioinformatist/image-debug-utils/graph/badge.svg?token=23U4M79DJH)](https://codecov.io/gh/bioinformatist/image-debug-utils)

Some niche but useful utilities for [`imageproc`](https://github.com/image-rs/imageproc).

## Usage

## Examples

### Visual Comparison

Demonstration of `remove_hypotenuse_owned` filtering out high-aspect-ratio artifacts (thin lines) from a synthetic shape.

| Before Processing | After `remove_hypotenuse_owned` |
| :---: | :---: |
| ![Before](assets/readme_before.png) | ![After](assets/readme_after.png) |
| *Original contours (Red) showing noisy thin lines* | *Processed contours (Green) with noise removed* |

### Feature Gallery

| Intelligent Bounding Boxes | Connected Components Labeling |
| :---: | :---: |
| ![Rects](assets/readme_rects.png) | ![Components](assets/readme_components.png) |
| *Converting Rotated Rects (Green) to AABB (Red)* | *Visualizing Top-5 connected regions with distinct colors* |

### Interactive Demo

Explore an interactive [WASM visualizer](examples/visualizer) to experiment with these utilities in real-time.
