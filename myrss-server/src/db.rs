use crate::models::*;
use anyhow::Result;
use sqlx::{PgPool, postgres::PgQueryResult, Row};
use uuid::Uuid;

pub async fn get_or_create_user(pool: &PgPool, username: &str, email: &str) -> Result<User> {
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (username, email)
        VALUES ($1, $2)
        ON CONFLICT (username) DO UPDATE SET email = EXCLUDED.email
        RETURNING id, username, email, created_at, updated_at
        "#
    )
    .bind(username)
    .bind(email)
    .fetch_one(pool)
    .await?;
    
    Ok(user)
}

pub async fn get_user_by_username(pool: &PgPool, username: &str) -> Result<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        r#"SELECT id, username, email, created_at, updated_at FROM users WHERE username = $1"#
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    
    Ok(user)
}

pub async fn create_or_get_feed(pool: &PgPool, url: &str) -> Result<Feed> {
    let feed = sqlx::query_as::<_, Feed>(
        r#"
        INSERT INTO feeds (url)
        VALUES ($1)
        ON CONFLICT (url) DO UPDATE SET url = EXCLUDED.url
        RETURNING id, url, title, description, last_fetched, created_at, updated_at
        "#
    )
    .bind(url)
    .fetch_one(pool)
    .await?;
    
    Ok(feed)
}

pub async fn update_feed_metadata(
    pool: &PgPool,
    feed_id: Uuid,
    title: Option<String>,
    description: Option<String>,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE feeds 
        SET title = COALESCE($2, title),
            description = COALESCE($3, description),
            last_fetched = NOW()
        WHERE id = $1
        "#
    )
    .bind(feed_id)
    .bind(title)
    .bind(description)
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn subscribe_to_feed(
    pool: &PgPool,
    user_id: Uuid,
    feed_id: Uuid,
    folder: Option<String>,
) -> Result<Subscription> {
    let subscription = sqlx::query_as::<_, Subscription>(
        r#"
        INSERT INTO subscriptions (user_id, feed_id, folder)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, feed_id) DO UPDATE SET folder = COALESCE($3, subscriptions.folder)
        RETURNING id, user_id, feed_id, custom_title, folder, created_at
        "#
    )
    .bind(user_id)
    .bind(feed_id)
    .bind(folder)
    .fetch_one(pool)
    .await?;
    
    Ok(subscription)
}

pub async fn get_user_subscriptions(pool: &PgPool, user_id: Uuid) -> Result<Vec<(Subscription, Feed)>> {
    let rows = sqlx::query(
        r#"
        SELECT 
            s.id as subscription_id,
            s.user_id,
            s.feed_id,
            s.custom_title,
            s.folder,
            s.created_at as subscription_created_at,
            f.id as feed_id,
            f.url,
            f.title,
            f.description,
            f.last_fetched,
            f.created_at as feed_created_at,
            f.updated_at as feed_updated_at
        FROM subscriptions s
        JOIN feeds f ON s.feed_id = f.id
        WHERE s.user_id = $1
        ORDER BY COALESCE(s.custom_title, f.title, f.url)
        "#
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let result = rows.into_iter().map(|row| {
        let subscription = Subscription {
            id: row.get("subscription_id"),
            user_id: row.get("user_id"),
            feed_id: row.get("feed_id"),
            custom_title: row.get("custom_title"),
            folder: row.get("folder"),
            created_at: row.get("subscription_created_at"),
        };
        let feed = Feed {
            id: row.get("feed_id"),
            url: row.get("url"),
            title: row.get("title"),
            description: row.get("description"),
            last_fetched: row.get("last_fetched"),
            created_at: row.get("feed_created_at"),
            updated_at: row.get("feed_updated_at"),
        };
        (subscription, feed)
    }).collect();

    Ok(result)
}

pub async fn create_or_update_item(pool: &PgPool, item: &Item) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO items (feed_id, guid, title, description, link, pub_date, author, content)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (feed_id, guid) DO UPDATE SET
            title = EXCLUDED.title,
            description = EXCLUDED.description,
            link = EXCLUDED.link,
            pub_date = EXCLUDED.pub_date,
            author = EXCLUDED.author,
            content = EXCLUDED.content
        "#
    )
    .bind(item.feed_id)
    .bind(&item.guid)
    .bind(&item.title)
    .bind(&item.description)
    .bind(&item.link)
    .bind(item.pub_date)
    .bind(&item.author)
    .bind(&item.content)
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn get_user_items(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<ItemWithReadStatus>> {
    let rows = sqlx::query(
        r#"
        SELECT 
            i.id,
            i.feed_id,
            i.guid,
            i.title,
            i.description,
            i.link,
            i.pub_date,
            i.author,
            i.content,
            i.created_at,
            f.title as feed_title,
            CASE WHEN ri.item_id IS NOT NULL THEN true ELSE false END as is_read
        FROM items i
        JOIN feeds f ON i.feed_id = f.id
        JOIN subscriptions s ON s.feed_id = f.id
        LEFT JOIN read_items ri ON ri.item_id = i.id AND ri.user_id = $1
        WHERE s.user_id = $1
        ORDER BY i.pub_date DESC NULLS LAST, i.created_at DESC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let items = rows.into_iter().map(|row| {
        ItemWithReadStatus {
            item: Item {
                id: row.get("id"),
                feed_id: row.get("feed_id"),
                guid: row.get("guid"),
                title: row.get("title"),
                description: row.get("description"),
                link: row.get("link"),
                pub_date: row.get("pub_date"),
                author: row.get("author"),
                content: row.get("content"),
                created_at: row.get("created_at"),
            },
            is_read: row.get("is_read"),
            feed_title: row.get("feed_title"),
        }
    }).collect();

    Ok(items)
}

pub async fn mark_items_read(pool: &PgPool, user_id: Uuid, item_ids: &[Uuid]) -> Result<()> {
    for item_id in item_ids {
        sqlx::query(
            r#"
            INSERT INTO read_items (user_id, item_id)
            VALUES ($1, $2)
            ON CONFLICT (user_id, item_id) DO NOTHING
            "#
        )
        .bind(user_id)
        .bind(item_id)
        .execute(pool)
        .await?;
    }
    
    Ok(())
}

pub async fn unsubscribe_from_feed(pool: &PgPool, user_id: Uuid, feed_id: Uuid) -> Result<PgQueryResult> {
    Ok(sqlx::query(
        r#"DELETE FROM subscriptions WHERE user_id = $1 AND feed_id = $2"#
    )
    .bind(user_id)
    .bind(feed_id)
    .execute(pool)
    .await?)
}