pub mod bridge;
mod cloud;
mod model;
mod providers;
mod storage;

use model::{ProviderView, Settings};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicI64, Ordering},
        Mutex,
    },
};
use storage::Store;
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
async fn export_token_csv(app: tauri::AppHandle, csv: String, days: u32) -> Result<bool, String> {
    if ![7, 14, 30].contains(&days)
        || csv.len() > 10_000
        || !csv.starts_with("date_utc,observed_tokens\n")
    {
        return Err("Invalid token export".into());
    }
    for line in csv.lines().skip(1) {
        let (date, tokens) = line.split_once(',').ok_or("Invalid token export")?;
        if date.len() != 10
            || !date.chars().all(|c| c.is_ascii_digit() || c == '-')
            || (!tokens.is_empty() && tokens.parse::<u64>().is_err())
        {
            return Err("Invalid token export".into());
        }
    }
    tauri::async_runtime::spawn_blocking(move || {
        let selected = app
            .dialog()
            .file()
            .add_filter("CSV", &["csv"])
            .set_file_name(format!("maxxit-codex-{days}-days.csv"))
            .blocking_save_file();
        let Some(selected) = selected else {
            return Ok(false);
        };
        let path = selected.into_path().map_err(|e| e.to_string())?;
        std::fs::write(path, csv).map_err(|e| e.to_string())?;
        Ok(true)
    })
    .await
    .map_err(|e| e.to_string())?
}

struct Runtime {
    store: Mutex<Store>,
    directory: PathBuf,
    pairing: Mutex<Option<Value>>,
    panel_hidden_at: AtomicI64,
}
fn collect(runtime: &Runtime) -> Result<Value, String> {
    let store = runtime.store.lock().map_err(|_| "Storage unavailable")?;
    let settings = store.settings();
    let codex = if settings.codex_enabled {
        providers::codex_local()
    } else {
        ProviderView::empty("codex")
    };
    let claude = if settings.claude_enabled {
        providers::claude_local(&runtime.directory, &settings.claude_account)
    } else {
        ProviderView::empty("claude")
    };
    for provider in [&codex, &claude] {
        if let Some(observation) = &provider.observation {
            store.record(observation)?;
        }
    }
    Ok(
        json!({"settings":settings,"providers":[codex,claude],"history":store.history()?,"projects":store.projects()?,"cloud":{"connected":false,"plan":"free"}}),
    )
}
#[tauri::command]
async fn snapshot(runtime: State<'_, Runtime>) -> Result<Value, String> {
    let mut data = collect(&runtime)?;
    let settings: Settings =
        serde_json::from_value(data["settings"].clone()).map_err(|e| e.to_string())?;
    data["cloud"] = cloud::state(&settings).await;
    Ok(data)
}
fn runtime_settings(runtime: &Runtime) -> Result<Settings, String> {
    Ok(runtime
        .store
        .lock()
        .map_err(|_| "Storage unavailable")?
        .settings())
}
#[tauri::command]
async fn cloud_start(
    runtime: State<'_, Runtime>,
    usage: bool,
    metadata: bool,
) -> Result<Value, String> {
    let settings = runtime_settings(&runtime)?;
    let value=cloud::request(&settings,"pairing/start",Some(json!({"name":"Maxxit desktop","consent":{"usage":usage,"metadata":metadata,"excerpts":false}})),false).await?;
    *runtime.pairing.lock().map_err(|_| "Pairing unavailable")? = Some(value.clone());
    Ok(
        json!({"code":value["code"],"expiresAt":value["expiresAt"],"url":format!("{}/workspace/connections?code={}",cloud::origin(&settings)?,value["code"].as_str().ok_or("Invalid pairing code")?)}),
    )
}
#[tauri::command]
async fn cloud_redeem(runtime: State<'_, Runtime>) -> Result<(), String> {
    let settings = runtime_settings(&runtime)?;
    let pairing = runtime
        .pairing
        .lock()
        .map_err(|_| "Pairing unavailable")?
        .clone()
        .ok_or("Start account connection first")?;
    let value = cloud::request(
        &settings,
        "pairing/redeem",
        Some(json!({"secret":pairing["secret"]})),
        false,
    )
    .await?;
    cloud::save_credential(value["token"].as_str().ok_or("Invalid device token")?)?;
    *runtime.pairing.lock().map_err(|_| "Pairing unavailable")? = None;
    Ok(())
}
#[tauri::command]
async fn cloud_disconnect(runtime: State<'_, Runtime>) -> Result<(), String> {
    let settings = runtime_settings(&runtime)?;
    cloud::request(&settings, "collector/disconnect", Some(json!({})), true).await?;
    cloud::forget_credential()?;
    let store = runtime.store.lock().map_err(|_| "Storage unavailable")?;
    let mut settings = store.settings();
    settings.cloud_sync = false;
    settings.ai_consent = false;
    settings.email = false;
    store.save_settings(&settings)
}
#[tauri::command]
async fn cloud_sync(runtime: State<'_, Runtime>) -> Result<(), String> {
    sync_cloud(&runtime).await
}
async fn sync_cloud(runtime: &Runtime) -> Result<(), String> {
    let data = collect(runtime)?;
    let settings = runtime_settings(runtime)?;
    if settings.cloud_sync {
        for provider in data["providers"].as_array().ok_or("Invalid providers")? {
            if !provider["observation"].is_null() {
                cloud::request(
                    &settings,
                    "desktop/usage",
                    Some(provider["observation"].clone()),
                    true,
                )
                .await?;
            }
        }
    }
    if settings.ai_consent {
        for project in data["projects"].as_array().ok_or("Invalid projects")? {
            cloud::request(&settings,"desktop/project",Some(json!({"id":project["id"],"name":project["name"],"description":project["description"]})),true).await?;
        }
    }
    Ok(())
}
#[tauri::command]
async fn cloud_action(runtime: State<'_, Runtime>, action: String) -> Result<Value, String> {
    if !["checkout", "portal", "generate"].contains(&action.as_str()) {
        return Err("Unknown cloud action".into());
    }
    if action == "generate" {
        let settings = runtime_settings(&runtime)?;
        if !settings.ai_consent {
            return Err("Enable project sharing before requesting ideas".into());
        }
        cloud::request(
            &settings,
            "desktop/preferences",
            Some(json!({"aiConsent":true,"reminders":settings.cloud_sync})),
            true,
        )
        .await?;
        sync_cloud(&runtime).await?;
    }
    cloud::request(
        &runtime_settings(&runtime)?,
        &format!("desktop/{action}"),
        Some(json!({})),
        true,
    )
    .await
}
#[tauri::command]
async fn cloud_preferences(runtime: State<'_, Runtime>) -> Result<(), String> {
    let settings = runtime_settings(&runtime)?;
    cloud::request(&settings,"desktop/preferences",Some(json!({"timezone":settings.timezone,"hoursBefore":settings.hours_before,"shortHoursBefore":settings.short_hours_before,"minRemaining":settings.min_remaining,"quietStart":settings.quiet_start,"quietEnd":settings.quiet_end,"dailyLimit":settings.daily_limit,"email":settings.email,"aiConsent":settings.ai_consent,"reminders":settings.cloud_sync})),true).await?;
    Ok(())
}
#[tauri::command]
fn open_link(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let parsed = url::Url::parse(&url).map_err(|_| "Invalid link")?;
    if parsed.scheme() != "https"
        || parsed.username() != ""
        || parsed.password().is_some()
        || !matches!(
            parsed.host_str(),
            Some("maxxit.app") | Some("polar.sh") | Some("sandbox.polar.sh") | Some("github.com")
        )
    {
        return Err("This link is not an allowed Maxxit destination".into());
    }
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn save_settings(runtime: State<'_, Runtime>, settings: Settings) -> Result<(), String> {
    if !["system", "light", "dark"].contains(&settings.theme.as_str())
        || !["codex", "claude"].contains(&settings.tray_provider.as_str())
        || !(0..=100).contains(&settings.min_remaining)
        || !(0..=23).contains(&settings.quiet_start)
        || !(0..=23).contains(&settings.quiet_end)
        || !(1..=7).contains(&settings.daily_limit)
        || !(1..=168).contains(&settings.hours_before)
        || !(1..=4).contains(&settings.short_hours_before)
    {
        return Err("Invalid preferences".into());
    }
    runtime
        .store
        .lock()
        .map_err(|_| "Storage unavailable")?
        .save_settings(&settings)
}
#[tauri::command]
fn claude_preview(runtime: State<'_, Runtime>) -> Result<Value, String> {
    bridge::preview(&runtime.directory)
}
#[tauri::command]
fn connect_provider(runtime: State<'_, Runtime>, provider: String) -> Result<(), String> {
    if provider == "claude" {
        bridge::install(&runtime.directory)?;
    } else if provider != "codex" {
        return Err("Unknown provider".into());
    }
    let store = runtime.store.lock().map_err(|_| "Storage unavailable")?;
    let mut settings = store.settings();
    if provider == "claude" {
        settings.claude_enabled = true;
    } else {
        settings.codex_enabled = true;
    }
    store.save_settings(&settings)
}
#[tauri::command]
fn disconnect_provider(runtime: State<'_, Runtime>, provider: String) -> Result<(), String> {
    if provider == "claude" {
        bridge::uninstall(&runtime.directory)?;
    } else if provider != "codex" {
        return Err("Unknown provider".into());
    }
    let store = runtime.store.lock().map_err(|_| "Storage unavailable")?;
    let mut settings = store.settings();
    if provider == "claude" {
        settings.claude_enabled = false;
    } else {
        settings.codex_enabled = false;
    }
    store.save_settings(&settings)
}
#[tauri::command]
fn save_project(
    runtime: State<'_, Runtime>,
    name: String,
    description: String,
) -> Result<(), String> {
    if name.trim().is_empty() || name.len() > 120 || description.len() > 10000 {
        return Err("Project name or description exceeds its limit".into());
    }
    runtime.store.lock().map_err(|_|"Storage unavailable")?.save_project(&json!({"id":uuid::Uuid::new_v4().to_string(),"name":name.trim(),"description":description,"createdAt":chrono::Utc::now().to_rfc3339()}))
}
#[tauri::command]
fn remove_project(runtime: State<'_, Runtime>, id: String) -> Result<(), String> {
    runtime
        .store
        .lock()
        .map_err(|_| "Storage unavailable")?
        .remove_project(&id)
}

#[tauri::command]
fn tray_resize(window: tauri::WebviewWindow, height: f64) -> Result<(), String> {
    if window.label() != "tray" || !height.is_finite() {
        return Err("Invalid panel size".into());
    }
    let available = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| m.work_area().size.height as f64 / m.scale_factor() - 8.0)
        .unwrap_or(860.0);
    let height = height.clamp(350.0, 860.0).min(available.max(200.0));
    window
        .set_size(tauri::LogicalSize::new(440.0, height))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn tray_action(app: tauri::AppHandle, page: String) -> Result<(), String> {
    if ![
        "Overview",
        "Analytics",
        "Connections",
        "Settings",
        "close",
        "quit",
    ]
    .contains(&page.as_str())
    {
        return Err("Unknown panel action".into());
    }
    if let Some(panel) = app.get_webview_window("tray") {
        let _ = panel.hide();
    }
    if page == "quit" {
        app.exit(0);
    } else if page != "close" {
        if let Some(main) = app.get_webview_window("main") {
            main.show().map_err(|e| e.to_string())?;
            main.set_focus().map_err(|e| e.to_string())?;
            main.emit("navigate-to", page).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn show_tray_panel(
    app: &tauri::AppHandle,
    position: tauri::PhysicalPosition<f64>,
    rect: tauri::Rect,
    toggle: bool,
) {
    // macOS can blur the panel on mouse-down before the tray click arrives.
    // Treat that click as dismissal rather than reopening the panel immediately.
    let elapsed = chrono::Utc::now().timestamp_millis()
        - app
            .state::<Runtime>()
            .panel_hidden_at
            .load(Ordering::Relaxed);
    if toggle && (0..200).contains(&elapsed) {
        return;
    }
    if let Some(panel) = app.get_webview_window("tray") {
        if toggle && panel.is_visible().unwrap_or(false) {
            let _ = panel.hide();
            return;
        }
        let monitors = panel.available_monitors().unwrap_or_default();
        let monitor = monitors.iter().find(|m| {
            position.x >= m.position().x as f64
                && position.x < (m.position().x as f64 + m.size().width as f64)
                && position.y >= m.position().y as f64
                && position.y < (m.position().y as f64 + m.size().height as f64)
        });
        let scale = monitor.map(|m| m.scale_factor()).unwrap_or(1.0);
        let anchor = rect.position.to_physical::<f64>(scale);
        let size = rect.size.to_physical::<f64>(scale);
        let mut x = anchor.x + size.width / 2.0 - 220.0 * scale;
        let mut y = anchor.y + size.height + 6.0 * scale;
        if let Some(m) = monitor {
            let work = m.work_area();
            let left = work.position.x as f64 + 8.0 * scale;
            let top = work.position.y as f64 + 4.0 * scale;
            x = x
                .max(left)
                .min((work.position.x as f64 + work.size.width as f64 - 448.0 * scale).max(left));
            let height = (860.0 * scale)
                .min(work.size.height as f64 - 8.0 * scale)
                .max(200.0 * scale);
            let _ = panel.set_size(tauri::PhysicalSize::new(
                (440.0 * scale) as u32,
                height as u32,
            ));
            y = y.max(top).min(
                (work.position.y as f64 + work.size.height as f64 - height - 4.0 * scale).max(top),
            );
        }
        let _ = panel.set_position(tauri::PhysicalPosition::new(x, y));
        let _ = panel.show();
        let _ = panel.set_focus();
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
            }
            let store =
                Store::open(&directory.join("maxxit.sqlite")).map_err(std::io::Error::other)?;
            app.manage(Runtime {
                store: Mutex::new(store),
                directory,
                pairing: Mutex::new(None),
                panel_hidden_at: AtomicI64::new(0),
            });
            #[cfg(target_os = "macos")]
            if let Some(panel) = app.get_webview_window("tray") {
                use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior as Behavior};
                // Setup runs on AppKit's main thread. Tauri owns this NSWindow;
                // the borrowed pointer is used only while the panel is alive.
                let native = unsafe { &*panel.ns_window()?.cast::<NSWindow>() };
                let behavior = native.collectionBehavior()
                    & !(Behavior::MoveToActiveSpace
                        | Behavior::FullScreenPrimary
                        | Behavior::FullScreenNone
                        | Behavior::Primary
                        | Behavior::Auxiliary);
                native.setCollectionBehavior(
                    behavior
                        | Behavior::CanJoinAllSpaces
                        | Behavior::FullScreenAuxiliary
                        | Behavior::CanJoinAllApplications
                        | Behavior::IgnoresCycle,
                );
            }
            let application_menu = tauri::menu::Menu::default(app.handle())?;
            let panel_item = tauri::menu::MenuItem::with_id(
                app,
                "show-usage",
                "Show usage panel",
                true,
                Some("CmdOrCtrl+Shift+U"),
            )?;
            for item in application_menu.items()? {
                if let tauri::menu::MenuItemKind::Submenu(submenu) = item {
                    if submenu.text()? == "View" {
                        submenu.append(&panel_item)?;
                    }
                }
            }
            app.set_menu(application_menu)?;
            let open =
                tauri::menu::MenuItem::with_id(app, "open", "Open Maxxit", true, None::<&str>)?;
            let refresh = tauri::menu::MenuItem::with_id(
                app,
                "refresh",
                "Refresh usage",
                true,
                None::<&str>,
            )?;
            let quit =
                tauri::menu::MenuItem::with_id(app, "quit", "Quit Maxxit", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&open, &refresh, &quit])?;
            let mut tray = tauri::tray::TrayIconBuilder::with_id("maxxit")
                .tooltip("Maxxit · connect a provider")
                .title("M")
                .menu(&menu)
                .show_menu_on_left_click(false);
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.on_menu_event(|app, event| match event.id.as_ref() {
                "open" => {
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
                "refresh" => {
                    let _ = app.emit("usage-updated", ());
                }
                "quit" => app.exit(0),
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if let tauri::tray::TrayIconEvent::Click {
                    button: tauri::tray::MouseButton::Left,
                    button_state: tauri::tray::MouseButtonState::Up,
                    position,
                    rect,
                    ..
                } = event
                {
                    let app = tray.app_handle();
                    show_tray_panel(app, position, rect, true);
                }
            })
            .build(app)?;
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let runtime = handle.state::<Runtime>();
                    if let Ok(data) = collect(&runtime) {
                        let preferred =
                            data["settings"]["trayProvider"].as_str().unwrap_or("codex");
                        let provider = data["providers"]
                            .as_array()
                            .and_then(|p| p.iter().find(|p| p["provider"] == preferred));
                        let window = provider.and_then(|p| p.pointer("/observation/windows/0"));
                        let fresh = provider
                            .and_then(|p| p.pointer("/observation/observedAt"))
                            .and_then(Value::as_str)
                            .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
                            .is_some_and(|t| chrono::Utc::now().timestamp() - t.timestamp() < 7200);
                        let used = window
                            .filter(|w| {
                                fresh
                                    && w["availability"] == "available"
                                    && w["resetsAt"]
                                        .as_str()
                                        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
                                        .is_some_and(|t| {
                                            t.timestamp() > chrono::Utc::now().timestamp()
                                        })
                            })
                            .and_then(|w| w["usedPercent"].as_f64());
                        let title = used
                            .map(|v| {
                                format!(
                                    "{} {:.0}%",
                                    if preferred == "codex" { "C" } else { "A" },
                                    100.0 - v
                                )
                            })
                            .unwrap_or_else(|| "M —".into());
                        if let Some(tray) = handle.tray_by_id("maxxit") {
                            let _ = tray.set_title(Some(&title));
                            let _ = tray.set_tooltip(Some(format!(
                                "Maxxit · {preferred} · remaining allowance"
                            )));
                        }
                        let _ = handle.emit("usage-updated", ());
                    }
                    let settings = runtime_settings(&runtime).unwrap_or_default();
                    if settings.cloud_sync || settings.ai_consent {
                        let _ = sync_cloud(&runtime).await;
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                }
            });
            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "show-usage" {
                if let Some(tray) = app.tray_by_id("maxxit") {
                    if let Ok(Some(rect)) = tray.rect() {
                        let scale = app
                            .get_webview_window("main")
                            .and_then(|w| w.scale_factor().ok())
                            .unwrap_or(1.0);
                        let position = rect.position.to_physical::<f64>(scale);
                        show_tray_panel(app, position, rect, false);
                    }
                }
            }
        })
        .on_window_event(|window, event| {
            if window.label() == "tray" {
                if matches!(event, tauri::WindowEvent::Focused(false)) {
                    window
                        .state::<Runtime>()
                        .panel_hidden_at
                        .store(chrono::Utc::now().timestamp_millis(), Ordering::Relaxed);
                    let _ = window.hide();
                }
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let runtime = window.state::<Runtime>();
                if runtime
                    .store
                    .lock()
                    .map(|s| s.settings().background)
                    .unwrap_or(true)
                {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            export_token_csv,
            tray_action,
            tray_resize,
            save_settings,
            claude_preview,
            connect_provider,
            disconnect_provider,
            save_project,
            remove_project,
            cloud_start,
            cloud_redeem,
            cloud_disconnect,
            cloud_sync,
            cloud_action,
            cloud_preferences,
            open_link
        ])
        .run(tauri::generate_context!())
        .expect("Maxxit could not start");
}
