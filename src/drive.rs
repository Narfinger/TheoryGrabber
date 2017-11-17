use errors::*;
use hyper;
use hyper::net::HttpsConnector;
use hyper_rustls;
use oauth2;
use oauth2::{Authenticator, DefaultAuthenticatorDelegate, ConsoleApplicationSecret,
             DiskTokenStorage, GetToken, FlowType};
use std;
use std::collections::HashMap;
use std::fs::File;
use reqwest;
use reqwest::header::{Headers, Authorization, Bearer};
use serde_json as json;
use types::Paper;

static UPLOAD_URL: &'static str = "https://www.googleapis.com/upload/drive/v3/files?uploadType=resumable";

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

fn make_filename(paper: &Paper) -> String {
    "testfile.pdf".to_string()
}

pub fn upload_file(tk: &oauth2::Token, f: File, paper: &Paper) -> Result<()> {
    //getting the proper resumeable session URL
    let client = reqwest::Client::new();
    let mut header = Headers::new();

    header.set(Authorization(Bearer { token: tk.access_token.to_owned() }));

    let filename = make_filename(paper);
    
    let mut metadata = HashMap::new();
    metadata.insert("name", filename);
    metadata.insert("mimeType", "application/pdf".to_string());    
    let query = client.post(UPLOAD_URL).headers(header.clone()).json(&metadata).build();

    
//    println!("{:?}", query);
    let res = client.execute(query.unwrap()).chain_err(
        || "Error in getting resumeable url",
    )?;

//    println!("{:?}", res);
    if res.status().is_success() {
        if let Some(loc) = res.headers().get::<reqwest::header::Location>() {
            client.put(loc.to_string().as_str()).headers(header).body(f).send().chain_err(|| "Error in uploading file to resumeable url")?;
            Ok(())
        } else {
            Err("no location header found".into())
        }
    } else {
        Err("something went wrong with getting resumeable url".into())
    }
}
