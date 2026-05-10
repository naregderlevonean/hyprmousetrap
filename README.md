[Cheese](Cheese.png)





# hyprmousetrap

A high-performance, DPI-aware hot-corner and edge-action daemon for **Hyprland**, written in Rust. It utilizes a powerful and fully programmable **Lua API**, allowing you to conditionally trigger complex actions when your mouse enters screen corners or edges.




## Features

- **Programmable Logic (Lua)**: Complete control via Lua scripting.
- **8 Active Zones**: 4 corners and 4 edges (`top`, `bottom`, `left`, `right`, `top-left`, `top-right`, `bottom-left`, `bottom-right`).
- **Pressure-Based Triggers**: Trigger actions by "pushing" the cursor against the screen edge.
- **Modifier Key Integration**: Read `super`, `shift`, `ctrl`, and `alt` during interaction.
- **DPI Aware**: Automatic logical coordinate scaling for HiDPI displays.
- **Trigger-based Logic**: Distinguish between simple hovering, window dragging, or clicking.
- **Intent Verification**: Configurable delays with real-time cursor tracking to prevent accidental triggers.





## Installation



### Easiest. Direct via Cargo

```bash
cargo install --git https://github.com/naregderlevonean/hyprmousetrap
```



### Local. From Source

```bash
git clone https://github.com/naregderlevonean/hyprmousetrap.git
cd hyprmousetrap
cargo build --release
cargo install --path .
```

*(Ensure `~/.cargo/bin` is in your `$PATH`)*



### Arch GNU/Linux. Using an AUR Helper

```bash
yay -S hyprmousetrap-git
```




## Requirements

To use Hotkeys, your user must have permission to read input devices:

```bash
sudo usermod -aG input $USER
```




## Configuration

**Path:** `~/.config/hypr/hyprmousetrap.lua`

Configuration is handled entirely through a Lua script. The daemon calls the `on_zone(ctx)` function whenever a zone interaction is detected, providing a context object.



### The Context Object (`ctx`)

| Property | Type | Description |
| :--- | :--- | :--- |
| `ctx.zone` | `string` | `top`, `bottom`, `left`, `right`, `top-left`, `top-right`, `bottom-left`, `bottom-right`. |
| `ctx.trigger` | `string\|nil` | Custom string (e.g., "drag") or `nil` for hover. |
| `ctx.monitor` | `string` | System name (e.g., `eDP-1`). |
| `ctx.specialworkspace`| `string\|nil` | Name of active special workspace. |
| `ctx.hotkeys` | `table` | Boolean flags: `ctrl`, `alt`, `shift`, `super`. |



### Return Value

To trigger an action, the Lua function must return a table containing:

- **pressure**: Accumulative raw movement delta (pushing force) required.
- **delay**: Number of milliseconds to wait (validating the cursor remains in the zone).
- **action**: The Hyprland dispatch action (e.g., "exec", "workspace").
- **args**: Arguments for the action.

Return `nil` to do nothing.




## Example Configuration

```lua
-- Screen zones and corner geometry settings
geometry = {
    default = { corner = 4, edge = 2 },
    ["eDP-1"] = { corner = 60, edge = 10 } 
}

function on_zone(ctx)
    -- Debug mode: Notify current zone when Shift is held
    if ctx.hotkeys.shift then
        return { action = "exec", args = "notify-send 'Mouse trapped in: " .. ctx.zone .. "'" }
    end

    -- Handle window dragging actions
    if ctx.trigger == "drag" then
        if ctx.zone == "top-left" or ctx.zone == "top-right" then
            -- Expand window to fullscreen when dragged to top corners
            return { action = "dispatch", args = "fullscreen 0" }
        end
    end

    -- Top-left corner: Toggle overview/launcher
    if ctx.zone == "top-left" then
        return { action = "dispatch", args = "overview:toggle" }
    end

    -- Top-right corner: Lock screen with a 2-second dwell delay
    if ctx.zone == "top-right" then
        return { delay = 2000, action = "exec", args = "hyprlock" }
    end

    -- Bottom-right corner: Close window with pressure
    -- No delay needed, just a firm "push" into the corner
    if ctx.zone == "bottom-right" then
        return { pressure = 500, action = "dispatch", args = "killactive" }
    end

    -- Bottom edge logic: Launch terminal and notify
    if ctx.zone == "bottom" then
        if ctx.specialworkspace then
            return { delay = 0, action = "dispatch", args = "togglespecialworkspace" }
        end
        -- Multi-action
        return {
            { action = "exec", args = "foot" },
            { action = "exec", args = "notify-send 'Terminal Spawned' 'Bottom edge trigger'" }
        }
    end

    -- Right edge: Navigate workspaces on specific monitor using Super key
    if ctx.zone == "right" and ctx.hotkeys.super and ctx.monitor == "DP-1" then
        return { action = "dispatch", args = "workspace +1" }
    end

    return {}
end```




## Usage



### Daemon Mode (Hover Triggers)

Run the daemon in your `hyprland.conf`:

```hyprlang
exec-once = hyprmousetrap
```



### Manual Triggers (Drag/Click)

Integrate with Hyprland mouse bindings to enable "Drag to corner" actions. The argument after `hyprmousetrap` is passed to the Lua script as `ctx.trigger`.

```hyprlang
bindn = , mouse:272, exec, hyprmousetrap drag
```




## License

This project is licensed under the GPL-3.0 License. See the `LICENSE` file for details.

