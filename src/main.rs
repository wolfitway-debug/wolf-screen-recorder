mod capture;
mod audio;
mod webcam;
mod hardware;
mod annotations;
mod focus;
mod config;
mod i18n;
mod editor;
mod hotkeys;
mod region;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::process::Command;
use image::RgbaImage;

use hardware::HardwareProfile;
use focus::FocusTracker;
use config::AppConfig;
use i18n::I18nEngine;
use editor::{AnnotationShape, AnnotationStack, EditorEngine, parse_color_hex, stroke_size_to_px};
use hotkeys::{HotkeyCommand, HotkeyDaemon};
use region::SelectedRegion;

slint::include_modules!();

fn sync_i18n_ui(ui: &AppWindow, i18n: &I18nEngine) {
    ui.set_txt_mode_desktop(i18n.t("mode_desktop").into());
    ui.set_txt_mode_window(i18n.t("mode_window").into());
    ui.set_txt_mode_region(i18n.t("mode_region").into());
    ui.set_txt_btn_snapshot(i18n.t("btn_snapshot").into());
    ui.set_txt_btn_record(i18n.t("btn_record").into());
    ui.set_txt_btn_open(i18n.t("btn_open").into());
    ui.set_txt_btn_studio(i18n.t("btn_studio").into());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;

    // Active image buffer & annotation stack for Studio Editor
    let active_base_image: Arc<Mutex<Option<RgbaImage>>> = Arc::new(Mutex::new(None));
    let active_annotation_stack: Arc<Mutex<AnnotationStack>> = Arc::new(Mutex::new(AnnotationStack::default()));

    // Active PIP webcam window handle
    let active_pip_window: Arc<Mutex<Option<WebcamPipWindow>>> = Arc::new(Mutex::new(None));

    // Load persistent configuration and i18n localization engine
    let config = Arc::new(Mutex::new(AppConfig::load()));
    let i18n = Arc::new(Mutex::new(I18nEngine::new()));

    // Apply initial config to UI
    {
        let cfg = config.lock().unwrap();
        ui.set_primary_mode(cfg.primary_mode.as_str().into());
        ui.set_lang(cfg.language.as_str().into());
        ui.set_active_encoder(cfg.hw_encoder_override.as_str().into());
        ui.set_save_path(cfg.save_directory.as_str().into());
        ui.set_watermark_text(cfg.watermark_text.as_str().into());
        if let Some(logo) = &cfg.watermark_logo_path {
            ui.set_watermark_logo_path(logo.as_str().into());
        }
        ui.set_watermark_position(cfg.watermark_position.as_str().into());
        ui.set_watermark_enabled(cfg.auto_watermark);
        ui.set_cinematic_zoom_enabled(cfg.cinematic_zoom);
        ui.set_hotkey_record(cfg.hotkey_toggle_record.as_str().into());
        ui.set_hotkey_snapshot(cfg.hotkey_snapshot.as_str().into());
        ui.set_hotkey_region(cfg.hotkey_region_select.as_str().into());
        ui.set_hotkey_cancel(cfg.hotkey_cancel.as_str().into());

        let mut i18n_lock = i18n.lock().unwrap();
        i18n_lock.set_language(&cfg.language);
        sync_i18n_ui(&ui, &i18n_lock);
    }

    // Initialize Hardware Auto-Detection Engine
    let hw_profile = HardwareProfile::detect();
    ui.set_hw_encoder_tag(hw_profile.encoder.tag().into());

    let initial_status = {
        let i = i18n.lock().unwrap();
        format!("{} ({})", i.t("status_ready"), hw_profile.encoder.display_name())
    };
    ui.set_status_message(initial_status.into());

    let focus_tracker = FocusTracker::new();

    // Module 1: Start Global System-Wide Hotkey Daemon thread
    let (hotkey_tx, hotkey_rx) = channel::<HotkeyCommand>();
    HotkeyDaemon::start(hotkey_tx);

    // Cross-thread Hotkey Command Dispatcher
    let ui_weak_hotkey = ui.as_weak();
    std::thread::spawn(move || {
        while let Ok(cmd) = hotkey_rx.recv() {
            let ui_weak = ui_weak_hotkey.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    match cmd {
                        HotkeyCommand::ToggleRecord => {
                            ui.invoke_toggle_recording();
                        }
                        HotkeyCommand::Snapshot => {
                            ui.invoke_take_snapshot();
                            ui.invoke_open_studio_editor();
                        }
                        HotkeyCommand::RegionSelect => {
                            ui.invoke_open_region_selector();
                        }
                        HotkeyCommand::Cancel => {
                            ui.set_show_settings_modal(false);
                            ui.set_show_editor_modal(false);
                        }
                    }
                }
            });
        }
    });

    // Primary Mode Switcher Handler (Image Snap vs Video Rec)
    let config_clone = config.clone();
    let ui_handle = ui.as_weak();
    ui.on_select_primary_mode(move |mode_str| {
        let ui = ui_handle.unwrap();
        ui.set_primary_mode(mode_str.clone());
        let mut cfg = config_clone.lock().unwrap();
        cfg.primary_mode = mode_str.as_str().to_string();
        cfg.save();
        println!("[Main] Switched primary mode to: {}", mode_str);
    });

    // Window position dragging handler
    let ui_handle = ui.as_weak();
    ui.on_move_window(move |delta_x, delta_y| {
        let ui = ui_handle.unwrap();
        let current_pos = ui.window().position();
        let new_x = current_pos.x + (delta_x as i32);
        let new_y = current_pos.y + (delta_y as i32);
        ui.window().set_position(slint::PhysicalPosition::new(new_x, new_y));
    });

    // Fullscreen Rubber-Band Region Selector Handler
    let ui_handle = ui.as_weak();
    ui.on_open_region_selector(move || {
        let ui = ui_handle.unwrap();
        ui.set_selected_mode("region".into());

        if let Ok(region_win) = RegionSelectorWindow::new() {
            // Get monitor resolution & live screen snapshot via xcap
            if let Ok(monitors) = xcap::Monitor::all() {
                if let Some(m) = monitors.first() {
                    let w = m.width();
                    let h = m.height();
                    region_win.set_monitor_w(w as i32);
                    region_win.set_monitor_h(h as i32);
                    region_win.window().set_size(slint::PhysicalSize::new(w, h));

                    if let Ok(rgba_img) = m.capture_image() {
                        let width = rgba_img.width();
                        let height = rgba_img.height();
                        let raw = rgba_img.into_raw();
                        let pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                            &raw, width, height
                        );
                        region_win.set_screen_snapshot(slint::Image::from_rgba8(pixel_buffer));
                        region_win.set_has_snapshot(true);
                    }
                }
            }

            let region_win_weak = region_win.as_weak();
            let ui_main_weak = ui.as_weak();
            region_win.on_selection_confirmed(move |x, y, w, h| {
                region::set_active_region(Some(SelectedRegion::new(x, y, w as u32, h as u32)));
                if let Some(main_ui) = ui_main_weak.upgrade() {
                    main_ui.set_active_region_badge(format!("({}×{})", w, h).into());
                    main_ui.set_status_message(format!("✂️ Region: {}×{} px", w, h).into());
                }
                if let Some(r_win) = region_win_weak.upgrade() {
                    let _ = r_win.hide();
                }
            });

            let region_win_cancel_weak = region_win.as_weak();
            let ui_main_weak_cancel = ui.as_weak();
            region_win.on_selection_cancelled(move || {
                region::set_active_region(None);
                if let Some(main_ui) = ui_main_weak_cancel.upgrade() {
                    main_ui.set_active_region_badge("".into());
                    main_ui.set_selected_mode("desktop".into());
                }
                if let Some(r_win) = region_win_cancel_weak.upgrade() {
                    let _ = r_win.hide();
                }
            });

            let _ = region_win.show();
        }
    });

    // Clear Region Callback Handler
    let ui_handle = ui.as_weak();
    ui.on_clear_region(move || {
        let ui = ui_handle.unwrap();
        region::set_active_region(None);
        ui.set_active_region_badge("".into());
        ui.set_status_message("Region cleared (Desktop Mode)".into());
        ui.set_selected_mode("desktop".into());
    });

    // Window Picker Handler — queries open windows dynamically
    let ui_handle = ui.as_weak();
    ui.on_open_window_picker(move || {
        let ui = ui_handle.unwrap();
        ui.set_show_window_picker(true);
        if let Ok(windows) = xcap::Window::all() {
            let mut list: Vec<WindowItem> = Vec::new();
            for win in windows {
                let title = win.title().to_string();
                let app = win.app_name().to_string();
                let x = win.x();
                let y = win.y();
                let w = win.width() as i32;
                let h = win.height() as i32;
                if w > 50 && h > 50 && !title.is_empty() {
                    list.push(WindowItem {
                        title: title.into(),
                        app_name: app.into(),
                        x,
                        y,
                        width: w,
                        height: h,
                    });
                }
            }
            if !list.is_empty() {
                let model = std::rc::Rc::new(slint::VecModel::from(list));
                ui.set_window_list(model.into());
            }
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_select_target_window(move |x, y, w, h, title| {
        let ui = ui_handle.unwrap();
        region::set_active_region(Some(SelectedRegion::new(x, y, w as u32, h as u32)));
        ui.set_status_message(format!("⊞ Window: {}", title).into());
        ui.set_show_window_picker(false);
    });

    // Language Toggle / Cycle Handler
    let i18n_clone = i18n.clone();
    let config_clone = config.clone();
    let ui_handle = ui.as_weak();
    ui.on_toggle_language(move || {
        let ui = ui_handle.unwrap();
        let current = ui.get_lang();
        let next = match current.as_str() {
            "en" => "ro",
            "ro" => "es",
            "es" => "de",
            "de" => "fr",
            "fr" => "ja",
            _ => "en",
        };
        ui.set_lang(next.into());
        let mut i18n_lock = i18n_clone.lock().unwrap();
        i18n_lock.set_language(next);
        sync_i18n_ui(&ui, &i18n_lock);
        config_clone.lock().unwrap().language = next.to_string();
        config_clone.lock().unwrap().save();

        let status_msg = i18n_lock.t("status_ready");
        ui.set_status_message(status_msg.into());
    });

    // Select specific language from Settings modal dropdown
    let i18n_clone = i18n.clone();
    let config_clone = config.clone();
    let ui_handle = ui.as_weak();
    ui.on_select_language(move |lang_code| {
        let ui = ui_handle.unwrap();
        let lang_str = lang_code.as_str();
        ui.set_lang(lang_str.into());
        let mut i18n_lock = i18n_clone.lock().unwrap();
        i18n_lock.set_language(lang_str);
        sync_i18n_ui(&ui, &i18n_lock);
        config_clone.lock().unwrap().language = lang_str.to_string();
        config_clone.lock().unwrap().save();
    });

    // Encoder Selection Override from Settings modal
    let config_clone = config.clone();
    let ui_handle = ui.as_weak();
    ui.on_select_encoder(move |enc_name| {
        let ui = ui_handle.unwrap();
        ui.set_active_encoder(enc_name.clone());
        config_clone.lock().unwrap().hw_encoder_override = enc_name.as_str().to_string();
    });

    // Save Config Callback
    let config_clone = config.clone();
    let ui_handle = ui.as_weak();
    ui.on_save_config(move || {
        let ui = ui_handle.unwrap();
        let mut cfg = config_clone.lock().unwrap();
        cfg.language = ui.get_lang().as_str().to_string();
        cfg.hw_encoder_override = ui.get_active_encoder().as_str().to_string();
        cfg.save_directory = ui.get_save_path().as_str().to_string();
        cfg.watermark_text = ui.get_watermark_text().as_str().to_string();
        let logo_str = ui.get_watermark_logo_path().as_str().to_string();
        cfg.watermark_logo_path = if logo_str.is_empty() { None } else { Some(logo_str) };
        cfg.watermark_position = ui.get_watermark_position().as_str().to_string();
        cfg.auto_watermark = ui.get_watermark_enabled();
        cfg.cinematic_zoom = ui.get_cinematic_zoom_enabled();
        cfg.hotkey_toggle_record = ui.get_hotkey_record().as_str().to_string();
        cfg.hotkey_snapshot = ui.get_hotkey_snapshot().as_str().to_string();
        cfg.hotkey_region_select = ui.get_hotkey_region().as_str().to_string();
        cfg.hotkey_cancel = ui.get_hotkey_cancel().as_str().to_string();
        cfg.save();
    });

    // Custom Watermark Logo Upload Handler
    let config_clone = config.clone();
    let ui_handle = ui.as_weak();
    ui.on_upload_logo_dialog(move || {
        let ui = ui_handle.unwrap();
        if let Some(logo_path) = EditorEngine::pick_logo_file() {
            let path_str = logo_path.to_string_lossy().to_string();
            ui.set_watermark_logo_path(path_str.as_str().into());
            let mut cfg = config_clone.lock().unwrap();
            cfg.watermark_logo_path = Some(path_str.clone());
            cfg.save();
            println!("[Config] Custom Watermark Logo set to: {}", path_str);
        }
    });

    // Clear Custom Watermark Logo Handler
    let config_clone = config.clone();
    let ui_handle = ui.as_weak();
    ui.on_clear_logo(move || {
        let ui = ui_handle.unwrap();
        ui.set_watermark_logo_path("".into());
        let mut cfg = config_clone.lock().unwrap();
        cfg.watermark_logo_path = None;
        cfg.save();
        println!("[Config] Custom Watermark Logo cleared.");
    });

    // Open External Image File Picker Handler
    let base_img_clone = active_base_image.clone();
    let stack_clone = active_annotation_stack.clone();
    let ui_handle = ui.as_weak();
    ui.on_open_file_dialog(move || {
        let ui = ui_handle.unwrap();
        if let Some(path) = EditorEngine::pick_image_file() {
            if let Ok(rgba) = EditorEngine::load_image(&path) {
                stack_clone.lock().unwrap().clear();
                let slint_img = EditorEngine::rgba_to_slint_image(&rgba);
                ui.set_canvas_image(slint_img);
                ui.set_has_canvas_image(true);
                ui.set_show_editor_modal(true);
                *base_img_clone.lock().unwrap() = Some(rgba);
                println!("[EditorEngine] Loaded image file from disk: {:?}", path);
            }
        }
    });

    // Copy Snapshot to Clipboard Handler
    let base_img_clone = active_base_image.clone();
    let stack_clone = active_annotation_stack.clone();
    let ui_handle = ui.as_weak();
    ui.on_copy_clipboard(move || {
        let ui = ui_handle.unwrap();
        if let Some(base) = base_img_clone.lock().unwrap().as_ref() {
            let rendered = stack_clone.lock().unwrap().render_shapes(base);
            if let Ok(()) = EditorEngine::copy_to_clipboard(&rendered) {
                ui.set_status_message("Copied snapshot to clipboard!".into());
            }
        }
    });

    // Undo Annotation Handler
    let base_img_clone = active_base_image.clone();
    let stack_clone = active_annotation_stack.clone();
    let ui_handle = ui.as_weak();
    ui.on_undo_annotation(move || {
        let ui = ui_handle.unwrap();
        stack_clone.lock().unwrap().undo();
        if let Some(base) = base_img_clone.lock().unwrap().as_ref() {
            let rendered = stack_clone.lock().unwrap().render_shapes(base);
            let slint_img = EditorEngine::rgba_to_slint_image(&rendered);
            ui.set_canvas_image(slint_img);
        }
    });

    // Clear Annotations Handler
    let base_img_clone = active_base_image.clone();
    let stack_clone = active_annotation_stack.clone();
    let ui_handle = ui.as_weak();
    ui.on_clear_annotations(move || {
        let ui = ui_handle.unwrap();
        stack_clone.lock().unwrap().clear();
        if let Some(base) = base_img_clone.lock().unwrap().as_ref() {
            let slint_img = EditorEngine::rgba_to_slint_image(base);
            ui.set_canvas_image(slint_img);
        }
    });

    // Open Studio Editor Handler
    let config_clone = config.clone();
    let base_img_clone = active_base_image.clone();
    let stack_clone = active_annotation_stack.clone();
    let ui_handle = ui.as_weak();
    ui.on_open_studio_editor(move || {
        let ui = ui_handle.unwrap();
        if base_img_clone.lock().unwrap().is_none() {
            let cfg = config_clone.lock().unwrap();
            let wm_text = if ui.get_watermark_enabled() { Some(cfg.watermark_text.as_str()) } else { None };
            let logo_pb = if ui.get_watermark_enabled() { cfg.watermark_logo_path.as_ref().map(std::path::PathBuf::from) } else { None };
            let pos = cfg.watermark_position.as_str();

            if let Ok(path) = capture::CaptureEngine::save_screenshot(wm_text, logo_pb.as_ref(), pos) {
                if let Ok(rgba) = EditorEngine::load_image(&path) {
                    stack_clone.lock().unwrap().clear();
                    let slint_img = EditorEngine::rgba_to_slint_image(&rgba);
                    ui.set_canvas_image(slint_img);
                    ui.set_has_canvas_image(true);
                    *base_img_clone.lock().unwrap() = Some(rgba);
                }
            }
        } else if let Some(base) = base_img_clone.lock().unwrap().as_ref() {
            let rendered = stack_clone.lock().unwrap().render_shapes(base);
            let slint_img = EditorEngine::rgba_to_slint_image(&rendered);
            ui.set_canvas_image(slint_img);
            ui.set_has_canvas_image(true);
        }
        ui.set_show_editor_modal(true);
    });

    // Unified Annotation Dispatch Handler — routes all 8 tool types with custom text parameter
    let base_img_clone = active_base_image.clone();
    let stack_clone = active_annotation_stack.clone();
    let ui_handle = ui.as_weak();
    ui.on_apply_annotation(move |tool, color_hex, stroke_idx, x1, y1, x2, y2, custom_text| {
        let ui = ui_handle.unwrap();
        let color = parse_color_hex(color_hex.as_str());
        let stroke_px = stroke_size_to_px(stroke_idx);

        let (lx, ly, rx, ry) = (
            x1.min(x2), y1.min(y2),
            x1.max(x2), y1.max(y2),
        );
        let w = (rx - lx).max(0) as u32;
        let h = (ry - ly).max(0) as u32;

        let shape = match tool.as_str() {
            "arrow" => AnnotationShape::Arrow {
                start: (x1 as f32, y1 as f32),
                end: (x2 as f32, y2 as f32),
                color,
                stroke: stroke_px,
            },
            "rect" => AnnotationShape::HighlightBox {
                x: lx, y: ly, width: w, height: h,
                color, stroke: stroke_px,
            },
            "oval" => AnnotationShape::Oval {
                x: lx, y: ly, width: w, height: h,
                color, stroke: stroke_px,
            },
            "blur" => AnnotationShape::RedactBox {
                x: lx, y: ly, width: w, height: h,
            },
            "text" => AnnotationShape::TextCallout {
                x: x1, y: y1,
                text: if custom_text.is_empty() { "Callout".to_string() } else { custom_text.as_str().to_string() },
                color,
            },
            "step" => {
                let step_num = stack_clone.lock().unwrap().next_step();
                AnnotationShape::StepNumber {
                    x: x1, y: y1,
                    num: step_num,
                    color,
                }
            },
            "pen" => AnnotationShape::Freehand {
                points: vec![(x1 as f32, y1 as f32), (x2 as f32, y2 as f32)],
                color,
                stroke: stroke_px,
            },
            "spotlight" => AnnotationShape::Spotlight {
                x: lx, y: ly, width: w, height: h,
            },
            _ => return,
        };

        stack_clone.lock().unwrap().push(shape);

        if let Some(base) = base_img_clone.lock().unwrap().as_ref() {
            let rendered = stack_clone.lock().unwrap().render_shapes(base);
            let slint_img = EditorEngine::rgba_to_slint_image(&rendered);
            ui.set_canvas_image(slint_img);
        }
    });

    // Interactive Studio Export Handler
    let config_clone = config.clone();
    let base_img_clone = active_base_image.clone();
    let stack_clone = active_annotation_stack.clone();
    let ui_handle = ui.as_weak();
    ui.on_save_editor_export(move || {
        let ui = ui_handle.unwrap();
        let save_dir = config_clone.lock().unwrap().save_directory.clone();
        if let Some(base) = base_img_clone.lock().unwrap().as_ref() {
            let rendered = stack_clone.lock().unwrap().render_shapes(base);
            let dyn_img = image::DynamicImage::ImageRgba8(rendered);
            if let Ok(path) = EditorEngine::export_edited_snapshot(&dyn_img, &save_dir) {
                println!("[EditorEngine] Exported edited showcase asset to: {:?}", path);
                ui.set_status_message(format!("Exported to {:?}", path.file_name().unwrap()).into());
                ui.set_show_editor_modal(false);
            }
        }
    });

    // Corporate Paddle Checkout Rails handler
    let config_clone = config.clone();
    ui.on_open_paddle_checkout(move || {
        let url = config_clone.lock().unwrap().paddle_checkout_url.clone();
        if let Err(e) = open::that(&url) {
            eprintln!("[Paddle] Failed to open browser link: {}", e);
        }
    });

    let recording_flag = Arc::new(AtomicBool::new(false));
    let audio_enabled_flag = Arc::new(AtomicBool::new(true));

    // Recording Engine Activation Handler
    let ui_handle = ui.as_weak();
    let flag_clone = recording_flag.clone();
    let audio_flag_clone = audio_enabled_flag.clone();
    let hw_profile_clone = hw_profile.clone();
    let focus_tracker_clone = focus_tracker.clone();
    let i18n_clone = i18n.clone();
    let config_rec_clone = config.clone();

    ui.on_toggle_recording(move || {
        let ui = ui_handle.unwrap();
        let current = ui.get_is_recording();
        let next_state = !current;
        ui.set_is_recording(next_state);

        flag_clone.store(next_state, Ordering::Relaxed);

        if next_state {
            let status_msg = {
                let i = i18n_clone.lock().unwrap();
                format!("{} {}", i.t("status_recording"), hw_profile_clone.encoder.tag())
            };
            ui.set_status_message(status_msg.into());

            let audio_enabled = ui.get_audio_enabled();
            audio_flag_clone.store(audio_enabled, Ordering::Relaxed);

            let audio_path_result = if audio_enabled {
                match audio::AudioEngine::start_microphone_recording(flag_clone.clone()) {
                    Ok(path) => {
                        println!("[Main] Audio recording started: {:?}", path);
                        Some(path)
                    }
                    Err(e) => {
                        eprintln!("[Main] Audio capture error: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            let recording_flag_for_thread = flag_clone.clone();
            let hw_profile_thread = hw_profile_clone.clone();
            let focus_tracker_thread = focus_tracker_clone.clone();
            let ui_handle_thread = ui.as_weak();
            let cinematic_zoom_enabled = ui.get_cinematic_zoom_enabled();
            let i18n_thread = i18n_clone.clone();
            let config_thread = config_rec_clone.clone();

            std::thread::spawn(move || {
                let temp_video_path = match capture::CaptureEngine::start_video_recording(
                    recording_flag_for_thread.clone(),
                    hw_profile_thread,
                    focus_tracker_thread.clone(),
                ) {
                    Ok(path) => path,
                    Err(e) => {
                        eprintln!("Failed to start video recording: {}", e);
                        return;
                    }
                };

                while recording_flag_for_thread.load(Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }

                std::thread::sleep(std::time::Duration::from_millis(300));

                let final_muxed_path = temp_video_path.with_file_name(format!(
                    "recording_{}.mp4",
                    chrono::Utc::now().timestamp()
                ));

                let (wm_enabled, wm_text, logo_path_opt, pos_opt) = {
                    let cfg = config_thread.lock().unwrap();
                    (
                        cfg.auto_watermark,
                        cfg.watermark_text.clone(),
                        cfg.watermark_logo_path.clone(),
                        cfg.watermark_position.clone(),
                    )
                };

                let mut ffmpeg_cmd = Command::new("ffmpeg");
                ffmpeg_cmd.arg("-y").arg("-i").arg(temp_video_path.to_str().unwrap());

                if let Some(audio_path) = &audio_path_result {
                    ffmpeg_cmd.arg("-i").arg(audio_path.to_str().unwrap());
                }

                let mut video_filters = Vec::new();

                if cinematic_zoom_enabled {
                    if let Some(vf_zoom) = focus_tracker_thread.build_ffmpeg_zoom_filter(1920, 1080, 1.3) {
                        video_filters.push(vf_zoom);
                    }
                }

                let mut custom_logo_applied = false;
                if wm_enabled {
                    if let Some(logo_p) = &logo_path_opt {
                        if std::path::Path::new(logo_p).exists() {
                            let overlay_pos = match pos_opt.as_str() {
                                "BottomLeft" => "overlay=20:main_h-overlay_h-20",
                                "TopRight" => "overlay=main_w-overlay_w-20:20",
                                "TopLeft" => "overlay=20:20",
                                _ => "overlay=main_w-overlay_w-20:main_h-overlay_h-20",
                            };
                            ffmpeg_cmd.arg("-i").arg(logo_p);
                            ffmpeg_cmd.arg("-filter_complex").arg(format!("[2:v]scale=140:-1[logo];[0:v][logo]{}", overlay_pos));
                            custom_logo_applied = true;
                        }
                    }

                    if !custom_logo_applied && !wm_text.is_empty() {
                        let (pos_x, pos_y) = match pos_opt.as_str() {
                            "BottomLeft" => ("20", "h-th-20"),
                            "TopRight" => ("w-tw-20", "20"),
                            "TopLeft" => ("20", "20"),
                            _ => ("w-tw-20", "h-th-20"),
                        };
                        let drawtext_vf = format!(
                            "drawtext=fontfile='/home/dan/development/wolf-screen-recorder/src/assets/Roboto-Regular.ttf':text='{}':fontcolor=0x22d45e:fontsize=22:box=1:boxcolor=0x080c14@0.8:boxborderw=6:x={}:y={}",
                            wm_text, pos_x, pos_y
                        );
                        video_filters.push(drawtext_vf);
                    }
                }

                if !custom_logo_applied && !video_filters.is_empty() {
                    ffmpeg_cmd.arg("-vf").arg(video_filters.join(","));
                }

                if audio_path_result.is_some() {
                    ffmpeg_cmd.args(&[
                        "-c:v", "libx264",
                        "-preset", "fast",
                        "-c:a", "aac",
                        "-b:a", "192k",
                        "-af", "aresample=async=1",
                        "-shortest",
                        final_muxed_path.to_str().unwrap(),
                    ]);
                } else {
                    ffmpeg_cmd.args(&[
                        "-c:v", "libx264",
                        "-preset", "fast",
                        final_muxed_path.to_str().unwrap(),
                    ]);
                }

                let status = ffmpeg_cmd.status();

                match status {
                    Ok(s) if s.success() => {
                        let _ = std::fs::remove_file(temp_video_path);
                        if let Some(audio_path) = audio_path_result {
                            let _ = std::fs::remove_file(audio_path);
                        }

                        let ui_weak = ui_handle_thread.clone();
                        let i18n_sub = i18n_thread.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                let msg = i18n_sub.lock().unwrap().t("status_complete");
                                ui.set_status_message(msg.into());
                                ui.set_show_support_modal(true);
                            }
                        });
                    }
                    _ => eprintln!("[CaptureEngine] FFmpeg post-process muxing failed."),
                }
            });
        } else {
            let status_msg = i18n_clone.lock().unwrap().t("status_muxing");
            ui.set_status_message(status_msg.into());
        }
    });

    // Snapshot Handler
    let config_clone = config.clone();
    let base_img_clone = active_base_image.clone();
    let stack_clone = active_annotation_stack.clone();
    let ui_handle = ui.as_weak();
    let i18n_clone = i18n.clone();
    ui.on_take_snapshot(move || {
        let ui = ui_handle.unwrap();
        let cfg = config_clone.lock().unwrap();
        let wm_text = if ui.get_watermark_enabled() { Some(cfg.watermark_text.as_str()) } else { None };
        let logo_pb = if ui.get_watermark_enabled() { cfg.watermark_logo_path.as_ref().map(std::path::PathBuf::from) } else { None };
        let pos = cfg.watermark_position.as_str();

        match capture::CaptureEngine::save_screenshot(wm_text, logo_pb.as_ref(), pos) {
            Ok(path) => {
                println!("Saved screenshot with watermark to: {:?}", path);
                if let Ok(rgba) = EditorEngine::load_image(&path) {
                    stack_clone.lock().unwrap().clear();
                    let slint_img = EditorEngine::rgba_to_slint_image(&rgba);
                    ui.set_canvas_image(slint_img);
                    ui.set_has_canvas_image(true);
                    *base_img_clone.lock().unwrap() = Some(rgba);
                }
                let msg = i18n_clone.lock().unwrap().t("status_complete");
                ui.set_status_message(msg.into());
            }
            Err(e) => eprintln!("Failed to capture snapshot: {}", e),
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_select_mode(move |mode| {
        let ui = ui_handle.unwrap();
        ui.set_selected_mode(mode);
    });

    let ui_handle = ui.as_weak();
    ui.on_select_resolution(move |res| {
        let ui = ui_handle.unwrap();
        ui.set_resolution(res);
    });

    let audio_flag_main = audio_enabled_flag.clone();
    let ui_handle = ui.as_weak();
    ui.on_toggle_audio(move || {
        let ui = ui_handle.unwrap();
        let current = ui.get_audio_enabled();
        let next = !current;
        ui.set_audio_enabled(next);
        audio_flag_main.store(next, Ordering::Relaxed);
    });

    // Floating Webcam PIP Window Toggle Handler
    let webcam_flag = Arc::new(AtomicBool::new(false));
    let webcam_flag_clone = webcam_flag.clone();
    let pip_win_clone = active_pip_window.clone();
    let ui_handle = ui.as_weak();
    ui.on_toggle_webcam(move || {
        let ui = ui_handle.unwrap();
        let current = ui.get_webcam_enabled();
        let next = !current;
        ui.set_webcam_enabled(next);
        webcam_flag_clone.store(next, Ordering::Relaxed);

        if next {
            if let Ok(pip_win) = WebcamPipWindow::new() {
                webcam::WebcamEngine::start_feed(webcam_flag_clone.clone(), pip_win.as_weak());

                let pip_weak = pip_win.as_weak();
                pip_win.on_move_window(move |delta_x, delta_y| {
                    if let Some(pip) = pip_weak.upgrade() {
                        let cur_pos = pip.window().position();
                        let new_x = cur_pos.x + delta_x as i32;
                        let new_y = cur_pos.y + delta_y as i32;
                        pip.window().set_position(slint::PhysicalPosition::new(new_x, new_y));
                    }
                });

                let pip_close_weak = pip_win.as_weak();
                let ui_close_weak = ui.as_weak();
                let flag_close_clone = webcam_flag.clone();
                pip_win.on_close_pip(move || {
                    flag_close_clone.store(false, Ordering::Relaxed);
                    if let Some(ui) = ui_close_weak.upgrade() {
                        ui.set_webcam_enabled(false);
                    }
                    if let Some(pip) = pip_close_weak.upgrade() {
                        let _ = pip.hide();
                    }
                });

                let _ = pip_win.show();
                *pip_win_clone.lock().unwrap() = Some(pip_win);
            }
        } else if let Some(pip) = pip_win_clone.lock().unwrap().take() {
            let _ = pip.hide();
        }
    });

    ui.run()?;
    Ok(())
}