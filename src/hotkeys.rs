use rdev::{listen, Event, EventType, Key};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyCommand {
    ToggleRecord,
    Snapshot,
    RegionSelect,
    Cancel,
}

#[allow(dead_code)]
pub struct HotkeyDaemon;

impl HotkeyDaemon {
    pub fn start(sender: Sender<HotkeyCommand>) {
        thread::spawn(move || {
            let meta_pressed = Arc::new(AtomicBool::new(false));
            let ctrl_pressed = Arc::new(AtomicBool::new(false));
            let shift_pressed = Arc::new(AtomicBool::new(false));

            let meta_clone = meta_pressed.clone();
            let ctrl_clone = ctrl_pressed.clone();
            let shift_clone = shift_pressed.clone();

            println!("[HotkeyDaemon] Starting global low-level key listener (Super+Shift+R/S/X)...");

            let callback = move |event: Event| {
                match event.event_type {
                    EventType::KeyPress(key) => {
                        match key {
                            Key::MetaLeft | Key::MetaRight => meta_clone.store(true, Ordering::Relaxed),
                            Key::ControlLeft | Key::ControlRight => ctrl_clone.store(true, Ordering::Relaxed),
                            Key::ShiftLeft | Key::ShiftRight => shift_clone.store(true, Ordering::Relaxed),
                            Key::KeyR => {
                                if (meta_clone.load(Ordering::Relaxed) || ctrl_clone.load(Ordering::Relaxed))
                                    && shift_clone.load(Ordering::Relaxed)
                                {
                                    println!("[HotkeyDaemon] Triggered Global ToggleRecord (Super/Ctrl + Shift + R)");
                                    let _ = sender.send(HotkeyCommand::ToggleRecord);
                                }
                            }
                            Key::KeyS => {
                                if (meta_clone.load(Ordering::Relaxed) || ctrl_clone.load(Ordering::Relaxed))
                                    && shift_clone.load(Ordering::Relaxed)
                                {
                                    println!("[HotkeyDaemon] Triggered Global Instant Snapshot (Super/Ctrl + Shift + S)");
                                    let _ = sender.send(HotkeyCommand::Snapshot);
                                }
                            }
                            Key::KeyX => {
                                if (meta_clone.load(Ordering::Relaxed) || ctrl_clone.load(Ordering::Relaxed))
                                    && shift_clone.load(Ordering::Relaxed)
                                {
                                    println!("[HotkeyDaemon] Triggered Global Region Selector (Super/Ctrl + Shift + X)");
                                    let _ = sender.send(HotkeyCommand::RegionSelect);
                                }
                            }
                            Key::Escape => {
                                let _ = sender.send(HotkeyCommand::Cancel);
                            }
                            _ => {}
                        }
                    }
                    EventType::KeyRelease(key) => {
                        match key {
                            Key::MetaLeft | Key::MetaRight => meta_clone.store(false, Ordering::Relaxed),
                            Key::ControlLeft | Key::ControlRight => ctrl_clone.store(false, Ordering::Relaxed),
                            Key::ShiftLeft | Key::ShiftRight => shift_clone.store(false, Ordering::Relaxed),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            };

            if let Err(error) = listen(callback) {
                eprintln!("[HotkeyDaemon] Error listening to low-level hotkeys: {:?}", error);
            }
        });
    }
}
