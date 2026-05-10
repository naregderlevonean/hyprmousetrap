use anyhow::Result;
use log::info;
use std::collections::HashSet;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Clone, Default)]
pub struct MouseManager {
    dx: Arc<AtomicI32>,
    dy: Arc<AtomicI32>,
}

impl MouseManager {
    pub fn new() -> Self {
        let manager = Self::default();
        manager.spawn_watcher();
        manager
    }

    pub fn consume_delta(&self) -> (i32, i32) {
        (
            self.dx.swap(0, Ordering::Relaxed),
            self.dy.swap(0, Ordering::Relaxed),
        )
    }

    fn spawn_watcher(&self) {
        let dx = self.dx.clone();
        let dy = self.dy.clone();

        thread::spawn(move || {
            let mut active_devices = HashSet::new();
            loop {
                if let Ok(entries) = std::fs::read_dir("/dev/input") {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let path_str = path.to_string_lossy().to_string();

                        if !active_devices.contains(&path_str) {
                            if let Ok(mut device) = evdev::Device::open(&path) {
                                let has_rel = device.supported_events().contains(evdev::EventType::RELATIVE);
                                let has_abs = device.supported_events().contains(evdev::EventType::ABSOLUTE);

                                if has_rel || has_abs {
                                    active_devices.insert(path_str.clone());
                                    let dx_c = dx.clone();
                                    let dy_c = dy.clone();

                                    thread::spawn(move || {
                                        let mut last_abs_x: Option<i32> = None;
                                        let mut last_abs_y: Option<i32> = None;

                                        loop {
                                            match device.fetch_events() {
                                                Ok(events) => {
                                                    for ev in events {
                                                        if ev.event_type() == evdev::EventType::RELATIVE {
                                                            match ev.code() {
                                                                0x00 => { dx_c.fetch_add(ev.value() as i32, Ordering::Relaxed); }
                                                                0x01 => { dy_c.fetch_add(ev.value() as i32, Ordering::Relaxed); }
                                                                _ => {}
                                                            }
                                                        } 
                                                        else if ev.event_type() == evdev::EventType::ABSOLUTE {
                                                            match ev.code() {
                                                                0x00 | 0x35 => { 
                                                                    let val = ev.value() as i32;
                                                                    if let Some(last) = last_abs_x {
                                                                        dx_c.fetch_add(val - last, Ordering::Relaxed);
                                                                    }
                                                                    last_abs_x = Some(val);
                                                                }
                                                                0x01 | 0x36 => {
                                                                    let val = ev.value() as i32;
                                                                    if let Some(last) = last_abs_y {
                                                                        dy_c.fetch_add(val - last, Ordering::Relaxed);
                                                                    }
                                                                    last_abs_y = Some(val);
                                                                }
                                                                0x39 => { 
                                                                    if ev.value() == -1 {
                                                                        last_abs_x = None;
                                                                        last_abs_y = None;
                                                                    }
                                                                }
                                                                _ => {}
                                                            }
                                                        }
                                                        else if ev.event_type() == evdev::EventType::KEY {
                                                            if ev.code() == 330 && ev.value() == 0 {
                                                                last_abs_x = None;
                                                                last_abs_y = None;
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                                    thread::sleep(Duration::from_millis(10));
                                                }
                                                Err(_) => break,
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    }
                }
                thread::sleep(Duration::from_secs(5));
            }
        });
    }
}
