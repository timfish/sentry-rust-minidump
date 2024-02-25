use minidumper_child::{ClientHandle, Error, MinidumperChild};
use sentry::{
    protocol::{Attachment, AttachmentType, Event, Value},
    Level,
};

#[cfg(feature = "ipc")]
use sentry::{Breadcrumb, User};

pub struct Handle {
    _handle: ClientHandle,
}

#[cfg(feature = "ipc")]
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq)]
pub enum ScopeUpdate {
    AddBreadcrumb(Breadcrumb),
    SetUser(Option<User>),
    SetExtra(String, Option<Value>),
    SetTag(String, Option<String>),
}

#[cfg(feature = "ipc")]
impl Handle {
    fn send_message(&self, update: &ScopeUpdate) {
        let buffer = serde_json::to_vec(update).expect("could not serialize scope update");
        self._handle.send_message(0, buffer).ok();
    }

    pub fn add_breadcrumb(&self, breadcrumb: Breadcrumb) {
        self.send_message(&ScopeUpdate::AddBreadcrumb(breadcrumb));
    }

    pub fn set_user(&self, user: Option<User>) {
        self.send_message(&ScopeUpdate::SetUser(user));
    }

    pub fn set_extra(&self, key: String, value: Option<Value>) {
        self.send_message(&ScopeUpdate::SetExtra(key, value));
    }

    pub fn set_tag(&self, key: String, value: Option<String>) {
        self.send_message(&ScopeUpdate::SetTag(key, value));
    }
}

#[must_use = "The return value from init() should not be dropped until the program exits"]
pub fn init(sentry_client: &sentry::Client) -> Result<Handle, Error> {
    let sentry_client = sentry_client.clone();

    let child = MinidumperChild::new();

    #[cfg(feature = "ipc")]
    let child = child.on_message(|_kind, buffer| {
        if let Ok(update) = serde_json::from_slice::<ScopeUpdate>(&buffer[..]) {
            match update {
                ScopeUpdate::AddBreadcrumb(b) => sentry::add_breadcrumb(b),
                ScopeUpdate::SetUser(u) => sentry::configure_scope(|scope| {
                    scope.set_user(u);
                }),
                ScopeUpdate::SetExtra(k, v) => sentry::configure_scope(|scope| match v {
                    Some(v) => scope.set_extra(&k, v),
                    None => scope.remove_extra(&k),
                }),
                ScopeUpdate::SetTag(k, v) => match v {
                    Some(v) => sentry::configure_scope(|scope| scope.set_tag(&k, &v)),
                    None => sentry::configure_scope(|scope| scope.remove_tag(&k)),
                },
            }
        }
    });

    let child = child.on_minidump(move |buffer, path| {
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

    child.spawn().map(|handle| Handle { _handle: handle })
}
