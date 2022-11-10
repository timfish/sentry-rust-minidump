use minidumper_child::{ClientHandle, Error, MinidumperChild};
use sentry::{
    protocol::{Attachment, AttachmentType, Event, Value},
    Level,
};

#[must_use = "The return value from init() should not be dropped until the program exits"]
pub fn init(sentry_client: &sentry::Client) -> Result<ClientHandle, Error> {
    let sentry_client = sentry_client.clone();

    let child = MinidumperChild::new().on_minidump(move |buffer, path| {
        sentry::with_scope(
            |scope| {
                // Remove event.process because this event came from the
                // main app process
                scope.remove_extra("event.process");

                let filename = path
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "minidump.dmp".to_string());

                scope.add_attachment(Attachment {
                    buffer,
                    filename,
                    ty: Some(AttachmentType::Minidump),
                    ..Default::default()
                });
            },
            || {
                sentry::capture_event(Event {
                    level: Level::Fatal,
                    ..Default::default()
                })
            },
        );

        // We need to flush because the server will exit after this closure returns
        sentry_client.flush(Some(std::time::Duration::from_secs(5)));
    });

    if child.is_crash_reporter_process() {
        // Set the event.origin so that it's obvious when Rust events come from
        // the crash reporter process rather than the main app process
        sentry::configure_scope(|scope| {
            scope.set_extra("event.process", Value::String("crash-reporter".to_string()));
        });
    }

    child.spawn()
}
