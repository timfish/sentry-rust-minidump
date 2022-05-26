use crash_handler::{make_crash_event, CrashContext, CrashEventResult, CrashHandler};
use minidumper::Client;
use std::{fmt::Debug, process::Child};

use crate::{socket_from_release, SERVER_ARG};

#[derive(thiserror::Error, Debug)]
pub enum ClientStartError {
    #[error(transparent)]
    StartServer(#[from] std::io::Error),
    #[error(transparent)]
    AttachCrashHandler(#[from] crash_handler::Error),
    #[error(transparent)]
    StartIpc(#[from] minidumper::Error),
}

enum State {
    StartServer,
    ConnectToServer(Child, u32),
    AttachCrashHandler(Child, Client),
    Complete(Child, CrashHandler),
    Error(ClientStartError),
}

impl Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StartServer => write!(f, "StartServer"),
            Self::ConnectToServer { .. } => write!(f, "ConnectToServer"),
            Self::AttachCrashHandler { .. } => write!(f, "AttachCrashHandler"),
            Self::Complete { .. } => write!(f, "Complete"),
            Self::Error(_) => write!(f, "Error"),
        }
    }
}

pub fn start(release: &str) -> Result<(Child, CrashHandler), ClientStartError> {
    let socket_name = socket_from_release(release);
    let mut state = State::StartServer;

    loop {
        state = match state {
            State::StartServer => {
                let result: Result<Child, std::io::Error> = (|| {
                    let current_exe = std::env::current_exe()?;
                    let server_process = std::process::Command::new(&current_exe)
                        .arg(SERVER_ARG)
                        .spawn()?;

                    Ok(server_process)
                })();

                match result {
                    Ok(server_process) => State::ConnectToServer(server_process, 0),
                    Err(e) => State::Error(e.into()),
                }
            }
            State::ConnectToServer(server_process, mut wait_time) => {
                match Client::with_name(&socket_name) {
                    Ok(client) => State::AttachCrashHandler(server_process, client),
                    Err(e) => {
                        if wait_time >= 3000 {
                            State::Error(e.into())
                        } else {
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            wait_time += 100;
                            State::ConnectToServer(server_process, wait_time)
                        }
                    }
                }
            }
            State::AttachCrashHandler(server_process, client) => {
                #[allow(unsafe_code)]
                match CrashHandler::attach(unsafe {
                    make_crash_event(move |crash_context: &CrashContext| {
                        CrashEventResult::Handled(client.request_dump(crash_context).is_ok())
                    })
                }) {
                    Ok(crash_handler) => State::Complete(server_process, crash_handler),
                    Err(e) => State::Error(e.into()),
                }
            }
            state => panic!("Should not continue on state: {:?}", state),
        };

        match state {
            State::Complete(server_process, crash_handler) => {
                return Ok((server_process, crash_handler))
            }
            State::Error(e) => return Err(e),
            _ => {
                // Continue
            }
        }
    }
}
