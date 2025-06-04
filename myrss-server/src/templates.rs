use maud::{html, Markup, DOCTYPE};
use crate::models::{ItemWithReadStatus, Label, SubscriptionWithLabels};

pub fn base_layout(title: &str, username: Option<&str>, content: Markup) -> Markup {
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
                            @if let Some(user) = username {
                                nav {
                                    span class="username" { (user) }
                                    a href="/feeds" class="nav-link" { "Manage Feeds" }
                                    a href="/labels" class="nav-link" { "Labels" }
                                    a href="/refresh" class="nav-link refresh-btn" { "Refresh All" }
                                    form action="/logout" method="post" class="logout-form" {
                                        button type="submit" class="logout-btn" { "Logout" }
                                    }
                                }
                            }
                        }
                    }
                }
                main class="container" {
                    (content)
                }
            }
        }
    }
}

pub fn login_page() -> Markup {
    base_layout("Login", None, html! {
        div class="auth-container" {
            h2 { "Login" }
            form action="/login" method="post" class="auth-form" {
                div class="form-group" {
                    label for="username" { "Username" }
                    input type="text" id="username" name="username" required autofocus;
                }
                div class="form-group" {
                    label for="password" { "Password" }
                    input type="password" id="password" name="password" required;
                }
                button type="submit" class="btn btn-primary" { "Login" }
            }
            p class="auth-switch" {
                "Don't have an account? "
                a href="/register" { "Register here" }
            }
        }
    })
}

pub fn login_page_with_error(error: &str) -> Markup {
    base_layout("Login", None, html! {
        div class="auth-container" {
            h2 { "Login" }
            div class="error-message" { (error) }
            form action="/login" method="post" class="auth-form" {
                div class="form-group" {
                    label for="username" { "Username" }
                    input type="text" id="username" name="username" required autofocus;
                }
                div class="form-group" {
                    label for="password" { "Password" }
                    input type="password" id="password" name="password" required;
                }
                button type="submit" class="btn btn-primary" { "Login" }
            }
            p class="auth-switch" {
                "Don't have an account? "
                a href="/register" { "Register here" }
            }
        }
    })
}

pub fn register_page() -> Markup {
    base_layout("Register", None, html! {
        div class="auth-container" {
            h2 { "Create Account" }
            form action="/register" method="post" class="auth-form" {
                div class="form-group" {
                    label for="username" { "Username" }
                    input type="text" id="username" name="username" required autofocus;
                }
                div class="form-group" {
                    label for="email" { "Email" }
                    input type="email" id="email" name="email" required;
                }
                div class="form-group" {
                    label for="password" { "Password" }
                    input type="password" id="password" name="password" required;
                }
                div class="form-group" {
                    label for="password_confirm" { "Confirm Password" }
                    input type="password" id="password_confirm" name="password_confirm" required;
                }
                button type="submit" class="btn btn-primary" { "Register" }
            }
            p class="auth-switch" {
                "Already have an account? "
                a href="/login" { "Login here" }
            }
        }
    })
}

pub fn register_page_with_error(error: &str) -> Markup {
    base_layout("Register", None, html! {
        div class="auth-container" {
            h2 { "Create Account" }
            div class="error-message" { (error) }
            form action="/register" method="post" class="auth-form" {
                div class="form-group" {
                    label for="username" { "Username" }
                    input type="text" id="username" name="username" required autofocus;
                }
                div class="form-group" {
                    label for="email" { "Email" }
                    input type="email" id="email" name="email" required;
                }
                div class="form-group" {
                    label for="password" { "Password" }
                    input type="password" id="password" name="password" required;
                }
                div class="form-group" {
                    label for="password_confirm" { "Confirm Password" }
                    input type="password" id="password_confirm" name="password_confirm" required;
                }
                button type="submit" class="btn btn-primary" { "Register" }
            }
            p class="auth-switch" {
                "Already have an account? "
                a href="/login" { "Login here" }
            }
        }
    })
}

pub fn home_page(username: &str, items: &[ItemWithReadStatus], has_more: bool, page: i64) -> Markup {
    base_layout("Home", Some(username), html! {
        div class="feed-items" {
            @if items.is_empty() && page == 1 {
                div class="empty-state" {
                    h2 { "No items yet" }
                    p { "Subscribe to some feeds to start reading!" }
                    a href="/feeds" class="btn btn-primary" { "Add Feeds" }
                }
            } @else {
                @for item in items {
                    article class={"feed-item" @if item.is_read { " read" }} data-item-id=(item.item.id) {
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
                                    span class="pub-date" { " • " (pub_date.format(&time::format_description::parse("[month repr:short] [day], [year]").unwrap()).unwrap_or_else(|_| "Unknown date".to_string())) }
                                }
                                @if let Some(author) = &item.item.author {
                                    span class="author" { " • by " (author) }
                                }
                            }
                        }
                        @if let Some(description) = &item.item.description {
                            div class="item-description" {
                                (maud::PreEscaped(description))
                            }
                        }
                        @if !item.is_read {
                            button class="mark-read-btn" data-item-id=(item.item.id) { "Mark as Read" }
                        }
                    }
                }
                
                @if page > 1 || has_more {
                    div class="pagination" {
                        @if page > 1 {
                            a href={"/?page=" (page - 1)} class="btn" { "Previous" }
                        }
                        span class="page-info" { "Page " (page) }
                        @if has_more {
                            a href={"/?page=" (page + 1)} class="btn" { "Next" }
                        }
                    }
                }
            }
        }
    })
}

pub fn feeds_page(username: &str, subscriptions: &[SubscriptionWithLabels], labels: &[Label]) -> Markup {
    base_layout("Manage Feeds", Some(username), html! {
        div class="feeds-page" {
            div class="add-feed-section" {
                h2 { "Add New Feed" }
                form action="/feeds/add" method="post" class="add-feed-form" {
                    div class="form-group" {
                        label for="url" { "Feed URL" }
                        input type="url" id="url" name="url" placeholder="https://example.com/feed.xml";
                    }
                    div class="form-group" {
                        label for="labels" { "Labels (comma-separated)" }
                        input type="text" id="labels" name="labels" placeholder="tech, news, personal";
                    }
                    div class="form-group" {
                        label for="content" { "Or paste RSS/Atom content" }
                        textarea id="content" name="content" rows="6" {}
                    }
                    button type="submit" class="btn btn-primary" { "Add Feed" }
                }
            }
            
            div class="subscriptions-section" {
                h2 { "Your Subscriptions" }
                @if subscriptions.is_empty() {
                    p class="empty-message" { "You haven't subscribed to any feeds yet." }
                } @else {
                    div class="subscription-list" {
                        @for sub in subscriptions {
                            div class="subscription-item" data-subscription-id=(sub.subscription.id) {
                                div class="subscription-info" {
                                    h3 { 
                                        (sub.feed_title.as_deref().unwrap_or(&sub.feed_url))
                                    }
                                    p class="feed-url" { (sub.feed_url) }
                                    div class="labels" {
                                        @for label in &sub.labels {
                                            span class="label" style={"background-color: " (label.color)} {
                                                (label.name)
                                            }
                                        }
                                        button class="edit-labels-btn" data-subscription-id=(sub.subscription.id) { "Edit Labels" }
                                    }
                                }
                                form action={"/feeds/" (sub.subscription.feed_id) "/unsubscribe"} method="post" class="inline-form" {
                                    button type="submit" class="btn btn-danger" 
                                        onclick="return confirm('Are you sure you want to unsubscribe from this feed?');" {
                                        "Unsubscribe"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Label edit modal
        div id="label-edit-modal" class="modal" style="display: none;" {
            div class="modal-content" {
                h3 { "Edit Labels" }
                form id="label-edit-form" method="post" {
                    div class="label-checkboxes" {
                        @for label in labels {
                            label class="checkbox-label" {
                                input type="checkbox" name="labels" value=(label.name);
                                span style={"background-color: " (label.color)} { (label.name) }
                            }
                        }
                    }
                    div class="form-group" {
                        label for="new-labels" { "Add new labels (comma-separated)" }
                        input type="text" id="new-labels" placeholder="label1, label2";
                    }
                    div class="modal-buttons" {
                        button type="submit" class="btn btn-primary" { "Save" }
                        button type="button" class="btn" onclick="closeModal()" { "Cancel" }
                    }
                }
            }
        }
    })
}

pub fn labels_page(username: &str, labels: &[Label]) -> Markup {
    base_layout("Manage Labels", Some(username), html! {
        div class="labels-page" {
            h2 { "Your Labels" }
            
            div class="add-label-section" {
                h3 { "Add New Label" }
                form action="/labels/add" method="post" class="add-label-form" {
                    div class="form-group inline" {
                        input type="text" name="name" placeholder="Label name" required;
                        input type="color" name="color" value="#3b82f6";
                        button type="submit" class="btn btn-primary" { "Add Label" }
                    }
                }
            }
            
            @if labels.is_empty() {
                p class="empty-message" { "You haven't created any labels yet." }
            } @else {
                div class="label-list" {
                    @for label in labels {
                        div class="label-item" {
                            span class="label-display" style={"background-color: " (label.color)} {
                                (label.name)
                            }
                            form action={"/labels/" (label.id) "/delete"} method="post" class="inline-form" {
                                button type="submit" class="btn btn-sm btn-danger" 
                                    onclick="return confirm('Delete this label? It will be removed from all feeds.');" {
                                    "Delete"
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}