# sentry-rust-minidump

Experimental code that wraps the `minidumper` and `crash-handler` crates to make it simpler to capture
and send minidumps from a separate process via Sentry Rust.

`sentry_rust_minidump::init` starts the current executable with an argument that
causes it to start in crash reporter mode. In this mode it waits for minidump
notification from the main process and handles writing and sending of the
minidump file as an attachment to Sentry.

The first closure is called in both the main and crash reporter processes and is used to configure
and start Sentry. The second closure is only called in the main process to run the
application code.

```toml
[dependencies]
sentry = "0.27"
sentry-rust-minidump = "0.1"
```

```rust
fn main() {
    sentry_rust_minidump::init(
        sentry::release_name!(),
        |is_crash_reporter_process| {
            // This code will be run early in both processes to setup sentry
            sentry::init((
                "__DSN__",
                sentry::ClientOptions {
                    release: sentry::release_name!(),
                    ..Default::default()
                },
            )) // You must return the guard!
        },
        || {
            // Run your app or whatever you were planning to do...
            app::run();

            // This will cause a minidump to be sent to Sentry
            unsafe {
                *std::ptr::null_mut() = true;
            }
        },
    );
}

```
