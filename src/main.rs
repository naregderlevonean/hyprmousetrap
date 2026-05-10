mod config;
mod zone;
mod input;
mod engine;

use anyhow::{Context, Result};
use log::{error, info};
use std::{env, path::PathBuf, thread, time::{Duration, SystemTime}};
use std::sync::{Arc, Mutex, atomic::AtomicBool};

use crate::config::{Hotkeys, LuaConfig};
use crate::input::MouseManager;
use crate::engine::TrapEngine;

struct ConfigCache {
    path: PathBuf,
    data: Option<LuaConfig>,
    last_modified: SystemTime,
}

impl ConfigCache {
    fn new(path: PathBuf) -> Self {
        let last_modified = std::fs::metadata(&path).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
        let data = LuaConfig::new(&path).ok();
        Self { path, data, last_modified }
    }

    fn get_data(&mut self) -> Option<&LuaConfig> {
        if let Ok(mtime) = std::fs::metadata(&self.path).and_then(|m| m.modified()) {
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
        self.data.as_ref()
    }
}

fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    let args: Vec<String> = env::args().collect();

    let current_trigger = args.get(1).cloned();
    let cli_hotkeys = args.get(2).and_then(|s| Hotkeys::from_str(s));
    let is_daemon = current_trigger.is_none();

    let home = env::var("HOME").context("HOME not set")?;
    let config_path = PathBuf::from(home).join(".config/hypr/hyprmousetrap.lua");

    let mut cache = ConfigCache::new(config_path);
    let mouse_manager = MouseManager::new();
    let mut engine = TrapEngine::new(mouse_manager);

    let hotkeys_state = Arc::new(Mutex::new(cli_hotkeys.unwrap_or_default()));
    let is_touch_down = Arc::new(AtomicBool::new(false));
    let is_last_touch = Arc::new(AtomicBool::new(false));

    if is_daemon {
        info!("hyprmousetrap starting in daemon mode...");
        loop {
            if let Some(config) = cache.get_data() {
                let _ = engine.check_and_execute(
                    config, &current_trigger, &hotkeys_state, 
                    &is_touch_down, &is_last_touch, false
                );
            }
            thread::sleep(Duration::from_millis(16));
        }
    } else {
        if let Some(config) = cache.get_data() {
            engine.check_and_execute(
                config, &current_trigger, &hotkeys_state, 
                &is_touch_down, &is_last_touch, true
            )?;
        }
    }
    Ok(())
}
