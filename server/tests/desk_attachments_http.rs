//! Desk attachments on a real RustFS (DeskAttachmentHttpTest.kt). The tests compare the whole bucket, so they run
//! one at a time.

mod support;

use chrono::Utc;
use reqwest::multipart::{Form, Part};
use serde_json::Value;
use support::*;
use tokio::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::const_new(());

const PNG: &[u8] = &[
    0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0x0D, b'I', b'H', b'D', b'R',
];

fn file(name: &str, bytes: &[u8]) -> Part {
    Part::bytes(bytes.to_vec()).file_name(name.to_owned())
}

fn files(parts: Vec<Part>) -> Form {
    parts
        .into_iter()
        .fold(Form::new(), |form, part| form.part("files", part))
}

async fn agent(app: &TestApp) -> (String, Api) {
    let (_, login) = app.new_user("SALES", PASSWORD).await;
    let api = app.as_user(&login).await;
    let name = api.get("/api/auth/me").await.body["displayName"]
        .as_str()
        .unwrap()
        .to_owned();
    (name, api)
}

#[tokio::test]
async fn an_agent_attaches_files_events_read_model_inline_image_and_downloaded_document() {
    let _serial = SERIAL.lock().await;
    let app = TestApp::start_with_files().await;
    let (name, c) = agent(&app).await;
    let ticket = report_ticket(&c, "MEDIUM", Utc::now(), "Test attachments").await;
    let id = ticket["id"].clone();
    let before = bucket_keys().await;
    let text = "linia 1\nlinia 2\n".as_bytes();

    let r = c
        .multipart(
            &format!("/api/desk/tickets/{id}/attachments"),
            files(vec![
                file("zrzut ekranu.png", PNG),
                file("C:\\logi\\log.txt", text),
            ]),
        )
        .await;
    assert_eq!(r.status, 200, "{:?}", r.body);
    assert_eq!(r.body["attachmentCount"], 2);
    assert_eq!(bucket_keys().await.difference(&before).count(), 2);

    let list = c
        .get(&format!("/api/desk/tickets/{id}/attachments"))
        .await
        .body;
    let mut names: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["fileName"].as_str().unwrap())
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["log.txt", "zrzut ekranu.png"],
        "folders are cut off the name"
    );
    let image = list
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["fileName"] == "zrzut ekranu.png")
        .unwrap();
    assert_eq!(image["contentType"], "image/png");
    assert_eq!(image["inline"], true);
    assert_eq!(image["addedBy"], name.as_str());

    let (status, headers, bytes) = c
        .download(&format!(
            "/api/desk/tickets/{id}/attachments/{}",
            image["id"].as_str().unwrap()
        ))
        .await;
    assert_eq!(status, 200);
    assert_eq!(bytes, PNG);
    assert_eq!(headers["content-type"], "image/png");
    let disposition = headers["content-disposition"].to_str().unwrap();
    assert!(disposition.starts_with("inline"), "{disposition}");
    assert!(
        disposition.contains("filename*=UTF-8''zrzut%20ekranu.png"),
        "{disposition}"
    );
    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert_eq!(headers["content-security-policy"], "sandbox");

    let log = list
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["fileName"] == "log.txt")
        .unwrap();
    let (_, headers, bytes) = c
        .download(&format!(
            "/api/desk/tickets/{id}/attachments/{}",
            log["id"].as_str().unwrap()
        ))
        .await;
    assert!(
        headers["content-disposition"]
            .to_str()
            .unwrap()
            .starts_with("attachment")
    );
    assert_eq!(bytes, text);
    let (_, headers, _) = c
        .download(&format!(
            "/api/desk/tickets/{id}/attachments/{}?download=true",
            image["id"].as_str().unwrap()
        ))
        .await;
    assert!(
        headers["content-disposition"]
            .to_str()
            .unwrap()
            .starts_with("attachment")
    );

    let events = c.get(&format!("/api/desk/tickets/{id}/events")).await.body;
    let added: Vec<Value> = events
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["type"] == "TicketAttachmentAdded")
        .map(|e| serde_json::from_str(e["data"].as_str().unwrap()).unwrap())
        .collect();
    assert_eq!(added.len(), 2);
    assert_eq!(added[0]["addedBy"], name.as_str());
    assert_eq!(added[0]["agent"]["name"], name.as_str());
}

#[tokio::test]
async fn refused_files_svg_html_empty_no_extension_and_nothing_is_stored() {
    let _serial = SERIAL.lock().await;
    let app = TestApp::start_with_files().await;
    let (_, c) = agent(&app).await;
    let ticket = report_ticket(&c, "MEDIUM", Utc::now(), "Test refused").await;
    let path = format!("/api/desk/tickets/{}/attachments", ticket["id"]);
    let before = bucket_keys().await;
    for (name, bytes, number) in [
        ("logo.svg", b"<svg/>".as_slice(), 51121),
        ("strona.html", b"<html/>".as_slice(), 51121),
        ("README", b"x".as_slice(), 51121),
        ("pusty.pdf", b"".as_slice(), 51122),
    ] {
        let r = c.multipart(&path, files(vec![file(name, bytes)])).await;
        assert_eq!(r.status, 400, "{name}");
        assert_eq!(r.body["errorNumber"], number, "{name}");
    }
    let mixed = c
        .multipart(
            &path,
            files(vec![file("ok.png", PNG), file("zly.exe", b"MZ")]),
        )
        .await;
    assert_eq!(mixed.body["errorNumber"], 51121);
    assert_eq!(
        bucket_keys().await,
        before,
        "checked before any byte is stored"
    );
    assert_eq!(
        c.get(&format!("/api/desk/tickets/{}", ticket["id"]))
            .await
            .body["attachmentCount"],
        0
    );
    assert_eq!(
        c.multipart(&path, Form::new().text("x", "y")).await.status,
        400,
        "no files"
    );
}

#[tokio::test]
async fn at_most_10_per_ticket_the_refused_upload_is_deleted_from_the_bucket() {
    let _serial = SERIAL.lock().await;
    let app = TestApp::start_with_files().await;
    let (_, c) = agent(&app).await;
    let ticket = report_ticket(&c, "MEDIUM", Utc::now(), "Test ten").await;
    let path = format!("/api/desk/tickets/{}/attachments", ticket["id"]);
    let eight: Vec<Part> = (1..=8)
        .map(|i| file(&format!("zrzut{i}.png"), PNG))
        .collect();
    assert_eq!(c.multipart(&path, files(eight)).await.status, 200);
    let before = bucket_keys().await;
    let three: Vec<Part> = (1..=3)
        .map(|i| file(&format!("nadmiar{i}.png"), PNG))
        .collect();
    let too_many = c.multipart(&path, files(three)).await;
    assert_eq!(too_many.status, 400);
    assert_eq!(too_many.body["errorNumber"], 51124);
    assert_eq!(
        bucket_keys().await,
        before,
        "files stored before the refusal are deleted again"
    );
    let two: Vec<Part> = (1..=2)
        .map(|i| file(&format!("ostatnie{i}.png"), PNG))
        .collect();
    assert_eq!(c.multipart(&path, files(two)).await.status, 200);
}

#[tokio::test]
async fn the_public_form_sends_the_ticket_and_its_files_in_one_request() {
    let _serial = SERIAL.lock().await;
    let app = TestApp::start_with_files().await;
    let anonymous = app.client();
    anonymous.csrf().await;
    let form = |extra: Option<Part>| {
        let form = Form::new()
            .text("title", "Test z plikiem")
            .text("description", "Zrzut w załączniku")
            .text("categoryKey", "it")
            .text("subcategoryKey", "it.hardware")
            .text("priority", "LOW")
            .text("requesterIncognito", "true");
        match extra {
            Some(part) => form.part("files", part),
            None => form,
        }
    };
    let sent = anonymous
        .multipart(
            "/api/public/desk/tickets",
            form(Some(file("blad.png", PNG))),
        )
        .await;
    assert_eq!(sent.status, 201, "{:?}", sent.body);
    let (_, c) = agent(&app).await;
    let ticket = c
        .get(&format!(
            "/api/desk/tickets/by-number/{}",
            sent.body["number"].as_str().unwrap()
        ))
        .await
        .body;
    assert_eq!(ticket["attachmentCount"], 1);
    let attachments = c
        .get(&format!("/api/desk/tickets/{}/attachments", ticket["id"]))
        .await
        .body;
    assert_eq!(attachments[0]["addedBy"], "incognito");
    let (status, _, _) = anonymous
        .download(&format!(
            "/api/desk/tickets/{}/attachments/{}",
            ticket["id"],
            attachments[0]["id"].as_str().unwrap()
        ))
        .await;
    assert_eq!(status, 401);

    assert_eq!(
        anonymous
            .multipart("/api/public/desk/tickets", form(None))
            .await
            .status,
        201
    );
    let bad = anonymous
        .multipart("/api/public/desk/tickets", form(Some(file("x.exe", b"MZ"))))
        .await;
    assert_eq!(bad.body["errorNumber"], 51121);
}

#[tokio::test]
async fn an_unknown_attachment_is_404() {
    let _serial = SERIAL.lock().await;
    let app = TestApp::start_with_files().await;
    let (_, c) = agent(&app).await;
    let ticket = report_ticket(&c, "MEDIUM", Utc::now(), "Test unknown").await;
    let (status, _, _) = c
        .download(&format!(
            "/api/desk/tickets/{}/attachments/00000000-0000-0000-0000-000000000001",
            ticket["id"]
        ))
        .await;
    assert_eq!(status, 404);
    let (status, _, _) = c
        .download(&format!(
            "/api/desk/tickets/{}/attachments/not-a-uuid",
            ticket["id"]
        ))
        .await;
    assert_eq!(status, 400);
}

#[tokio::test]
async fn readiness_includes_the_bucket() {
    let _serial = SERIAL.lock().await;
    let with = TestApp::start_with_files().await;
    let r = with.client().get("/actuator/health/readiness").await;
    assert_eq!(r.status, 200);
    assert_eq!(r.body["components"]["storage"]["status"], "UP");
    let without = TestApp::start().await;
    let r = without.client().get("/actuator/health/readiness").await;
    assert_eq!(r.status, 503);
    assert_eq!(r.body["components"]["storage"]["status"], "DOWN");
}
