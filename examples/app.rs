#![allow(deprecated)]

fn main() {
    // Use this to skip logic in the crash reporter process.
    if sentry_rust_minidump::is_crash_reporter_process() {
        eprintln!("starting crash reporter process");
    }

    let client = sentry::init("http://abc123@127.0.0.1:8123/12345");

    // Everything before here runs in both app and crash reporter processes
    let crash_handler = sentry_rust_minidump::Builder::new()
        .crashes_dir(std::env::temp_dir().join("sentry-rust-minidump-example"))
        .inherit_args(true)
        .process_name("app-crash-reporter")
        .before_capture(|scope, path| {
            eprintln!("minidump captured at {}", path.display());
            scope.set_tag("crash_reporter", "example");
        })
        .flush_timeout(std::time::Duration::from_secs(10))
        .install(&client)
        .expect("could not initialize crash reporter");
    // Everything after here runs in only the app process

    crash_handler.set_user(Some(sentry::User {
        username: Some("john_doe".into()),
        email: Some("john@doe.town".into()),
        ..Default::default()
    }));

    std::thread::sleep(std::time::Duration::from_secs(10));

    unsafe { sadness_generator::raise_segfault() };
}
