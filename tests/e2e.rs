use sentry::protocol::{AttachmentType, EnvelopeItem};
use std::{error::Error, process::Command, time::Duration};

#[actix_rt::test]
async fn test_example_app() -> Result<(), Box<dyn Error>> {
    let envelope_rx = sentry_test_server::server(("127.0.0.1", 8123))?;

    // We need to await at some point otherwise the server doesn't seem to start
    actix_rt::time::sleep(Duration::from_secs(2)).await;

    Command::new("cargo")
        .args(["run", "--example", "app", "--all-features"])
        .spawn()?
        .wait()?;

    let env = envelope_rx.recv_timeout(Duration::from_secs(15))?;

    if let Ok(json) = sentry_test_server::to_json_pretty(&env) {
        println!("{}", json);
    }

    let env_item = env
        .items()
        .find(|item| matches!(item, EnvelopeItem::Event(_)))
        .expect("envelope should have an event");

    let event = match env_item {
        EnvelopeItem::Event(event) => event.clone(),
        _ => unreachable!("envelope should have an event"),
    };

    let user = event.user.expect("event should have a user");

    assert_eq!(user.email, Some("john@doe.town".into()));
    assert_eq!(user.username, Some("john_doe".into()));

    let env_item = env
        .items()
        .find(|item| matches!(item, EnvelopeItem::Attachment(_)))
        .expect("envelope should have an attachment");

    let attachment = match env_item {
        EnvelopeItem::Attachment(attachment) => attachment,
        _ => unreachable!("envelope should have an attachment"),
    };

    assert!(attachment.filename.ends_with(".dmp"));
    assert_eq!(attachment.ty, Some(AttachmentType::Minidump));
    assert!(attachment.buffer.len() > 10_000);
    assert!(String::from_utf8_lossy(&attachment.buffer[..20]).starts_with("MDMP"));

    Ok(())
}
