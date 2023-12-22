use article_scraper::ArticleScraper;
use reqwest::Client;
use url::Url;
use async_trait::async_trait;

use log::*;

use std::io::Cursor;

use quick_xml::reader::Reader;
use quick_xml::events::Event;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone)]
pub struct ExtractionOpts {
    /// Whether to limit the number of items in the feed
    /// None implies no restriction
    pub max_items: Option<usize>,

    /// Whether to keep items where extraction failes
    pub keep_failed: bool,

    /// Whether to keep the original content,
    /// so only append the full-text
    pub keep_original_content: bool,
}

#[derive(Debug, Clone)]
struct NotAFeedTypeError;
impl std::error::Error for NotAFeedTypeError {}
impl std::fmt::Display for NotAFeedTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NotAFeedTypeError")
    }
}

#[derive(Debug, Clone)]
struct NoUrlError;
impl std::error::Error for NoUrlError {}
impl std::fmt::Display for NoUrlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoUrlError")
    }
}

#[derive(Debug, Clone)]
struct NoArticleError;
impl std::error::Error for NoArticleError {}
impl std::fmt::Display for NoArticleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoArticleError")
    }
}

#[async_trait]
pub trait PatchableFeed : ToString {
    /// MIME Type to use for this feed
    fn mime_type(&self) -> &'static str;

    /// Path the given feed to include full-text content
    async fn patch_feed(&mut self, article_scraper: &ArticleScraper, client: &Client, extraction_opts: &ExtractionOpts);
}

#[async_trait]
impl PatchableFeed for rss::Channel {
    fn mime_type(&self) -> &'static str {
        return "text/xml";
    }

    /* fn write_to<W: std::io::Write>(&self, writer: W) -> Result<W> {
        self.write_to(writer)
    } */


    async fn patch_feed(&mut self, article_scraper: &ArticleScraper, client: &Client, extraction_opts: &ExtractionOpts) {
        let items = self.items();

        // Handle max_items
        let len = if let Some(max_items) = extraction_opts.max_items {
            std::cmp::min(max_items, items.len())
        } else {
            items.len()
        };

        let new_items = futures::future::join_all(items[..len].iter().map(|item| async move {
            // Get fulltext
            match item_to_article(article_scraper, &client, &item).await {
                Ok(str) => {
                    let body = if extraction_opts.keep_original_content {
                        (if let Some(content) = item.content() {
                            content.to_string()
                        } else {
                            item.description().map(|text| {text.to_owned()}).unwrap_or("".to_string())
                        }) + &str
                    } else {
                        str
                    };

                    let mut new_item = item.clone();
                    new_item.set_content(Some(body));
                    Some(new_item)
                }
                Err(_e) => {
                    if extraction_opts.keep_failed {
                        Some(item.clone())
                    } else {
                        None
                    }
                },
            }
        })).await.into_iter().filter_map(|x| x).collect::<Vec<_>>();

        self.set_items(new_items);
    }
}

#[async_trait]
impl PatchableFeed for atom_syndication::Feed {
    fn mime_type(&self) -> &'static str {
        return "application/atom+xml";
    }

    async fn patch_feed(&mut self, article_scraper: &ArticleScraper, client: &Client, extraction_opts: &ExtractionOpts) {
        let items = self.entries();

        // Handle max_items
        let len = if let Some(max_items) = extraction_opts.max_items {
            std::cmp::min(max_items, items.len())
        } else {
            items.len()
        };

        let new_items = futures::future::join_all(items[..len].iter().map(|item| async move {
            // Get fulltext
            match entry_to_article(article_scraper, &client, &item).await {
                Ok(str) => {
                    let body = if extraction_opts.keep_original_content {
                        (if let Some(content) = item.content() {
                            content.value().unwrap_or("").to_string()
                        } else {
                            item.summary().map(|text| {text.value.to_owned()}).unwrap_or("".to_string())
                        }) + &str
                    } else {
                        str
                    };

                    let mut new_item = item.clone();
                    new_item.set_summary(None);
                    let mut content = atom_syndication::Content::default();
                    content.set_value(Some(body));
                    content.set_content_type(Some("html".to_string()));
                    new_item.set_content(Some(content));
                    Some(new_item)
                }
                Err(_e) => {
                    if extraction_opts.keep_failed {
                        Some(item.clone())
                    } else {
                        None
                    }
                },
            }
        })).await.into_iter().filter_map(|x| x).collect::<Vec<_>>();

        self.set_entries(new_items);
    }
}


#[derive(Debug)]
pub enum FeedType {
    AtomFeed,
    RssFeed,
}

/// Determine the feed type within `content`
pub fn determine_feed_type(content: &[u8]) -> Result<FeedType> {
    let mut reader = Reader::from_reader(Cursor::new(content));
    reader.trim_text(true);
    let mut buf = Vec::new();

    // Idea (stolen from newsboats Parser::parse_xmlnode):
    // Look at the xml's root-node,
    // atom uses "feed", RSS used "rss" and RDF uses "RDF"
    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => return Err(e.into()),
            Ok(Event::Eof) => return Err(NotAFeedTypeError.into()),
            Ok(Event::Start(e)) => {
                match e.name().as_ref() {
                    b"feed"    => return Ok(FeedType::AtomFeed),
                    b"rss"     => return Ok(FeedType::RssFeed),
                    b"rdf:RDF" => return Ok(FeedType::RssFeed),
                    _ => {
                        debug!("Feed starts with tag: {:?}", e.name());
                        return Err(NotAFeedTypeError.into());
                    },
                }
            }
            // Skip decls, comments, ...
            _ => (),
        }
        buf.clear();
    }
}

pub async fn get_fulltext_feed(scraper: &ArticleScraper, feed_url: &str, extraction_opts: &ExtractionOpts) -> Result<Box<dyn PatchableFeed + Send>> {
    let client = Client::new();
    let mut patchable = get_feed(&client, feed_url).await?;
    patchable.patch_feed(&scraper, &client, &extraction_opts).await;

    Ok(patchable)
}

/// Fetch `url` and transform it to a parsed, patchable Feed
async fn get_feed(client: &Client, url: &str) -> Result<Box<dyn PatchableFeed + Send>> {
    debug!("Fetching: {}", url);

    let content = client.get(url).send()
        .await?
        .bytes()
        .await?;

    let feedtype = determine_feed_type(&content);
    debug!("Determined FeedType: {:?}", feedtype);

    return match feedtype? {
        FeedType::RssFeed  => Ok(Box::new(rss::Channel::read_from(&content[..])?)),
        FeedType::AtomFeed => Ok(Box::new(atom_syndication::Feed::read_from(&content[..])?)),

    };
}

/// Helper converting an url to full-text content
async fn url_to_article(scraper: &ArticleScraper, client: &Client, url_str: &str) -> Result<String> {
    debug!("Retrieving fulltext for {}", url_str);
    let url = Url::parse(&url_str)?;
    let article = scraper.parse(&url, false, &client, None).await?;
    trace!("Fulltext: {:?}", article.html);
    return article.html.ok_or(NoArticleError.into())
}

async fn item_to_article(scraper: &ArticleScraper, client: &Client, item: &rss::Item) -> Result<String> {
    if let Some(url_str) = &item.link {
        url_to_article(&scraper, &client, &url_str).await
    } else {
        Err(NoUrlError.into())
    }
}

fn get_primary_link(entry: &atom_syndication::Entry) -> Option<String> {
    entry.links().into_iter().filter(|link| {
        link.rel() == "alternate"
    }).next().map(|l| {l.href().to_owned()})
}

async fn entry_to_article(scraper: &ArticleScraper, client: &Client, entry: &atom_syndication::Entry) -> Result<String> {
    if let Some(url_str) = get_primary_link(entry) {
        url_to_article(&scraper, &client, &url_str).await
    } else {
        Err(NoUrlError.into())
    }
}
