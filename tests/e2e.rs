use sentry::protocol::AttachmentType;
use std::{process::Command, time::Duration};

#[actix_rt::test]
async fn test_example_app() -> Result<(), Box<dyn std::error::Error>> {
    let envelope_rx = sentry_test_server::server(("127.0.0.1", 8080))?;

    // We need to await at some point otherwise the server doesn't seem to start
    actix_rt::time::sleep(Duration::from_secs(1)).await;

    Command::new("cargo")
        .args(&["run", "--example", "app"])
        .spawn()?
        .wait()?;

    let env = envelope_rx.recv_timeout(Duration::from_secs(2))?;

    if let Ok(json) = sentry_test_server::to_json_pretty(&env) {
        println!("{}", json);
    }

    let item = env
        .items()
        .find(|item| matches!(item, sentry::protocol::EnvelopeItem::Attachment(_)))
        .expect("envelope should have an attachment");

    let attachment = match item {
        sentry::protocol::EnvelopeItem::Attachment(attachment) => attachment,
        _ => unreachable!("envelope should have an attachment"),
    };

    assert!(attachment.filename.ends_with(".dmp"));
    assert_eq!(attachment.ty, Some(AttachmentType::Minidump));
    assert!(attachment.buffer.len() > 10_000);
    assert!(String::from_utf8_lossy(&attachment.buffer).starts_with("MDMP"));

    Ok(())
}
