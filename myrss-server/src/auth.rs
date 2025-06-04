use crate::{db, models::*, templates, AppState};
use actix_session::Session;
use actix_web::{web, HttpResponse, Result};
use maud::Markup;
use serde::Deserialize;
use sha2::{Sha256, Digest};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

#[derive(Deserialize)]
pub struct RegisterForm {
    username: String,
    email: String,
    password: String,
    password_confirm: String,
}

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub async fn login_page() -> Result<HttpResponse> {
    let html: Markup = templates::login_page();
    Ok(HttpResponse::Ok().content_type("text/html").body(html.into_string()))
}

pub async fn register_page() -> Result<HttpResponse> {
    let html: Markup = templates::register_page();
    Ok(HttpResponse::Ok().content_type("text/html").body(html.into_string()))
}

pub async fn login(
    session: Session,
    state: web::Data<AppState>,
    form: web::Form<LoginForm>,
) -> Result<HttpResponse> {
    let password_hash = hash_password(&form.password);
    
    match db::authenticate_user(&state.db_pool, &form.username, &password_hash).await {
        Ok(user) => {
            session.insert("user_id", user.id)?;
            session.insert("username", user.username)?;
            Ok(HttpResponse::SeeOther()
                .append_header(("Location", "/"))
                .finish())
        }
        Err(_) => {
            let html: Markup = templates::login_page_with_error("Invalid username or password");
            Ok(HttpResponse::Ok().content_type("text/html").body(html.into_string()))
        }
    }
}

pub async fn register(
    session: Session,
    state: web::Data<AppState>,
    form: web::Form<RegisterForm>,
) -> Result<HttpResponse> {
    if form.password != form.password_confirm {
        let html: Markup = templates::register_page_with_error("Passwords do not match");
        return Ok(HttpResponse::Ok().content_type("text/html").body(html.into_string()));
    }
    
    if form.username.is_empty() || form.password.is_empty() {
        let html: Markup = templates::register_page_with_error("Username and password are required");
        return Ok(HttpResponse::Ok().content_type("text/html").body(html.into_string()));
    }
    
    let password_hash = hash_password(&form.password);
    
    match db::create_user(&state.db_pool, &form.username, &form.email, &password_hash).await {
        Ok(user) => {
            session.insert("user_id", user.id)?;
            session.insert("username", user.username)?;
            Ok(HttpResponse::SeeOther()
                .append_header(("Location", "/"))
                .finish())
        }
        Err(e) => {
            log::error!("Failed to create user: {}", e);
            let error_msg = if e.to_string().contains("duplicate key") {
                "Username already exists"
            } else {
                "Failed to create account"
            };
            let html: Markup = templates::register_page_with_error(error_msg);
            Ok(HttpResponse::Ok().content_type("text/html").body(html.into_string()))
        }
    }
}

pub async fn logout(session: Session) -> Result<HttpResponse> {
    session.purge();
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/login"))
        .finish())
}

pub async fn get_current_user(session: &Session, state: &AppState) -> Option<User> {
    let user_id = session.get::<Uuid>("user_id").ok()??;
    db::get_user_by_id(&state.db_pool, user_id).await.ok()
}