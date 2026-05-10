use anyhow::Result;
use hyprland::data::{CursorPosition, Monitor, Monitors};
use hyprland::shared::{HyprData, HyprDataVec};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::config::{Hotkeys, LuaConfig, Action};
use crate::zone::{detect_zone, get_logical_size, Zone};
use crate::input::MouseManager;

pub struct TrapEngine {
    last_zone: Zone,
    mouse: MouseManager,
    cached_monitors: Vec<Monitor>,
    last_pos: (i64, i64),
}

impl TrapEngine {
    pub fn new(mouse: MouseManager) -> Self {
        Self {
            last_zone: Zone::None,
            mouse,
            cached_monitors: Vec::new(),
            last_pos: (-1, -1),
        }
    }

    pub fn check_and_execute(
        &mut self,
        config: &LuaConfig,
        current_trigger: &Option<String>,
        hotkeys_state: &Arc<Mutex<Hotkeys>>,
        is_touch_down: &Arc<AtomicBool>,
        is_last_touch: &Arc<AtomicBool>,
        force: bool,
    ) -> Result<()> {
        let cursor = match CursorPosition::get() {
            Ok(c) => c,
            Err(_) => return Ok(()),
        };

        if self.last_pos == (cursor.x, cursor.y) && !force {
            return Ok(());
        }
        self.last_pos = (cursor.x, cursor.y);

        let mut mon_opt = self.find_cached_monitor(&cursor);
        if mon_opt.is_none() {
            if let Ok(monitors) = Monitors::get() {
                self.cached_monitors = monitors.to_vec();
                mon_opt = self.find_cached_monitor(&cursor);
            }
        }

        let mon = match mon_opt {
            Some(m) => m,
            None => return Ok(()),
        };

        let (c_size, thickness) = config.get_geometry(&mon.name);
        let current_zone = detect_zone(&cursor, &mon, c_size, thickness);

        if is_last_touch.load(Ordering::Relaxed) && !is_touch_down.load(Ordering::Relaxed) {
            self.last_zone = Zone::None;
            return Ok(());
        }

        if current_zone != Zone::None && (force || current_zone != self.last_zone) {
            let active_special_ws = if mon.special_workspace.name.is_empty() { 
                None 
            } else { 
                Some(mon.special_workspace.name.as_str()) 
            };
            
            let current_hk = hotkeys_state.lock().map(|h| h.clone()).unwrap_or_default();

            if let Ok(actions) = config.evaluate_zone(
                &mon.name,
                current_trigger.as_deref(),
                current_zone.as_str(),
                active_special_ws,
                &current_hk,
            ) {
                for action in actions {
                    if self.handle_dwell_and_pressure(&action, current_zone, &mon, c_size, thickness) {
                        let _ = action.execute();
                    }
                }
            }
        }

        self.last_zone = current_zone;
        Ok(())
    }

    fn find_cached_monitor(&self, cursor: &CursorPosition) -> Option<Monitor> {
        self.cached_monitors.iter().find(|m| {
            let (w, h) = get_logical_size(m);
            cursor.x >= m.x as i64 && cursor.x < m.x as i64 + w && 
            cursor.y >= m.y as i64 && cursor.y < m.y as i64 + h
        }).cloned()
    }

    fn handle_dwell_and_pressure(&self, action: &Action, zone: Zone, mon: &Monitor, c_size: i64, thickness: i64) -> bool {
        if action.delay_ms == 0 && action.pressure == 0 {
            return true;
        }

        let mut waited = 0;
        let mut accumulated_pressure = 0;
        self.mouse.consume_delta();

        loop {
            let time_met = action.delay_ms == 0 || waited >= action.delay_ms;
            let pressure_met = action.pressure == 0 || accumulated_pressure >= action.pressure;

            if time_met && pressure_met {
                return true;
            }

            thread::sleep(Duration::from_millis(10));
            waited += 10;

            if let Ok(new_cursor) = CursorPosition::get() {
                if detect_zone(&new_cursor, mon, c_size, thickness) != zone {
                    return false;
                }
            }

            if action.pressure > 0 {
                let (c_dx, c_dy) = self.mouse.consume_delta();
                let push = match zone {
                    Zone::Top => -c_dy,
                    Zone::Bottom => c_dy,
                    Zone::Left => -c_dx,
                    Zone::Right => c_dx,
                    Zone::TopLeft => -c_dx - c_dy,
                    Zone::TopRight => c_dx - c_dy,
                    Zone::BottomLeft => -c_dx + c_dy,
                    Zone::BottomRight => c_dx + c_dy,
                    _ => 0,
                };
                accumulated_pressure = (accumulated_pressure + push).max(0);
            }
        }
    }
}
