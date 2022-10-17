use actix_web::{
    post,
    web::{self, Bytes},
    App, HttpResponse, HttpServer, Responder,
};
use crossbeam_channel::{Receiver, Sender};
use sentry::protocol::{AttachmentType, Envelope};
use std::{net, process::Command, time::Duration};

struct AppState {
    envelope_tx: Sender<Envelope>,
}

#[post("/api/{_project_id}/envelope/")]
async fn envelope(
    _project_id: web::Path<String>,
    req_body: Bytes,
    state: web::Data<AppState>,
) -> impl Responder {
    state
        .envelope_tx
        .send(Envelope::from_slice(&req_body).expect("invalid envelope"))
        .expect("could not send envelope");

    HttpResponse::Ok()
}

pub fn server<A>(address: A) -> std::io::Result<Receiver<Envelope>>
where
    A: net::ToSocketAddrs,
{
    let (envelope_tx, envelope_rx) = crossbeam_channel::bounded(1);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                envelope_tx: envelope_tx.clone(),
            }))
            .route("/", web::to(HttpResponse::Ok))
            .service(envelope)
    })
    .bind(address)?
    .run();

    actix_rt::spawn(async move { server.await });

    Ok(envelope_rx)
}

#[actix_rt::test]
async fn test_example_app() -> Result<(), Box<dyn std::error::Error>> {
    let envelope_rx = server(("127.0.0.1", 8080))?;

    // We need to await at some point otherwise the server doesn't seem to start
    actix_rt::time::sleep(Duration::from_secs(1)).await;

    Command::new("cargo")
        .args(&["run", "--example", "app"])
        .spawn()?
        .wait()?;

    let env = envelope_rx.recv_timeout(Duration::from_secs(2))?;

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
