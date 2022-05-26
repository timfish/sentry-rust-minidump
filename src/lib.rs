use crash_handler::CrashHandler;
use minidumper::{Client, Server};
use sentry::{
    protocol::{Attachment, AttachmentType},
    ClientInitGuard,
};
use std::{borrow::Cow, fs, path::PathBuf, process::Child};

struct Handler {
    crashes_dir: PathBuf,
}

impl Handler {
    pub fn new(app_key: &str) -> Self {
        let crashes_dir = dirs_next::data_local_dir()
            .expect("Could not find local config directory")
            .join(format!("{} Crashes", app_key));

        Handler { crashes_dir }
    }
}

impl minidumper::ServerHandler for Handler {
    /// Called when a crash has been received and a backing file needs to be
    /// created to store it.
    fn create_minidump_file(&self) -> Result<(std::fs::File, std::path::PathBuf), std::io::Error> {
        fs::create_dir_all(&self.crashes_dir).unwrap();

        let path = self
            .crashes_dir
            .join(format!("{}.dmp", uuid::Uuid::new_v4()));

        Ok((std::fs::File::create(&path)?, path))
    }

    /// Called when a crash has been fully written as a minidump to the provided
    /// file. Also returns the full heap buffer as well.
    fn on_minidump_created(
        &self,
        result: Result<minidumper::MinidumpBinary, minidumper::Error>,
    ) -> minidumper::LoopAction {
        match result {
            Ok(md_bin) => {
                let attachment = Attachment {
                    filename: md_bin
                        .path
                        .file_name()
                        .expect("minidump should have filename")
                        .to_string_lossy()
                        .to_string(),
                    ty: Some(AttachmentType::Minidump),
                    buffer: md_bin.contents.expect("minidump should have contents"),
                    ..Default::default()
                };

                sentry::with_scope(
                    |scope| scope.add_attachment(attachment),
                    || sentry::capture_event(Default::default()),
                );

                let _ = fs::remove_file(md_bin.path);
            }
            Err(e) => {
                sentry::capture_error(&e);
            }
        }

        // Tells the server to exit, which will in turn exit the process
        minidumper::LoopAction::Exit
    }

    fn on_message(&self, _kind: u32, _buffer: Vec<u8>) {
        //
    }
}

fn get_app_unique_key(release: Option<Cow<str>>) -> String {
    release
        .map(|r| r.to_string())
        .unwrap_or_else(|| {
            std::env::current_exe()
                .expect("could not get current exe")
                .file_name()
                .expect("could not get current exe")
                .to_string_lossy()
                .to_string()
        })
        .replace('@', "-")
        .replace('.', "-")
}

fn start_server(app_key: &str) {
    let mut server = Server::with_name(app_key).expect("failed to create server");

    let ab = std::sync::atomic::AtomicBool::new(false);

    server
        .run(Box::new(Handler::new(app_key)), &ab)
        .expect("failed to run server");
}

fn start_client(app_key: &str, server_arg: &str) -> (CrashHandler, Option<Child>) {
    let mut _server_proc = None;

    // Attempt to connect to the server
    let client = loop {
        if let Ok(client) = Client::with_name(app_key) {
            break client;
        }

        let exe = std::env::current_exe().expect("unable to find ourselves");

        _server_proc = Some(
            std::process::Command::new(exe)
                .arg(server_arg)
                .stdout(std::process::Stdio::piped())
                .spawn()
                .expect("unable to spawn server process"),
        );

        // Give it time to start
        std::thread::sleep(std::time::Duration::from_millis(1000));
    };

    #[allow(unsafe_code)]
    let _crash_handler = crash_handler::CrashHandler::attach(unsafe {
        crash_handler::make_crash_event(move |crash_context: &crash_handler::CrashContext| {
            crash_handler::CrashEventResult::Handled(client.request_dump(crash_context).is_ok())
        })
    })
    .expect("failed to attach signal handler");

    (_crash_handler, _server_proc)
}

pub fn init<F: FnOnce() -> ClientInitGuard, A: FnOnce()>(
    release: Option<Cow<str>>,
    init_sentry: F,
    run_app: A,
) {
    let _sentry_guard = init_sentry();

    let app_key = get_app_unique_key(release);
    let server_arg = format!("--crash-reporter-process-{}", app_key);

    if std::env::args().any(|a| a == server_arg) {
        start_server(&app_key);
    } else {
        let _guard = start_client(&app_key, &server_arg);
        run_app()
    }
}
