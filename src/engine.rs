use anyhow::{Context, Result};
use hyprland::data::{CursorPosition, Monitors};
use hyprland::shared::HyprData; 
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
}

impl TrapEngine {
    pub fn new(mouse: MouseManager) -> Self {
        Self {
            last_zone: Zone::None,
            mouse,
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
        let cursor = CursorPosition::get().context("Failed to get cursor position")?;
        let monitors = Monitors::get().context("Failed to get monitors info")?;

        let mon = match monitors.iter().find(|m| {
            let (w, h) = get_logical_size(m);
            cursor.x >= m.x as i64 && cursor.x < m.x as i64 + w && 
            cursor.y >= m.y as i64 && cursor.y < m.y as i64 + h
        }) {
            Some(m) => m,
            None => return Ok(()),
        };

        let (c_size, thickness) = config.get_geometry(&mon.name);
        let current_zone = detect_zone(&cursor, mon, c_size, thickness);

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
                    if self.handle_dwell_and_pressure(&action, current_zone, mon, c_size, thickness) {
                        let _ = action.execute();
                    }
                }
            }
        }

        self.last_zone = current_zone;
        Ok(())
    }

    fn handle_dwell_and_pressure(&self, action: &Action, zone: Zone, mon: &hyprland::data::Monitor, c_size: i64, thickness: i64) -> bool {
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
