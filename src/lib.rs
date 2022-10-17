mod client;
mod server;

mod constants {
    pub const CRASH_REPORTER_ARG: &str = "--crash-reporter-server";
    pub const SERVER_STALE_TIMEOUT: u64 = 5000;
    pub const CLIENT_CONNECT_RETRY: u64 = 50;
    pub const CLIENT_CONNECT_TIMEOUT: u64 = 3000;
    pub const CLIENT_SERVER_POLL: u64 = SERVER_STALE_TIMEOUT / 2;
}

pub fn is_crash_reporter_process() -> bool {
    std::env::args().any(|arg| arg.starts_with(constants::CRASH_REPORTER_ARG))
}

#[must_use = "The return value of init should not be dropped until the program exits"]
pub fn init(sentry_client: &sentry::Client) -> Option<client::ClientHandle> {
    let release = sentry_client
        .options()
        .release
        .as_ref()
        .map(|r| r.to_string())
        .expect("A release must be set in sentry::ClientOptions");

    if is_crash_reporter_process() {
        server::start(&release);
        // The server has exited which means the main app process has crashed or
        // exited.
        // Because we're going to force-exit, we need to flush to ensure any
        // events are sent.
        sentry_client.flush(Some(std::time::Duration::from_secs(5)));
        // We have to force exit so that the app code after here does not run in
        // the crash reporter process.
        std::process::exit(0);
    } else {
        client::start(&release).ok()
    }
}
