use crate::CRASH_REPORTER_ARG;
use crash_handler::{make_crash_event, CrashContext, CrashEventResult, CrashHandler};
use std::{
    fmt::Debug,
    process::{Child, Command},
    sync::Arc,
    time::Duration,
};
use uuid::Uuid;

#[derive(thiserror::Error, Debug)]
pub enum ClientStartError {
    #[error(transparent)]
    StartServer(#[from] std::io::Error),
    #[error(transparent)]
    AttachCrashHandler(#[from] crash_handler::Error),
    #[error(transparent)]
    StartIpc(#[from] minidumper::Error),
}

pub struct ClientHandle(Child, CrashHandler);

pub fn start(release: &str) -> Result<ClientHandle, ClientStartError> {
    // We add a uuid at the end of the release so multiple instances of the
    // same app can each have a crash reporter process
    let socket_name = format!(
        "{}-{}",
        release
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>(),
        Uuid::new_v4()
    );

    let server_arg = format!("{}={}", CRASH_REPORTER_ARG, socket_name);

    let server_process = Command::new(std::env::current_exe()?)
        .arg(server_arg)
        .spawn()?;

    let mut wait_time = 0;

    loop {
        match minidumper::Client::with_name(&socket_name).map(Arc::new) {
            Ok(client) => {
                std::thread::spawn({
                    let client = client.clone();
                    move || loop {
                        std::thread::sleep(Duration::from_secs(3));

                        if client.ping().is_err() {
                            break;
                        }
                    }
                });

                match CrashHandler::attach(unsafe {
                    make_crash_event(move |crash_context: &CrashContext| {
                        CrashEventResult::Handled(client.request_dump(crash_context).is_ok())
                    })
                }) {
                    Ok(crash_handler) => return Ok(ClientHandle(server_process, crash_handler)),
                    Err(e) => return Err(e.into()),
                }
            }
            Err(e) => {
                if wait_time < 3000 {
                    std::thread::sleep(Duration::from_millis(100));
                    wait_time += 100;
                } else {
                    return Err(e.into());
                }
            }
        }
    }
}
