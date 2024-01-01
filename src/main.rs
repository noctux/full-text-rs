use article_scraper::ArticleScraper;
use log::*;

mod cli;
mod config;
mod feeds;
mod webserver;

use cli::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    match cli_opts.cmd {
        Command::Serve {} => {
            webserver::serve(
                conf.listen,
                conf.fulltext_rss_filters.extraction_defaults,
                conf.fulltext_rss_filters.extraction_limits).await?;
        },
        Command::MakeFulltext { url } => {
            let scraper = ArticleScraper::new(ftr_configs.as_deref()).await;

            let extract_conf : feeds::ExtractionOpts = conf.fulltext_rss_filters.extraction_defaults.into();
            let effective = extract_conf.bound_by_limits(&conf.fulltext_rss_filters.extraction_limits);
            let feed_res = feeds::get_fulltext_feed(&scraper, &url, &effective).await;
            match feed_res {
                Ok(feed) => {
                    println!("{}", feed.to_string());
                },
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    Ok(())
}
