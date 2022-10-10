use crate::{socket_from_release, CRASH_REPORTER_ARG};
use crash_handler::{make_crash_event, CrashContext, CrashEventResult, CrashHandler};
use std::{
    fmt::Debug,
    process::{Child, Command},
    time::Duration,
};

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
    let socket_name = socket_from_release(release);

    let server_process = Command::new(std::env::current_exe()?)
        .arg(CRASH_REPORTER_ARG)
        .spawn()?;

    let mut wait_time = 0;

    loop {
        match minidumper::Client::with_name(&socket_name) {
            Ok(client) => {
                #[allow(unsafe_code)]
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
