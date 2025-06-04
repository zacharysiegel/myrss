use crate::models::*;
use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

// User management functions
pub async fn create_user(pool: &PgPool, username: &str, email: &str, password_hash: &str) -> Result<User> {
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (username, email, password_hash)
        VALUES ($1, $2, $3)
        RETURNING id, username, email, password_hash, created_at, updated_at
        "#
    )
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await?;
    
    Ok(user)
}

pub async fn authenticate_user(pool: &PgPool, username: &str, password_hash: &str) -> Result<User> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, username, email, password_hash, created_at, updated_at 
        FROM users 
        WHERE username = $1 AND password_hash = $2
        "#
    )
    .bind(username)
    .bind(password_hash)
    .fetch_one(pool)
    .await?;
    
    Ok(user)
}

pub async fn get_user_by_id(pool: &PgPool, user_id: Uuid) -> Result<User> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, username, email, password_hash, created_at, updated_at 
        FROM users 
        WHERE id = $1
        "#
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    
    Ok(user)
}

pub async fn get_user_by_username(pool: &PgPool, username: &str) -> Result<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        r#"SELECT id, username, email, password_hash, created_at, updated_at FROM users WHERE username = $1"#
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    
    Ok(user)
}

// Feed management functions
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
    title: &str,
    description: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE feeds 
        SET title = $2, description = $3, last_fetched = NOW(), updated_at = NOW()
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

// Subscription management functions
pub async fn subscribe_to_feed(pool: &PgPool, user_id: Uuid, feed_id: Uuid) -> Result<Subscription> {
    let subscription = sqlx::query_as::<_, Subscription>(
        r#"
        INSERT INTO subscriptions (user_id, feed_id)
        VALUES ($1, $2)
        ON CONFLICT (user_id, feed_id) DO UPDATE SET user_id = EXCLUDED.user_id
        RETURNING id, user_id, feed_id, custom_title, created_at
        "#
    )
    .bind(user_id)
    .bind(feed_id)
    .fetch_one(pool)
    .await?;
    
    Ok(subscription)
}

pub async fn unsubscribe_from_feed(pool: &PgPool, user_id: Uuid, feed_id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        DELETE FROM subscriptions 
        WHERE user_id = $1 AND feed_id = $2
        "#
    )
    .bind(user_id)
    .bind(feed_id)
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn get_user_subscriptions(pool: &PgPool, user_id: Uuid) -> Result<Vec<Subscription>> {
    let subscriptions = sqlx::query_as::<_, Subscription>(
        r#"
        SELECT id, user_id, feed_id, custom_title, created_at
        FROM subscriptions
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    
    Ok(subscriptions)
}

pub async fn get_user_subscriptions_with_labels(pool: &PgPool, user_id: Uuid) -> Result<Vec<SubscriptionWithLabels>> {
    let rows = sqlx::query(
        r#"
        SELECT 
            s.id, s.user_id, s.feed_id, s.custom_title, s.created_at,
            f.title as feed_title, f.url as feed_url,
            COALESCE(
                json_agg(
                    json_build_object(
                        'id', l.id,
                        'user_id', l.user_id,
                        'name', l.name,
                        'color', l.color,
                        'created_at', l.created_at
                    ) ORDER BY l.name
                ) FILTER (WHERE l.id IS NOT NULL), 
                '[]'::json
            ) as labels
        FROM subscriptions s
        JOIN feeds f ON s.feed_id = f.id
        LEFT JOIN subscription_labels sl ON s.id = sl.subscription_id
        LEFT JOIN labels l ON sl.label_id = l.id
        WHERE s.user_id = $1
        GROUP BY s.id, s.user_id, s.feed_id, s.custom_title, s.created_at, f.title, f.url
        ORDER BY s.created_at DESC
        "#
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    
    let mut subscriptions = Vec::new();
    for row in rows {
        let subscription = Subscription {
            id: row.get("id"),
            user_id: row.get("user_id"),
            feed_id: row.get("feed_id"),
            custom_title: row.get("custom_title"),
            created_at: row.get("created_at"),
        };
        
        let labels_json: serde_json::Value = row.get("labels");
        let labels: Vec<Label> = serde_json::from_value(labels_json)?;
        
        subscriptions.push(SubscriptionWithLabels {
            subscription,
            feed_title: row.get("feed_title"),
            feed_url: row.get("feed_url"),
            labels,
        });
    }
    
    Ok(subscriptions)
}

pub async fn user_owns_subscription(pool: &PgPool, user_id: Uuid, subscription_id: Uuid) -> Result<bool> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) 
        FROM subscriptions 
        WHERE id = $1 AND user_id = $2
        "#
    )
    .bind(subscription_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    
    Ok(count > 0)
}

// Label management functions
pub async fn create_label(pool: &PgPool, user_id: Uuid, name: &str, color: &str) -> Result<Label> {
    let label = sqlx::query_as::<_, Label>(
        r#"
        INSERT INTO labels (user_id, name, color)
        VALUES ($1, $2, $3)
        RETURNING id, user_id, name, color, created_at
        "#
    )
    .bind(user_id)
    .bind(name)
    .bind(color)
    .fetch_one(pool)
    .await?;
    
    Ok(label)
}

pub async fn get_or_create_label(pool: &PgPool, user_id: Uuid, name: &str) -> Result<Label> {
    let label = sqlx::query_as::<_, Label>(
        r#"
        INSERT INTO labels (user_id, name)
        VALUES ($1, $2)
        ON CONFLICT (user_id, name) DO UPDATE SET name = EXCLUDED.name
        RETURNING id, user_id, name, color, created_at
        "#
    )
    .bind(user_id)
    .bind(name)
    .fetch_one(pool)
    .await?;
    
    Ok(label)
}

pub async fn get_user_labels(pool: &PgPool, user_id: Uuid) -> Result<Vec<Label>> {
    let labels = sqlx::query_as::<_, Label>(
        r#"
        SELECT id, user_id, name, color, created_at
        FROM labels
        WHERE user_id = $1
        ORDER BY name
        "#
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    
    Ok(labels)
}

pub async fn delete_user_label(pool: &PgPool, user_id: Uuid, label_id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        DELETE FROM labels 
        WHERE id = $1 AND user_id = $2
        "#
    )
    .bind(label_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn add_label_to_subscription(pool: &PgPool, subscription_id: Uuid, label_id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO subscription_labels (subscription_id, label_id)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING
        "#
    )
    .bind(subscription_id)
    .bind(label_id)
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn clear_subscription_labels(pool: &PgPool, subscription_id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        DELETE FROM subscription_labels 
        WHERE subscription_id = $1
        "#
    )
    .bind(subscription_id)
    .execute(pool)
    .await?;
    
    Ok(())
}

// Item management functions
pub async fn create_or_update_item(
    pool: &PgPool,
    feed_id: Uuid,
    guid: &str,
    title: &str,
    description: Option<&str>,
    link: Option<&str>,
    pub_date: Option<time::OffsetDateTime>,
    author: Option<&str>,
    content: Option<&str>,
) -> Result<Uuid> {
    let result = sqlx::query(
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
        RETURNING id
        "#
    )
    .bind(feed_id)
    .bind(guid)
    .bind(title)
    .bind(description)
    .bind(link)
    .bind(pub_date)
    .bind(author)
    .bind(content)
    .fetch_one(pool)
    .await?;
    
    Ok(result.get(0))
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
            i.id, i.feed_id, i.guid, i.title, i.description, i.link, 
            i.pub_date, i.author, i.content, i.created_at,
            f.title as feed_title,
            COALESCE(ur.is_read, false) as is_read
        FROM items i
        JOIN feeds f ON i.feed_id = f.id
        JOIN subscriptions s ON s.feed_id = f.id
        LEFT JOIN user_read_items ur ON ur.item_id = i.id AND ur.user_id = $1
        WHERE s.user_id = $1
        ORDER BY COALESCE(i.pub_date, i.created_at) DESC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    
    let mut items = Vec::new();
    for row in rows {
        let item = Item {
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
        };
        
        items.push(ItemWithReadStatus {
            item,
            is_read: row.get("is_read"),
            feed_title: row.get("feed_title"),
        });
    }
    
    Ok(items)
}

pub async fn mark_items_read(pool: &PgPool, user_id: Uuid, item_ids: &[Uuid]) -> Result<()> {
    for item_id in item_ids {
        sqlx::query(
            r#"
            INSERT INTO user_read_items (user_id, item_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#
        )
        .bind(user_id)
        .bind(item_id)
        .execute(pool)
        .await?;
    }
    
    Ok(())
}