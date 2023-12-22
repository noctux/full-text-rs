use article_scraper::ArticleScraper;
use log::*;

mod cli;
mod config;
mod feeds;
mod webserver;

#[tokio::main]
async fn main() {
    // Parse CLI Options
    let cli_opts = cli::init();
    trace!("Parsed CLI options: {:?}", cli_opts);

    // Parse configuration file (required)
    // confique considers nonexistent files as "ok".
    // Yes, there is a TOCTOU problem, but well... this is not a safetycritical codepath
    match std::path::Path::new(&cli_opts.config_file).try_exists() {
        Ok(exists) => if !exists { panic!("File {:?} is not readable", cli_opts.config_file) }
        Err(err) => panic!("File {:?} is not readable: {:?}", cli_opts.config_file, err)
    }

    let conf = config::load_config(&cli_opts.config_file)
        .unwrap_or_else(|error| {
            panic!("Reading config failed: {:?}", error);
        });
    trace!("Parsed Config from {:?}:\n{:?}", cli_opts.config_file, conf);

    // Create a properly configured ArticleScraper instance
    let ftr_configs = if conf.fulltext_rss_filters.use_filters {
            match conf.fulltext_rss_filters.filter_path {
                Some(pathbuf) => Some(pathbuf.into_boxed_path()),
                None => panic!("setting use_filters, requires a valid filter_path")
            }
        } else {
            None
        };

    webserver::serve().await;

    let scraper = ArticleScraper::new(ftr_configs.as_deref()).await;

    // let url =  "https://news.ycombinator.com/rss";
    let url = "https://www.heise.de/security/rss/news-atom.xml";
    let extract_conf = feeds::ExtractionOpts {
        max_items: Some(5),
        keep_failed: true,
        keep_original_content: true
    };
    let feed_res = feeds::get_fulltext_feed(&scraper,url, &extract_conf).await;
    match feed_res {
        Ok(feed) => println!("{}", feed.to_string()),
        Err(e) => println!("{:?}", e)
    };
}
