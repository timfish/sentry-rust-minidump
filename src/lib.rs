mod client;
mod server;

const SERVER_ARG: &str = "--start-crash-reporter-server";

fn get_release_fallback() -> String {
    std::env::current_exe()
        .expect("could not get current exe")
        .file_name()
        .expect("current_exe should have a file name")
        .to_string_lossy()
        .to_string()
}

pub fn socket_from_release(release: &str) -> String {
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
    let is_crash_reporter = std::env::args().any(|a| a == SERVER_ARG);
    let _sentry_guard = init_sentry(is_crash_reporter);
    let release = release
        .map(|r| r.into())
        .unwrap_or_else(get_release_fallback);

    if is_crash_reporter {
        server::start(&release);
    } else {
        let handler = client::start(&release);

        if let Err(e) = handler {
            sentry::capture_error(&e);
        }

        run_app()
    }
}
