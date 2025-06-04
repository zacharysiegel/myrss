use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, middleware::Logger};
use anyhow::Result;

#[derive(Clone)]
struct AppState {
    backend_url: String,
}

impl AppState {
    fn from_env() -> Result<Self> {
        Ok(AppState {
            backend_url: std::env::var("MYRSS_BACKEND_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
        })
    }
}

async fn proxy(
    req: HttpRequest,
    body: web::Bytes,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    // Simple pass-through proxy without authentication
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

    // Copy headers except host
    for (name, value) in req.headers() {
        if name != "host" {
            if let Ok(value_str) = value.to_str() {
                backend_req = backend_req.header(name.as_str(), value_str);
            }
        }
    }

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

    log::info!("Starting proxy at http://{}:{}", host, port);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(Logger::default())
            .default_service(web::route().to(proxy))
    })
    .bind((host.as_str(), port))?
    .run()
    .await?;

    Ok(())
}