//! Only allowlisted diagnostics cross the local storage and network boundary.
use crate::diagnostics_queue::{Health, Queue};
use sentry::{
    protocol::{EnvelopeItem, ItemContainer, Log, LogAttribute, LogLevel},
    Envelope,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::{
    cell::RefCell,
    collections::HashMap,
    future::Future,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::{Duration, Instant, SystemTime},
};
use uuid::Uuid;

static DIAGNOSTICS: OnceLock<Arc<Diagnostics>> = OnceLock::new();
thread_local! { static SYNC_OPERATION: RefCell<Option<Operation>> = const { RefCell::new(None) }; }
tokio::task_local! { static ASYNC_OPERATION: Operation; }
const EVENT_NAMES: &[&str] = &[
    "app.start",
    "app.ready",
    "app.shutdown",
    "app.previous_exit_unclean",
    "app.start.failed",
    "app.panic",
    "app.crash",
    "ui.command.started",
    "ui.command.completed",
    "ui.command.recovered",
    "ui.command.failed",
    "ui.render.failed",
    "ui.unhandled_exception",
    "ui.unhandled_rejection",
    "ui.listener.failed",
    "tray.operation.failed",
    "autostart.read.failed",
    "autostart.change.failed",
    "provider.connect.started",
    "provider.connect.completed",
    "provider.disconnect.completed",
    "provider.operation.failed",
    "provider.collection.completed",
    "provider.collection.failed",
    "provider.parse.failed",
    "provider.schema.unsupported",
    "provider.source.read_failed",
    "provider.normalization.rejected",
    "provider.observation.stale",
    "provider.observation.recovered",
    "bridge.install.started",
    "bridge.install.completed",
    "bridge.install.failed",
    "bridge.restore.failed",
    "bridge.capture.failed",
    "bridge.previous_command.failed",
    "bridge.previous_command.timeout",
    "storage.open.completed",
    "storage.open.failed",
    "storage.migration.started",
    "storage.migration.completed",
    "storage.migration.failed",
    "storage.operation.failed",
    "storage.record.corrupt",
    "keychain.read.failed",
    "keychain.write.failed",
    "keychain.verify.failed",
    "keychain.restore.failed",
    "keychain.delete.failed",
    "pairing.started",
    "pairing.completed",
    "pairing.expired",
    "pairing.failed",
    "account.disconnect.failed",
    "cloud.request.completed",
    "cloud.request.retry",
    "cloud.request.failed",
    "cloud.sync.started",
    "cloud.sync.completed",
    "cloud.sync.failed",
    "cloud.sync.recovered",
    "cloud.response.invalid",
    "cloud.response.too_large",
    "cloud.auth.revoked",
    "background.cycle.failed",
    "background.task.panicked",
    "background.recovered",
    "settings.save.failed",
    "project.save.failed",
    "project.delete.failed",
    "export.write.failed",
    "update.check.started",
    "update.available",
    "update.download.started",
    "update.download.completed",
    "update.install.completed",
    "update.relaunch.started",
    "update.check.failed",
    "update.download.failed",
    "update.verify.failed",
    "update.install.failed",
    "update.cleanup.failed",
    "update.relaunch.failed",
    "diagnostics.test",
    "diagnostics.consent.changed",
];
const COMMANDS: &[&str] = &[
    "snapshot",
    "export_token_csv",
    "tray_action",
    "tray_resize",
    "save_settings",
    "claude_preview",
    "connect_provider",
    "disconnect_provider",
    "save_project",
    "remove_project",
    "cloud_start",
    "cloud_redeem",
    "cloud_disconnect",
    "cloud_sync",
    "cloud_action",
    "cloud_preferences",
    "open_link",
    "background_cycle",
    "app_start",
    "bridge_capture",
    "updates",
    "frontend",
    "diagnostics_test",
];
fn now() -> i64 {
    chrono::Utc::now().timestamp()
}
fn id() -> String {
    Uuid::new_v4().simple().to_string()
}
fn valid_id(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn event_name(name: &str) -> &str {
    if EVENT_NAMES.contains(&name) {
        name
    } else {
        "ui.command.failed"
    }
}
fn origin(value: &str) -> &str {
    if ["main", "tray", "background", "bridge", "crash-reporter"].contains(&value) {
        value
    } else {
        "background"
    }
}
fn is_failure(name: &str) -> bool {
    name.ends_with("failed")
        || [
            "app.crash",
            "app.panic",
            "ui.unhandled_exception",
            "ui.unhandled_rejection",
            "storage.record.corrupt",
            "provider.normalization.rejected",
            "provider.schema.unsupported",
            "bridge.previous_command.timeout",
            "cloud.response.invalid",
            "cloud.response.too_large",
            "cloud.auth.revoked",
        ]
        .contains(&name)
}
fn code(value: &str) -> String {
    event_name(value).to_ascii_uppercase().replace('.', "_")
}
pub fn attributes(input: &Value) -> Map<String, Value> {
    let mut out = Map::new();
    let Some(object) = input.as_object() else {
        return out;
    };
    for (key, value) in object {
        let numeric = [
            "elapsed_ms",
            "retry_delay_ms",
            "source_line",
            "status",
            "attempt",
            "bytes",
            "rejected_count",
            "examined_count",
            "os_code",
            "sqlite_code",
            "schema_version",
            "rows",
            "timeout_ms",
            "consecutive_failures",
        ];
        if numeric.contains(&key.as_str())
            && value
                .as_i64()
                .is_some_and(|v| (0..=1_000_000_000_000).contains(&v))
        {
            out.insert(key.clone(), value.clone());
            continue;
        }
        let allowed = match key.as_str() {
            "source_file" => value.as_str().is_some_and(|v| {
                [
                    "cloud.rs",
                    "storage.rs",
                    "bridge.rs",
                    "providers.rs",
                    "model.rs",
                    "lib.rs",
                    "main.rs",
                    "observability.rs",
                ]
                .contains(&v)
            }),
            "provider" => value
                .as_str()
                .is_some_and(|v| ["codex", "claude"].contains(&v)),
            "method" => value.as_str().is_some_and(|v| ["GET", "POST"].contains(&v)),
            "route" => value.as_str().is_some_and(|v| {
                [
                    "pairing/start",
                    "pairing/redeem",
                    "collector/disconnect",
                    "desktop/state",
                    "desktop/usage",
                    "desktop/project",
                    "desktop/preferences",
                    "desktop/generate",
                    "desktop/checkout",
                    "desktop/portal",
                ]
                .contains(&v)
            }),
            "stage" => value.as_str().is_some_and(|v| {
                [
                    "start",
                    "read",
                    "parse",
                    "normalize",
                    "write",
                    "verify",
                    "restore",
                    "delete",
                    "open",
                    "migrate",
                    "prune",
                    "check",
                    "download",
                    "install",
                    "cleanup",
                    "relaunch",
                    "http",
                    "response",
                    "keychain",
                    "collection",
                    "settings",
                    "tray",
                    "listener",
                ]
                .contains(&v)
            }),
            "operation_id" | "error_id" | "event_id" => value.as_str().is_some_and(valid_id),
            "command" => value.as_str().is_some_and(|v| COMMANDS.contains(&v)),
            "target_version" => value.as_str().is_some_and(|v| {
                v.len() < 32 && v.bytes().all(|b| b.is_ascii_digit() || b == b'.')
            }),
            "recoverable" | "retained" | "restored" | "timeout" | "connection_error" => {
                value.is_boolean()
            }
            _ => false,
        };
        if allowed {
            out.insert(key.clone(), value.clone());
        }
    }
    out
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Record {
    pub name: String,
    pub timestamp: i64,
    pub operation_id: String,
    pub origin: String,
    pub attributes: Map<String, Value>,
}
#[derive(Clone)]
pub struct Operation {
    id: String,
    name: String,
    origin: String,
    epoch: i64,
    start: Instant,
    history: Arc<Mutex<Vec<Record>>>,
    omitted: Arc<AtomicUsize>,
    captured: Arc<Mutex<Option<String>>>,
}
impl Operation {
    pub fn new(name: &str, window: &str, supplied: Option<&str>) -> Self {
        Self {
            id: supplied
                .filter(|v| valid_id(v))
                .map(str::to_owned)
                .unwrap_or_else(id),
            name: if COMMANDS.contains(&name) {
                name.into()
            } else {
                "frontend".into()
            },
            origin: origin(window).into(),
            epoch: diagnostics().map(|d| d.queue.consent().2).unwrap_or(-1),
            start: Instant::now(),
            history: Arc::new(Mutex::new(vec![])),
            omitted: Arc::new(AtomicUsize::new(0)),
            captured: Arc::new(Mutex::new(None)),
        }
    }
    pub fn record(&self, name: &str, attrs: Value) {
        let Some(d) = diagnostics() else {
            return;
        };
        if !d.queue.consent().0 || d.queue.consent().2 != self.epoch {
            return;
        }
        let record = Record {
            name: event_name(name).into(),
            timestamp: now(),
            operation_id: self.id.clone(),
            origin: self.origin.clone(),
            attributes: attributes(&attrs),
        };
        if let Ok(mut history) = self.history.lock() {
            history.push(record.clone());
            if history.len() > 50 {
                history.remove(0);
                self.omitted.fetch_add(1, Ordering::Relaxed);
            }
        }
        d.remember(&record, &self.name, self.epoch);
        if !name.starts_with("ui.command.")
            && name != "provider.collection.completed"
            && !is_failure(name)
        {
            d.log(name, self, attrs);
        }
    }
    pub fn fail(&self, name: &str, attrs: Value) -> Option<String> {
        self.record(name, attrs.clone());
        let mut captured = self.captured.lock().ok()?;
        if let Some(report) = captured.as_ref() {
            if let Some(d) = diagnostics() {
                let mut attrs = attributes(&attrs);
                attrs.insert("event_id".into(), json!(report));
                attrs.insert("error_id".into(), json!(report));
                d.log(name, self, attrs.into());
            }
            return captured.clone();
        }
        let d = diagnostics()?;
        if !d.queue.consent().0 || d.queue.consent().2 != self.epoch {
            return None;
        }
        let signature = format!("{}:{}", self.origin, self.name);
        let stack = sentry::integrations::backtrace::current_stacktrace()
            .and_then(|s| serde_json::to_value(s).ok());
        let event = json!({"event_id":id(),"platform":"native","level":"error","exception":{"values":[{"type":"DesktopError","stacktrace":stack}]},"debug_meta":{"images":sentry::integrations::debug_images::debug_images()}});
        let result = d.capture(event, name, self);
        if let Some(report) = &result {
            if let Ok(mut incidents) = d.incidents.lock() {
                incidents.insert(signature, report.clone());
            }
        }
        *captured = result.clone();
        result
    }
}
fn current() -> Operation {
    ASYNC_OPERATION
        .try_with(Clone::clone)
        .ok()
        .or_else(|| SYNC_OPERATION.with(|o| o.borrow().clone()))
        .unwrap_or_else(|| Operation::new("frontend", "background", None))
}
pub fn record(name: &str, attrs: Value) {
    current().record(name, attrs);
}
#[track_caller]
pub fn report<T, E: std::fmt::Display>(result: Result<T, E>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            map_error("tray.operation.failed", "tray", error);
            None
        }
    }
}
#[track_caller]
pub fn failure(name: &str, mut attrs: Value) {
    let source = std::panic::Location::caller();
    if let Some(attrs) = attrs.as_object_mut() {
        attrs.insert(
            "source_file".into(),
            json!(source.file().rsplit('/').next().unwrap_or("unknown")),
        );
        attrs.insert("source_line".into(), json!(source.line()));
    }
    current().fail(name, attrs);
}
#[track_caller]
pub fn map_error<E: std::fmt::Display>(
    name: &'static str,
    stage: &'static str,
    error: E,
) -> String {
    failure(name, json!({"stage":stage}));
    error.to_string()
}
#[track_caller]
pub fn storage_error(error: rusqlite::Error) -> String {
    let sqlite_code = match &error {
        rusqlite::Error::SqliteFailure(e, _) => Some(e.extended_code),
        _ => None,
    };
    failure(
        "storage.operation.failed",
        json!({"sqlite_code":sqlite_code}),
    );
    error.to_string()
}
#[track_caller]
pub fn io_error(name: &'static str, stage: &'static str, error: std::io::Error) -> String {
    failure(name, json!({"stage":stage,"os_code":error.raw_os_error()}));
    error.to_string()
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub message: String,
    pub report_id: Option<String>,
    pub expected: bool,
}
fn expected(message: &str) -> bool {
    [
        "Invalid preferences",
        "Invalid token export",
        "Unknown provider",
        "Unknown cloud action",
        "Unknown panel action",
        "Invalid panel size",
        "Invalid link",
        "This link is not an allowed Maxxit destination",
        "Project name or description exceeds its limit",
        "Connect your Maxxit account first",
        "Start account connection first",
        "Enable project sharing before requesting ideas",
    ]
    .contains(&message)
}
pub fn observe<T>(
    operation: Operation,
    work: impl FnOnce() -> Result<T, String>,
) -> Result<T, CommandError> {
    struct Restore(Option<Operation>);
    impl Drop for Restore {
        fn drop(&mut self) {
            SYNC_OPERATION.with(|o| {
                o.replace(self.0.take());
            });
        }
    }
    let _restore = Restore(SYNC_OPERATION.with(|o| o.replace(Some(operation.clone()))));
    operation.record("ui.command.started", json!({"command":operation.name}));
    let result = work();
    finish(&operation, result)
}
pub async fn observe_async<T>(
    operation: Operation,
    work: impl Future<Output = Result<T, String>>,
) -> Result<T, CommandError> {
    ASYNC_OPERATION
        .scope(operation.clone(), async move {
            operation.record("ui.command.started", json!({"command":operation.name}));
            finish(&operation, work.await)
        })
        .await
}
fn finish<T>(operation: &Operation, result: Result<T, String>) -> Result<T, CommandError> {
    match result {
        Ok(value) => {
            if operation.captured.lock().is_ok_and(|v| v.is_none()) {
                if let Some(d) = diagnostics() {
                    if let Ok(mut incidents) = d.incidents.lock() {
                        if let Some(report) =
                            incidents.remove(&format!("{}:{}", operation.origin, operation.name))
                        {
                            operation.record(
                                if operation.origin == "background" {
                                    "background.recovered"
                                } else {
                                    "ui.command.recovered"
                                },
                                json!({"error_id":report,"event_id":report}),
                            );
                        }
                    }
                }
            }
            operation.record("ui.command.completed",json!({"elapsed_ms":operation.start.elapsed().as_millis() as u64,"command":operation.name}));
            Ok(value)
        }
        Err(message) => {
            let expected = expected(&message);
            let report_id = if expected {
                None
            } else {
                operation.fail("ui.command.failed",json!({"elapsed_ms":operation.start.elapsed().as_millis() as u64,"command":operation.name}))
            };
            Err(CommandError {
                message,
                report_id,
                expected,
            })
        }
    }
}

pub struct Diagnostics {
    pub queue: Arc<Queue>,
    gate: tokio::sync::Mutex<()>,
    dsn: Option<sentry::types::Dsn>,
    pub boot_id: String,
    release: String,
    environment: String,
    os_version: String,
    started: Instant,
    directory: PathBuf,
    incidents: Mutex<HashMap<String, String>>,
}
pub fn diagnostics() -> Option<&'static Arc<Diagnostics>> {
    DIAGNOSTICS.get()
}
pub fn app_start() {
    if let Some(d) = diagnostics() {
        if d.queue.begin_session().unwrap_or(false) {
            record("app.previous_exit_unclean", json!({}));
        }
        record("app.start", json!({}));
    }
}
pub fn flush_on_exit() {
    if let Some(d) = diagnostics().cloned() {
        let _ = std::thread::spawn(move || {
            if let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                runtime.block_on(d.deliver(1));
            }
        })
        .join();
    }
}
pub fn app_shutdown() {
    record("app.shutdown", json!({}));
    if let Some(d) = diagnostics() {
        let _ = d.queue.end_session();
    }
}
pub fn data_directory() -> Option<PathBuf> {
    #[cfg(debug_assertions)]
    {
        let args: Vec<String> = std::env::args().collect();
        if let Some(index) = args.iter().position(|v| v == "--diagnostics-probe-dir") {
            return args.get(index + 1).map(PathBuf::from);
        }
    }
    dirs::data_dir().map(|p| p.join("app.maxxit.desktop"))
}
pub fn initialize(directory: PathBuf, crash_reporter: bool) {
    let Ok(queue) = Queue::open(&directory.join("diagnostics")) else {
        return;
    };
    let dsn = option_env!("MAXXIT_SENTRY_DSN")
        .unwrap_or("")
        .parse::<sentry::types::Dsn>()
        .ok()
        .filter(|d| d.scheme().to_string() == "https" && d.secret_key().is_none());
    let os_version = std::process::Command::new("/usr/bin/sw_vers")
        .arg("-productVersion")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| s.len() < 32 && s.bytes().all(|b| b.is_ascii_digit() || b == b'.'))
        .unwrap_or_else(|| "unavailable".into());
    let d = Arc::new(Diagnostics {
        queue: Arc::new(queue),
        gate: tokio::sync::Mutex::new(()),
        dsn,
        boot_id: id(),
        release: format!(
            "maxxit-desktop@{}+{}",
            env!("CARGO_PKG_VERSION"),
            env!("MAXXIT_SOURCE_COMMIT")
        ),
        environment: option_env!("MAXXIT_SENTRY_ENVIRONMENT")
            .filter(|v| ["production", "staging", "development"].contains(v))
            .unwrap_or(if cfg!(debug_assertions) {
                "development"
            } else {
                "production"
            })
            .into(),
        os_version,
        started: Instant::now(),
        directory,
        incidents: Mutex::new(HashMap::new()),
    });
    if DIAGNOSTICS.set(d.clone()).is_err() {
        return;
    }
    configure_sdk(d, crash_reporter);
}
fn configure_sdk(d: Arc<Diagnostics>, crash_reporter: bool) {
    let native = d.queue.consent().1;
    // The placeholder activates SDK processing only. The custom transport never sends to this DSN.
    let sdk_dsn = d
        .dsn
        .clone()
        .or_else(|| "https://disabled@localhost/1".parse().ok());
    let transport = Arc::new(DurableTransport(d.clone()));
    let filter = d.clone();
    let mut options = sentry::ClientOptions::new()
        .maybe_release(Some(d.release.clone()))
        .environment(d.environment.clone())
        .send_default_pii(false)
        .sample_rate(1.0)
        .traces_sample_rate(0.0)
        .transport(transport)
        .before_send(move |event| {
            if !filter.queue.consent().0 {
                return None;
            }
            let raw = serde_json::to_value(&event).ok()?;
            if raw
                .pointer("/tags/diagnostics_generation")
                .and_then(Value::as_str)
                .and_then(|g| g.parse::<i64>().ok())
                .is_some_and(|g| g != filter.queue.consent().2)
            {
                return None;
            }
            let panic = raw
                .pointer("/exception/values/0/type")
                .and_then(Value::as_str)
                == Some("panic");
            let operation = if event.level == sentry::Level::Fatal && !panic {
                Operation::new(
                    raw.pointer("/tags/command")
                        .and_then(Value::as_str)
                        .unwrap_or("frontend"),
                    raw.pointer("/tags/origin")
                        .and_then(Value::as_str)
                        .unwrap_or("crash-reporter"),
                    raw.pointer("/tags/operation_id").and_then(Value::as_str),
                )
            } else {
                current()
            };
            let name = if event.level == sentry::Level::Fatal && !panic {
                "app.crash"
            } else {
                "app.panic"
            };
            serde_json::from_value(filter.safe_event(&raw, name, &operation)).ok()
        });
    options.dsn = sdk_dsn;
    if native || crash_reporter {
        let q = d.queue.clone();
        let integration = sentry::integrations::minidump::MinidumpIntegration::new()
            .crashes_dir(d.directory.join("diagnostics/crashes"))
            .on_process({
                let directory = d.directory.clone();
                move |command| {
                    if cfg!(debug_assertions) {
                        command.arg("--diagnostics-probe-dir").arg(&directory);
                    }
                }
            })
            .inherit_args(false)
            .process_name("Maxxit crash reporter")
            .before_capture(move |scope, path| {
                if !q.consent().0 || !q.consent().1 {
                    scope.clear_attachments();
                }
                // The SDK already owns the attachment buffer. Only the bounded durable envelope may remain.
                let _ = std::fs::remove_file(path);
            });
        options = options.add_integration(integration);
    }
    let client = sentry::Client::with_options(sentry::apply_defaults(options));
    sentry::Hub::main().bind_client(Some(Arc::new(client)));
}
impl Diagnostics {
    fn remember(&self, record: &Record, command: &str, epoch: i64) {
        if !self.queue.consent().0 || self.queue.consent().2 != epoch {
            return;
        }
        if let Ok(body) = serde_json::to_string(record) {
            let _ = self.queue.remember(&body, epoch);
        }
        let breadcrumb = sentry::protocol::Breadcrumb {
            category: Some(record.name.clone()),
            message: Some(record.name.clone()),
            data: record
                .attributes
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            ..Default::default()
        };
        sentry::with_integration(
            |i: &sentry::integrations::minidump::MinidumpIntegration, _| {
                i.add_breadcrumb(breadcrumb);
                i.set_tag("operation_id".into(), Some(record.operation_id.clone()));
                i.set_tag("command".into(), Some(command.to_owned()));
                i.set_tag("origin".into(), Some(record.origin.clone()));
                i.set_tag("boot_id".into(), Some(self.boot_id.clone()));
                i.set_tag("diagnostics_generation".into(), Some(epoch.to_string()));
            },
        );
    }
    pub fn health(&self) -> Health {
        self.queue.health(self.dsn.is_some(), now())
    }
    pub async fn consent(&self, enabled: bool, native: bool) -> Result<Health, String> {
        let _gate = self.gate.lock().await;
        self.queue
            .set_consent(enabled, native)
            .map_err(|_| "Diagnostic preferences could not be saved".to_owned())?;
        if !native {
            configure_sdk(
                diagnostics().ok_or("Diagnostics unavailable")?.clone(),
                false,
            );
        }
        Ok(self.health())
    }
    fn safe_event(&self, raw: &Value, name: &str, operation: &Operation) -> Value {
        let event_id = raw["event_id"]
            .as_str()
            .filter(|v| valid_id(v))
            .map(str::to_owned)
            .unwrap_or_else(id);
        let mut history = operation
            .history
            .lock()
            .map(|v| v.clone())
            .unwrap_or_default();
        let prior = self
            .queue
            .history()
            .iter()
            .filter_map(|r| serde_json::from_value::<Record>(r.clone()).ok())
            .filter(|r| r.operation_id == operation.id || r.name.starts_with("app."))
            .collect::<Vec<_>>();
        for record in prior {
            if !history.iter().any(|r| {
                r.timestamp == record.timestamp
                    && r.name == record.name
                    && r.operation_id == record.operation_id
            }) {
                history.push(record);
            }
        }
        history.sort_by_key(|r| r.timestamp);
        let history_count = history.len() + operation.omitted.load(Ordering::Relaxed);
        if history.len() > 50 {
            history.drain(..history.len() - 50);
        }
        while history.len() > 1
            && serde_json::to_vec(&history).is_ok_and(|body| body.len() > 16 * 1024)
        {
            history.remove(0);
        }
        let omitted_count = history_count - history.len();
        // Source context, user, request, original messages, locals and arbitrary extras are never copied.
        let frames = raw
            .pointer("/exception/values/0/stacktrace/frames")
            .or_else(|| raw.pointer("/stacktrace/frames"))
            .and_then(Value::as_array)
            .map(|frames| frames.iter().take(80).map(safe_frame).collect::<Vec<_>>())
            .unwrap_or_default();
        let images = raw
            .pointer("/debug_meta/images")
            .and_then(Value::as_array)
            .map(|images| images.iter().take(100).map(safe_image).collect::<Vec<_>>())
            .unwrap_or_default();
        let mut exceptions = raw
            .pointer("/exception/values")
            .and_then(Value::as_array)
            .map(|values| values.iter().rev().take(8).collect::<Vec<_>>())
            .unwrap_or_default();
        exceptions.reverse();
        let exceptions: Vec<Value> = if exceptions.is_empty() {
            vec![
                json!({"type":"DesktopError","value":code(name),"stacktrace":{"frames":frames},"mechanism":{"type":"maxxit","handled":!name.starts_with("app.")}}),
            ]
        } else {
            exceptions.iter().map(|exception| {
                let ty=match exception["type"].as_str().unwrap_or("") {"TypeError"=>"TypeError","SyntaxError"=>"SyntaxError","RangeError"=>"RangeError","ReferenceError"=>"ReferenceError","Panic"|"panic"=>"Panic",_=>"DesktopError"};
                let frames=exception.pointer("/stacktrace/frames").and_then(Value::as_array).map(|frames|frames.iter().rev().take(80).map(safe_frame).collect::<Vec<_>>()).unwrap_or_default();
                let frames=frames.into_iter().rev().collect::<Vec<_>>();
                json!({"type":ty,"value":code(name),"stacktrace":{"frames":frames},"mechanism":{"type":"maxxit","handled":!name.starts_with("app.")}})
            }).collect()
        };
        json!({"event_id":event_id,"timestamp":raw["timestamp"].as_f64().unwrap_or(now() as f64),"platform":if raw["platform"]=="javascript"{"javascript"}else{"native"},"level":if name=="app.crash" || name=="app.panic" || name=="app.start.failed"{"fatal"}else{"error"},"release":self.release,"environment":self.environment,"sdk":{"name":"maxxit.desktop","version":"1","packages":[{"name":"sentry-rust","version":"0.49.3"},{"name":"@sentry/react","version":"10.75.0"}]},"message":code(name),"fingerprint":["{{ default }}",event_name(name)],"exception":{"values":exceptions},"debug_meta":{"images":images},"tags":{"event_name":event_name(name),"diagnostics_generation":operation.epoch.to_string(),"error_code":code(name),"operation_id":operation.id,"command":operation.name,"error_id":event_id,"origin":operation.origin,"source_commit":env!("MAXXIT_SOURCE_COMMIT")},"contexts":{"operation":{"name":operation.name,"operation_id":operation.id,"elapsed_ms":operation.start.elapsed().as_millis() as u64,"stack_origin":"capture_boundary"},"os":{"name":"macOS","version":self.os_version},"device":{"arch":std::env::consts::ARCH}},"extra":{"schema_version":1,"boot_id":raw.pointer("/tags/boot_id").and_then(Value::as_str).filter(|v|valid_id(v)).unwrap_or(&self.boot_id),"app_uptime_ms":self.started.elapsed().as_millis() as u64,"omitted_event_count":omitted_count,"diagnostic_history":history},"breadcrumbs":{"values":history.iter().map(|r|json!({"timestamp":r.timestamp,"category":r.name,"message":r.name,"level":"info","data":r.attributes})).collect::<Vec<_>>()}})
    }
    pub fn capture(&self, raw: Value, name: &str, operation: &Operation) -> Option<String> {
        let safe = self.safe_event(&raw, name, operation);
        let event_id = safe["event_id"].as_str()?.to_owned();
        // Rust's typed Event does not support JavaScript sourcemap images. Preserve the
        // sanitized cross-platform JSON in a standard envelope instead of dropping Debug IDs.
        let payload = serde_json::to_vec(&safe).ok()?;
        let mut body = format!(
            "{{\"event_id\":\"{event_id}\"}}\n{{\"type\":\"event\",\"length\":{}}}\n",
            payload.len()
        )
        .into_bytes();
        body.extend(payload);
        body.push(b'\n');
        if !self
            .queue
            .store(
                &event_id,
                &body,
                "error",
                ["app.crash", "app.panic", "app.start.failed"].contains(&name),
                operation.epoch,
                now(),
            )
            .ok()?
        {
            return None;
        }
        self.remember(
            &Record {
                name: event_name(name).into(),
                timestamp: now(),
                operation_id: operation.id.clone(),
                origin: operation.origin.clone(),
                attributes: attributes(&json!({"event_id":event_id,"error_id":event_id})),
            },
            &operation.name,
            operation.epoch,
        );
        self.log(
            name,
            operation,
            json!({"event_id":event_id,"error_id":event_id}),
        );
        Some(event_id)
    }
    pub fn log(&self, name: &str, operation: &Operation, attrs: Value) {
        if !EVENT_NAMES.contains(&name) {
            return;
        }
        let mut attributes = attributes(&attrs);
        attributes.insert("operation_id".into(), json!(operation.id));
        attributes.insert("release".into(), json!(self.release));
        attributes.insert("origin".into(), json!(operation.origin));
        attributes.insert("boot_id".into(), json!(self.boot_id));
        let log = Log {
            level: if is_failure(name) {
                LogLevel::Error
            } else {
                LogLevel::Info
            },
            body: name.into(),
            trace_id: None,
            timestamp: SystemTime::now(),
            severity_number: None,
            attributes: attributes
                .into_iter()
                .map(|(k, v)| (k, LogAttribute(v)))
                .collect(),
        };
        let mut envelope = Envelope::new();
        envelope.add_item(EnvelopeItem::ItemContainer(ItemContainer::Logs(vec![log])));
        let mut body = vec![];
        if envelope.to_writer(&mut body).is_ok() {
            let _ = self
                .queue
                .store(&id(), &body, "log", false, operation.epoch, now());
        }
    }
    pub async fn deliver(&self, budget: usize) {
        let Some(dsn) = &self.dsn else {
            return;
        };
        let Ok(_gate) = self.gate.try_lock() else {
            return;
        };
        let Ok(client) = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build()
        else {
            return;
        };
        for _ in 0..budget {
            if !self.queue.consent().0 {
                return;
            }
            let Ok(Some(item)) = self.queue.next(now()) else {
                return;
            };
            let auth = format!(
                "Sentry sentry_version=7,sentry_client=maxxit-desktop/1,sentry_key={}",
                dsn.public_key()
            );
            let response = client
                .post(dsn.envelope_api_url())
                .header("X-Sentry-Auth", auth)
                .header("Content-Type", "application/x-sentry-envelope")
                .body(item.body.clone())
                .send()
                .await;
            match response {
                Ok(response) => {
                    let status = response.status();
                    if let Some(value) = response
                        .headers()
                        .get("x-sentry-rate-limits")
                        .and_then(|v| v.to_str().ok())
                    {
                        apply_limits(&self.queue, value, now());
                    }
                    if status.as_u16() == 429 {
                        let seconds = response
                            .headers()
                            .get("retry-after")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse::<i64>().ok())
                            .unwrap_or(60)
                            .clamp(1, 86400);
                        if response.headers().get("x-sentry-rate-limits").is_none() {
                            let _ = self.queue.rate_limit("all", now() + seconds);
                        }
                        let _ = self.queue.retry(&item, now(), "rate_limited");
                    } else if status.is_success() {
                        let _ = self.queue.accepted(&item, now());
                    } else if status.is_server_error() {
                        let _ = self.queue.retry(&item, now(), "retrying");
                    } else {
                        let _ = self.queue.rejected(&item);
                    }
                }
                Err(_) => {
                    let _ = self.queue.retry(&item, now(), "retrying");
                    return;
                }
            }
        }
    }
    pub fn preview(&self) -> Value {
        json!({"schemaVersion":1,"release":self.release,"environment":self.environment,"health":self.health(),"recentEvents":self.queue.history(),"nativeDumpContentsIncluded":false})
    }
}
fn apply_limits(queue: &Queue, value: &str, time: i64) {
    for limit in value.split(',') {
        let mut fields = limit.trim().split(':');
        let seconds = fields
            .next()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| v.is_finite())
            .map(|v| v.ceil() as i64)
            .unwrap_or(60)
            .clamp(1, 86400);
        let categories = fields.next().unwrap_or("");
        if categories.is_empty() {
            let _ = queue.rate_limit("all", time + seconds);
        }
        for category in categories.split(';') {
            let category = match category {
                "log_item" | "log_byte" => "log",
                other => other,
            };
            if ["error", "log", "attachment"].contains(&category) {
                let _ = queue.rate_limit(category, time + seconds);
            }
        }
    }
}
fn safe_frame(frame: &Value) -> Value {
    let mut result = Map::new();
    for key in [
        "lineno",
        "colno",
        "image_addr",
        "instruction_addr",
        "symbol_addr",
        "in_app",
    ] {
        let value = &frame[key];
        if value.is_number()
            || value.is_boolean()
            || value.as_str().is_some_and(|s| {
                s.starts_with("0x") && s[2..].bytes().all(|b| b.is_ascii_hexdigit())
            })
        {
            result.insert(key.into(), value.clone());
        }
    }
    for key in ["filename", "function", "package"] {
        if let Some(value) = frame[key].as_str() {
            let value = if key == "filename" {
                value.rsplit(['/', '\\']).next().unwrap_or("")
            } else {
                value
            };
            if value.len() <= 256
                && value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-<>(),$ []".contains(&b))
                && !value.contains('@')
            {
                result.insert(
                    key.into(),
                    json!(if key == "filename"
                        && frame["filename"]
                            .as_str()
                            .is_some_and(|s| s.starts_with("app:///assets/"))
                    {
                        format!("app:///assets/{value}")
                    } else {
                        value.to_owned()
                    }),
                );
            }
        }
    }
    result.into()
}
fn safe_image(image: &Value) -> Value {
    let mut out = Map::new();
    for key in [
        "type",
        "debug_id",
        "id",
        "uuid",
        "code_id",
        "image_addr",
        "image_vmaddr",
        "image_size",
        "arch",
    ] {
        let v = &image[key];
        if v.is_number()
            || v.as_str().is_some_and(|s| {
                s.len() < 128
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
            })
        {
            out.insert(key.into(), v.clone());
        }
    }
    for key in ["code_file", "debug_file", "name"] {
        if let Some(v) = image[key].as_str().and_then(|s| s.rsplit('/').next()) {
            out.insert(
                key.into(),
                json!(if image["type"] == "sourcemap" {
                    format!("app:///assets/{v}")
                } else {
                    v.to_owned()
                }),
            );
        }
    }
    out.into()
}
struct DurableTransport(Arc<Diagnostics>);
impl sentry::Transport for DurableTransport {
    fn send_envelope(&self, envelope: Envelope) {
        let (enabled, native, epoch) = self.0.queue.consent();
        if !enabled {
            return;
        }
        if !envelope.items().any(|item| matches!(item, EnvelopeItem::Event(event) if event.tags.get("diagnostics_generation").and_then(|g|g.parse::<i64>().ok())==Some(epoch))) { return; }
        let mut clean = Envelope::new();
        let mut event_id = id();
        let mut attachment = false;
        for item in envelope.items() {
            match item {
                EnvelopeItem::Event(event) => {
                    event_id = event.event_id.simple().to_string();
                    clean.add_item(EnvelopeItem::Event(event.clone()));
                    let operation = Operation::new(
                        event
                            .tags
                            .get("command")
                            .map(String::as_str)
                            .unwrap_or("frontend"),
                        event
                            .tags
                            .get("origin")
                            .map(String::as_str)
                            .unwrap_or("background"),
                        event.tags.get("operation_id").map(String::as_str),
                    );
                    self.0.log(
                        event
                            .tags
                            .get("event_name")
                            .map(String::as_str)
                            .unwrap_or("app.panic"),
                        &operation,
                        json!({"event_id":event_id,"error_id":event_id}),
                    );
                }
                EnvelopeItem::Attachment(value)
                    if native && value.ty == Some(sentry::protocol::AttachmentType::Minidump) =>
                {
                    let mut value = value.clone();
                    value.filename = "minidump.dmp".into();
                    clean.add_item(value);
                    attachment = true;
                }
                _ => {}
            }
        }
        if clean.items().next().is_none() {
            return;
        }
        let mut body = vec![];
        if clean.to_writer(&mut body).is_ok() {
            let _ = self.0.queue.store(
                &event_id,
                &body,
                if attachment { "attachment" } else { "error" },
                true,
                epoch,
                now(),
            );
        }
    }
    fn flush(&self, _timeout: Duration) -> bool {
        self.0.health().pending == 0
    } // Persistence is synchronous; network delivery is a separate bounded worker.
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FrontendInput {
    pub name: String,
    pub operation_id: Option<String>,
    pub generation: i64,
    pub attributes: Option<Value>,
    pub event: Option<Value>,
}
#[tauri::command]
pub fn diagnostics_record(
    window: tauri::WebviewWindow,
    input: FrontendInput,
) -> Result<Option<String>, String> {
    if !["main", "tray"].contains(&window.label()) || !EVENT_NAMES.contains(&input.name.as_str()) {
        return Err("Invalid diagnostic event".into());
    }
    let d = diagnostics().ok_or("Diagnostics unavailable")?;
    if !d.queue.consent().0 || d.queue.consent().2 != input.generation {
        return Ok(None);
    }
    let operation = Operation::new(
        input
            .attributes
            .as_ref()
            .and_then(|attrs| attrs["command"].as_str())
            .unwrap_or("frontend"),
        window.label(),
        input.operation_id.as_deref(),
    );
    operation.record(&input.name, input.attributes.unwrap_or_else(|| json!({})));
    if let Some(event) = input.event {
        if serde_json::to_vec(&event)
            .map_err(|_| "Invalid diagnostic event")?
            .len()
            > 128 * 1024
        {
            return Err("Diagnostic event exceeds its limit".into());
        }
        Ok(d.capture(event, &input.name, &operation))
    } else {
        Ok(None)
    }
}
#[tauri::command]
pub fn diagnostics_health() -> Result<Health, String> {
    Ok(diagnostics().ok_or("Diagnostics unavailable")?.health())
}
#[tauri::command]
pub async fn diagnostics_consent(enabled: bool, native_crashes: bool) -> Result<Health, String> {
    diagnostics()
        .ok_or("Diagnostics unavailable")?
        .consent(enabled, native_crashes)
        .await
}
#[tauri::command]
pub fn diagnostics_preview() -> Result<Value, String> {
    Ok(diagnostics().ok_or("Diagnostics unavailable")?.preview())
}
#[tauri::command]
pub async fn diagnostics_test() -> Result<Value, String> {
    let d = diagnostics().ok_or("Diagnostics unavailable")?;
    if !d.queue.consent().0 {
        return Err("Enable diagnostic reporting first".into());
    }
    let operation = Operation::new("diagnostics_test", "main", None);
    operation.record("diagnostics.test", json!({"stage":"start"}));
    let event_id = d.capture(json!({}), "diagnostics.test", &operation);
    d.deliver(4).await;
    Ok(
        json!({"eventId":event_id,"health":d.health(),"status":"Check the event ID in Sentry to confirm processing."}),
    )
}
#[tauri::command]
pub async fn diagnostics_flush() -> Result<Health, String> {
    let d = diagnostics().ok_or("Diagnostics unavailable")?;
    d.deliver(1).await;
    Ok(d.health())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (PathBuf, Diagnostics, Operation) {
        let dir = std::env::temp_dir().join(id());
        let queue = Arc::new(Queue::open(&dir).unwrap());
        queue.set_consent(true, false).unwrap();
        let d = Diagnostics {
            queue,
            gate: tokio::sync::Mutex::new(()),
            dsn: None,
            boot_id: id(),
            release: "maxxit-desktop@0.1.10+synthetic".into(),
            environment: "development".into(),
            os_version: "14.0".into(),
            started: Instant::now(),
            directory: dir.clone(),
            incidents: Mutex::new(HashMap::new()),
        };
        let mut operation = Operation::new("diagnostics_test", "main", None);
        operation.epoch = d.queue.consent().2;
        (dir, d, operation)
    }
    #[test]
    fn complete_event_scrubbing_preserves_debug_ids_and_correlated_history() {
        let (dir, d, op) = fixture();
        d.queue
            .remember(
                &serde_json::to_string(&Record {
                    name: "ui.command.started".into(),
                    timestamp: now(),
                    operation_id: op.id.clone(),
                    origin: "main".into(),
                    attributes: attributes(
                        &json!({"command":"snapshot","password":"PRIVATE_MARKER"}),
                    ),
                })
                .unwrap(),
                op.epoch,
            )
            .unwrap();
        let raw = json!({"message":"PRIVATE_MARKER","user":{"email":"PRIVATE_MARKER"},"request":{"headers":{"Authorization":"PRIVATE_MARKER"}},"extra":{"project":"PRIVATE_MARKER"},"tags":{"token":"PRIVATE_MARKER"},"exception":{"values":[{"type":"TypeError","value":"PRIVATE_MARKER","stacktrace":{"frames":[{"filename":"app:///assets/index-safe.js","lineno":5,"vars":{"password":"PRIVATE_MARKER"},"context_line":"PRIVATE_MARKER"}]}}]},"debug_meta":{"images":[{"type":"sourcemap","code_file":"/Users/PRIVATE_MARKER/index-safe.js","debug_id":"12345678-1234-1234-1234-123456789abc"}]},"platform":"javascript"});
        let event_id = d.capture(raw, "ui.render.failed", &op).unwrap();
        let body = d.queue.next(now()).unwrap().unwrap().body;
        let text = String::from_utf8(body).unwrap();
        assert!(!text.contains("PRIVATE_MARKER"));
        assert!(text.contains(&event_id));
        assert!(text.contains("12345678-1234-1234-1234-123456789abc"));
        assert!(text.contains("app:///assets/index-safe.js"));
        assert!(text.contains("ui.command.started"));
        assert!(text.contains(&op.id));
        drop(d);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn native_debug_images_survive_typed_sdk_processing() {
        let (dir, d, op) = fixture();
        let raw =
            json!({"debug_meta":{"images":sentry::integrations::debug_images::debug_images()}});
        let safe = d.safe_event(&raw, "app.panic", &op);
        let event: sentry::protocol::Event<'static> = serde_json::from_value(safe).unwrap();
        assert!(!event.debug_meta.images.is_empty());
        drop(d);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn real_http_transport_retries_without_losing_event_or_logging_credentials() {
        use std::io::{Read, Write};
        let (dir, mut d, op) = fixture();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        d.dsn = Some(
            format!("http://public@{}/1", listener.local_addr().unwrap())
                .parse()
                .unwrap(),
        );
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut buffer = [0u8; 16384];
            let size = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..size]);
            assert!(request.starts_with("POST /api/1/envelope/"));
            assert!(!request.contains("PRIVATE_MARKER"));
            stream.write_all(b"HTTP/1.1 429 Too Many Requests\r\nContent-Length: 0\r\nRetry-After: 60\r\nX-Sentry-Rate-Limits: 60:error:organization\r\nConnection: close\r\n\r\n").unwrap();
        });
        let event_id = d
            .capture(json!({"message":"PRIVATE_MARKER"}), "diagnostics.test", &op)
            .unwrap();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(d.deliver(1));
        server.join().unwrap();
        assert_eq!(d.health().delivery_state, "rate_limited");
        assert_eq!(d.health().pending, 2);
        let item = d.queue.next(now() + 61).unwrap().unwrap();
        assert_eq!(item.id, event_id);
        assert_eq!(item.attempts, 1);
        drop(d);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    #[ignore = "Explicit synthetic network test against the configured DSN"]
    fn sentry_ingestion_smoke() {
        let (dir, mut d, op) = fixture();
        d.dsn = Some(env!("MAXXIT_SENTRY_DSN").parse().unwrap());
        let event_id = d.capture(json!({}), "diagnostics.test", &op).unwrap();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(d.deliver(2));
        assert_eq!(d.health().delivery_state, "accepted");
        assert_eq!(d.health().pending, 0);
        assert_eq!(d.health().last_event_id.as_deref(), Some(event_id.as_str()));
        println!("Synthetic event accepted by Sentry ingestion: {event_id}. Verify processing in the dashboard.");
        drop(d);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn concurrent_async_operations_keep_their_own_context() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let a = Operation::new("snapshot", "main", None);
            let b = Operation::new("background_cycle", "background", None);
            let expected_a = a.id.clone();
            let expected_b = b.id.clone();
            let (a, b) = tokio::join!(
                observe_async(a, async {
                    tokio::task::yield_now().await;
                    Ok(current().id)
                }),
                observe_async(b, async {
                    tokio::task::yield_now().await;
                    Ok(current().id)
                })
            );
            assert_eq!(a.unwrap().as_str(), expected_a);
            assert_eq!(b.unwrap().as_str(), expected_b);
            assert_ne!(expected_a, expected_b);
        });
    }
    #[test]
    fn sdk_envelopes_cannot_replay_across_consent_generations() {
        use sentry::Transport;
        let (dir, d, op) = fixture();
        let event: sentry::protocol::Event<'static> =
            serde_json::from_value(d.safe_event(&json!({}), "app.panic", &op)).unwrap();
        d.queue.set_consent(false, false).unwrap();
        d.queue.set_consent(true, true).unwrap();
        let d = Arc::new(d);
        let transport = DurableTransport(d.clone());
        transport.send_envelope(Envelope::from(event));
        assert_eq!(d.health().pending, 0);
        drop(transport);
        drop(d);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn private_attributes_are_dropped_and_ids_are_validated() {
        let safe = attributes(
            &json!({"token":"PRIVATE_MARKER","provider":"codex","route":"desktop/state?secret=PRIVATE_MARKER","project":"PRIVATE_MARKER","stage":"PRIVATE_MARKER","status":503,"event_id":"PRIVATE_MARKER"}),
        );
        assert_eq!(
            safe,
            json!({"provider":"codex","status":503})
                .as_object()
                .unwrap()
                .clone()
        );
    }
    #[test]
    fn frames_exclude_paths_locals_and_source_context() {
        let safe = safe_frame(
            &json!({"filename":"/Users/PRIVATE_MARKER/project/src/App.tsx","function":"render","vars":{"token":"PRIVATE_MARKER"},"context_line":"PRIVATE_MARKER","lineno":42}),
        );
        assert!(!safe.to_string().contains("PRIVATE_MARKER"));
        assert_eq!(safe["filename"], "App.tsx");
    }
    #[test]
    fn category_limits_are_independent() {
        let dir = std::env::temp_dir().join(id());
        let q = Queue::open(&dir).unwrap();
        q.set_consent(true, false).unwrap();
        let epoch = q.consent().2;
        q.store("log", b"safe", "log", false, epoch, 100).unwrap();
        q.store("error", b"safe", "error", true, epoch, 100)
            .unwrap();
        apply_limits(&q, "60:log_item;log_byte:organization", 100);
        assert_eq!(q.next(101).unwrap().unwrap().id, "error");
        drop(q);
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[tauri::command]
pub async fn diagnostics_export(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_dialog::DialogExt;
    let body =
        serde_json::to_vec_pretty(&diagnostics().ok_or("Diagnostics unavailable")?.preview())
            .map_err(|_| "Diagnostic preview could not be encoded")?;
    tauri::async_runtime::spawn_blocking(move || {
        let Some(file) = app
            .dialog()
            .file()
            .add_filter("JSON", &["json"])
            .set_file_name("maxxit-diagnostics.json")
            .blocking_save_file()
        else {
            return Ok(false);
        };
        let path = file.into_path().map_err(|_| "Invalid export location")?;
        use std::io::Write;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(path)
            .map_err(|_| "Diagnostic export could not be opened")?;
        file.write_all(&body)
            .map_err(|_| "Diagnostic export could not be written")?;
        Ok(true)
    })
    .await
    .map_err(|_| "Diagnostic export interrupted")?
}

/// Only debug builds expose synthetic subprocess probes. No provider or Keychain access.
#[cfg(debug_assertions)]
pub fn probe(directory: PathBuf, mode: &str) {
    let queue = Queue::open(&directory.join("diagnostics")).expect("Probe queue");
    queue
        .set_consent(true, mode == "abort")
        .expect("Probe consent");
    drop(queue);
    initialize(directory, false);
    let result: Result<(), CommandError> =
        observe(Operation::new("diagnostics_test", "main", None), || {
            record("diagnostics.test", json!({"stage":"start"}));
            match mode {
                "panic" => panic!("PRIVATE_MARKER synthetic panic"),
                "abort" => std::process::abort(),
                _ => Err("PRIVATE_MARKER synthetic command failure".into()),
            }
        });
    if result.is_err() {
        println!("Synthetic failure persisted");
    }
}
