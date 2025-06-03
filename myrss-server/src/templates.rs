use maud::{html, Markup, DOCTYPE};
use crate::models::{Feed, ItemWithReadStatus, Subscription};

pub fn base_layout(title: &str, username: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " - MyRSS" }
                link rel="stylesheet" href="/static/style.css";
                script src="/static/app.js" defer {}
            }
            body {
                header class="main-header" {
                    div class="container" {
                        div class="header-content" {
                            h1 { 
                                a href="/" class="logo" { "MyRSS" }
                            }
                            nav {
                                span class="username" { "👤 " (username) }
                                a href="/feeds" class="nav-link" { "Manage Feeds" }
                                a href="/refresh" class="nav-link refresh-btn" { "🔄 Refresh All" }
                            }
                        }
                    }
                }
                main class="container" {
                    (content)
                }
                footer {
                    div class="container" {
                        p { "MyRSS - Your personal RSS reader" }
                    }
                }
            }
        }
    }
}

pub fn home_page(username: &str, items: &[ItemWithReadStatus], has_more: bool, page: i64) -> Markup {
    base_layout("Home", username, html! {
        div class="feed-items" {
            @if items.is_empty() && page == 1 {
                div class="empty-state" {
                    h2 { "No items yet" }
                    p { "Subscribe to some feeds to start reading!" }
                    a href="/feeds" class="btn btn-primary" { "Add Feeds" }
                }
            } @else {
                @for item in items {
                    article class={"item" @if item.is_read { " read" }} data-item-id=(item.item.id) {
                        div class="item-header" {
                            h3 class="item-title" {
                                @if let Some(link) = &item.item.link {
                                    a href=(link) target="_blank" rel="noopener" { (item.item.title) }
                                } @else {
                                    (item.item.title)
                                }
                            }
                            div class="item-meta" {
                                span class="feed-name" { (item.feed_title.as_deref().unwrap_or("Unknown Feed")) }
                                @if let Some(pub_date) = item.item.pub_date {
                                    span class="item-date" { " • " (format_date(pub_date)) }
                                }
                                @if let Some(author) = &item.item.author {
                                    span class="item-author" { " • by " (author) }
                                }
                            }
                        }
                        @if let Some(description) = &item.item.description {
                            div class="item-content" {
                                (maud::PreEscaped(sanitize_html(description)))
                            }
                        }
                        div class="item-actions" {
                            @if !item.is_read {
                                button class="btn btn-small mark-read" data-item-id=(item.item.id) { 
                                    "Mark as Read" 
                                }
                            }
                        }
                    }
                }
                
                div class="pagination" {
                    @if page > 1 {
                        a href=(format!("/?page={}", page - 1)) class="btn" { "← Previous" }
                    }
                    span class="page-info" { "Page " (page) }
                    @if has_more {
                        a href=(format!("/?page={}", page + 1)) class="btn" { "Next →" }
                    }
                }
            }
        }
    })
}

pub fn feeds_page(username: &str, subscriptions: &[(Subscription, Feed)]) -> Markup {
    base_layout("Manage Feeds", username, html! {
        div class="feeds-container" {
            section class="add-feed-section" {
                h2 { "Add New Feed" }
                form method="post" action="/feeds/add" class="add-feed-form" {
                    div class="form-group" {
                        label for="url" { "Feed URL" }
                        input type="url" name="url" id="url" placeholder="https://example.com/feed.xml" class="form-input";
                    }
                    div class="form-group" {
                        label for="content" { "Or paste RSS/XML content" }
                        textarea name="content" id="content" rows="6" class="form-input" 
                            placeholder="Paste RSS XML content here if you have it" {}
                    }
                    div class="form-group" {
                        label for="folder" { "Folder (optional)" }
                        input type="text" name="folder" id="folder" placeholder="Technology, News, etc." class="form-input";
                    }
                    button type="submit" class="btn btn-primary" { "Add Feed" }
                }
            }
            
            section class="subscriptions-section" {
                h2 { "Your Subscriptions" }
                @if subscriptions.is_empty() {
                    p class="empty-message" { "You haven't subscribed to any feeds yet." }
                } @else {
                    div class="feeds-list" {
                        @for (subscription, feed) in subscriptions {
                            div class="feed-card" {
                                div class="feed-info" {
                                    h3 class="feed-title" {
                                        (subscription.custom_title.as_deref()
                                            .or(feed.title.as_deref())
                                            .unwrap_or(&feed.url))
                                    }
                                    p class="feed-url" { (feed.url) }
                                    @if let Some(description) = &feed.description {
                                        p class="feed-description" { (description) }
                                    }
                                    @if let Some(folder) = &subscription.folder {
                                        span class="feed-folder" { "📁 " (folder) }
                                    }
                                    @if let Some(last_fetched) = feed.last_fetched {
                                        p class="feed-meta" { 
                                            "Last updated: " (format_date(last_fetched))
                                        }
                                    }
                                }
                                div class="feed-actions" {
                                    form method="post" action=(format!("/feeds/{}/unsubscribe", feed.id)) 
                                        class="inline-form" {
                                        button type="submit" class="btn btn-danger btn-small" 
                                            onclick="return confirm('Unsubscribe from this feed?')" {
                                            "Unsubscribe"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

pub fn error_page(username: &str, error: &str) -> Markup {
    base_layout("Error", username, html! {
        div class="error-container" {
            h2 { "Error" }
            p class="error-message" { (error) }
            a href="/" class="btn" { "Go Home" }
        }
    })
}

fn format_date(date: time::OffsetDateTime) -> String {
    let now = time::OffsetDateTime::now_utc();
    let duration = now - date;
    
    if duration.whole_days() == 0 {
        if duration.whole_hours() == 0 {
            if duration.whole_minutes() == 0 {
                "just now".to_string()
            } else {
                format!("{}m ago", duration.whole_minutes())
            }
        } else {
            format!("{}h ago", duration.whole_hours())
        }
    } else if duration.whole_days() < 7 {
        format!("{}d ago", duration.whole_days())
    } else {
        date.format(&time::format_description::parse("[month repr:short] [day], [year]").unwrap())
            .unwrap_or_else(|_| date.to_string())
    }
}

fn sanitize_html(html: &str) -> String {
    // Basic HTML sanitization - in production, use a proper HTML sanitizer
    html.replace("<script", "&lt;script")
        .replace("</script>", "&lt;/script&gt;")
        .replace("javascript:", "")
}