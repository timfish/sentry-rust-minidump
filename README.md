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
    let client = sentry::init(("__DSN__", sentry::ClientOptions {
        release: sentry::release_name!(),
        debug: true,
        ..Default::default()
    }));

    // Everything before here runs in both app and crash reporter processes
    let _guard = sentry_rust_minidump::init(&client);
    // Everything after here runs in only the app process

    std::thread::sleep(std::time::Duration::from_secs(2));

    #[allow(deref_nullptr)]
    unsafe {
        *std::ptr::null_mut() = true;
    }
}
```
