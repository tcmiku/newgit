use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::{
    io::{Read, Write},
    sync::Mutex,
};
use tauri::{AppHandle, Emitter, Manager};

use crate::Shared;

struct TerminalSession {
    id: String,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

#[derive(Default)]
pub struct TerminalState {
    inner: Mutex<TerminalInner>,
}

#[derive(Default)]
struct TerminalInner {
    desired_id: Option<String>,
    session: Option<TerminalSession>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalOutput {
    session_id: String,
    data: Vec<u8>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalExit {
    session_id: String,
    code: Option<u32>,
}

fn terminate(mut session: TerminalSession) {
    let _ = session.killer.kill();
}

fn size(cols: u16, rows: u16) -> Result<PtySize, String> {
    if !(2..=500).contains(&cols) || !(2..=300).contains(&rows) {
        return Err("终端尺寸无效。".into());
    }
    Ok(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })
}

#[tauri::command]
pub async fn terminal_start(
    app: AppHandle,
    session_id: String,
    repository: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    if session_id.is_empty() || session_id.len() > 128 {
        return Err("终端会话无效。".into());
    }
    let pty_size = size(cols, rows)?;
    tauri::async_runtime::spawn_blocking(move || {
        let shared = app.state::<Shared>();
        let root = {
            let state = shared.lock().map_err(|_| "仓库状态不可用。")?;
            let root = state.root.as_ref().ok_or("请先打开仓库。")?;
            if crate::git::display_path(root) != repository {
                return Err("当前仓库已切换，请重新打开终端。".into());
            }
            root.clone()
        };

        let previous = {
            let state = app.state::<TerminalState>();
            let mut inner = state.inner.lock().map_err(|_| "终端状态不可用。")?;
            inner.desired_id = Some(session_id.clone());
            inner.session.take()
        };
        if let Some(previous) = previous {
            terminate(previous);
        }

        let pair = native_pty_system()
            .openpty(pty_size)
            .map_err(|error| format!("无法创建终端：{error}"))?;
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| format!("无法读取终端：{error}"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| format!("无法写入终端：{error}"))?;

        #[cfg(target_os = "windows")]
        let mut command = CommandBuilder::new("powershell.exe");
        #[cfg(not(target_os = "windows"))]
        let mut command =
            CommandBuilder::new(std::env::var_os("SHELL").unwrap_or_else(|| "/bin/zsh".into()));
        #[cfg(target_os = "windows")]
        command.arg("-NoLogo");
        command.cwd(root.as_os_str());
        command.env("TERM", "xterm-256color");
        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| format!("无法启动 Shell：{error}"))?;
        drop(pair.slave);
        let killer = child.clone_killer();

        let session = TerminalSession {
            id: session_id.clone(),
            master: pair.master,
            writer,
            killer,
        };
        let cancelled = {
            let state = app.state::<TerminalState>();
            let mut inner = state.inner.lock().map_err(|_| "终端状态不可用。")?;
            if inner.desired_id.as_deref() == Some(session_id.as_str()) {
                inner.session = Some(session);
                None
            } else {
                Some(session)
            }
        };
        if let Some(cancelled) = cancelled {
            terminate(cancelled);
            let _ = child.wait();
            return Err("终端启动已取消。".into());
        }

        let output_app = app.clone();
        let output_id = session_id.clone();
        std::thread::spawn(move || stream_output(reader, output_app, output_id));
        std::thread::spawn(move || {
            let code = child.wait().ok().map(|status| status.exit_code());
            let _ = app.emit_to("main", "terminal-exit", TerminalExit { session_id, code });
        });
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

fn stream_output(mut reader: Box<dyn Read + Send>, app: AppHandle, session_id: String) {
    let mut buffer = [0u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(length) => {
                let _ = app.emit_to(
                    "main",
                    "terminal-output",
                    TerminalOutput {
                        session_id: session_id.clone(),
                        data: buffer[..length].to_vec(),
                    },
                );
            }
        }
    }
    let state = app.state::<TerminalState>();
    let finished = state.inner.lock().ok().and_then(|mut inner| {
        if inner
            .session
            .as_ref()
            .is_some_and(|session| session.id == session_id)
        {
            inner.session.take()
        } else {
            None
        }
    });
    if let Some(finished) = finished {
        terminate(finished);
    }
}

#[tauri::command]
pub async fn terminal_write(
    app: AppHandle,
    session_id: String,
    data: Vec<u8>,
) -> Result<(), String> {
    if data.len() > 65536 {
        return Err("终端输入过长。".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<TerminalState>();
        let mut inner = state.inner.lock().map_err(|_| "终端状态不可用。")?;
        let session = inner
            .session
            .as_mut()
            .filter(|session| session.id == session_id);
        let Some(session) = session else {
            return Ok(());
        };
        session
            .writer
            .write_all(&data)
            .map_err(|error| error.to_string())?;
        session.writer.flush().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn terminal_resize(
    app: AppHandle,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let pty_size = size(cols, rows)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<TerminalState>();
        let inner = state.inner.lock().map_err(|_| "终端状态不可用。")?;
        if let Some(session) = inner
            .session
            .as_ref()
            .filter(|session| session.id == session_id)
        {
            session
                .master
                .resize(pty_size)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn terminal_stop(app: AppHandle, session_id: String) {
    let _ = tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<TerminalState>();
        let old = state.inner.lock().ok().and_then(|mut inner| {
            if inner.desired_id.as_deref() == Some(session_id.as_str()) {
                inner.desired_id = None;
            }
            if inner
                .session
                .as_ref()
                .is_some_and(|session| session.id == session_id)
            {
                inner.session.take()
            } else {
                None
            }
        });
        if let Some(old) = old {
            terminate(old);
        }
    })
    .await;
}

pub fn shutdown(app: &AppHandle) {
    let state = app.state::<TerminalState>();
    let old = state.inner.lock().ok().and_then(|mut inner| {
        inner.desired_id = None;
        inner.session.take()
    });
    if let Some(old) = old {
        terminate(old);
    }
}
