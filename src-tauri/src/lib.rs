mod git;
#[cfg(target_os = "macos")]
mod menu_bar;

use notify::{RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::Emitter;
#[cfg(target_os = "macos")]
use tauri::Manager;

#[derive(Default)]
struct Repository {
    root: Option<PathBuf>,
    watcher: Option<notify::RecommendedWatcher>,
}

type Shared = Arc<Mutex<Repository>>;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RepositoryChange {
    root: String,
    paths: Vec<String>,
    git_state: bool,
}

fn git_relative<'a>(path: &'a Path, git_dir: Option<&Path>) -> Option<&'a Path> {
    if let Some(relative) = git_dir.and_then(|dir| path.strip_prefix(dir).ok()) {
        return Some(relative);
    }
    let mut parts = path.components();
    while let Some(part) = parts.next() {
        if part.as_os_str() == ".git" {
            return Some(parts.as_path());
        }
    }
    None
}

fn git_state_change(relative: &Path) -> bool {
    matches!(
        relative
            .components()
            .next()
            .and_then(|part| part.as_os_str().to_str()),
        Some(
            "index"
                | "HEAD"
                | "config"
                | "config.worktree"
                | "packed-refs"
                | "refs"
                | "FETCH_HEAD"
                | "MERGE_HEAD"
                | "rebase-merge"
                | "rebase-apply"
                | "sequencer"
        )
    )
}

fn relevant_change(path: &Path, git_dir: Option<&Path>) -> bool {
    if let Some(relative) = git_relative(path, git_dir) {
        return git_state_change(relative);
    }
    for part in path.components() {
        match part.as_os_str().to_str() {
            Some("node_modules" | "target" | "dist" | "objects" | ".next") => return false,
            _ => {}
        }
    }
    true
}

#[cfg(test)]
mod watcher_tests {
    use super::relevant_change;
    use std::path::Path;

    #[test]
    fn ignores_git_noise_and_build_output_but_keeps_repository_changes() {
        let git_dir = Path::new("repo/.git");
        assert!(relevant_change(
            Path::new("repo/src/main.ts"),
            Some(git_dir)
        ));
        assert!(relevant_change(Path::new("repo/.git/index"), Some(git_dir)));
        assert!(relevant_change(
            Path::new("repo/.git/refs/heads/main"),
            Some(git_dir)
        ));
        assert!(!relevant_change(
            Path::new("repo/.git/COMMIT_EDITMSG"),
            Some(git_dir)
        ));
        assert!(!relevant_change(
            Path::new("repo/.git/objects/ab/123"),
            Some(git_dir)
        ));
        assert!(!relevant_change(
            Path::new("repo/src-tauri/target/release/app.exe"),
            Some(git_dir)
        ));
    }
}

#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "camelCase")]
enum Request {
    Open { path: String },
    Close,
    Snapshot,
    Diff { path: String, staged: bool },
    Mutate { action: git::Mutation },
    History { offset: usize, all: bool },
    GitLog { limit: usize, all: bool },
    Branches,
    Remotes,
    CommitPatch { oid: String },
}

fn watch(root: &Path, app: tauri::AppHandle) -> Option<notify::RecommendedWatcher> {
    let (tx, rx) = std::sync::mpsc::channel();
    let git_dir = git::git_dir(root)
        .ok()
        .and_then(|dir| dir.canonicalize().ok());
    let event_git_dir = git_dir.clone();
    let thread_git_dir = git_dir.clone();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event {
            if matches!(event.kind, notify::EventKind::Access(_)) {
                return;
            }
            let relevant: Vec<_> = event
                .paths
                .into_iter()
                .filter(|path| relevant_change(path, event_git_dir.as_deref()))
                .collect();
            if !relevant.is_empty() {
                let _ = tx.send(relevant);
            }
        }
    })
    .ok()?;
    watcher.watch(root, RecursiveMode::Recursive).ok()?;
    if let Some(dir) = git_dir {
        if !dir.starts_with(root) {
            let _ = watcher.watch(&dir, RecursiveMode::Recursive);
        }
    }
    let path = git::display_path(root);
    let root = root.to_path_buf();
    std::thread::spawn(move || {
        while let Ok(mut changed) = rx.recv() {
            while let Ok(next) = rx.recv_timeout(Duration::from_millis(250)) {
                changed.extend(next);
            }
            let mut paths = HashSet::new();
            let mut git_state = false;
            for entry in changed {
                if git_relative(&entry, thread_git_dir.as_deref()).is_some() {
                    git_state = true;
                } else if let Ok(relative) = entry.strip_prefix(&root) {
                    paths.insert(relative.to_string_lossy().replace('\\', "/"));
                } else {
                    git_state = true;
                }
            }
            let mut paths: Vec<_> = paths.into_iter().collect();
            paths.sort();
            let _ = app.emit(
                "repository-changed",
                RepositoryChange {
                    root: path.clone(),
                    paths,
                    git_state,
                },
            );
        }
    });
    Some(watcher)
}

#[tauri::command]
async fn git_request(
    request: Request,
    repository: Option<String>,
    state: tauri::State<'_, Shared>,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut state = shared
            .lock()
            .map_err(|_| "仓库状态不可用，请重新启动应用。")?;
        if matches!(request, Request::Close) {
            state.watcher = None;
            state.root = None;
            return Ok(serde_json::Value::Null);
        }
        if let Request::Open { path } = request {
            let root = git::open(&path)?;
            let snapshot = git::snapshot(&root)?;
            state.watcher = watch(&root, app);
            state.root = Some(root);
            return serde_json::to_value(snapshot).map_err(|e| e.to_string());
        }
        let root = state.root.as_ref().ok_or("请先打开仓库。")?;
        if repository.as_deref() != Some(git::display_path(root).as_str()) {
            return Err("当前仓库已切换，请刷新后重试。".into());
        }
        let value = match request {
            Request::Snapshot => serde_json::to_value(git::snapshot(root)?),
            Request::Diff { path, staged } => serde_json::to_value(git::diff(root, &path, staged)?),
            Request::Mutate { action } => serde_json::to_value(git::mutate(root, action)?),
            Request::History { offset, all } => {
                serde_json::to_value(git::history(root, offset, all)?)
            }
            Request::GitLog { limit, all } => serde_json::to_value(git::git_log(root, limit, all)?),
            Request::Branches => serde_json::to_value(git::branches(root)?),
            Request::Remotes => serde_json::to_value(git::remotes(root)?),
            Request::CommitPatch { oid } => serde_json::to_value(git::commit_patch(root, &oid)?),
            Request::Open { .. } | Request::Close => unreachable!(),
        };
        value.map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn launch_path() -> Option<String> {
    std::env::args().nth(1).filter(|arg| !arg.starts_with('-'))
}

#[tauri::command]
async fn set_menu_bar_mode(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        tauri::async_runtime::spawn_blocking(move || menu_bar::set_mode(app, enabled))
            .await
            .map_err(|error| error.to_string())?
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, enabled);
        Err("菜单栏模式仅支持 macOS。".into())
    }
}

#[tauri::command]
fn hide_menu_bar_panel(app: tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    menu_bar::hide_panel(app);
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Shared::default())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.manage(menu_bar::MenuBarState::default());
            Ok(())
        })
        .on_window_event(|window, event| {
            #[cfg(target_os = "macos")]
            menu_bar::on_window_event(window, event);
            #[cfg(not(target_os = "macos"))]
            let _ = (window, event);
        })
        .invoke_handler(tauri::generate_handler![
            git_request,
            launch_path,
            set_menu_bar_mode,
            hide_menu_bar_panel,
            quit_app
        ])
        .build(tauri::generate_context!())
        .expect("Unable to start gitpane")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                if let Err(error) = menu_bar::restore_window(app) {
                    eprintln!("{error}");
                }
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}
