use image::{DynamicImage, Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_line_segment_mut};
use imageproc::rect::Rect;
use std::path::PathBuf;

#[allow(dead_code)]
pub struct EditorEngine;

#[allow(dead_code)]
impl EditorEngine {
    /// Apply pixelated / blur redaction region over sensitive credentials
    pub fn apply_blur_redaction(img: &mut RgbaImage, x: i32, y: i32, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        let rect = Rect::at(x.max(0), y.max(0)).of_size(width, height);
        let dark_redact = Rgba([15, 23, 42, 220]);
        let accent_border = Rgba([0, 255, 102, 255]);

        // Draw solid dark redaction box
        draw_filled_rect_mut(img, rect, dark_redact);
        // Draw crisp accent border around redacted credential zone
        draw_hollow_rect_mut(img, rect, accent_border);
    }

    /// Apply vector arrow annotation
    pub fn draw_arrow(img: &mut RgbaImage, start_x: f32, start_y: f32, end_x: f32, end_y: f32) {
        let arrow_color = Rgba([0, 255, 102, 255]);
        draw_line_segment_mut(
            img,
            (start_x, start_y),
            (end_x, end_y),
            arrow_color,
        );
    }

    /// Save processed image from editor to disk
    pub fn export_edited_snapshot(dyn_img: &DynamicImage, output_dir: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut path = PathBuf::from(output_dir);
        std::fs::create_dir_all(&path)?;
        let filename = format!("showcase_export_{}.png", chrono::Utc::now().timestamp());
        path.push(filename);

        dyn_img.save(&path)?;
        Ok(path)
    }
}
