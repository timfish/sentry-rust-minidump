use crate::socket_from_release;
use minidumper::{LoopAction, MinidumpBinary, Server, ServerHandler};
use sentry::{
    protocol::{Attachment, AttachmentType, Event, Value},
    Level, ScopeUpdate,
};
use std::{
    fs::{self, File},
    io,
    path::PathBuf,
    sync::atomic::AtomicBool,
};
use uuid::Uuid;

fn attachment_from_minidump(minidump: MinidumpBinary) -> (Attachment, PathBuf) {
    (
        Attachment {
            filename: minidump
                .path
                .file_name()
                .expect("minidump should have filename")
                .to_string_lossy()
                .to_string(),
            ty: Some(AttachmentType::Minidump),
            buffer: minidump.contents.expect("minidump should have contents"),
            content_type: None,
        },
        minidump.path,
    )
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

                let _ = fs::remove_file(path);
            }
            Err(e) => {
                sentry::capture_error(&e);
            }
        }

        // Tells the server to exit, which will in turn exit the process
        LoopAction::Exit
    }

    fn on_message(&self, kind: u32, buffer: Vec<u8>) {
        if let 1 = kind {
            let update: ScopeUpdate =
                bincode::deserialize(&buffer[..]).expect("should be valid bincode");

            match update {
                ScopeUpdate::AddBreadcrumb(b) => sentry::add_breadcrumb(b),
                ScopeUpdate::ClearBreadcrumbs => {
                    sentry::configure_scope(|scope| scope.clear_breadcrumbs())
                }
                ScopeUpdate::User(user) => sentry::configure_scope(|scope| scope.set_user(user)),
                ScopeUpdate::SetExtra(k, v) => {
                    sentry::configure_scope(|scope| scope.set_extra(&k, v))
                }
                ScopeUpdate::RemoveExtra(k) => {
                    sentry::configure_scope(|scope| scope.remove_extra(&k))
                }
                ScopeUpdate::SetTag(k, v) => sentry::configure_scope(|scope| scope.set_tag(&k, v)),
                ScopeUpdate::RemoveTag(k) => sentry::configure_scope(|scope| scope.remove_tag(&k)),
                _ => todo!(),
            }
        }
    }
}

pub fn get_app_crashes_dir(release: &str) -> PathBuf {
    dirs_next::data_local_dir()
        .expect("Could not find local data directory")
        .join(release)
        .join("Crashes")
}

pub fn start(release: &str) {
    // Set the event.origin so that it's obvious when events come from the crash
    // reporter process rather than the main app process
    sentry::configure_scope(|scope| {
        scope.set_extra("event.process", Value::String("crash-reporter".to_string()));
    });

    let socket_name = socket_from_release(release);
    let mut server = Server::with_name(&socket_name).expect("failed to create server");

    let handler = Handler::new(get_app_crashes_dir(release));
    let shutdown = AtomicBool::new(false);

    server
        .run(Box::new(handler), &shutdown)
        .expect("failed to run server");
}
