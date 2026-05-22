use anyhow::{anyhow, Context, Result};
use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

fn runtime_dir() -> Result<PathBuf> {
    if let Ok(xdg) = env::var("XDG_RUNTIME_DIR") {
        return Ok(PathBuf::from(xdg));
    }
    // fallback: /run/user/<uid>
    let uid = unsafe { libc::geteuid() };
    Ok(PathBuf::from(format!("/run/user/{uid}")))
}

fn socket_path() -> Result<PathBuf> {
    let his = env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .context("HYPRLAND_INSTANCE_SIGNATURE not set (are you running under Hyprland?)")?;
    let base = runtime_dir()?;
    Ok(base.join("hypr").join(his).join(".socket.sock"))
}

/// Send a raw command to Hyprland's .socket.sock (hyprctl-like socket).
/// Example commands:
/// - "j/cursorpos"
/// - "j/monitors"
/// - "dispatch hl.dsp.window.close()"
/// - "eval hl.dispatch(hl.dsp.window.close())"
/// - "keyword general:border_size 10"
pub fn request(cmd: &str) -> Result<String> {
    let path = socket_path()?;
    let mut stream =
        UnixStream::connect(&path).with_context(|| format!("Failed to connect to {path:?}"))?;

    stream
        .write_all(cmd.as_bytes())
        .with_context(|| format!("Failed to write IPC command: {cmd}"))?;
    // Hyprland expects the client to close; shutdown write side to signal EOF.
    let _ = stream.shutdown(std::net::Shutdown::Write);

    let mut buf = String::new();
    stream
        .read_to_string(&mut buf)
        .context("Failed to read IPC response")?;

    if buf.is_empty() {
        return Err(anyhow!("Empty IPC response for command: {cmd}"));
    }

    Ok(buf)
}
