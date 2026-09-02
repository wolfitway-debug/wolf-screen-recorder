use image::{DynamicImage, Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_line_segment_mut};
use imageproc::rect::Rect;
use std::path::PathBuf;
use slint::{Image, SharedPixelBuffer};

#[allow(dead_code)]
pub struct EditorEngine;

#[allow(dead_code)]
impl EditorEngine {
    /// Native Open File picker using rfd
    pub fn pick_image_file() -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
            .set_title("Select Image to Annotate")
            .pick_file()
    }

    /// Load image from path into RgbaImage
    pub fn load_image(path: &PathBuf) -> Result<RgbaImage, Box<dyn std::error::Error>> {
        let img = image::open(path)?;
        Ok(img.to_rgba8())
    }

    /// Convert RgbaImage directly to slint::Image buffer for live canvas rendering
    pub fn rgba_to_slint_image(rgba: &RgbaImage) -> Image {
        let buffer = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
            rgba.as_raw(),
            rgba.width(),
            rgba.height(),
        );
        Image::from_rgba8(buffer)
    }

    /// Copy RgbaImage to system clipboard using arboard
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

    /// Apply pixelated / blur redaction region over sensitive credentials
    pub fn apply_blur_redaction(img: &mut RgbaImage, x: i32, y: i32, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        let rect = Rect::at(x.max(0), y.max(0)).of_size(width, height);
        let dark_redact = Rgba([15, 23, 42, 220]);
        let accent_border = Rgba([0, 255, 102, 255]);

        draw_filled_rect_mut(img, rect, dark_redact);
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
