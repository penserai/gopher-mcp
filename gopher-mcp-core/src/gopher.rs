use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use thiserror::Error;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_RESPONSE_SIZE: u64 = 2 * 1024 * 1024; // 2 MiB

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemType {
    TextFile = '0' as isize,
    Menu = '1' as isize,
    Search = '7' as isize,
    Binary = '9' as isize,
    Gif = 'g' as isize,
    Image = 'I' as isize,
    Info = 'i' as isize,
    Html = 'h' as isize,
    Unknown = '?' as isize,
}

impl From<char> for ItemType {
    fn from(c: char) -> Self {
        match c {
            '0' => ItemType::TextFile,
            '1' => ItemType::Menu,
            '7' => ItemType::Search,
            '9' => ItemType::Binary,
            'g' => ItemType::Gif,
            'I' => ItemType::Image,
            'i' => ItemType::Info,
            'h' => ItemType::Html,
            _ => ItemType::Unknown,
        }
    }
}

impl ItemType {
    pub fn to_char(&self) -> char {
        match self {
            ItemType::TextFile => '0',
            ItemType::Menu => '1',
            ItemType::Search => '7',
            ItemType::Binary => '9',
            ItemType::Gif => 'g',
            ItemType::Image => 'I',
            ItemType::Info => 'i',
            ItemType::Html => 'h',
            ItemType::Unknown => '?',
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ItemType::TextFile => "TextFile",
            ItemType::Menu => "Menu",
            ItemType::Search => "Search",
            ItemType::Binary => "Binary",
            ItemType::Gif => "Gif",
            ItemType::Image => "Image",
            ItemType::Info => "Info",
            ItemType::Html => "Html",
            ItemType::Unknown => "Unknown",
        }
    }

    pub fn mime(&self) -> &'static str {
        match self {
            ItemType::TextFile => "text/plain",
            ItemType::Menu => "application/x-gopher-menu",
            ItemType::Binary => "application/octet-stream",
            ItemType::Gif => "image/gif",
            ItemType::Image => "image/jpeg",
            ItemType::Html => "text/html",
            _ => "text/plain",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MenuItem {
    pub itype: ItemType,
    pub display: String,
    pub selector: String,
    pub host: String,
    pub port: u16,
}

#[derive(Error, Debug)]
pub enum GopherError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Connection timed out")]
    Timeout,
}

pub struct GopherClient;

impl GopherClient {
    async fn send_raw(host: &str, port: u16, payload: &str) -> Result<String, GopherError> {
        let mut stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(format!("{}:{}", host, port)))
            .await
            .map_err(|_| GopherError::Timeout)??;

        stream.write_all(payload.as_bytes()).await?;
        stream.shutdown().await?;

        let mut buffer = Vec::new();
        timeout(
            READ_TIMEOUT,
            (&mut stream).take(MAX_RESPONSE_SIZE).read_to_end(&mut buffer),
        )
        .await
        .map_err(|_| GopherError::Timeout)??;

        Ok(String::from_utf8_lossy(&buffer).into_owned())
    }

    fn parse_menu_lines(content: &str) -> Vec<MenuItem> {
        let mut items = Vec::new();

        for line in content.lines() {
            if line == "." {
                break;
            }
            if line.is_empty() {
                continue;
            }

            let itype = ItemType::from(line.chars().next().unwrap_or('?'));
            let fields: Vec<&str> = line[1..].split('\t').collect();

            if fields.len() >= 3 {
                items.push(MenuItem {
                    itype,
                    display: fields[0].to_string(),
                    selector: fields[1].to_string(),
                    host: fields[2].to_string(),
                    port: fields.get(3).and_then(|p| p.parse().ok()).unwrap_or(70),
                });
            } else if itype == ItemType::Info {
                items.push(MenuItem {
                    itype,
                    display: fields[0].to_string(),
                    selector: String::new(),
                    host: String::new(),
                    port: 0,
                });
            }
        }

        items
    }

    pub async fn fetch_text(host: &str, port: u16, selector: &str) -> Result<String, GopherError> {
        let content = Self::send_raw(host, port, &format!("{}\r\n", selector)).await?;
        let mut lines: Vec<&str> = content.lines().collect();

        // Remove the trailing dot terminator if present
        if let Some(last) = lines.last() {
            if *last == "." {
                lines.pop();
            }
        }

        Ok(lines.join("\n"))
    }

    pub async fn fetch_menu(host: &str, port: u16, selector: &str) -> Result<Vec<MenuItem>, GopherError> {
        let content = Self::send_raw(host, port, &format!("{}\r\n", selector)).await?;
        Ok(Self::parse_menu_lines(&content))
    }

    pub async fn search(host: &str, port: u16, selector: &str, query: &str) -> Result<Vec<MenuItem>, GopherError> {
        let content = Self::send_raw(host, port, &format!("{}\t{}\r\n", selector, query)).await?;
        Ok(Self::parse_menu_lines(&content))
    }
}
