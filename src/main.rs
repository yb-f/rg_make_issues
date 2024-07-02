use dotenv::dotenv;
use octocrab::Octocrab;
use reqwest;
use secrecy::Secret;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

const SEPERATOR: &str = "─────────────────────────────────────────────────────────────────────────────────────────────────────────";

struct Config {
    gh_auth_token: String,
    gh_owner: String,
    gh_repo: String,
    thread_id: i32,
    api_key: String,
    api_user_id: String,
    base_url: String,
    username: String,
}

impl Config {
    fn new() -> Self {
        Self {
            gh_auth_token: env::var("GH_AUTH_TOKEN").expect("GH_AUTH_TOKEN must be set"),
            gh_owner: env::var("GH_OWNER").expect("GH_OWNER must be set"),
            gh_repo: env::var("GH_REPO").expect("GH_REPO must be set"),
            thread_id: env::var("THREAD_ID")
                .expect("THREAD_ID must be set")
                .parse()
                .expect("THREAD_ID must be an integer"),
            api_key: env::var("API_KEY").expect("API_KEY must be set"),
            api_user_id: env::var("API_USER_ID").expect("API_USER_ID must be set"),
            base_url: env::var("BASE_URL").expect("BASE_URL must be set"),
            username: env::var("USERNAME").expect("USERNAME must be set"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Thread {
    reply_count: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct ThreadResponse {
    thread: Thread,
}

#[derive(Serialize, Deserialize, Debug)]
struct Post {
    is_unread: bool,
    message: String,
    username: String,
    post_id: i32,
    position: i32,
}

struct PostObject {
    message: String,
    username: String,
    post_id: i32,
    position: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct PostResponse {
    posts: Vec<Post>,
}

fn create_headers(
    config: &Config,
) -> Result<reqwest::header::HeaderMap, Box<dyn std::error::Error>> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("XF-Api-User", config.api_user_id.parse()?);
    headers.insert("XF-Api-Key", config.api_key.parse()?);
    headers.insert("Content-Type", "applications/json".parse()?);
    Ok(headers)
}

async fn get_pages(config: &Config) -> Result<i32, Box<dyn std::error::Error>> {
    let url = format!("{}/threads/{}", config.base_url, config.thread_id);
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .headers(create_headers(config)?)
        .send()
        .await?;

    if response.status() == reqwest::StatusCode::OK {
        let json: ThreadResponse = response.json().await?;
        let reply_count = json.thread.reply_count;
        let pages = (reply_count as f32 / 40.0).ceil() as i32;
        Ok(pages)
    } else if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        println!("Status: Authentication failed.");
        Ok(0)
    } else {
        println!("Unexpected status code: {:?}", response.status());
        Ok(0)
    }
}

async fn get_posts(
    pages: i32,
    config: &Config,
) -> Result<Vec<PostObject>, Box<dyn std::error::Error>> {
    let mut all_posts: Vec<PostObject> = Vec::new();
    let mut current_page = pages;
    while current_page > 0 {
        let url = format!(
            "{}/threads/{}/posts/?page={}",
            config.base_url, config.thread_id, current_page
        );
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .headers(create_headers(config)?)
            .send()
            .await?;
        if response.status() == reqwest::StatusCode::OK {
            let json: PostResponse = response.json().await?;
            let mut unread_posts: Vec<PostObject> = json
                .posts
                .into_iter()
                .filter(|post| post.is_unread && post.username != config.username)
                .map(|post| PostObject {
                    message: post.message,
                    username: post.username,
                    post_id: post.post_id,
                    position: post.position,
                })
                .collect();
            if unread_posts.is_empty() {
                break;
            }
            if unread_posts.is_empty() && current_page > 1 {
                current_page -= 1;
            } else {
                all_posts.append(&mut unread_posts);
                break;
            }
        } else if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            println!("Status: Authentication failed.");
            break;
        } else {
            println!("Unexpected status code: {:?}", response.status());
            break;
        }

        current_page -= 1;
    }
    Ok(all_posts)
}

async fn process_messages(
    message_store: Vec<PostObject>,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    for message in message_store {
        println!(
            "User: {}\nMessage: {}\nStore message as issue? (y/n)",
            message.username, message.message
        );
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if input.trim() == "y" {
            create_issue(message, config).await?;
        }
        println!("\n{}\n\n", SEPERATOR);
    }
    Ok(())
}

async fn create_issue(
    message: PostObject,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let token: Secret<String> = Secret::new(config.gh_auth_token.to_string());
    let octocrab = Octocrab::builder().personal_token(token).build()?;

    let issue = octocrab
        .issues(config.gh_owner.clone(), config.gh_repo.clone())
        .create(format!(
            "{} - {} - {}",
            message.username, message.post_id, message.position
        ))
        .body(message.message)
        .send()
        .await?;
    println!("Issue created: {}", issue.html_url);
    Ok(())
}

async fn mark_as_read(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let url = format!("{}/threads/{}/mark-read", config.base_url, config.thread_id);
    let client = reqwest::Client::new();
    let mut headers = create_headers(config).unwrap();
    headers.insert("date", epoch.to_string().parse()?);
    headers.insert("Content-Type", "applications/json".parse()?);
    let response = client.get(&url).headers(headers).send().await?;
    if response.status() == reqwest::StatusCode::OK {
        println!("All messages marked as read.");
    } else if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        println!("Status: Authentication failed.");
    } else {
        println!("Unexpected status code: {:?}", response.status());
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let config = Config::new();

    let pages = get_pages(&config).await?;
    let message_store = get_posts(pages, &config).await?;
    if !message_store.is_empty() {
        process_messages(message_store, &config).await?;
    } else {
        println!("No new messages.");
        return Ok(());
    }
    println!("Mark all messages as read? (y/n)");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if input.trim() == "y" {
        mark_as_read(&config).await?;
    }

    Ok(())
}
