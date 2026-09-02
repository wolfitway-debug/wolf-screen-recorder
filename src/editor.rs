use image::{DynamicImage, Rgba, RgbaImage};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_filled_rect_mut,
    draw_hollow_ellipse_mut, draw_hollow_rect_mut, draw_line_segment_mut, draw_text_mut,
};
use imageproc::rect::Rect;
use ab_glyph::{FontRef, PxScale};
use std::path::PathBuf;
use slint::{Image, SharedPixelBuffer};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AnnotationShape {
    Arrow { start: (f32, f32), end: (f32, f32), color: Rgba<u8>, stroke: f32 },
    TextCallout { x: i32, y: i32, text: String, color: Rgba<u8> },
    RedactBox { x: i32, y: i32, width: u32, height: u32 },
    HighlightBox { x: i32, y: i32, width: u32, height: u32, color: Rgba<u8>, stroke: f32 },
    Oval { x: i32, y: i32, width: u32, height: u32, color: Rgba<u8>, stroke: f32 },
    StepNumber { x: i32, y: i32, num: u32, color: Rgba<u8> },
    Freehand { points: Vec<(f32, f32)>, color: Rgba<u8>, stroke: f32 },
    Spotlight { x: i32, y: i32, width: u32, height: u32 },
}

#[derive(Debug, Clone)]
pub struct AnnotationStack {
    shapes: Vec<AnnotationShape>,
    pub step_counter: u32,
}

impl Default for AnnotationStack {
    fn default() -> Self {
        Self {
            shapes: Vec::new(),
            step_counter: 0,
        }
    }
}

impl AnnotationStack {
    pub fn push(&mut self, shape: AnnotationShape) {
        if matches!(shape, AnnotationShape::StepNumber { .. }) {
            self.step_counter += 1;
        }
        self.shapes.push(shape);
    }

    pub fn undo(&mut self) -> Option<AnnotationShape> {
        if let Some(shape) = self.shapes.pop() {
            if matches!(shape, AnnotationShape::StepNumber { .. }) {
                self.step_counter = self.step_counter.saturating_sub(1);
            }
            Some(shape)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.shapes.clear();
        self.step_counter = 0;
    }

    pub fn next_step(&self) -> u32 {
        self.step_counter + 1
    }

    pub fn render_shapes(&self, base_rgba: &RgbaImage) -> RgbaImage {
        let mut img = base_rgba.clone();
        for shape in &self.shapes {
            match shape {
                AnnotationShape::Arrow { start, end, color, stroke } => {
                    let thickness = (*stroke as i32).max(1);
                    for t in 0..thickness {
                        let offset = t as f32 - thickness as f32 / 2.0;
                        draw_line_segment_mut(&mut img, (start.0 + offset, start.1), (end.0 + offset, end.1), *color);
                        draw_line_segment_mut(&mut img, (start.0, start.1 + offset), (end.0, end.1 + offset), *color);
                    }
                    let dx = end.0 - start.0;
                    let dy = end.1 - start.1;
                    let len = (dx * dx + dy * dy).sqrt().max(0.001);
                    let ux = dx / len;
                    let uy = dy / len;
                    let arrow_len = (14.0 + thickness as f32 * 2.0).min(len * 0.5);
                    let tip1 = (end.0 - arrow_len * (ux - uy * 0.5), end.1 - arrow_len * (uy + ux * 0.5));
                    let tip2 = (end.0 - arrow_len * (ux + uy * 0.5), end.1 - arrow_len * (uy - ux * 0.5));
                    draw_line_segment_mut(&mut img, *end, tip1, *color);
                    draw_line_segment_mut(&mut img, *end, tip2, *color);
                    for frac in 0..=10 {
                        let f = frac as f32 / 10.0;
                        let mid = (tip1.0 + f * (tip2.0 - tip1.0), tip1.1 + f * (tip2.1 - tip1.1));
                        draw_line_segment_mut(&mut img, *end, mid, *color);
                    }
                }

                AnnotationShape::TextCallout { x, y, text, color } => {
                    let display_text = if text.trim().is_empty() { "Callout" } else { text.as_str() };
                    let font_data = include_bytes!("assets/Roboto-Regular.ttf");
                    let char_count = display_text.chars().count().max(1);
                    let text_width = (char_count as i32 * 9 + 20).min(400);
                    let bg_rect = Rect::at((*x - 8).max(0), (*y - 8).max(0)).of_size(text_width as u32, 34);
                    draw_filled_rect_mut(&mut img, bg_rect, Rgba([8, 12, 20, 235]));
                    draw_hollow_rect_mut(&mut img, bg_rect, *color);
                    if let Ok(font) = FontRef::try_from_slice(font_data) {
                        draw_text_mut(&mut img, *color, *x, *y, PxScale::from(16.0), &font, display_text);
                    }
                }

                AnnotationShape::RedactBox { x, y, width, height } => {
                    if *width > 0 && *height > 0 {
                        let x0 = (*x).max(0);
                        let y0 = (*y).max(0);
                        let x2 = (x + *width as i32).min(img.width() as i32);
                        let y2 = (y + *height as i32).min(img.height() as i32);
                        if x2 > x0 && y2 > y0 {
                            let block = 10u32;
                            let bx = x0 as u32;
                            let by = y0 as u32;
                            let bw = (x2 - x0) as u32;
                            let bh = (y2 - y0) as u32;
                            let mut px = bx;
                            while px < bx + bw && px < img.width() {
                                let mut py = by;
                                while py < by + bh && py < img.height() {
                                    let sample = *img.get_pixel(px.min(img.width()-1), py.min(img.height()-1));
                                    let ex = (px + block).min(bx + bw).min(img.width());
                                    let ey = (py + block).min(by + bh).min(img.height());
                                    for dx in px..ex {
                                        for dy in py..ey {
                                            img.put_pixel(dx, dy, sample);
                                        }
                                    }
                                    py += block;
                                }
                                px += block;
                            }
                            let rect = Rect::at(x0, y0).of_size(bw, bh);
                            draw_hollow_rect_mut(&mut img, rect, Rgba([34, 212, 94, 220]));
                        }
                    }
                }

                AnnotationShape::HighlightBox { x, y, width, height, color, stroke } => {
                    if *width > 0 && *height > 0 {
                        let thickness = (*stroke as i32).max(1);
                        for t in 0..thickness {
                            let adjusted_x = (x + t).max(0);
                            let adjusted_y = (y + t).max(0);
                            let adjusted_w = (*width as i32 - t * 2).max(1) as u32;
                            let adjusted_h = (*height as i32 - t * 2).max(1) as u32;
                            let rect = Rect::at(adjusted_x, adjusted_y).of_size(adjusted_w, adjusted_h);
                            draw_hollow_rect_mut(&mut img, rect, *color);
                        }
                    }
                }

                AnnotationShape::Oval { x, y, width, height, color, stroke } => {
                    if *width > 0 && *height > 0 {
                        let cx = x + (*width as i32 / 2);
                        let cy = y + (*height as i32 / 2);
                        let rx = ((*width as i32) / 2).max(1);
                        let ry = ((*height as i32) / 2).max(1);
                        let thickness = (*stroke as i32).max(1);
                        for t in 0..thickness {
                            draw_hollow_ellipse_mut(&mut img, (cx, cy), (rx - t).max(1), (ry - t).max(1), *color);
                        }
                    }
                }

                AnnotationShape::StepNumber { x, y, num, color } => {
                    let radius = 18i32;
                    draw_filled_circle_mut(&mut img, (*x, *y), radius, *color);
                    draw_hollow_ellipse_mut(&mut img, (*x, *y), radius + 1, radius + 1, Rgba([255, 255, 255, 240]));
                    let font_data = include_bytes!("assets/Roboto-Regular.ttf");
                    if let Ok(font) = FontRef::try_from_slice(font_data) {
                        let label = num.to_string();
                        let offset_x = if *num >= 10 { 10 } else { 5 };
                        draw_text_mut(
                            &mut img,
                            Rgba([0, 0, 0, 255]),
                            x - offset_x,
                            y - 9,
                            PxScale::from(20.0),
                            &font,
                            &label,
                        );
                    }
                }

                AnnotationShape::Freehand { points, color, stroke } => {
                    let thickness = (*stroke as i32).max(1);
                    if points.len() >= 2 {
                        for window in points.windows(2) {
                            let p1 = window[0];
                            let p2 = window[1];
                            for t in 0..thickness {
                                let offset = t as f32 - thickness as f32 / 2.0;
                                draw_line_segment_mut(&mut img, (p1.0 + offset, p1.1), (p2.0 + offset, p2.1), *color);
                                draw_line_segment_mut(&mut img, (p1.0, p1.1 + offset), (p2.0, p2.1 + offset), *color);
                            }
                        }
                    } else if let Some(p) = points.first() {
                        draw_filled_circle_mut(&mut img, (p.0 as i32, p.1 as i32), (thickness / 2).max(1), *color);
                    }
                }

                AnnotationShape::Spotlight { x, y, width, height } => {
                    let img_w = img.width();
                    let img_h = img.height();
                    let x1 = (*x).max(0) as u32;
                    let y1 = (*y).max(0) as u32;
                    let x2 = (*x + *width as i32).min(img_w as i32) as u32;
                    let y2 = (*y + *height as i32).min(img_h as i32) as u32;
                    for px in 0..img_w {
                        for py in 0..img_h {
                            let in_spotlight = px >= x1 && px < x2 && py >= y1 && py < y2;
                            if !in_spotlight {
                                let pixel = img.get_pixel_mut(px, py);
                                pixel[0] = (pixel[0] as f32 * 0.25) as u8;
                                pixel[1] = (pixel[1] as f32 * 0.25) as u8;
                                pixel[2] = (pixel[2] as f32 * 0.25) as u8;
                            }
                        }
                    }
                    if x2 > x1 && y2 > y1 {
                        let rect = Rect::at(x1 as i32, y1 as i32).of_size(x2 - x1, y2 - y1);
                        draw_hollow_rect_mut(&mut img, rect, Rgba([255, 255, 255, 220]));
                    }
                }
            }
        }
        img
    }
}

pub fn parse_color_hex(hex: &str) -> Rgba<u8> {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return Rgba([34, 212, 94, 255]); // fallback: wolf green
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(34);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(212);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(94);
    Rgba([r, g, b, 255])
}

pub fn stroke_size_to_px(size: i32) -> f32 {
    match size {
        1 => 2.0,
        3 => 8.0,
        _ => 4.0,
    }
}

#[allow(dead_code)]
pub struct EditorEngine;

#[allow(dead_code)]
impl EditorEngine {
    pub fn pick_image_file() -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
            .set_title("Open Image for Annotation")
            .pick_file()
    }

    pub fn pick_logo_file() -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Logo Images", &["png", "jpg", "jpeg", "webp"])
            .set_title("Select Custom Watermark Logo")
            .pick_file()
    }

    pub fn load_image(path: &PathBuf) -> Result<RgbaImage, Box<dyn std::error::Error>> {
        let img = image::open(path)?;
        Ok(img.to_rgba8())
    }

    pub fn rgba_to_slint_image(rgba: &RgbaImage) -> Image {
        let buffer = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
            rgba.as_raw(),
            rgba.width(),
            rgba.height(),
        );
        Image::from_rgba8(buffer)
    }

    pub fn copy_to_clipboard(rgba: &RgbaImage) -> Result<(), Box<dyn std::error::Error>> {
        let mut clipboard = arboard::Clipboard::new()?;
        let img_data = arboard::ImageData {
            width: rgba.width() as usize,
            height: rgba.height() as usize,
            bytes: std::borrow::Cow::Borrowed(rgba.as_raw()),
        };
        clipboard.set_image(img_data)?;
        println!("[EditorEngine] Copied annotated snapshot to system clipboard!");
        Ok(())
    }

    pub fn export_edited_snapshot(
        dyn_img: &DynamicImage,
        output_dir: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut path = PathBuf::from(output_dir);
        std::fs::create_dir_all(&path)?;
        let filename = format!("showcase_export_{}.png", chrono::Utc::now().timestamp());
        path.push(filename);
        dyn_img.save(&path)?;
        Ok(path)
    }
}
