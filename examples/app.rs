fn main() {
    let client = sentry::init((
        "http://abc123@127.0.0.1:8080/12345",
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    // Everything before here runs in both app and crash reporter processes
    let _guard = sentry_rust_minidump::init(&client);
    // Everything after here runs in only the app process

    std::thread::sleep(std::time::Duration::from_secs(10));

    #[allow(deref_nullptr)]
    unsafe {
        *std::ptr::null_mut() = true;
    }
}
