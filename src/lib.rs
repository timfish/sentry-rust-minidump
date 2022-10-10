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

pub fn is_crash_reporter_process() -> bool {
    std::env::args().any(|arg| arg == CRASH_REPORTER_ARG)
}

#[must_use = "The return value of init should not be dropped until the program exits"]
pub fn init(client: &sentry::Client) -> Option<client::ClientHandle> {
    let release = client
        .options()
        .release
        .as_ref()
        .map(|r| r.to_string())
        .or_else(get_release_fallback)
        .expect("A release must be set in sentry::ClientOptions");

    if is_crash_reporter_process() {
        server::start(&release);
        client.flush(Some(std::time::Duration::from_secs(5)));
        // We have to force exit so that the app code after here does not run in
        // the crash reporter process.
        std::process::exit(0);
    } else {
        match client::start(&release) {
            Ok(handler) => Some(handler),
            Err(e) => {
                sentry::capture_error(&e);
                None
            }
        }
    }
}
