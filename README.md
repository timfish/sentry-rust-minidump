# `sentry-rust-minidump`

![Master branch integration test status](https://img.shields.io/github/actions/workflow/status/timfish/sentry-rust-minidump/test.yml?label=Integration%20Tests&style=for-the-badge)

Uses the [`minidumper-child`](https://github.com/timfish/minidumper-child) crate
to capture minidumps from a separate process and sends them to Sentry as
attachments via the Sentry Rust SDK.

`sentry_rust_minidump::init` starts the current executable again with an
environment variable that causes it to start in crash reporter mode. In this
mode it waits for a minidump notification from the main app process and
handles writing and sending the minidump file as an attachment to Sentry.

Everything before `sentry_rust_minidump::init` is called in both the main and
crash reporter processes and should configure and start Sentry. Everything after
`sentry_rust_minidump::init` is only called in the main process to run your
application code.

```toml
[dependencies]
sentry = "0.49"
sentry-rust-minidump = "0.17"
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

## Configuration

Use `Builder` when the defaults do not fit:

```rust
fn main() {
    // Optionally skip logic in the crash reporter process.
    if sentry_rust_minidump::is_crash_reporter_process() {
        // ...
    }

    let client = sentry::init("__YOUR_DSN__");

    let _guard = sentry_rust_minidump::Builder::new()
        // Where minidumps are written before upload.
        // Default: `Crashes` in the system temp directory.
        .crashes_dir("/var/lib/my-app/crashes")
        // Pass this process's arguments to the crash reporter.
        // Default: false.
        .inherit_args(true)
        // The name shown by `ps`. Sets `argv[0]` on unix only.
        .process_name("my-app-crash-reporter")
        // Change the marker variable if the default could clash.
        // Then use `Builder::is_crash_reporter_process` instead of the
        // free function.
        .server_env_var("_MY_APP_CRASH_REPORTER")
        // Runs in the crash reporter before the event is sent. The scope
        // already holds the minidump attachment.
        .before_capture(|scope, path| {
            scope.set_tag("minidump", path.display().to_string());
        })
        // How long the crash reporter waits for the upload. Default: 5s.
        .flush_timeout(std::time::Duration::from_secs(15))
        // Full access to the spawn `Command` when the above is not enough.
        .on_process(|command| {
            command.env("RUST_LOG", "warn");
        })
        .install(&client)
        .expect("could not start crash reporter");

    App::run();
}
```

`install` never returns in the crash reporter process. It exits with status 0
when the app exits or crashes, and with status 1 if the crash reporter itself
fails to start. That failure is sent to Sentry first. An `Err` from `install`
therefore always comes from the app process.

The returned `Handle` keeps the crash reporter attached. Drop it and the crash
reporter exits. If there is no good place to keep it, call `Handle::leak`.

## `ipc` feature

By default there is no scope synchronisation from the app process to the crash
reporter process. This means that native crash event will be missing
breadcrumbs, user, tags or extra added to the scope in the app.

When the `ipc` feature is enabled, you can send scope updates to the crash
reporter process:

```rust
fn main() {
    let client = sentry::init("__YOUR_DSN__");

    // Everything before here runs in both app and crash reporter processes
    let crash_reporter = sentry_rust_minidump::init(&client).expect("crash reported didn't start");
    // Everything after here runs in only the app process

    crash_reporter.add_breadcrumb(...);
    crash_reporter.set_user(...);
    crash_reporter.set_extra(...);
    crash_reporter.set_tag(...);

    // Don't drop crash_reporter or the reporter process will close!
}
```
