use crate::{auth, db, models::*, rss_fetcher, templates, AppState};
use actix_session::Session;
use actix_web::{web, HttpRequest, HttpResponse, Result};
use maud::Markup;
use uuid::Uuid;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/").route(web::get().to(index)))
        .service(web::resource("/login").route(web::get().to(auth::login_page)).route(web::post().to(auth::login)))
        .service(web::resource("/register").route(web::get().to(auth::register_page)).route(web::post().to(auth::register)))
        .service(web::resource("/logout").route(web::post().to(auth::logout)))
        .service(web::resource("/feeds").route(web::get().to(feeds)))
        .service(web::resource("/feeds/add").route(web::post().to(add_feed)))
        .service(web::resource("/feeds/{id}/labels").route(web::post().to(update_feed_labels)))
        .service(web::resource("/feeds/{id}/unsubscribe").route(web::post().to(unsubscribe)))
        .service(web::resource("/labels").route(web::get().to(manage_labels)))
        .service(web::resource("/labels/add").route(web::post().to(add_label)))
        .service(web::resource("/labels/{id}/delete").route(web::post().to(delete_label)))
        .service(web::resource("/refresh").route(web::get().to(refresh_feeds)))
        .service(web::resource("/api/items/mark-read").route(web::post().to(mark_read)));
}

async fn require_auth(session: &Session, state: &AppState) -> Result<User> {
    match auth::get_current_user(session, state).await {
        Some(user) => Ok(user),
        None => {
            let response = HttpResponse::SeeOther()
                .append_header(("Location", "/login"))
                .finish();
            Err(actix_web::error::InternalError::from_response("", response).into())
        }
    }
}

async fn index(session: Session, state: web::Data<AppState>, req: HttpRequest) -> Result<HttpResponse> {
    let user = require_auth(&session, &state).await?;
    
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

async fn feeds(session: Session, state: web::Data<AppState>) -> Result<HttpResponse> {
    let user = require_auth(&session, &state).await?;
    
    let subscriptions = db::get_user_subscriptions_with_labels(&state.db_pool, user.id)
        .await
        .map_err(|e| {
            log::error!("Failed to get subscriptions: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to get subscriptions")
        })?;
    
    let labels = db::get_user_labels(&state.db_pool, user.id)
        .await
        .map_err(|e| {
            log::error!("Failed to get labels: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to get labels")
        })?;
    
    let html: Markup = templates::feeds_page(&user.username, &subscriptions, &labels);
    Ok(HttpResponse::Ok().content_type("text/html").body(html.into_string()))
}

async fn add_feed(
    session: Session,
    state: web::Data<AppState>,
    form: web::Form<AddFeedRequest>,
) -> Result<HttpResponse> {
    let user = require_auth(&session, &state).await?;
    
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
    let subscription = db::subscribe_to_feed(&state.db_pool, user.id, feed.id)
        .await
        .map_err(|e| {
            log::error!("Failed to subscribe to feed: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to subscribe to feed")
        })?;
    
    // Add labels if provided
    if let Some(label_names) = &form.labels {
        for label_name in label_names {
            if let Ok(label) = db::get_or_create_label(&state.db_pool, user.id, label_name).await {
                let _ = db::add_label_to_subscription(&state.db_pool, subscription.id, label.id).await;
            }
        }
    }
    
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/feeds"))
        .finish())
}

async fn update_feed_labels(
    session: Session,
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    form: web::Form<UpdateFeedLabelsRequest>,
) -> Result<HttpResponse> {
    let user = require_auth(&session, &state).await?;
    let subscription_id = path.into_inner();
    
    // Verify the subscription belongs to the user
    if !db::user_owns_subscription(&state.db_pool, user.id, subscription_id).await
        .map_err(|e| {
            log::error!("Failed to check subscription ownership: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to check subscription ownership")
        })? {
        return Err(actix_web::error::ErrorForbidden("Access denied"));
    }
    
    // Clear existing labels
    db::clear_subscription_labels(&state.db_pool, subscription_id).await
        .map_err(|e| {
            log::error!("Failed to clear subscription labels: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to clear labels")
        })?;
    
    // Add new labels
    for label_name in &form.labels {
        if let Ok(label) = db::get_or_create_label(&state.db_pool, user.id, label_name).await {
            let _ = db::add_label_to_subscription(&state.db_pool, subscription_id, label.id).await;
        }
    }
    
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/feeds"))
        .finish())
}

async fn unsubscribe(
    session: Session,
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let user = require_auth(&session, &state).await?;
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

async fn manage_labels(session: Session, state: web::Data<AppState>) -> Result<HttpResponse> {
    let user = require_auth(&session, &state).await?;
    
    let labels = db::get_user_labels(&state.db_pool, user.id)
        .await
        .map_err(|e| {
            log::error!("Failed to get labels: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to get labels")
        })?;
    
    let html: Markup = templates::labels_page(&user.username, &labels);
    Ok(HttpResponse::Ok().content_type("text/html").body(html.into_string()))
}

async fn add_label(
    session: Session,
    state: web::Data<AppState>,
    form: web::Form<serde_json::Value>,
) -> Result<HttpResponse> {
    let user = require_auth(&session, &state).await?;
    
    if let Some(name) = form.get("name").and_then(|v| v.as_str()) {
        let color = form.get("color").and_then(|v| v.as_str()).unwrap_or("#3b82f6");
        
        db::create_label(&state.db_pool, user.id, name, color)
            .await
            .map_err(|e| {
                log::error!("Failed to create label: {}", e);
                actix_web::error::ErrorInternalServerError("Failed to create label")
            })?;
    }
    
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/labels"))
        .finish())
}

async fn delete_label(
    session: Session,
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let user = require_auth(&session, &state).await?;
    let label_id = path.into_inner();
    
    db::delete_user_label(&state.db_pool, user.id, label_id)
        .await
        .map_err(|e| {
            log::error!("Failed to delete label: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to delete label")
        })?;
    
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/labels"))
        .finish())
}

async fn refresh_feeds(session: Session, state: web::Data<AppState>) -> Result<HttpResponse> {
    let user = require_auth(&session, &state).await?;
    
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
    session: Session,
    state: web::Data<AppState>,
    json: web::Json<MarkReadRequest>,
) -> Result<HttpResponse> {
    let user = require_auth(&session, &state).await?;
    
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