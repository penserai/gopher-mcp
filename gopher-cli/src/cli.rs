use std::io::{self, IsTerminal, Read};

use anyhow::{bail, Result};
use serde_json::json;

use crate::client::{BrowseItem, ContentClient};

/// Determine whether output should be JSON.
/// JSON is used when: --json flag is set, OR stdout is not a terminal (piped).
pub fn use_json(flag: bool) -> bool {
    flag || !io::stdout().is_terminal()
}

fn type_indicator(item_type: &str) -> &'static str {
    match item_type {
        "1" => "[+]",
        "0" => "[T]",
        "7" => "[?]",
        "h" => "[H]",
        "i" => "   ",
        _ => "[.]",
    }
}

fn print_items(items: &[BrowseItem], json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(items)?);
    } else {
        for item in items {
            if item.item_type == "i" {
                println!("      {}", item.display);
            } else {
                println!(
                    "{} {:<40} {}",
                    type_indicator(&item.item_type),
                    item.display,
                    item.path
                );
            }
        }
    }
    Ok(())
}

/// Print a structured error and exit with code 1.
pub fn handle_error(err: anyhow::Error, json: bool) -> ! {
    if json {
        let msg = format!("{:#}", err);
        eprintln!("{}", json!({ "error": msg }));
    } else {
        eprintln!("error: {:#}", err);
    }
    std::process::exit(1);
}

pub async fn browse(client: &dyn ContentClient, path: &str, json: bool) -> Result<()> {
    let items = client.browse(path).await?;
    print_items(&items, json)
}

pub async fn fetch(client: &dyn ContentClient, path: &str, json: bool) -> Result<()> {
    let text = client.fetch(path).await?;
    if json {
        println!("{}", json!({ "path": path, "content": text }));
    } else {
        print!("{}", text);
        // Ensure trailing newline for clean shell output
        if !text.ends_with('\n') {
            println!();
        }
    }
    Ok(())
}

pub async fn search(
    client: &dyn ContentClient,
    path: &str,
    query: &str,
    json: bool,
) -> Result<()> {
    let items = client.search(path, query).await?;
    print_items(&items, json)
}

pub async fn publish(
    client: &dyn ContentClient,
    path: &str,
    content: Option<String>,
    json: bool,
) -> Result<()> {
    let body = match content {
        Some(c) => c,
        None => {
            if io::stdin().is_terminal() {
                bail!("No content provided. Pass --content or pipe via stdin.");
            }
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };
    client.publish(path, &body).await?;
    if json {
        println!("{}", json!({ "ok": true, "path": path, "action": "published" }));
    } else {
        println!("Published: {}", path);
    }
    Ok(())
}

pub async fn delete(client: &dyn ContentClient, path: &str, json: bool) -> Result<()> {
    client.delete(path).await?;
    if json {
        println!("{}", json!({ "ok": true, "path": path, "action": "deleted" }));
    } else {
        println!("Deleted: {}", path);
    }
    Ok(())
}

pub async fn dump(
    client: &dyn ContentClient,
    source: &str,
    destination: &str,
    max_depth: u32,
    json: bool,
) -> Result<()> {
    let result = client.dump(source, destination, max_depth).await?;
    if json {
        println!(
            "{}",
            json!({
                "ok": true,
                "source": source,
                "destination": destination,
                "published": result.published,
                "skipped": result.skipped,
            })
        );
    } else {
        println!(
            "Dumped {} documents ({} skipped) from {} to {}",
            result.published, result.skipped, source, destination
        );
    }
    Ok(())
}
