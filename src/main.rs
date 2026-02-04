use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "moyn", about = "Developer microblogging from your terminal")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Store your API token
    Login,
    /// Publish a markdown file as a post
    Publish {
        /// Path to the markdown file
        file: PathBuf,
    },
    /// List your posts
    Posts,
    /// Delete a post by ID
    Delete {
        /// Post ID to delete
        id: u64,
    },
    /// List all spaces you own or are a member of
    Spaces,
    /// Manage spaces
    Space {
        #[command(subcommand)]
        command: SpaceCommands,
    },
}

#[derive(Subcommand)]
enum SpaceCommands {
    /// Create a new space
    Create {
        /// Display name (required)
        #[arg(short, long)]
        name: String,
        /// Custom slug (optional, auto-generated from name if omitted)
        #[arg(short, long)]
        slug: Option<String>,
        /// Space description
        #[arg(short, long)]
        description: Option<String>,
        /// Visibility: public, unlisted, or private
        #[arg(short, long, default_value = "private")]
        visibility: String,
    },
    /// Show details of a specific space
    Show {
        /// Space slug
        slug: String,
    },
}

#[derive(Serialize, Deserialize)]
struct Config {
    api_token: String,
    api_url: String,
}

#[derive(Deserialize)]
struct PostsResponse {
    posts: Vec<Post>,
}

#[derive(Deserialize, Debug)]
struct PostResponse {
    post: Post,
}

#[derive(Deserialize, Debug)]
struct Post {
    id: u64,
    title: String,
    slug: String,
    url: String,
}

#[derive(Serialize)]
struct CreatePostRequest {
    post: CreatePost,
}

#[derive(Serialize)]
struct CreatePost {
    title: String,
    content: String,
    published: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct SpacesResponse {
    spaces: Vec<Space>,
}

#[derive(Deserialize, Debug)]
struct SpaceResponse {
    space: Space,
}

#[derive(Deserialize, Debug)]
struct Space {
    slug: String,
    name: String,
    description: Option<String>,
    visibility: String,
    access_token: Option<String>,
    url: String,
    token_url: Option<String>,
}

#[derive(Serialize)]
struct CreateSpaceRequest {
    space: CreateSpace,
}

#[derive(Serialize)]
struct CreateSpace {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    visibility: Option<String>,
}

#[derive(Deserialize, Default)]
struct Frontmatter {
    title: Option<String>,
    published: Option<bool>,
    tags: Option<Vec<String>>,
    slug: Option<String>,
    space: Option<String>,
}

struct ParsedContent {
    frontmatter: Frontmatter,
    content: String,
}

fn parse_frontmatter(content: &str) -> ParsedContent {
    let trimmed = content.trim_start();

    if !trimmed.starts_with("---") {
        return ParsedContent {
            frontmatter: Frontmatter::default(),
            content: content.to_string(),
        };
    }

    // Find the end of frontmatter
    let after_first_marker = &trimmed[3..];
    if let Some(end_pos) = after_first_marker.find("\n---") {
        let yaml_content = &after_first_marker[..end_pos].trim();
        let remaining_content = &after_first_marker[end_pos + 4..];

        // Parse the YAML
        match serde_yaml::from_str::<Frontmatter>(yaml_content) {
            Ok(fm) => ParsedContent {
                frontmatter: fm,
                content: remaining_content.trim_start_matches('\n').to_string(),
            },
            Err(_) => ParsedContent {
                frontmatter: Frontmatter::default(),
                content: content.to_string(),
            },
        }
    } else {
        ParsedContent {
            frontmatter: Frontmatter::default(),
            content: content.to_string(),
        }
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .expect("Could not find config directory")
        .join("moyn")
        .join("config.json")
}

fn load_config() -> Result<Config, String> {
    let path = config_path();
    let content = fs::read_to_string(&path)
        .map_err(|_| "Not logged in. Run `moyn login` first.".to_string())?;
    serde_json::from_str(&content).map_err(|e| format!("Invalid config: {}", e))
}

fn save_config(config: &Config) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Could not create config dir: {}", e))?;
    }
    let content = serde_json::to_string_pretty(config).map_err(|e| format!("Could not serialize config: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Could not write config: {}", e))
}

fn client(config: &Config) -> reqwest::blocking::Client {
    reqwest::blocking::Client::new()
}

fn login() -> Result<(), String> {
    print!("Enter your API token (from your profile page): ");
    io::stdout().flush().unwrap();

    let mut token = String::new();
    io::stdin()
        .read_line(&mut token)
        .map_err(|e| format!("Could not read input: {}", e))?;
    let token = token.trim().to_string();

    if !token.starts_with("moyn_") {
        return Err("Invalid token format. Token should start with 'moyn_'".to_string());
    }

    print!("Enter API URL [http://localhost:3000]: ");
    io::stdout().flush().unwrap();

    let mut url = String::new();
    io::stdin()
        .read_line(&mut url)
        .map_err(|e| format!("Could not read input: {}", e))?;
    let url = url.trim();
    let url = if url.is_empty() {
        "http://localhost:3000".to_string()
    } else {
        url.to_string()
    };

    let config = Config {
        api_token: token,
        api_url: url,
    };

    save_config(&config)?;
    println!("Logged in successfully!");
    Ok(())
}

fn extract_title(content: &str, filename: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            return trimmed[2..].trim().to_string();
        }
    }
    // Fallback to filename without extension
    PathBuf::from(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string()
}

fn publish(file: PathBuf) -> Result<(), String> {
    let config = load_config()?;

    let raw_content = fs::read_to_string(&file)
        .map_err(|e| format!("Could not read file: {}", e))?;

    let parsed = parse_frontmatter(&raw_content);

    // Use frontmatter title, or fall back to heading/filename extraction
    let title = parsed.frontmatter.title
        .unwrap_or_else(|| extract_title(&parsed.content, file.to_str().unwrap_or("post")));

    // Use frontmatter published value, or default to true
    let published = parsed.frontmatter.published.unwrap_or(true);

    let request = CreatePostRequest {
        post: CreatePost {
            title: title.clone(),
            content: raw_content,
            published,
            slug: parsed.frontmatter.slug,
            tags: parsed.frontmatter.tags,
        },
    };

    // Determine endpoint based on space
    let endpoint = match &parsed.frontmatter.space {
        Some(space) => format!("{}/api/v1/spaces/{}/posts", config.api_url, space),
        None => format!("{}/api/v1/posts", config.api_url),
    };

    let response = client(&config)
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", config.api_token))
        .json(&request)
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Failed to publish: {} - {}", status, body));
    }

    let post_response: PostResponse = response
        .json()
        .map_err(|e| format!("Could not parse response: {}", e))?;

    println!("Published: {}", post_response.post.title);
    println!("URL: {}", post_response.post.url);
    Ok(())
}

fn posts() -> Result<(), String> {
    let config = load_config()?;

    let response = client(&config)
        .get(format!("{}/api/v1/posts", config.api_url))
        .header("Authorization", format!("Bearer {}", config.api_token))
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Failed to fetch posts: {} - {}", status, body));
    }

    let posts_response: PostsResponse = response
        .json()
        .map_err(|e| format!("Could not parse response: {}", e))?;

    if posts_response.posts.is_empty() {
        println!("No posts yet.");
        return Ok(());
    }

    println!("{:<6} {:<40} {}", "ID", "TITLE", "URL");
    println!("{}", "-".repeat(80));
    for post in posts_response.posts {
        println!("{:<6} {:<40} {}", post.id, truncate(&post.title, 38), post.url);
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

fn delete(id: u64) -> Result<(), String> {
    let config = load_config()?;

    let response = client(&config)
        .delete(format!("{}/api/v1/posts/{}", config.api_url, id))
        .header("Authorization", format!("Bearer {}", config.api_token))
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().as_u16() == 204 {
        println!("Post {} deleted.", id);
        Ok(())
    } else if response.status().as_u16() == 404 {
        Err(format!("Post {} not found.", id))
    } else {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        Err(format!("Failed to delete: {} - {}", status, body))
    }
}

fn spaces() -> Result<(), String> {
    let config = load_config()?;

    let response = client(&config)
        .get(format!("{}/api/v1/spaces", config.api_url))
        .header("Authorization", format!("Bearer {}", config.api_token))
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Failed to fetch spaces: {} - {}", status, body));
    }

    let spaces_response: SpacesResponse = response
        .json()
        .map_err(|e| format!("Could not parse response: {}", e))?;

    if spaces_response.spaces.is_empty() {
        println!("No spaces yet. Create one with `moyn space create <slug>`");
        return Ok(());
    }

    println!("{:<20} {:<30} {:<10} {}", "SLUG", "NAME", "VISIBILITY", "URL");
    println!("{}", "-".repeat(75));
    for space in spaces_response.spaces {
        println!(
            "{:<20} {:<30} {:<10} {}",
            truncate(&space.slug, 18),
            truncate(&space.name, 28),
            space.visibility,
            space.url
        );
    }
    Ok(())
}

fn space_create(
    name: String,
    slug: Option<String>,
    description: Option<String>,
    visibility: String,
) -> Result<(), String> {
    let config = load_config()?;

    // Validate visibility
    if !["public", "unlisted", "private"].contains(&visibility.as_str()) {
        return Err(format!(
            "Invalid visibility '{}'. Must be one of: public, unlisted, private",
            visibility
        ));
    }

    let request = CreateSpaceRequest {
        space: CreateSpace {
            slug,
            name,
            description,
            visibility: Some(visibility),
        },
    };

    let response = client(&config)
        .post(format!("{}/api/v1/spaces", config.api_url))
        .header("Authorization", format!("Bearer {}", config.api_token))
        .json(&request)
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Failed to create space: {} - {}", status, body));
    }

    let space_response: SpaceResponse = response
        .json()
        .map_err(|e| format!("Could not parse response: {}", e))?;

    let space = space_response.space;
    println!("Created space: {}", space.name);
    println!("URL: {}", space.url);

    if let Some(token_url) = space.token_url {
        println!("Share URL: {}", token_url);
    }

    println!("\nPublish to this space:");
    println!("  Add `space: {}` to your markdown frontmatter", space.slug);

    Ok(())
}

fn space_show(slug: String) -> Result<(), String> {
    let config = load_config()?;

    let response = client(&config)
        .get(format!("{}/api/v1/spaces/{}", config.api_url, slug))
        .header("Authorization", format!("Bearer {}", config.api_token))
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().as_u16() == 404 {
        return Err(format!("Space '{}' not found or you don't have access.", slug));
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Failed to fetch space: {} - {}", status, body));
    }

    let space_response: SpaceResponse = response
        .json()
        .map_err(|e| format!("Could not parse response: {}", e))?;

    let space = space_response.space;

    println!("Space: {}", space.name);
    println!("  Slug:       {}", space.slug);
    println!("  Visibility: {}", space.visibility);
    if let Some(desc) = &space.description {
        if !desc.is_empty() {
            println!("  Description: {}", desc);
        }
    }
    println!("  URL:        {}", space.url);

    if let Some(token_url) = &space.token_url {
        println!("  Share URL:  {}", token_url);
    }

    if let Some(token) = &space.access_token {
        println!("  Access Token: {}", token);
    }

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Login => login(),
        Commands::Publish { file } => publish(file),
        Commands::Posts => posts(),
        Commands::Delete { id } => delete(id),
        Commands::Spaces => spaces(),
        Commands::Space { command } => match command {
            SpaceCommands::Create { name, slug, description, visibility } => {
                space_create(name, slug, description, visibility)
            }
            SpaceCommands::Show { slug } => space_show(slug),
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
