//! **Deprecated:** use [`sentry-minidump`] instead. It is part of the
//! official Sentry Rust SDK. Enable the `minidump` feature of `sentry`
//! and add `sentry::minidump::MinidumpIntegration` to your
//! `ClientOptions`. This crate will get no more updates.
//!
//! [`sentry-minidump`]: https://docs.rs/sentry-minidump
//!
//! Captures native crashes as minidumps in a separate process and sends
//! them to Sentry as attachments.
//!
//! [`init`] covers the common case. Use [`Builder`] when the crash
//! reporter process needs a custom crashes directory, environment
//! variable name, process name or arguments.

// Deprecated items still use each other inside this crate.
#![allow(deprecated)]

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

pub use minidumper_child::Error;
use minidumper_child::{ClientHandle, MinidumperChild};
use sentry::{
    protocol::{Attachment, AttachmentType, Event, Value},
    Level, Scope,
};

#[cfg(feature = "ipc")]
use sentry::{Breadcrumb, User};

/// The environment variable that marks the crash reporter process.
///
/// Change it with [`Builder::server_env_var`].
pub const DEFAULT_SERVER_ENV_VAR: &str = "_CRASH_REPORTER_SERVER";

/// The default time to wait for the crash event to upload.
pub const DEFAULT_FLUSH_TIMEOUT: Duration = Duration::from_secs(5);

/// Returns `true` if the current process is the crash reporter process.
///
/// This checks [`DEFAULT_SERVER_ENV_VAR`]. If you set a custom name with
/// [`Builder::server_env_var`], use [`Builder::is_crash_reporter_process`]
/// instead.
///
/// It is safe to call this before the Sentry client exists, for example
/// to skip log output in the crash reporter process.
#[deprecated(
    since = "0.18.1",
    note = "use `sentry::minidump::MinidumpIntegration::is_crash_reporter_process` instead"
)]
pub fn is_crash_reporter_process() -> bool {
    std::env::var_os(DEFAULT_SERVER_ENV_VAR).is_some()
}

/// Keeps the crash reporter process alive.
///
/// Dropping the handle detaches the crash handler and the crash reporter
/// process exits. Keep it alive for the life of the program or call
/// [`Handle::leak`].
#[must_use = "The handle should not be dropped until the program exits"]
pub struct Handle {
    _handle: ClientHandle,
}

impl Handle {
    /// Keeps the crash reporter attached for the rest of the program.
    ///
    /// Use this when there is no convenient place to store the handle.
    pub fn leak(self) {
        std::mem::forget(self);
    }
}

#[cfg(feature = "ipc")]
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq)]
pub enum ScopeUpdate {
    AddBreadcrumb(Breadcrumb),
    SetUser(Option<User>),
    SetExtra(String, Option<Value>),
    SetTag(String, Option<String>),
}

#[cfg(feature = "ipc")]
impl Handle {
    fn send_message(&self, update: &ScopeUpdate) {
        let buffer = serde_json::to_vec(update).expect("could not serialize scope update");
        self._handle.send_message(0, buffer).ok();
    }

    pub fn add_breadcrumb(&self, breadcrumb: Breadcrumb) {
        self.send_message(&ScopeUpdate::AddBreadcrumb(breadcrumb));
    }

    pub fn set_user(&self, user: Option<User>) {
        self.send_message(&ScopeUpdate::SetUser(user));
    }

    pub fn set_extra(&self, key: String, value: Option<Value>) {
        self.send_message(&ScopeUpdate::SetExtra(key, value));
    }

    pub fn set_tag(&self, key: String, value: Option<String>) {
        self.send_message(&ScopeUpdate::SetTag(key, value));
    }
}

type OnProcess = Box<dyn FnOnce(&mut Command) + Send + Sync + 'static>;
type BeforeCapture = Box<dyn Fn(&mut Scope, &Path) + Send + Sync + 'static>;

/// Configures and starts the crash reporter.
///
/// ```no_run
/// let client = sentry::init("__YOUR_DSN__");
///
/// let _guard = sentry_rust_minidump::Builder::new()
///     .crashes_dir("/var/lib/my-app/crashes")
///     .inherit_args(true)
///     .process_name("my-app-crash-reporter")
///     .install(&client)
///     .expect("could not start crash reporter");
/// ```
#[deprecated(
    since = "0.18.1",
    note = "use `sentry::minidump::MinidumpIntegration` with `ClientOptions::add_integration` instead"
)]
#[must_use = "Call install() or the crash reporter won't start"]
pub struct Builder {
    crashes_dir: Option<PathBuf>,
    server_env_var: String,
    inherit_args: bool,
    process_name: Option<OsString>,
    on_process: Option<OnProcess>,
    before_capture: Option<BeforeCapture>,
    flush_timeout: Duration,
    client_connect_timeout: Option<Duration>,
    server_stale_timeout: Option<Duration>,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            crashes_dir: None,
            server_env_var: DEFAULT_SERVER_ENV_VAR.to_owned(),
            inherit_args: false,
            process_name: None,
            on_process: None,
            before_capture: None,
            flush_timeout: DEFAULT_FLUSH_TIMEOUT,
            client_connect_timeout: None,
            server_stale_timeout: None,
        }
    }
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if the current process is the crash reporter process.
    ///
    /// This respects [`Builder::server_env_var`] and does not need a Sentry
    /// client, so it can run before logging or Sentry are set up.
    pub fn is_crash_reporter_process(&self) -> bool {
        std::env::var_os(&self.server_env_var).is_some()
    }

    /// Sets the directory where minidumps are written before upload.
    ///
    /// Defaults to `Crashes` inside the system temp directory.
    pub fn crashes_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.crashes_dir = Some(dir.into());
        self
    }

    /// Sets the environment variable that marks the crash reporter process.
    ///
    /// Defaults to [`DEFAULT_SERVER_ENV_VAR`]. Change it when the default
    /// could clash with another crash reporter in the same process tree.
    pub fn server_env_var(mut self, name: impl Into<String>) -> Self {
        self.server_env_var = name.into();
        self
    }

    /// Passes the arguments of the current process to the crash reporter
    /// process.
    ///
    /// Useful if the app needs to read its config from command-line
    /// arguments.
    /// Defaults to `false`.
    pub fn inherit_args(mut self, inherit: bool) -> Self {
        self.inherit_args = inherit;
        self
    }

    /// Sets the process name of the crash reporter as shown by `ps`.
    ///
    /// This sets `argv[0]` on unix. It has no effect on other platforms.
    pub fn process_name(mut self, name: impl Into<OsString>) -> Self {
        self.process_name = Some(name.into());
        self
    }

    /// Modifies the [`Command`] used to spawn the crash reporter process.
    ///
    /// Runs after [`Builder::inherit_args`] and [`Builder::process_name`]
    /// are applied. The marker environment variable is set after this
    /// callback, so `env_clear` does not break process detection.
    pub fn on_process<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut Command) + Send + Sync + 'static,
    {
        self.on_process = Some(Box::new(f));
        self
    }

    /// Runs in the crash reporter process before the crash event is sent.
    ///
    /// The scope already holds the minidump attachment. Use this to add
    /// tags or to log the path of the minidump.
    pub fn before_capture<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Scope, &Path) + Send + Sync + 'static,
    {
        self.before_capture = Some(Box::new(f));
        self
    }

    /// Sets how long the crash reporter waits for the upload to finish.
    ///
    /// Defaults to [`DEFAULT_FLUSH_TIMEOUT`].
    pub fn flush_timeout(mut self, timeout: Duration) -> Self {
        self.flush_timeout = timeout;
        self
    }

    /// Sets how long the app waits for the crash reporter to accept a
    /// connection.
    pub fn client_connect_timeout(mut self, timeout: Duration) -> Self {
        self.client_connect_timeout = Some(timeout);
        self
    }

    /// Sets how long the crash reporter waits without a ping from the app
    /// before it exits.
    pub fn server_stale_timeout(mut self, timeout: Duration) -> Self {
        self.server_stale_timeout = Some(timeout);
        self
    }

    /// Starts the crash reporter.
    ///
    /// In the app process this spawns the crash reporter and returns a
    /// [`Handle`]. In the crash reporter process this never returns: it
    /// exits with status 0 when the app exits or crashes, and with status
    /// 1 if the crash reporter fails to start. In that case the error is
    /// sent to Sentry first.
    ///
    /// An `Err` therefore always comes from the app process.
    #[must_use = "The return value from install() should not be dropped until the program exits"]
    pub fn install(self, sentry_client: &sentry::Client) -> Result<Handle, Error> {
        let sentry_client = sentry_client.clone();
        let is_crash_reporter_process = self.is_crash_reporter_process();

        let mut child = MinidumperChild::new().with_server_env_var(self.server_env_var);

        if let Some(dir) = self.crashes_dir {
            child = child.with_crashes_dir(dir);
        }

        if let Some(timeout) = self.client_connect_timeout {
            child = child.with_client_connect_timeout(timeout);
        }

        if let Some(timeout) = self.server_stale_timeout {
            child = child.with_server_stale_timeout(timeout);
        }

        let inherit_args = self.inherit_args;
        let process_name = self.process_name;
        let on_process = self.on_process;
        child = child.on_process(move |command| {
            if inherit_args {
                command.args(std::env::args_os().skip(1));
            }

            #[cfg(unix)]
            if let Some(name) = process_name {
                use std::os::unix::process::CommandExt;
                command.arg0(name);
            }
            #[cfg(not(unix))]
            let _ = process_name;

            if let Some(on_process) = on_process {
                on_process(command);
            }
        });

        #[cfg(feature = "ipc")]
        let child = child.on_message(|_kind, buffer| {
            if let Ok(update) = serde_json::from_slice::<ScopeUpdate>(&buffer[..]) {
                match update {
                    ScopeUpdate::AddBreadcrumb(b) => sentry::add_breadcrumb(b),
                    ScopeUpdate::SetUser(u) => sentry::configure_scope(|scope| {
                        scope.set_user(u);
                    }),
                    ScopeUpdate::SetExtra(k, v) => sentry::configure_scope(|scope| match v {
                        Some(v) => scope.set_extra(&k, v),
                        None => scope.remove_extra(&k),
                    }),
                    ScopeUpdate::SetTag(k, v) => match v {
                        Some(v) => sentry::configure_scope(|scope| scope.set_tag(&k, &v)),
                        None => sentry::configure_scope(|scope| scope.remove_tag(&k)),
                    },
                }
            }
        });

        let flush_timeout = self.flush_timeout;
        let before_capture = self.before_capture;
        let client = sentry_client.clone();
        let child = child.on_minidump(move |buffer, path| {
            sentry::with_scope(
                |scope| {
                    // Remove event.process because this event came from the
                    // main app process
                    scope.remove_extra("event.process");

                    let filename = path
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "minidump.dmp".to_string());

                    scope.add_attachment(Attachment {
                        buffer,
                        filename,
                        ty: Some(AttachmentType::Minidump),
                        ..Default::default()
                    });

                    if let Some(before_capture) = &before_capture {
                        before_capture(scope, path);
                    }
                },
                || {
                    sentry::capture_event(Event {
                        level: Level::Fatal,
                        ..Default::default()
                    })
                },
            );

            // We need to flush because the server will exit after this closure returns
            client.flush(Some(flush_timeout));
        });

        if is_crash_reporter_process {
            // Set event.process so that it's obvious when Rust events come from
            // the crash reporter process rather than the main app process
            sentry::configure_scope(|scope| {
                scope.set_extra("event.process", Value::String("crash-reporter".to_string()));
            });
        }

        match child.spawn() {
            Ok(handle) => Ok(Handle { _handle: handle }),
            Err(err) if is_crash_reporter_process => {
                // Never fall through into the app code from the crash
                // reporter process. That would start a second copy of the
                // app.
                sentry::capture_error(&err);
                sentry_client.flush(Some(flush_timeout));
                std::process::exit(1);
            }
            Err(err) => Err(err),
        }
    }
}

/// Starts the crash reporter with the default configuration.
///
/// This is the same as `Builder::new().install(sentry_client)`.
#[deprecated(
    since = "0.18.1",
    note = "use `sentry::minidump::MinidumpIntegration` with `ClientOptions::add_integration` instead"
)]
#[must_use = "The return value from init() should not be dropped until the program exits"]
pub fn init(sentry_client: &sentry::Client) -> Result<Handle, Error> {
    Builder::new().install(sentry_client)
}
