mod client;
mod server;

const CRASH_REPORTER_ARG: &str = "--start-crash-reporter-server";

fn get_release_fallback() -> Option<String> {
    std::env::current_exe().ok().and_then(|current_exe| {
        current_exe
            .file_name()
            .map(|file_name| file_name.to_string_lossy().to_string())
    })
}

pub(crate) fn socket_from_release(release: &str) -> String {
    release
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

pub fn init<Release, SentryInitFn, RunAppFn>(
    release: Option<Release>,
    init_sentry: SentryInitFn,
    run_app: RunAppFn,
) where
    Release: Into<String>,
    SentryInitFn: FnOnce(bool) -> sentry::ClientInitGuard,
    RunAppFn: FnOnce(),
{
    let is_crash_reporter = std::env::args().any(|a| a == CRASH_REPORTER_ARG);
    let _sentry_guard = init_sentry(is_crash_reporter);

    let release: Option<String> = release.map(|r| r.into()).or_else(get_release_fallback);

    if is_crash_reporter {
        if let Some(release) = release {
            server::start(&release);
        }
    } else {
        if let Some(release) = release {
            let handler = client::start(&release);

            if let Err(e) = handler {
                sentry::capture_error(&e);
            }
        }

        run_app()
    }
}
