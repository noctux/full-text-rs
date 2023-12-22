use std::path::PathBuf;
use structopt::StructOpt;
use stderrlog;

#[derive(StructOpt, Debug)]
#[structopt()]
pub struct CliOpt {
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    pub config_file: PathBuf,

    // Private, only used to initialize stderrlog
    /// Silence all output
    #[structopt(short = "q", long = "quiet")]
    quiet: bool,
    /// Verbose mode (-v, -vv, -vvv, etc)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,
}

pub fn init() -> CliOpt {
    let opt = CliOpt::from_args();

    // We limit logging to the the parent namespace of this module
    let parent_module = module_path!().rsplit_once("::").unwrap().0;

    // configure logging
    stderrlog::new()
        .module(parent_module)
        .module("article_scraper")
        .quiet(opt.quiet)
        .verbosity(opt.verbose)
        .init()
        .unwrap();

    return opt;
}
