use hyper;
use hyper::net::HttpsConnector;
use hyper_rustls;
use oauth2;
use oauth2::{Authenticator, DefaultAuthenticatorDelegate, ConsoleApplicationSecret,
             DiskTokenStorage, GetToken, FlowType};
use std::fs::File;
use std;
use serde_json as json;
use tokio_core;

pub fn setup_oauth() -> oauth2::Token {
    let f = File::open("client_secret.json").expect("Did not find client_secret.json");
    let secret = json::from_reader::<File,ConsoleApplicationSecret>(f).unwrap().installed.unwrap();
    let mut cwd = std::env::current_dir().unwrap();
    cwd.push("tk");
    let cwd: String = String::from(cwd.to_str().expect("string conversion error"));
    let ntk = DiskTokenStorage::new(&cwd).expect("disk storage token is broken");

    //let mut core = tokio_core::reactor::Core::new().unwrap();
    //let handle = core.handle();

    //let client = HttpsConnector::new(2,&handle);
    let client = hyper::Client::with_connector(HttpsConnector::new(hyper_rustls::TlsClient::new()));
    let realtk = Authenticator::new(&secret,
                                    DefaultAuthenticatorDelegate,
                                    client,
                                    ntk,
                                    Some(FlowType::InstalledInteractive))
        .token(&["https://www.googleapis.com/auth/drive3"]);
    if let Err(e) = realtk {
        panic!("Error in token generation: {:?}", e);
    }
    realtk.unwrap()
}
