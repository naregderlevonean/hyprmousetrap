mod config;
mod zone;
mod input;
mod engine;

use anyhow::{Context, Result};
use log::{error, info};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{env, path::PathBuf, thread, time::Duration};
use std::sync::{mpsc, Arc, Mutex, atomic::AtomicBool};

use crate::config::{Hotkeys, LuaConfig};
use crate::input::MouseManager;
use crate::engine::TrapEngine;

fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    let args: Vec<String> = env::args().collect();

    let current_trigger = args.get(1).cloned();
    let cli_hotkeys = args.get(2).and_then(|s| Hotkeys::from_str(s));
    let is_daemon = current_trigger.is_none();

    let home = env::var("HOME").context("HOME not set")?;
    let config_path = PathBuf::from(home).join(".config/hypr/hyprmousetrap.lua");

    let mouse_manager = MouseManager::new();
    let mut engine = TrapEngine::new(mouse_manager);

    let hotkeys_state = Arc::new(Mutex::new(cli_hotkeys.unwrap_or_default()));
    let is_touch_down = Arc::new(AtomicBool::new(false));
    let is_last_touch = Arc::new(AtomicBool::new(false));

    let mut current_config = LuaConfig::new(&config_path).ok();

    if is_daemon {
        info!("hyprmousetrap starting in daemon mode...");
        
        let (tx, rx) = mpsc::channel();
        let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_)) {
                    let _ = tx.send(());
                }
            }
        }).context("Failed to setup config file watcher")?;
        
        watcher.watch(&config_path, RecursiveMode::NonRecursive).ok();

        loop {
            let mut reload_requested = false;
            while rx.try_recv().is_ok() {
                reload_requested = true;
            }

            if reload_requested {
                info!("Config change detected, reloading...");
                match LuaConfig::new(&config_path) {
                    Ok(cfg) => current_config = Some(cfg),
                    Err(_) => error!("Failed to reload config - check Lua syntax"),
                }
            }

            if let Some(config) = &current_config {
                let _ = engine.check_and_execute(
                    config, &current_trigger, &hotkeys_state, 
                    &is_touch_down, &is_last_touch, false
                );
            }
            thread::sleep(Duration::from_millis(16));
        }
    } else {
        if let Some(config) = &current_config {
            engine.check_and_execute(
                config, &current_trigger, &hotkeys_state, 
                &is_touch_down, &is_last_touch, true
            )?;
        }
    }
    
    Ok(())
}
