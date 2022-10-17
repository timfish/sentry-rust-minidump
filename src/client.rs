use crate::constants::*;
use crash_handler::{make_crash_event, CrashContext, CrashEventResult, CrashHandler};
use std::{fmt::Debug, process, sync::Arc, time::Duration};

#[derive(thiserror::Error, Debug)]
pub enum ClientStartError {
    #[error(transparent)]
    StartClient(#[from] std::io::Error),
    #[error(transparent)]
    AttachCrashHandler(#[from] crash_handler::Error),
    #[error(transparent)]
    StartIpc(#[from] minidumper::Error),
}

pub struct ClientHandle(process::Child, CrashHandler);

pub fn start(release: &str) -> Result<ClientHandle, ClientStartError> {
    // The socket name is unique because we don't share the crash reporter
    // process between multiple instances of the app.
    let socket_name = format!(
        "{}-{}",
        release
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>(),
        uuid::Uuid::new_v4()
    );

    let server_arg = format!("{}={}", CRASH_REPORTER_ARG, socket_name);

    let server_process = process::Command::new(std::env::current_exe()?)
        .arg(server_arg)
        .spawn()?;

    let mut wait_time = 0;

    let client = loop {
        match minidumper::Client::with_name(&socket_name).map(Arc::new) {
            Ok(client) => break client,
            Err(e) => {
                if wait_time < CLIENT_CONNECT_TIMEOUT {
                    std::thread::sleep(Duration::from_millis(CLIENT_CONNECT_RETRY));
                    wait_time += CLIENT_CONNECT_RETRY;
                } else {
                    return Err(e.into());
                }
            }
        }
    };

    // Start a thread that pings the server so that it doesn't timeout and exit
    std::thread::spawn({
        let client = client.clone();
        move || loop {
            std::thread::sleep(Duration::from_millis(CLIENT_SERVER_POLL));

            if client.ping().is_err() {
                break;
            }
        }
    });

    Ok(CrashHandler::attach(unsafe {
        make_crash_event(move |crash_context: &CrashContext| {
            CrashEventResult::Handled(client.request_dump(crash_context).is_ok())
        })
    })
    .map(|crash_handler| ClientHandle(server_process, crash_handler))?)
}
