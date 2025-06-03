use crate::{db, models::Item};
use anyhow::{Context, Result};
use rss::Channel;
use sqlx::PgPool;
use time::OffsetDateTime;
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
        Some(channel.title.clone()),
        Some(channel.description.clone()),
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

        let item = Item {
            id: Uuid::new_v4(),
            feed_id,
            guid,
            title: rss_item.title.clone().unwrap_or_else(|| "Untitled".to_string()),
            description: rss_item.description.clone(),
            link: rss_item.link.clone(),
            pub_date,
            author: rss_item.author.clone()
                .or_else(|| rss_item.dublin_core_ext.as_ref().and_then(|dc| dc.creators.first().cloned())),
            content: rss_item.content.clone(),
            created_at: OffsetDateTime::now_utc(),
        };

        db::create_or_update_item(pool, &item).await?;
    }

    Ok(())
}

pub async fn fetch_all_user_feeds(pool: &PgPool, user_id: Uuid) -> Result<()> {
    let subscriptions = db::get_user_subscriptions(pool, user_id).await?;
    
    for (_, feed) in subscriptions {
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
    
    Ok(())
}