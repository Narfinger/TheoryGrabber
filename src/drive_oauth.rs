use crate::errors::*;
use serde_json as json;
use oauth2;
use url::Url;
use chrono::{DateTime,Duration, Utc};
use reqwest;
use std::fs::File;
use std::net::TcpListener;
use std::io::{BufRead, BufReader, Write};
use app_dirs::*;
use crate::types::APP_INFO;

/// Represents a token and the date it was created
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Token {
    pub tk: oauth2::Token,
    created: DateTime<Utc>,
}

pub trait Expireing {
    fn expired(&self) -> bool;
}

impl Expireing for Token {
    fn expired(&self) -> bool {
        Utc::now() >= self.created.checked_add_signed(
            //we assume seconds here because I do not know what it is in
            Duration::seconds(i64::from(self.tk.expires_in.unwrap_or(0)))).unwrap()
    }
}

#[derive(Deserialize)]
struct Installed {
    client_id: String,
    auth_uri: String,
    token_uri: String,
    client_secret: String,
}

fn get_client_secrets() -> Installed {
    #[derive(Deserialize)]
    struct ClientSecret {
        installed: Installed,
    };
    let f = include_str!("../client_secret.json");
    json::from_str::<ClientSecret>(f).unwrap().installed
}

/// Initital authorization to get the token
fn authorize() -> Result<oauth2::Token> {
    let secret = get_client_secrets();
    let mut config = oauth2::Config::new(secret.client_id, secret.client_secret, secret.auth_uri, secret.token_uri);
    config = config.add_scope("https://www.googleapis.com/auth/drive.file");
    config = config.set_redirect_url("http://localhost:8080");
    let authorize_url = config.authorize_url();
    println!("Open this URL in your browser:\n{}\n", authorize_url.to_string());
    let mut code = String::new();

    // A very naive implementation of the redirect server.
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    for stream in listener.incoming() {
        if let Ok(mut s) = stream {
            {
                let mut reader = BufReader::new(&s);

                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();

                let redirect_url = request_line.split_whitespace().nth(1).unwrap();
                let url = Url::parse(&("http://localhost".to_string() + redirect_url)).unwrap();

                let code_pair = url.query_pairs().find(|pair| {
                    let &(ref key, _) = pair;
                    key == "code"
                }).unwrap();

                let (_, value) = code_pair;
                code = value.into_owned();
            }
            let response = "HTTP/1.1 301 Moved Permanently\r\n Location: http://localhost:8000";
            s.write_all(response.as_bytes()).unwrap();

            // The server will terminate itself after collecting the first code.
            break;
            }
        };
    config.exchange_code(code).map_err(std::convert::Into::into)
}

/// Refreshes a token with the saved (`oldtocken`) refresh token
fn refresh(oldtoken: &Token) -> Result<Token> {
    let secret = get_client_secrets();
    let params = [("refresh_token", oldtoken.tk.refresh_token.clone().unwrap()),
                    ("client_id", secret.client_id),
                    ("client_secret", secret.client_secret),
                    ("grant_type", String::from("refresh_token"))];
    let client = reqwest::Client::new();
    let res = client.post("https://www.googleapis.com/oauth2/v4/token")
        .form(&params)
        .send()?;
    #[derive(Deserialize)]
    struct Response {
        access_token: String,
        expires_in: u32,
    }
    let new_response: Response = json::from_reader(res)?;

    //changing to new token, take the old one as a copy
    let mut newtk = oldtoken.tk.clone();
    newtk.access_token = new_response.access_token;
    newtk.expires_in = Some(new_response.expires_in);
    Ok(Token { created: Utc::now(), tk: newtk })
}

/// Gets the oauth2 token, either saved in tk.json or creates a new. if not current, refreshes it
pub fn setup_oauth() -> Result<oauth2::Token> {
    let mut path = app_root(AppDataType::UserConfig, &APP_INFO).expect("Error in app dir");
    path.push("tk.json");
    let f = File::open(&path);
    let tk = if let Ok(f7) = f {
        json::from_reader(f7)
    }
    else {
        let f = File::create(&path)?;
        let tk = Token{ tk: authorize().unwrap(), created: Utc::now()};
        json::to_writer(f, &tk).expect("Could not write token");
        Ok(tk)
    }?;
    if tk.expired() {
        refresh(&tk).map(|s| s.tk)
    } else {
        Ok(tk.tk)
    }
}