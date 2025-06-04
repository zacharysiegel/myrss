use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    #[serde(skip)]
    pub password_hash: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Feed {
    pub id: Uuid,
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_fetched: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Subscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub feed_id: Uuid,
    pub custom_title: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Label {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub color: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionWithLabels {
    #[serde(flatten)]
    pub subscription: Subscription,
    pub feed_title: Option<String>,
    pub feed_url: String,
    pub labels: Vec<Label>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Item {
    pub id: Uuid,
    pub feed_id: Uuid,
    pub guid: String,
    pub title: String,
    pub description: Option<String>,
    pub link: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub pub_date: Option<OffsetDateTime>,
    pub author: Option<String>,
    pub content: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemWithReadStatus {
    #[serde(flatten)]
    pub item: Item,
    pub is_read: bool,
    pub feed_title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddFeedRequest {
    pub url: Option<String>,
    pub content: Option<String>,
    pub labels: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFeedLabelsRequest {
    pub labels: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct MarkReadRequest {
    pub item_ids: Vec<Uuid>,
}