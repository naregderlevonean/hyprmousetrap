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
end
