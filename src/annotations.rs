use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_circle_mut, draw_hollow_circle_mut, draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use ab_glyph::{FontRef, PxScale};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ClickAnnotation {
    pub x: i32,
    pub y: i32,
    pub radius: i32,
    pub color: Rgba<u8>,
}

#[derive(Debug, Clone)]
pub struct BoundingBoxAnnotation {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub color: Rgba<u8>,
}

pub struct AnnotationEngine;

impl AnnotationEngine {
    /// Draw click highlight radius, bounding boxes, drop shadows, and brand watermark onto an image frame.
    pub fn apply_annotations(
        img: &mut RgbaImage,
        clicks: &[ClickAnnotation],
        boxes: &[BoundingBoxAnnotation],
        watermark_text: Option<&str>,
        logo_path: Option<&PathBuf>,
        position: &str,
    ) {
        // 1. Draw Bounding Boxes with subtle drop shadow effect
        for b in boxes {
            let shadow_color = Rgba([0, 0, 0, 120]);
            let shadow_rect = Rect::at(b.x + 3, b.y + 3).of_size(b.width, b.height);
            draw_hollow_rect_mut(img, shadow_rect, shadow_color);

            let main_rect = Rect::at(b.x, b.y).of_size(b.width, b.height);
            draw_hollow_rect_mut(img, main_rect, b.color);
        }

        // 2. Draw Click Radii
        for c in clicks {
            let outer_color = Rgba([c.color[0], c.color[1], c.color[2], 160]);
            let inner_color = Rgba([c.color[0], c.color[1], c.color[2], 230]);
            
            draw_hollow_circle_mut(img, (c.x, c.y), c.radius, outer_color);
            draw_hollow_circle_mut(img, (c.x, c.y), c.radius + 1, outer_color);
            draw_filled_circle_mut(img, (c.x, c.y), 4, inner_color);
        }

        // 3. Stamp Custom Image Logo if configured
        if let Some(path) = logo_path {
            if let Ok(logo_img) = image::open(path) {
                let logo_rgba = logo_img.to_rgba8();
                let (logo_w, logo_h) = logo_img.dimensions();

                let (offset_x, offset_y) = Self::calculate_corner_position(
                    img.width(),
                    img.height(),
                    logo_w,
                    logo_h,
                    position,
                );

                for x in 0..logo_w {
                    for y in 0..logo_h {
                        let target_x = offset_x + x;
                        let target_y = offset_y + y;
                        if target_x < img.width() && target_y < img.height() {
                            let logo_pixel = logo_rgba.get_pixel(x, y);
                            if logo_pixel[3] > 10 {
                                img.put_pixel(target_x, target_y, *logo_pixel);
                            }
                        }
                    }
                }
            }
        }

        // 4. Automated Text Watermarking
        if let Some(text) = watermark_text {
            let width = img.width();
            let height = img.height();

            let (margin_x, margin_y) = Self::calculate_corner_position(
                width,
                height,
                170,
                26,
                position,
            );

            let bg_rect = Rect::at(margin_x as i32 - 4, margin_y as i32 - 4).of_size(170, 26);
            let bg_color = Rgba([18, 22, 30, 200]);
            
            for x in bg_rect.left()..bg_rect.right() {
                for y in bg_rect.top()..bg_rect.bottom() {
                    if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
                        let pixel = img.get_pixel_mut(x as u32, y as u32);
                        pixel[0] = ((pixel[0] as u16 + bg_color[0] as u16) / 2) as u8;
                        pixel[1] = ((pixel[1] as u16 + bg_color[1] as u16) / 2) as u8;
                        pixel[2] = ((pixel[2] as u16 + bg_color[2] as u16) / 2) as u8;
                    }
                }
            }

            let font_data = include_bytes!("assets/Roboto-Regular.ttf");
            if let Ok(font) = FontRef::try_from_slice(font_data) {
                draw_text_mut(
                    img,
                    Rgba([0, 255, 102, 240]),
                    margin_x as i32,
                    margin_y as i32,
                    PxScale::from(16.0),
                    &font,
                    text,
                );
            }
        }
    }

    fn calculate_corner_position(
        img_w: u32,
        img_h: u32,
        elem_w: u32,
        elem_h: u32,
        position: &str,
    ) -> (u32, u32) {
        match position {
            "BottomLeft" => (20, img_h.saturating_sub(elem_h + 20)),
            "TopRight" => (img_w.saturating_sub(elem_w + 20), 20),
            "TopLeft" => (20, 20),
            _ => (img_w.saturating_sub(elem_w + 20), img_h.saturating_sub(elem_h + 20)), // BottomRight default
        }
    }

    pub fn process_snapshot_with_watermark(
        dyn_img: &mut DynamicImage,
        watermark: &str,
    ) {
        let mut rgba = dyn_img.to_rgba8();
        let default_click = ClickAnnotation {
            x: (rgba.width() / 2) as i32,
            y: (rgba.height() / 2) as i32,
            radius: 24,
            color: Rgba([0, 255, 102, 255]),
        };

        Self::apply_annotations(&mut rgba, &[default_click], &[], Some(watermark), None, "BottomRight");
        *dyn_img = DynamicImage::ImageRgba8(rgba);
    }
}
