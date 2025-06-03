use crate::{db, models::*, rss_fetcher, templates, AppState};
use actix_web::{web, HttpRequest, HttpResponse, Result};
use maud::Markup;
use uuid::Uuid;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/").route(web::get().to(index)))
        .service(web::resource("/feeds").route(web::get().to(feeds)))
        .service(web::resource("/feeds/add").route(web::post().to(add_feed)))
        .service(web::resource("/feeds/{id}/unsubscribe").route(web::post().to(unsubscribe)))
        .service(web::resource("/refresh").route(web::get().to(refresh_feeds)))
        .service(web::resource("/api/items/mark-read").route(web::post().to(mark_read)));
}

async fn get_auth_user(req: &HttpRequest, state: &AppState) -> Result<User> {
    let username = req
        .headers()
        .get(&state.config.auth_header)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Not authenticated"))?;

    let user = db::get_or_create_user(
        &state.db_pool,
        username,
        &format!("{}@myrss.local", username),
    )
    .await
    .map_err(|e| {
        log::error!("Failed to get user: {}", e);
        actix_web::error::ErrorInternalServerError("Failed to get user")
    })?;

    Ok(user)
}

async fn index(req: HttpRequest, state: web::Data<AppState>) -> Result<HttpResponse> {
    let user = get_auth_user(&req, &state).await?;
    
    let page = req
        .match_info()
        .query("page")
        .parse::<i64>()
        .unwrap_or(1)
        .max(1);
    
    let limit = 50;
    let offset = (page - 1) * limit;
    
    let items = db::get_user_items(&state.db_pool, user.id, limit + 1, offset)
        .await
        .map_err(|e| {
            log::error!("Failed to get items: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to get items")
        })?;
    
    let has_more = items.len() > limit as usize;
    let items: Vec<_> = items.into_iter().take(limit as usize).collect();
    
    let html: Markup = templates::home_page(&user.username, &items, has_more, page);
    Ok(HttpResponse::Ok().content_type("text/html").body(html.into_string()))
}

async fn feeds(req: HttpRequest, state: web::Data<AppState>) -> Result<HttpResponse> {
    let user = get_auth_user(&req, &state).await?;
    
    let subscriptions = db::get_user_subscriptions(&state.db_pool, user.id)
        .await
        .map_err(|e| {
            log::error!("Failed to get subscriptions: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to get subscriptions")
        })?;
    
    let html: Markup = templates::feeds_page(&user.username, &subscriptions);
    Ok(HttpResponse::Ok().content_type("text/html").body(html.into_string()))
}

async fn add_feed(
    req: HttpRequest,
    state: web::Data<AppState>,
    form: web::Form<AddFeedRequest>,
) -> Result<HttpResponse> {
    let user = get_auth_user(&req, &state).await?;
    
    let feed_content = if let Some(url) = &form.url {
        if !url.is_empty() {
            url.clone()
        } else if let Some(content) = &form.content {
            content.clone()
        } else {
            return Ok(HttpResponse::BadRequest().body("Please provide a URL or RSS content"));
        }
    } else if let Some(content) = &form.content {
        content.clone()
    } else {
        return Ok(HttpResponse::BadRequest().body("Please provide a URL or RSS content"));
    };
    
    // Try to parse the feed first
    let channel = rss_fetcher::fetch_and_parse_feed(&feed_content)
        .await
        .map_err(|e| {
            log::error!("Failed to parse feed: {}", e);
            actix_web::error::ErrorBadRequest(format!("Failed to parse feed: {}", e))
        })?;
    
    // Create or get the feed
    let feed_url = form.url.as_deref().unwrap_or(&feed_content);
    let feed = db::create_or_get_feed(&state.db_pool, feed_url)
        .await
        .map_err(|e| {
            log::error!("Failed to create feed: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to create feed")
        })?;
    
    // Update feed items
    rss_fetcher::update_feed_items(&state.db_pool, feed.id, &channel)
        .await
        .map_err(|e| {
            log::error!("Failed to update feed items: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to update feed items")
        })?;
    
    // Subscribe the user to the feed
    db::subscribe_to_feed(&state.db_pool, user.id, feed.id, form.folder.clone())
        .await
        .map_err(|e| {
            log::error!("Failed to subscribe to feed: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to subscribe to feed")
        })?;
    
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/feeds"))
        .finish())
}

async fn unsubscribe(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let user = get_auth_user(&req, &state).await?;
    let feed_id = path.into_inner();
    
    db::unsubscribe_from_feed(&state.db_pool, user.id, feed_id)
        .await
        .map_err(|e| {
            log::error!("Failed to unsubscribe: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to unsubscribe")
        })?;
    
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/feeds"))
        .finish())
}

async fn refresh_feeds(req: HttpRequest, state: web::Data<AppState>) -> Result<HttpResponse> {
    let user = get_auth_user(&req, &state).await?;
    
    // In production, this should be done in a background job
    rss_fetcher::fetch_all_user_feeds(&state.db_pool, user.id)
        .await
        .map_err(|e| {
            log::error!("Failed to refresh feeds: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to refresh feeds")
        })?;
    
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/"))
        .finish())
}

async fn mark_read(
    req: HttpRequest,
    state: web::Data<AppState>,
    json: web::Json<MarkReadRequest>,
) -> Result<HttpResponse> {
    let user = get_auth_user(&req, &state).await?;
    
    db::mark_items_read(&state.db_pool, user.id, &json.item_ids)
        .await
        .map_err(|e| {
            log::error!("Failed to mark items as read: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to mark items as read")
        })?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true
    })))
}