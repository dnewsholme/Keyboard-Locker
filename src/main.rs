use eframe::egui;
use evdev::{Device, Key};
use std::sync::{Arc, Mutex};
use std::thread;
use crossbeam_channel::{unbounded, Sender, Receiver};

#[derive(PartialEq, Clone)]
struct AppState {
    status: Status,
    unlock_key_code: u16, // The raw evdev code (Default 16 = KEY_Q)
    unlock_key_name: String,
}

#[derive(PartialEq, Clone)]
enum Status { Idle, Locked }

struct KeyLockApp {
    state: Arc<Mutex<AppState>>,
    command_tx: Sender<bool>,
}

impl KeyLockApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = unbounded::<bool>();
        let state = Arc::new(Mutex::new(AppState {
            status: Status::Idle,
            unlock_key_code: 16, // KEY_Q
            unlock_key_name: "Q".to_string(),
        }));

        let thread_state = Arc::clone(&state);
        thread::spawn(move || {
            keyboard_worker(rx, thread_state);
        });

        Self { state, command_tx: tx }
    }
}

fn keyboard_worker(rx: Receiver<bool>, state: Arc<Mutex<AppState>>) {
    let mut device = loop {
        if let Some(dev) = find_keyboard() { break dev; }
        thread::sleep(std::time::Duration::from_secs(2));
    };

    let mut locked = false;
    let mut ctrl_pressed = false;

    loop {
        // 1. Check for UI commands
        if let Ok(cmd) = rx.try_recv() {
            locked = cmd;
            if locked { let _ = device.grab(); } else { let _ = device.ungrab(); }
        }

        if locked {
            let mut should_unlock = false;

            // Use a scoped block or collect events to free the borrow on 'device'
            if let Ok(events) = device.fetch_events() {
                for event in events {
                    if let evdev::InputEventKind::Key(key) = event.kind() {
                        let is_down = event.value() != 0;
                        let current_target = state.lock().unwrap().unlock_key_code;

                        if key == Key::KEY_LEFTCTRL || key == Key::KEY_RIGHTCTRL {
                            ctrl_pressed = is_down;
                        }

                        if key.code() == current_target && is_down && ctrl_pressed {
                            should_unlock = true; 
                            // We don't call ungrab here yet!
                        }
                    }
                }
            } // 'events' iterator is dropped here, freeing the borrow on 'device'

            if should_unlock {
                let _ = device.ungrab();
                locked = false;
                let mut s = state.lock().unwrap();
                s.status = Status::Idle;
            }
        }
        thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn find_keyboard() -> Option<Device> {
    evdev::enumerate().find(|(_, dev)| {
        dev.supported_keys().map_or(false, |k| k.contains(Key::KEY_ENTER))
    }).map(|(_, dev)| dev)
}

impl eframe::App for KeyLockApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut state = self.state.lock().unwrap();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Wayland Input Controller");
            ui.add_space(8.0);

            ui.group(|ui| {
                ui.label("Configuration");
                ui.horizontal(|ui| {
                    ui.label("Unlock Key: CTRL +");
                    let response = ui.add(egui::TextEdit::singleline(&mut state.unlock_key_name).char_limit(1));
                    
                    if response.changed() {
                        state.unlock_key_name = state.unlock_key_name.to_uppercase();
                        // Map character to evdev Key Code (Basic mapping A-Z)
                        if let Some(c) = state.unlock_key_name.chars().next() {
                            state.unlock_key_code = match c {
                                'A'..='Z' => get_code_from_char(c),
                                _ => 16, // Default to Q
                            };
                        }
                    }
                });
            });

            ui.add_space(20.0);

            if state.status == Status::Idle {
                if ui.button("🔒 LOCK NOW").clicked() {
                    state.status = Status::Locked;
                    let _ = self.command_tx.send(true);
                }
            } else {
                ui.colored_label(egui::Color32::RED, "!!! KEYBOARD LOCKED !!!");
                ui.label(format!("Press CTRL + {} to unlock", state.unlock_key_name));
                if ui.button("🔓 CLICK TO UNLOCK").clicked() {
                    state.status = Status::Idle;
                    let _ = self.command_tx.send(false);
                }
            }
        });
        
        // Ensure UI stays responsive while locked
        ctx.request_repaint();
    }
}

fn get_code_from_char(c: char) -> u16 {
    // Simplified mapping for demonstration
    match c {
        'Q' => 16, 'W' => 17, 'E' => 18, 'R' => 19, 'T' => 20, 
        'A' => 30, 'S' => 31, 'D' => 32, 'F' => 33, 'G' => 34,
        _ => 16,
    }
}

fn load_icon() -> egui::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let icon = include_bytes!("icon.png");
        let image = image::load_from_memory(icon)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    egui::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([320.0, 240.0])
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native("Keyboard Locker", options, Box::new(|cc| Box::new(KeyLockApp::new(cc))))
}