use crate::socket_from_release;
use minidumper::{LoopAction, MinidumpBinary, Server, ServerHandler};
use sentry::{
    protocol::{Attachment, Event, Value},
    Level,
};
use std::{
    fs::{self, File},
    io,
    path::PathBuf,
    sync::atomic::AtomicBool,
};
use uuid::Uuid;

fn attachment_from_minidump(minidump: MinidumpBinary) -> (Option<Attachment>, PathBuf) {
    let attachment = minidump.contents.and_then(|buffer| {
        minidump.path.file_name().map(|name| -> Attachment {
            Attachment {
                buffer,
                filename: name.to_string_lossy().to_string(),
                ..Default::default()
            }
        })
    });

    (attachment, minidump.path)
}

struct Handler {
    crashes_dir: PathBuf,
}

impl Handler {
    pub fn new(crashes_dir: PathBuf) -> Self {
        Handler { crashes_dir }
    }
}

impl ServerHandler for Handler {
    /// Called when a crash has been received and a backing file needs to be
    /// created to store it.
    fn create_minidump_file(&self) -> Result<(File, PathBuf), io::Error> {
        fs::create_dir_all(&self.crashes_dir)?;
        let file_name = format!("{}.dmp", Uuid::new_v4());
        let path = self.crashes_dir.join(file_name);
        Ok((File::create(&path)?, path))
    }

    /// Called when a crash has been fully written as a minidump to the provided
    /// file. Also returns the full heap buffer as well.
    fn on_minidump_created(&self, result: Result<MinidumpBinary, minidumper::Error>) -> LoopAction {
        match result {
            Ok(minidump) => {
                let (attachment, path) = attachment_from_minidump(minidump);

                if let Some(attachment) = attachment {
                    sentry::with_scope(
                        |scope| {
                            // Remove event.process because this event came from the
                            // main app process
                            scope.remove_extra("event.process");
                            scope.add_attachment(attachment);
                        },
                        || {
                            sentry::capture_event(Event {
                                level: Level::Fatal,
                                ..Default::default()
                            })
                        },
                    );
                }

                let _ = fs::remove_file(path);
            }
            Err(e) => {
                sentry::capture_error(&e);
            }
        }

        // Tells the server to exit, which will in turn exit the process
        LoopAction::Exit
    }

    fn on_message(&self, _kind: u32, _buffer: Vec<u8>) {
        //
    }
}

pub fn get_app_crashes_dir(release: &str) -> Option<PathBuf> {
    dirs_next::data_local_dir().map(|p| p.join(release).join("Crashes"))
}

pub fn start(release: &str) {
    // Set the event.origin so that it's obvious when events come from the crash
    // reporter process rather than the main app process
    sentry::configure_scope(|scope| {
        scope.set_extra("event.process", Value::String("crash-reporter".to_string()));
    });

    let socket_name = socket_from_release(release);

    if let Some(crashes_dir) = get_app_crashes_dir(release) {
        if let Ok(mut server) = Server::with_name(&socket_name) {
            let handler = Handler::new(crashes_dir);
            let shutdown = AtomicBool::new(false);

            let _ = server.run(Box::new(handler), &shutdown);
        }
    }
}
