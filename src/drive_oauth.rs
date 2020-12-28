use crate::types::APP_INFO;
use anyhow::Result;
use app_dirs::*;
use chrono::{DateTime, Utc};
use oauth2::reqwest::http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use serde_json as json;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::Duration;

/// Represents a token and the date it was created
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Token {
    pub tk: oauth2::basic::BasicTokenResponse,
    created: DateTime<Utc>,
}

pub trait Expireing {
    fn expired(&self) -> bool;
}

impl Expireing for Token {
    fn expired(&self) -> bool {
        let dur = Duration::new(self.tk.expires_in().map(|s| s.as_secs()).unwrap_or(0), 0);
        Utc::now()
            .checked_add_signed(chrono::Duration::from_std(dur).unwrap())
            .map(|d| d >= self.created)
            .unwrap_or(true)
        //we assume seconds here because I do not know what it is in
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
fn authorize() -> Result<oauth2::basic::BasicTokenResponse> {
    let secret = get_client_secrets();
    let config = oauth2::basic::BasicClient::new(
        ClientId::new(secret.client_id),
        Some(ClientSecret::new(secret.client_secret)),
        AuthUrl::new(secret.auth_uri).unwrap(),
        Some(TokenUrl::new(secret.token_uri).unwrap()),
    )
    .set_redirect_url(RedirectUrl::new("http://localhost:8080".to_string()).unwrap());
    let (authorize_url, _) = config
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/drive.file".to_string(),
        ))
        .url();
    println!(
        "Open this URL in your browser:\n{}\n",
        authorize_url.as_str()
    );
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
                let url =
                    url::Url::parse(&("http://localhost".to_string() + redirect_url)).unwrap();

                let code_pair = url
                    .query_pairs()
                    .find(|pair| {
                        let &(ref key, _) = pair;
                        key == "code"
                    })
                    .unwrap();

                let (_, value) = code_pair;
                code = value.into_owned();
            }
            let response = "HTTP/1.1 301 Moved Permanently\r\n Location: http://localhost:8000";
            s.write_all(response.as_bytes()).unwrap();

            // The server will terminate itself after collecting the first code.
            break;
        }
    }
    config
        .exchange_code(AuthorizationCode::new(code))
        .request(http_client)
        .map_err(|_| anyhow!("Some authorization error"))
}

/// Refreshes a token with the saved (`oldtocken`) refresh token
fn refresh(oldtoken: &Token) -> Result<Token> {
    let secret = get_client_secrets();
    let params = [
        (
            "refresh_token",
            oldtoken.tk.refresh_token().unwrap().secret().clone(),
        ),
        ("client_id", secret.client_id),
        ("client_secret", secret.client_secret),
        ("grant_type", String::from("refresh_token")),
    ];
    let client = reqwest::blocking::Client::new();
    let res = client
        .post("https://www.googleapis.com/oauth2/v4/token")
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
    newtk.set_access_token(oauth2::AccessToken::new(new_response.access_token));
    let expires: u64 = new_response.expires_in.into();
    newtk.set_expires_in(Some(&Duration::from_secs(expires)));
    Ok(Token {
        created: Utc::now(),
        tk: newtk,
    })
}

/// Gets the oauth2 token, either saved in tk.json or creates a new. if not current, refreshes it
pub fn setup_oauth() -> Result<oauth2::basic::BasicTokenResponse> {
    let mut path = app_root(AppDataType::UserConfig, &APP_INFO).expect("Error in app dir");
    path.push("tk.json");
    let f = File::open(&path);
    let tk = if let Ok(f7) = f {
        json::from_reader(f7)
    } else {
        let f = File::create(&path)?;
        let tk = Token {
            tk: authorize().unwrap(),
            created: Utc::now(),
        };
        json::to_writer(f, &tk).expect("Could not write token");
        Ok(tk)
    }?;
    if tk.expired() {
        refresh(&tk).map(|s| s.tk)
    } else {
        Ok(tk.tk)
    }
}
