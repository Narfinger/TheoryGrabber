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
use config::write_directory_id;
use types::Paper;

static UPLOAD_URL: &'static str = "https://www.googleapis.com/upload/drive/v3/files?uploadType=resumable";
static DIRECTORY_URL: &'static str = "https://www.googleapis.com/drive/v3/files";
static DIRECTORY_NAME: &'static str = "TheoryGrabber";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileCreateResponse {
    kind: String,
    id: String,
    name: String,
    mime_type: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileUploadJSON {
    name: String,
    mime_type: String,
    parents: Vec<String>
} 
    
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

fn get_last_name_initials(author: &str) -> char {
    let lastname = author.split_whitespace().nth(1).expect(
        "No lastname found?",
    ); //lastname
    lastname.chars().next().unwrap()
}

fn author_string(paper: &Paper) -> String {
    paper
        .authors
        .iter()
        .fold(String::from(""), |acc, i| {
            acc + &get_last_name_initials(i).to_string()
        })
        .to_uppercase()
}

fn make_filename(paper: &Paper) -> String {
    let datestring = paper.published.format("%Y-%m-%d");
    let mut title = paper.title.clone();
    title.truncate(75);

    datestring.to_string() + "-" + &author_string(paper) + "-" + &title + ".pdf"
}

//do progress bar

pub fn create_directory(tk: &oauth2::Token) -> Result<()> {
    let client = reqwest::Client::new();
    let mut header = Headers::new();

    header.set(Authorization(Bearer { token: tk.access_token.to_owned() }));
    let mut metadata = HashMap::new();
    metadata.insert("name", DIRECTORY_NAME);
    metadata.insert("mimeType", "application/vnd.google-apps.folder");

    let mut res = client
        .post(DIRECTORY_URL)
        .headers(header.clone())
        .json(&metadata)
        .send().chain_err(|| "Error in sending to create directory")?;
    
    let response: FileCreateResponse = res.json().chain_err(|| "Error in decoding Response")?;
    write_directory_id(response.id)?;

    Ok(())
}

pub fn upload_file(tk: &oauth2::Token, f: File, paper: &Paper, fileid: String) -> Result<()> {
    //getting the proper resumeable session URL
    let client = reqwest::Client::new();
    let mut header = Headers::new();

    header.set(Authorization(Bearer { token: tk.access_token.to_owned() }));

    let filename = make_filename(paper);

    let metadata = FileUploadJSON {
        name: filename,
        mime_type: "application/pdf".to_string(),
        parents: vec![fileid],
    };
    
    let query = client
        .post(UPLOAD_URL)
        .headers(header.clone())
        .json(&metadata)
        .build();

    let res = client.execute(query.unwrap()).chain_err(
        || "Error in getting resumeable url",
    )?;

    if res.status().is_success() {
        if let Some(loc) = res.headers().get::<reqwest::header::Location>() {
            client
                .put(&loc.to_string())
                .headers(header)
                .body(f)
                .send()
                .chain_err(|| "Error in uploading file to resumeable url")?;
            Ok(())
        } else {
            Err("no location header found".into())
        }
    } else {
        Err("something went wrong with getting resumeable url".into())
    }
}
