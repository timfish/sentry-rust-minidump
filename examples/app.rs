fn main() {
    let client = sentry::init("http://abc123@127.0.0.1:8123/12345");

    // Everything before here runs in both app and crash reporter processes
    let crash_handler =
        sentry_rust_minidump::init(&client).expect("could not initialize crash reporter");
    // Everything after here runs in only the app process

    crash_handler.set_user(Some(sentry::User {
        username: Some("john_doe".into()),
        email: Some("john@doe.town".into()),
        ..Default::default()
    }));

    std::thread::sleep(std::time::Duration::from_secs(10));

    unsafe { sadness_generator::raise_segfault() };
}
