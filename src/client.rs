use crate::{socket_from_release, CRASH_REPORTER_ARG};
use crash_handler::{make_crash_event, CrashContext, CrashEventResult, CrashHandler};
use std::{
    fmt::Debug,
    process::{Child, Command},
    sync::Arc,
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

pub fn start(release: &str) -> Result<(Child, CrashHandler), ClientStartError> {
    let socket_name = socket_from_release(release);

    let server_process = Command::new(std::env::current_exe()?)
        .arg(CRASH_REPORTER_ARG)
        .spawn()?;

    let mut wait_time = 0;

    loop {
        match minidumper::Client::with_name(&socket_name).map(Arc::new) {
            Ok(client) => {
                let cloned_client = client.clone();

                sentry::configure_scope(|scope| {
                    scope.add_scope_listener(move |update| {
                        let encoded: Vec<u8> =
                            bincode::serialize(update).expect("should be able to serialise");

                        cloned_client
                            .send_message(1, &encoded)
                            .expect("IPC should work without fail no?");
                    })
                });

                #[allow(unsafe_code)]
                match CrashHandler::attach(unsafe {
                    make_crash_event(move |crash_context: &CrashContext| {
                        CrashEventResult::Handled(client.request_dump(crash_context).is_ok())
                    })
                }) {
                    Ok(crash_handler) => return Ok((server_process, crash_handler)),
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
