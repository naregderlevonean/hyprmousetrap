mod config;
mod zone;

use anyhow::{Context, Result};
use hyprland::data::{CursorPosition, Monitors};
use hyprland::shared::HyprData;
use log::{error, info};
use std::env;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use crate::config::{Hotkeys, LuaConfig};
use crate::zone::{detect_zone, get_logical_size, Zone};

struct ConfigCache {
    path: PathBuf,
    data: Option<LuaConfig>,
    last_modified: SystemTime,
}

impl ConfigCache {
    fn new(path: PathBuf) -> Self {
        let last_modified = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let data = LuaConfig::new(&path).ok();
        Self { path, data, last_modified }
    }

    fn get_data(&mut self) -> Option<&LuaConfig> {
        if let Ok(meta) = std::fs::metadata(&self.path) {
            if let Ok(mtime) = meta.modified() {
                if mtime > self.last_modified {
                    info!("Config change detected, reloading...");
                    if let Ok(new_data) = LuaConfig::new(&self.path) {
                        self.data = Some(new_data);
                        self.last_modified = mtime;
                    } else {
                        error!("Failed to reload config - check Lua syntax");
                    }
                }
            }
        }
        self.data.as_ref()
    }
}

fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    let args: Vec<String> = env::args().collect();

    let current_trigger = if args.len() > 1 { Some(args[1].clone()) } else { None };
    let cli_hotkeys = if args.len() > 2 { Hotkeys::from_str(&args[2]) } else { None };
    let is_daemon = current_trigger.is_none();

    let home = env::var("HOME").context("HOME not set")?;
    let config_path = PathBuf::from(home).join(".config/hypr/hyprmousetrap.lua");

    let mut cache = ConfigCache::new(config_path);
    let mut last_zone = Zone::None;

    let hotkeys_state = Arc::new(Mutex::new(cli_hotkeys.unwrap_or_default()));
    let is_touch_down = Arc::new(AtomicBool::new(false));
    let is_last_touch = Arc::new(AtomicBool::new(false));

    if is_daemon {
        info!("hyprmousetrap starting in daemon mode...");
        loop {
            if let Some(config) = cache.get_data() {
                let _ = run_check(
                    config,
                    &mut last_zone,
                    &current_trigger,
                    &hotkeys_state,
                    &is_touch_down,
                    &is_last_touch,
                    false
                );
            }
            thread::sleep(Duration::from_millis(16));
        }
    } else {
        if let Some(config) = cache.get_data() {
            run_check(
                config,
                &mut Zone::None,
                &current_trigger,
                &hotkeys_state,
                &is_touch_down,
                &is_last_touch,
                true
            )?;
        }
    }
    Ok(())
}

fn run_check(
    config: &LuaConfig,
    last_zone: &mut Zone,
    current_trigger: &Option<String>,
    hotkeys_state: &Arc<Mutex<Hotkeys>>,
    is_touch_down: &Arc<AtomicBool>,
    is_last_touch: &Arc<AtomicBool>,
    force: bool,
) -> Result<()> {
    let cursor = CursorPosition::get().context("Failed to get cursor position")?;
    let monitors = Monitors::get().context("Failed to get monitors info")?;

    if let Some(mon) = monitors.iter().find(|m| {
        let (w, h) = get_logical_size(m);
        cursor.x >= m.x as i64 && cursor.x < m.x as i64 + w && 
        cursor.y >= m.y as i64 && cursor.y < m.y as i64 + h
    }) {
        let (c_size, thickness) = config.get_geometry(&mon.name);
        let current_zone = detect_zone(&cursor, mon, c_size, thickness);

        if is_last_touch.load(Ordering::Relaxed) && !is_touch_down.load(Ordering::Relaxed) {
            *last_zone = Zone::None;
            return Ok(());
        }

        if current_zone != Zone::None && (force || current_zone != *last_zone) {
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
                    if action.delay_ms > 0 {
                        let step = 10;
                        let mut waited = 0;
                        let mut aborted = false;

                        while waited < action.delay_ms {
                            thread::sleep(Duration::from_millis(step));
                            waited += step;

                            if let Ok(new_cursor) = CursorPosition::get() {
                                if detect_zone(&new_cursor, mon, c_size, thickness) != current_zone {
                                    aborted = true;
                                    break;
                                }
                            }
                        }
                        
                        if aborted { 
                            continue; 
                        }
                    }
                    let _ = action.execute();
                }
            }
        }
        *last_zone = current_zone;
    }
    Ok(())
}
