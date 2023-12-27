use confique::Config;

#[derive(Config, Debug)]
pub struct Conf {
    #[config(nested)]
    pub fulltext_rss_filters: FullTextRSSFilterConf,

    #[config(nested)]
    pub server: ServerConf
}

#[derive(Config, Debug)]
pub struct FullTextRSSFilterConf {
    pub filter_path: Option<std::path::PathBuf>,

    #[config(default = true)]
    pub use_filters: bool,

    #[config(nested)]
    pub extraction_defaults: ExtractionOpts,

    #[config(nested)]
    pub extraction_limits: ExtractionLimits,
}

#[derive(Config, Debug, Clone, Copy)]
pub struct ExtractionOpts {
    pub max_items: Option<usize>,
    #[config(default = true)]
    pub keep_failed: bool,
    #[config(default = false)]
    pub keep_original_content: bool,
}

#[derive(Config, Copy, Clone, Debug)]
pub struct ExtractionLimits {
    pub max_items: Option<usize>,
}

impl Into<super::feeds::ExtractionOpts> for ExtractionOpts {
    fn into(self) -> super::feeds::ExtractionOpts {
        super::feeds::ExtractionOpts {
            max_items: self.max_items,
            keep_failed: self.keep_failed,
            keep_original_content: self.keep_original_content,
        }
    }
}

#[derive(Config, Debug)]
pub struct ServerConf {
}

pub fn load_config(file: &std::path::Path) -> Result<Conf, confique::Error>  {
    return Conf::builder()
        .env()
        .file(file)
        .load()
        ;
}
