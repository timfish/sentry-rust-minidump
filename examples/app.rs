fn main() {
    sentry_rust_minidump::init(
        sentry::release_name!(),
        |_| {
            // This code will run in both processes and setup sentry
            sentry::init((
                "https://233a45e5efe34c47a3536797ce15dafa@o447951.ingest.sentry.io/5650507",
                sentry::ClientOptions {
                    release: sentry::release_name!(),
                    debug: true,
                    ..Default::default()
                },
            ))
        },
        || {
            // This code only runs in the main app process
            std::thread::sleep(std::time::Duration::from_secs(2));

            #[allow(deref_nullptr)]
            unsafe {
                *std::ptr::null_mut() = true;
            }
        },
    );
}
