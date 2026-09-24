use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    ActivationPolicy, AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WindowEvent,
};

#[derive(Default)]
pub struct MenuBarState {
    enabled: AtomicBool,
    close_on_click: AtomicBool,
    last_blur: Mutex<Option<Instant>>,
}

fn position_panel(app: &AppHandle, window: &tauri::WebviewWindow) -> Result<(), String> {
    let tray = app
        .tray_by_id("gitpane-menu-bar")
        .ok_or("菜单栏图标不可用")?;
    let rect = tray
        .rect()
        .map_err(|e| e.to_string())?
        .ok_or("无法获取菜单栏图标位置")?;
    // macOS tray coordinates are physical pixels, including on Retina displays.
    let origin = rect.position.to_physical::<f64>(1.0);
    let extent = rect.size.to_physical::<f64>(1.0);
    if extent.height <= 0.0 {
        return Err("菜单栏图标正在布局，请稍后重试".into());
    }
    let anchor = PhysicalPosition::new(origin.x + extent.width / 2.0, origin.y + extent.height);
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;
    let monitor = monitors
        .iter()
        .find(|monitor| {
            let p = monitor.position();
            let s = monitor.size();
            anchor.x >= f64::from(p.x)
                && anchor.x < f64::from(p.x) + f64::from(s.width)
                && origin.y >= f64::from(p.y)
                && origin.y < f64::from(p.y) + f64::from(s.height)
        })
        .ok_or("无法确定菜单栏所在屏幕")?;
    let scale = monitor.scale_factor();
    let size = LogicalSize::new(420.0, 560.0).to_physical::<u32>(scale);
    let work = monitor.work_area();
    let margin = (8.0 * scale).round() as i32;
    let left = work.position.x + margin;
    let right = work.position.x + work.size.width as i32 - size.width as i32 - margin;
    let x = (anchor.x.round() as i32 - size.width as i32 / 2).clamp(left, right.max(left));
    let y = anchor.y.round() as i32 + (6.0 * scale).round() as i32;
    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())
}

pub fn set_mode(app: AppHandle, enabled: bool) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("主窗口不可用")?;
    let state = app.state::<MenuBarState>();
    if enabled {
        if state.enabled.load(Ordering::SeqCst) {
            return Ok(());
        }
        window.hide().map_err(|error| error.to_string())?;
        if let Some(tray) = app.tray_by_id("gitpane-menu-bar") {
            tray.set_visible(true).map_err(|error| error.to_string())?;
        } else {
            let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))
                .map_err(|error| error.to_string())?;
            let quit = MenuItem::with_id(&app, "tray-quit", "退出 GitPane", true, None::<&str>)
                .map_err(|error| error.to_string())?;
            let open = MenuItem::with_id(&app, "tray-open", "打开面板", true, None::<&str>)
                .map_err(|error| error.to_string())?;
            let restore = MenuItem::with_id(&app, "tray-restore", "恢复主窗口", true, None::<&str>)
                .map_err(|error| error.to_string())?;
            let menu = Menu::with_items(&app, &[&open, &restore, &quit])
                .map_err(|error| error.to_string())?;
            TrayIconBuilder::with_id("gitpane-menu-bar")
                .icon(icon)
                .icon_as_template(true)
                .tooltip("GitPane · 点击打开")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id() == "tray-quit" {
                        app.exit(0);
                    } else if event.id() == "tray-restore" {
                        if let Err(error) = restore_window(app) {
                            eprintln!("{error}");
                        }
                    } else if event.id() == "tray-open" {
                        if let Err(error) = show_panel(app) {
                            eprintln!("{error}");
                        }
                    }
                })
                .on_tray_icon_event(|icon, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state,
                        ..
                    } = event
                    {
                        if let Some(window) = icon.app_handle().get_webview_window("main") {
                            let state = icon.app_handle().state::<MenuBarState>();
                            // Clicking the tray may blur the panel before mouse-up. Do not
                            // immediately reopen the panel that this same click dismissed.
                            let just_blurred =
                                state.last_blur.lock().unwrap().is_some_and(|time| {
                                    time.elapsed() < Duration::from_millis(250)
                                });
                            if button_state == MouseButtonState::Down {
                                state.close_on_click.store(
                                    window.is_visible().unwrap_or(false) || just_blurred,
                                    Ordering::SeqCst,
                                );
                                return;
                            }
                            if state.close_on_click.swap(false, Ordering::SeqCst) {
                                let _ = window.hide();
                            } else {
                                if let Err(error) = show_panel(icon.app_handle()) {
                                    eprintln!("{error}");
                                }
                            }
                        }
                    }
                })
                .build(&app)
                .map_err(|error| error.to_string())?;
        }
        window
            .set_decorations(false)
            .map_err(|error| error.to_string())?;
        window
            .set_always_on_top(true)
            .map_err(|error| error.to_string())?;
        app.set_activation_policy(ActivationPolicy::Accessory)
            .map_err(|error| error.to_string())?;
        window
            .set_resizable(false)
            .map_err(|error| error.to_string())?;
        state.enabled.store(true, Ordering::SeqCst);
        if let Some(tray) = app.tray_by_id(crate::SYSTEM_TRAY_ID) {
            tray.set_visible(false).map_err(|error| error.to_string())?;
        }
        *state.last_blur.lock().unwrap() = None;
        // AppKit initially reports a zero-height status item. Wait on the blocking
        // worker while its main run loop lays out the icon, then anchor the panel.
        let mut last_error = String::new();
        for _ in 0..40 {
            match show_panel(&app) {
                Ok(()) => return Ok(()),
                Err(error) => last_error = error,
            }
            thread::sleep(Duration::from_millis(25));
        }
        return Err(last_error);
    } else {
        restore_window(&app)?;
    }
    Ok(())
}

fn show_panel(app: &AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("主窗口不可用")?;
    if app.state::<MenuBarState>().enabled.load(Ordering::SeqCst) {
        position_panel(app, &window)?;
    }
    window.unminimize().map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

pub fn restore_window(app: &AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("主窗口不可用")?;
    app.state::<MenuBarState>()
        .enabled
        .store(false, Ordering::SeqCst);
    app.set_activation_policy(ActivationPolicy::Regular)
        .map_err(|e| e.to_string())?;
    window.set_always_on_top(false).map_err(|e| e.to_string())?;
    window.set_decorations(true).map_err(|e| e.to_string())?;
    window.set_resizable(true).map_err(|e| e.to_string())?;
    window
        .set_min_size(Some(LogicalSize::new(860.0, 580.0)))
        .map_err(|e| e.to_string())?;
    window
        .set_size(LogicalSize::new(1320.0, 850.0))
        .map_err(|e| e.to_string())?;
    window.center().map_err(|e| e.to_string())?;
    app.emit("menu-bar-restored", ())
        .map_err(|e| e.to_string())?;
    show_panel(app)?;
    // Keep the registered icon for reuse. Dropping the last TrayIcon from an async
    // command removes NSStatusItem off the main thread and crashes macOS.
    // set_visible dispatches its native work to the main thread through Tauri.
    if let Some(tray) = app.tray_by_id("gitpane-menu-bar") {
        tray.set_visible(false).map_err(|error| error.to_string())?;
    }
    if let Some(tray) = app.tray_by_id(crate::SYSTEM_TRAY_ID) {
        tray.set_visible(true).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn hide_panel(app: AppHandle) {
    let state = app.state::<MenuBarState>();
    if state.enabled.load(Ordering::SeqCst) {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
    }
}

pub fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    if window.label() != "main" {
        return;
    }
    let app = window.app_handle();
    let Some(state) = app.try_state::<MenuBarState>() else {
        return;
    };
    if !state.enabled.load(Ordering::SeqCst) {
        return;
    }
    match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let _ = window.hide();
        }
        WindowEvent::Focused(false) if !window.is_focused().unwrap_or(false) => {
            if window.is_visible().unwrap_or(false) {
                *state.last_blur.lock().unwrap() = Some(Instant::now());
                let _ = window.hide();
            }
        }
        _ => {}
    }
}
