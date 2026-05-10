![Cheese](Cheese.png)

# hyprmousetrap

A high-performance, DPI-aware hot-corner and edge-action daemon for **Hyprland**, written in Rust. It utilizes a powerful and fully programmable **Lua API**, allowing you to conditionally trigger complex actions when your mouse enters screen corners or edges.

## Features

* **Programmable Logic (Lua)**: Complete control via Lua scripting.
* **8 Active Zones**: 4 corners and 4 edges (`top`, `bottom`, `left`, `right`, `top-left`, `top-right`, `bottom-left`, `bottom-right`).
* **Pressure-Based Triggers**: Trigger actions by "pushing" the cursor against the screen edge.
* **Modifier Key Integration**: Read `super`, `shift`, `ctrl`, and `alt` during interaction.
* **DPI Aware**: Automatic logical coordinate scaling for HiDPI displays.
* **Trigger-based Logic**: Distinguish between simple hovering, window dragging, or clicking.
* **Intent Verification**: Configurable delays with real-time cursor tracking to prevent accidental triggers.
* **Context Aware**: Detects active special workspaces and specific monitors.

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
| --- | --- | --- |
| `ctx.zone` | `string` | `top`, `bottom`, `left`, `right`, `top-left`, `top-right`, `bottom-left`, `bottom-right`. |
| `ctx.trigger` | `string|nil` | Custom string (e.g., "drag") or `nil` for hover. |
| `ctx.monitor` | `string` | System name (e.g., `eDP-1`). |
| `ctx.specialworkspace` | `string|nil` | Name of active special workspace (if any). |
| `ctx.hotkeys` | `table` | Boolean flags: `ctrl`, `alt`, `shift`, `super`. |

### Return Value

To trigger an action, the Lua function must return a table containing:

* **pressure**: Accumulative raw movement delta (pushing force) required.
* **delay**: Number of milliseconds to wait (dwell time).
* **action**: The Hyprland dispatch action (e.g., "exec", "workspace").
* **args**: Arguments for the action.

Return `nil` or `{}` to do nothing.

## Usage

### Daemon Mode (Hover Triggers)

Run the daemon in your `hyprland.conf`:

```hyprlang
exec-once = hyprmousetrap

```

### Manual Triggers (Drag/Click)

Integrate with Hyprland mouse bindings to enable "Drag to corner" actions.

```hyprlang
bindn = , mouse:272, exec, hyprmousetrap drag

```

## License

This project is licensed under the GPL-3.0 License. See the `LICENSE` file for details.
