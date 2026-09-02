use image::{DynamicImage, Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_line_segment_mut, draw_text_mut};
use imageproc::rect::Rect;
use ab_glyph::{FontRef, PxScale};
use std::path::PathBuf;
use slint::{Image, SharedPixelBuffer};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AnnotationShape {
    Arrow { start: (f32, f32), end: (f32, f32), color: Rgba<u8> },
    TextCallout { x: i32, y: i32, text: String, color: Rgba<u8> },
    RedactBox { x: i32, y: i32, width: u32, height: u32 },
    HighlightBox { x: i32, y: i32, width: u32, height: u32, color: Rgba<u8> },
}

#[derive(Debug, Clone, Default)]
pub struct AnnotationStack {
    shapes: Vec<AnnotationShape>,
}

impl AnnotationStack {
    pub fn push(&mut self, shape: AnnotationShape) {
        self.shapes.push(shape);
    }

    pub fn undo(&mut self) -> Option<AnnotationShape> {
        self.shapes.pop()
    }

    pub fn clear(&mut self) {
        self.shapes.clear();
    }

    pub fn render_shapes(&self, base_rgba: &RgbaImage) -> RgbaImage {
        let mut img = base_rgba.clone();
        for shape in &self.shapes {
            match shape {
                AnnotationShape::Arrow { start, end, color } => {
                    draw_line_segment_mut(&mut img, *start, *end, *color);
                    // Draw arrowhead tip
                    let dx = end.0 - start.0;
                    let dy = end.1 - start.1;
                    let angle = dy.atan2(dx);
                    let arrow_len = 12.0f32;
                    let tip1 = (
                        end.0 - arrow_len * (angle - 0.4).cos(),
                        end.1 - arrow_len * (angle - 0.4).sin(),
                    );
                    let tip2 = (
                        end.0 - arrow_len * (angle + 0.4).cos(),
                        end.1 - arrow_len * (angle + 0.4).sin(),
                    );
                    draw_line_segment_mut(&mut img, *end, tip1, *color);
                    draw_line_segment_mut(&mut img, *end, tip2, *color);
                }
                AnnotationShape::TextCallout { x, y, text, color } => {
                    let bg_rect = Rect::at(*x - 4, *y - 4).of_size(140, 24);
                    draw_filled_rect_mut(&mut img, bg_rect, Rgba([15, 23, 42, 220]));
                    draw_hollow_rect_mut(&mut img, bg_rect, *color);

                    let font_data = include_bytes!("assets/Roboto-Regular.ttf");
                    if let Ok(font) = FontRef::try_from_slice(font_data) {
                        draw_text_mut(
                            &mut img,
                            *color,
                            *x,
                            *y,
                            PxScale::from(14.0),
                            &font,
                            text,
                        );
                    }
                }
                AnnotationShape::RedactBox { x, y, width, height } => {
                    if *width > 0 && *height > 0 {
                        let rect = Rect::at((*x).max(0), (*y).max(0)).of_size(*width, *height);
                        draw_filled_rect_mut(&mut img, rect, Rgba([15, 23, 42, 230]));
                        draw_hollow_rect_mut(&mut img, rect, Rgba([0, 255, 102, 255]));
                    }
                }
                AnnotationShape::HighlightBox { x, y, width, height, color } => {
                    if *width > 0 && *height > 0 {
                        let rect = Rect::at((*x).max(0), (*y).max(0)).of_size(*width, *height);
                        draw_hollow_rect_mut(&mut img, rect, *color);
                    }
                }
            }
        }
        img
    }
}

#[allow(dead_code)]
pub struct EditorEngine;

#[allow(dead_code)]
impl EditorEngine {
    pub fn pick_image_file() -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
            .set_title("Select Image to Annotate")
            .pick_file()
    }

    pub fn pick_logo_file() -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Logos", &["png", "jpg", "jpeg", "webp"])
            .set_title("Select Custom Watermark Logo Image")
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
        println!("[EditorEngine] Successfully copied snapshot to system clipboard!");
        Ok(())
    }

    pub fn export_edited_snapshot(dyn_img: &DynamicImage, output_dir: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut path = PathBuf::from(output_dir);
        std::fs::create_dir_all(&path)?;
        let filename = format!("showcase_export_{}.png", chrono::Utc::now().timestamp());
        path.push(filename);

        dyn_img.save(&path)?;
        Ok(path)
    }
}
