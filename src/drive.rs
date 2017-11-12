use errors::*;
use hyper;
use hyper::net::HttpsConnector;
use hyper_rustls;
use oauth2;
use oauth2::{Authenticator, DefaultAuthenticatorDelegate, ConsoleApplicationSecret,
             DiskTokenStorage, GetToken, FlowType};
use std::fs::File;
use std;
use reqwest;
use reqwest::mime;
use reqwest::header::{Headers, ContentType};
use serde_json as json;

static UPLOAD_URL: &'static str = "https://www.googleapis.com/upload/drive/v3?uploadType=media?access_token=";

pub fn setup_oauth2() -> oauth2::Token {
    let f = File::open("client_secret.json").expect("Did not find client_secret.json");
    let secret = json::from_reader::<File, ConsoleApplicationSecret>(f)
        .unwrap()
        .installed
        .unwrap();
    let mut cwd = std::env::current_dir().unwrap();
    cwd.push("tk");
    let cwd: String = String::from(cwd.to_str().expect("string conversion error"));
    let ntk = DiskTokenStorage::new(&cwd).expect("disk storage token is broken");

    //let mut core = tokio_core::reactor::Core::new().unwrap();
    //let handle = core.handle();

    //let client = HttpsConnector::new(2,&handle);
    let client = hyper::Client::with_connector(HttpsConnector::new(hyper_rustls::TlsClient::new()));
    let realtk = Authenticator::new(
        &secret,
        DefaultAuthenticatorDelegate,
        client,
        ntk,
        Some(FlowType::InstalledInteractive),
    ).token(&["https://www.googleapis.com/auth/drive.file"]);
    if let Err(e) = realtk {
        panic!("Error in token generation: {:?}", e);
    }
    realtk.unwrap()
}

pub fn upload_file(tk: &oauth2::Token, f: File) -> Result<()> {
    let url = UPLOAD_URL.to_owned() + tk.access_token.as_str();

    let client = reqwest::Client::new();
    let mut header = Headers::new();
    // let mime: mime::Mime = "application/pdf".parse().chain_err(
    //     || "Cannot convert to pdf mime type",
    // )?;
    let mime: mime::Mime = "text/plain".parse().chain_err(|| "Cannot convert to text mime type")?;

    
    header.set(ContentType(mime));
    //header.set_raw("content-lenth", length);

    println!("We got everything done expect sending");



    let res = client
        .post(url.as_str())
        .headers(header)
        .body(f)
        .send()
        .chain_err(|| "Error in uploading");
    println!("client result: {:?}", res);

    Ok(())
}
