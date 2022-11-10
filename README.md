# `sentry-rust-minidump` 

![Master branch integration test status](https://img.shields.io/github/workflow/status/timfish/sentry-rust-minidump/Test/master?label=Integration%20Tests&style=for-the-badge)

Uses the [`minidumper-child`](https://github.com/timfish/minidumper-child) crate
to capture minidumps from a separate process and sends them to Sentry as
attachments via Sentry Rust. 

`sentry_rust_minidump::init` starts the current executable again with an argument that
causes it to start in crash reporter mode. In this mode it waits for minidump
notification from the main app process and handles writing and sending of the
minidump file as an attachment to Sentry.

Everything before `sentry_rust_minidump::init` is called in both the main and
crash reporter processes and should configure and start Sentry. Everything
after `sentry_rust_minidump::init` is only called in the main process to run
your application code.

```toml
[dependencies]
sentry = "0.28"
sentry-rust-minidump = "0.3"
```

```rust
fn main() {
    let client = sentry::init("__YOUR_DSN__");

    // Everything before here runs in both app and crash reporter processes
    let _guard = sentry_rust_minidump::init(&client);
    // Everything after here runs in only the app process

    App::run();

    // This will cause a minidump to be sent to Sentry 
    #[allow(deref_nullptr)]
    unsafe {
        *std::ptr::null_mut() = true;
    }
}
```
