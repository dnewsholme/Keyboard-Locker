use eframe::egui;
use evdev::{Device, Key};
use std::sync::{Arc, Mutex};
use std::thread;
use crossbeam_channel::{unbounded, Sender, Receiver};
use std::path::PathBuf;
use std::os::unix::io::AsRawFd;

#[derive(PartialEq, Clone)]
struct AppState {
    status: Status,
    unlock_key_code: u16, // The raw evdev code (Default 16 = KEY_Q)
    unlock_key_name: String,
    devices: Vec<(String, PathBuf)>,
    selected_device: Option<PathBuf>,
    error_msg: Option<String>,
}

#[derive(PartialEq, Clone)]
enum Status { Idle, Locked }

struct KeyLockApp {
    state: Arc<Mutex<AppState>>,
    unlock_tx: Option<Sender<()>>,
}

impl KeyLockApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let state = Arc::new(Mutex::new(AppState {
            status: Status::Idle,
            unlock_key_code: 16, // KEY_Q
            unlock_key_name: "Q".to_string(),
            devices: Vec::new(),
            selected_device: None,
            error_msg: None,
        }));

        let thread_state = Arc::clone(&state);

        thread::spawn(move || {
            loop {
                let devices = scan_devices();
                {
                    let mut s = thread_state.lock().unwrap();
                    s.devices = devices;
                    
                    // Auto-select the first device if none is selected
                    if s.selected_device.is_none() {
                        if let Some(first) = s.devices.first() {
                            s.selected_device = Some(first.1.clone());
                        }
                    }
                    
                    // If selected device is no longer present, deselect it
                    if let Some(selected) = &s.selected_device {
                        if !s.devices.iter().any(|(_, p)| p == selected) {
                            s.selected_device = None;
                        }
                    }
                }
                thread::sleep(std::time::Duration::from_secs(2));
            }
        });

        Self { state, unlock_tx: None }
    }
}

fn lock_worker(path: PathBuf, rx: Receiver<()>, state: Arc<Mutex<AppState>>) {
    let mut device = match Device::open(&path) {
        Ok(d) => d,
        Err(e) => {
            state.lock().unwrap().error_msg = Some(format!("Failed to open device: {}", e));
            return;
        }
    };

    println!("Attempting to grab device: {}", device.name().unwrap_or("Unknown"));
    if let Err(e) = device.grab() {
        eprintln!("Failed to grab device: {}", e);
        let mut s = state.lock().unwrap();
        s.error_msg = Some(format!("Lock failed: {}", e));
        s.status = Status::Idle;
        return;
    }
    println!("Device grabbed successfully");

    // Set non-blocking to allow polling rx and device
    set_nonblocking(device.as_raw_fd());

    let mut ctrl_pressed = false;

    loop {
        // Check for unlock signal from UI
        if rx.try_recv().is_ok() {
            break;
        }

        let mut should_unlock = false;

        match device.fetch_events() {
            Ok(events) => {
                for event in events {
                    if let evdev::InputEventKind::Key(key) = event.kind() {
                        let is_down = event.value() != 0;
                        let current_target = state.lock().unwrap().unlock_key_code;

                        if key == Key::KEY_LEFTCTRL || key == Key::KEY_RIGHTCTRL {
                            ctrl_pressed = is_down;
                        }

                        if key.code() == current_target && is_down && ctrl_pressed {
                            should_unlock = true;
                            break;
                        }
                    }
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {},
            Err(e) => {
                eprintln!("Error reading events: {}", e);
                let mut s = state.lock().unwrap();
                s.error_msg = Some(format!("Device disconnected: {}", e));
                s.status = Status::Idle;
                return;
            },
        }

        if should_unlock {
            let _ = device.ungrab();
            state.lock().unwrap().status = Status::Idle;
            return;
        }

        thread::sleep(std::time::Duration::from_millis(10));
    }

    let _ = device.ungrab();
}

fn scan_devices() -> Vec<(String, PathBuf)> {
    let mut devices = Vec::new();
    for (path, dev) in evdev::enumerate() {
        if dev.supported_keys().map_or(false, |k| k.contains(Key::KEY_A)) {
            let name = format!("{} ({})", dev.name().unwrap_or("Unknown"), path.display());
            devices.push((name, path));
        }
    }
    devices
}

fn set_nonblocking(fd: std::os::unix::io::RawFd) {
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
}

impl eframe::App for KeyLockApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut state = self.state.lock().unwrap();
        
        // Clean up unlock channel if worker finished on its own
        if state.status == Status::Idle && self.unlock_tx.is_some() {
            self.unlock_tx = None;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Wayland Input Controller");
            ui.add_space(8.0);

            if let Some(err) = &state.error_msg {
                ui.colored_label(egui::Color32::RED, err);
            }

            if state.devices.is_empty() {
                ui.add_space(10.0);
                ui.colored_label(egui::Color32::YELLOW, "⚠ No keyboards detected!");
                ui.label("Ensure you are in the 'input' group and have REBOOTED.");
                ui.monospace(format!("sudo usermod -aG input {}", std::env::var("USER").unwrap_or("user".into())));
            } else {
                ui.label("Select Keyboard to Lock:");
                let devices = state.devices.clone();
                let selected_text = devices.iter()
                    .find(|(_, p)| Some(p) == state.selected_device.as_ref())
                    .map(|(n, _)| n.as_str())
                    .unwrap_or("Select Device")
                    .to_string();

                egui::ComboBox::from_id_source("device_selector")
                    .selected_text(selected_text)
                    .width(250.0)
                    .show_ui(ui, |ui| {
                        for (name, path) in &devices {
                            ui.selectable_value(&mut state.selected_device, Some(path.clone()), name);
                        }
                    });
            }

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
                if ui.add_enabled(state.selected_device.is_some(), egui::Button::new("🔒 LOCK NOW")).clicked() {
                    if let Some(path) = state.selected_device.clone() {
                        state.status = Status::Locked;
                        state.error_msg = None;
                        
                        let (tx, rx) = unbounded::<()>();
                        self.unlock_tx = Some(tx);
                        
                        let thread_state = self.state.clone();
                        thread::spawn(move || lock_worker(path, rx, thread_state));
                    }
                }
            } else {
                ui.colored_label(egui::Color32::RED, "!!! KEYBOARD LOCKED !!!");
                ui.label(format!("Press CTRL + {} to unlock", state.unlock_key_name));
                if ui.button("🔓 CLICK TO UNLOCK").clicked() {
                    if let Some(tx) = &self.unlock_tx {
                        let _ = tx.send(());
                    }
                }
            }
        });
        
        // Ensure UI stays responsive while locked
        ctx.request_repaint();
    }
}

fn get_code_from_char(c: char) -> u16 {
    match c {
        'Q' => 16, 'W' => 17, 'E' => 18, 'R' => 19, 'T' => 20, 'Y' => 21, 'U' => 22, 'I' => 23, 'O' => 24, 'P' => 25,
        'A' => 30, 'S' => 31, 'D' => 32, 'F' => 33, 'G' => 34, 'H' => 35, 'J' => 36, 'K' => 37, 'L' => 38,
        'Z' => 44, 'X' => 45, 'C' => 46, 'V' => 47, 'B' => 48, 'N' => 49, 'M' => 50,
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