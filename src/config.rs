use anyhow::{Context, Result};
use hyprland::dispatch::{Dispatch, DispatchType};
use hyprland::keyword::Keyword;
use mlua::{Function as LuaFunction, Lua, Table as LuaTable, Value as LuaValue};
use std::fs::read_to_string;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Hotkeys {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
}

impl Hotkeys {
    pub fn from_str(s: &str) -> Option<Self> {
        let s = s.trim().to_lowercase();
        if s.is_empty() { return None; }
        let mut hk = Hotkeys::default();
        if s.contains("ctrl") { hk.ctrl = true; }
        if s.contains("alt") { hk.alt = true; }
        if s.contains("shift") { hk.shift = true; }
        if s.contains("super") { hk.super_key = true; }
        if hk == Hotkeys::default() { None } else { Some(hk) }
    }
}

#[derive(Debug, Clone)]
pub struct Action {
    pub delay_ms: u64,
    pub pressure: i32, 
    pub action: String,
    pub args: String,
}

impl Action {
    pub fn execute(&self) -> Result<()> {
        match self.action.as_str() {
            "exec" => {
                Dispatch::call(DispatchType::Exec(&self.args))
                    .context("Failed to call Hyprland exec dispatcher")?;
            }
            "keyword" => {
                if let Some((k, v)) = self.args.split_once(' ') {
                    Keyword::set(k, v).context("Failed to set Hyprland keyword")?;
                } else {
                    Keyword::set(&self.args, "").context("Failed to set Hyprland keyword")?;
                }
            }
            "dispatch" => {
                if let Some((d, a)) = self.args.split_once(' ') {
                    Dispatch::call(DispatchType::Custom(d, a)).context("Failed to call dispatcher")?;
                } else {
                    Dispatch::call(DispatchType::Custom(&self.args, "")).context("Failed to call dispatcher")?;
                }
            }
            _ => {
                log::warn!("Unknown action type: {}", self.action);
            }
        }
        Ok(())
    }
}

pub struct LuaConfig {
    lua: Lua,
}

impl LuaConfig {
    pub fn new(path: &Path) -> Result<Self> {
        let lua = Lua::new();
        let script = read_to_string(path).context("Could not read config file")?;
        lua.load(&script).exec().context("Lua syntax error")?;
        Ok(Self { lua })
    }

    pub fn get_geometry(&self, monitor: &str) -> (i64, i64) {
        let globals = self.lua.globals();
        if let Ok(geometry) = globals.get::<_, LuaTable>("geometry") {
            if let Ok(mon_geom) = geometry.get::<_, LuaTable>(monitor) {
                return (
                    mon_geom.get("corner").unwrap_or(32),
                    mon_geom.get("edge").unwrap_or(8),
                );
            }
            if let Ok(def_geom) = geometry.get::<_, LuaTable>("default") {
                return (
                    def_geom.get("corner").unwrap_or(32),
                    def_geom.get("edge").unwrap_or(8),
                );
            }
        }
        (32, 8)
    }

    pub fn evaluate_zone(
        &self,
        monitor: &str,
        trigger: Option<&str>,
        zone: &str,
        special_ws: Option<&str>,
        hotkeys: &Hotkeys,
    ) -> Result<Vec<Action>> {
        let globals = self.lua.globals();
        let on_zone: LuaFunction = globals.get("on_zone").context("Lua function 'on_zone' not found")?;

        let ctx = self.lua.create_table()?;
        ctx.set("monitor", monitor)?;
        ctx.set("trigger", trigger)?;
        ctx.set("zone", zone)?;
        ctx.set("specialworkspace", special_ws)?;

        let hk = self.lua.create_table()?;
        hk.set("ctrl", hotkeys.ctrl)?;
        hk.set("alt", hotkeys.alt)?;
        hk.set("shift", hotkeys.shift)?;
        hk.set("super", hotkeys.super_key)?;
        ctx.set("hotkeys", hk)?;

        let res: Option<LuaValue> = on_zone.call(ctx).context("Error executing on_zone")?;
        
        Ok(self.parse_actions(res))
    }

    fn parse_actions(&self, res: Option<LuaValue>) -> Vec<Action> {
        let mut actions = Vec::new();
        if let Some(LuaValue::Table(t)) = res {
            if t.contains_key("action").unwrap_or(false) {
                actions.push(Action {
                    delay_ms: t.get("delay").unwrap_or(0),
                    pressure: t.get("pressure").unwrap_or(0),
                    action: t.get("action").unwrap_or_default(),
                    args: t.get("args").unwrap_or_default(),
                });
            } else {
                for pair in t.pairs::<i64, LuaTable>() {
                    if let Ok((_, at)) = pair {
                        actions.push(Action {
                            delay_ms: at.get("delay").unwrap_or(0),
                            pressure: at.get("pressure").unwrap_or(0),
                            action: at.get("action").unwrap_or_default(),
                            args: at.get("args").unwrap_or_default(),
                        });
                    }
                }
            }
        }
        actions
    }
}
