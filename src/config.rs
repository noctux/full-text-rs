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
