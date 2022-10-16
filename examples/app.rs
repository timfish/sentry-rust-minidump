fn main() {
    let client = sentry::init((
        "https://233a45e5efe34c47a3536797ce15dafa@o447951.ingest.sentry.io/5650507",
        sentry::ClientOptions {
            release: sentry::release_name!(),
            debug: true,
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
