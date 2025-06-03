use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, middleware::Logger};
use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use myrss_secrets::SecretsReader;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    username: String,
    password_hash: String,
}

#[derive(Clone)]
struct AppState {
    users: HashMap<String, User>,
    backend_url: String,
    auth_header: String,
}

impl AppState {
    fn from_env() -> Result<Self> {
        let secrets_file = std::env::var("MYRSS_SECRETS_FILE")
            .unwrap_or_else(|_| "secrets.yaml".to_string());
        let master_password = std::env::var("MYRSS_MASTER_PASSWORD")
            .expect("MYRSS_MASTER_PASSWORD must be set");

        let secrets = SecretsReader::new(&secrets_file, master_password)?;
        
        // Load users from secrets
        let users_json = secrets.get("auth_users")?;
        let users: Vec<User> = serde_json::from_str(&users_json)?;
        let users_map: HashMap<String, User> = users
            .into_iter()
            .map(|u| (u.username.clone(), u))
            .collect();

        Ok(AppState {
            users: users_map,
            backend_url: std::env::var("MYRSS_BACKEND_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            auth_header: std::env::var("MYRSS_AUTH_HEADER")
                .unwrap_or_else(|_| "X-Authenticated-User".to_string()),
        })
    }
}

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn extract_basic_auth(req: &HttpRequest) -> Option<(String, String)> {
    req.headers()
        .get("Authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Basic ")?
        .parse::<String>()
        .ok()
        .and_then(|encoded| {
            BASE64.decode(encoded).ok()
        })
        .and_then(|decoded| {
            String::from_utf8(decoded).ok()
        })
        .and_then(|credentials| {
            let parts: Vec<&str> = credentials.splitn(2, ':').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
}

async fn auth_proxy(
    req: HttpRequest,
    body: web::Bytes,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    // Check for basic auth
    let (username, password) = match extract_basic_auth(&req) {
        Some(creds) => creds,
        None => {
            return Ok(HttpResponse::Unauthorized()
                .append_header(("WWW-Authenticate", "Basic realm=\"MyRSS\""))
                .body("Authentication required"));
        }
    };

    // Verify credentials
    let user = match state.users.get(&username) {
        Some(user) if user.password_hash == hash_password(&password) => user,
        _ => {
            return Ok(HttpResponse::Unauthorized()
                .append_header(("WWW-Authenticate", "Basic realm=\"MyRSS\""))
                .body("Invalid credentials"));
        }
    };

    // Forward request to backend with auth header
    let client = reqwest::Client::new();
    let method = req.method();
    let path = req.uri().path();
    let query = req.uri().query().unwrap_or("");
    
    let url = if query.is_empty() {
        format!("{}{}", state.backend_url, path)
    } else {
        format!("{}{}?{}", state.backend_url, path, query)
    };

    // Convert actix Method to reqwest Method
    let req_method = match method.as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "HEAD" => reqwest::Method::HEAD,
        "OPTIONS" => reqwest::Method::OPTIONS,
        "PATCH" => reqwest::Method::PATCH,
        _ => return Err(actix_web::error::ErrorBadRequest("Unsupported method")),
    };

    let mut backend_req = client.request(req_method, &url);

    // Copy headers except Authorization
    for (name, value) in req.headers() {
        if name != "authorization" && name != "host" {
            if let Ok(value_str) = value.to_str() {
                backend_req = backend_req.header(name.as_str(), value_str);
            }
        }
    }

    // Add authenticated user header
    backend_req = backend_req.header(&state.auth_header, &user.username);

    // Add body if present
    if !body.is_empty() {
        backend_req = backend_req.body(body.to_vec());
    }

    // Send request
    let backend_resp = backend_req.send().await
        .map_err(|e| {
            log::error!("Backend request failed: {}", e);
            actix_web::error::ErrorBadGateway("Backend unavailable")
        })?;

    // Build response
    let status = backend_resp.status();
    let mut resp = HttpResponse::build(actix_web::http::StatusCode::from_u16(status.as_u16()).unwrap());

    // Copy response headers
    for (name, value) in backend_resp.headers() {
        if let Ok(value_str) = value.to_str() {
            resp.append_header((name.as_str(), value_str));
        }
    }

    let body = backend_resp.bytes().await
        .map_err(|e| {
            log::error!("Failed to read backend response: {}", e);
            actix_web::error::ErrorBadGateway("Failed to read response")
        })?;

    Ok(resp.body(body))
}

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let state = AppState::from_env()?;
    let host = std::env::var("MYRSS_AUTH_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("MYRSS_AUTH_PORT")
        .unwrap_or_else(|_| "8081".to_string())
        .parse()?;

    log::info!("Starting auth proxy at http://{}:{}", host, port);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(Logger::default())
            .default_service(web::route().to(auth_proxy))
    })
    .bind((host.as_str(), port))?
    .run()
    .await?;

    Ok(())
}