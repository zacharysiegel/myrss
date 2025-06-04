use crate::{db, models::Feed};
use anyhow::{Context, Result};
use rss::Channel;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn fetch_and_parse_feed(url: &str) -> Result<Channel> {
    let content = if url.starts_with("http://") || url.starts_with("https://") {
        let response = reqwest::get(url).await?;
        response.bytes().await?
    } else {
        // Treat as raw XML content
        url.as_bytes().to_vec().into()
    };

    let channel = Channel::read_from(&content[..])
        .context("Failed to parse RSS feed")?;
    
    Ok(channel)
}

pub async fn update_feed_items(pool: &PgPool, feed_id: Uuid, channel: &Channel) -> Result<()> {
    // Update feed metadata
    db::update_feed_metadata(
        pool,
        feed_id,
        &channel.title,
        Some(&channel.description),
    )
    .await?;

    // Process items
    for rss_item in &channel.items {
        let guid = rss_item.guid
            .as_ref()
            .map(|g| g.value.clone())
            .or_else(|| rss_item.link.clone())
            .unwrap_or_else(|| {
                format!("{}-{}", 
                    rss_item.title.as_deref().unwrap_or("no-title"),
                    rss_item.pub_date.as_deref().unwrap_or("no-date")
                )
            });

        let pub_date = rss_item.pub_date
            .as_ref()
            .and_then(|date_str| {
                // Try to parse various date formats
                time::OffsetDateTime::parse(date_str, &time::format_description::well_known::Rfc2822).ok()
                    .or_else(|| time::OffsetDateTime::parse(date_str, &time::format_description::well_known::Rfc3339).ok())
            });

        let title = rss_item.title.clone().unwrap_or_else(|| "Untitled".to_string());
        let description = rss_item.description.as_deref();
        let link = rss_item.link.as_deref();
        let author = rss_item.author.as_deref()
            .or_else(|| rss_item.dublin_core_ext.as_ref().and_then(|dc| dc.creators.first().map(|s| s.as_str())));
        let content = rss_item.content.as_deref();

        db::create_or_update_item(
            pool, 
            feed_id, 
            &guid, 
            &title, 
            description, 
            link, 
            pub_date, 
            author, 
            content
        ).await?;
    }

    Ok(())
}

pub async fn fetch_all_user_feeds(pool: &PgPool, user_id: Uuid) -> Result<()> {
    let subscriptions = db::get_user_subscriptions(pool, user_id).await?;
    
    // Get feed URLs for all subscriptions
    let feed_ids: Vec<Uuid> = subscriptions.iter().map(|s| s.feed_id).collect();
    
    // Fetch all feeds
    for feed_id in feed_ids {
        // Get feed info
        let feed_result: Result<Feed> = sqlx::query_as::<_, Feed>(
            "SELECT id, url, title, description, last_fetched, created_at, updated_at FROM feeds WHERE id = $1"
        )
        .bind(feed_id)
        .fetch_one(pool)
        .await
        .map_err(Into::into);
        
        if let Ok(feed) = feed_result {
            match fetch_and_parse_feed(&feed.url).await {
                Ok(channel) => {
                    if let Err(e) = update_feed_items(pool, feed.id, &channel).await {
                        log::error!("Failed to update feed {}: {}", feed.url, e);
                    }
                }
                Err(e) => {
                    log::error!("Failed to fetch feed {}: {}", feed.url, e);
                }
            }
        }
    }
    
    Ok(())
}