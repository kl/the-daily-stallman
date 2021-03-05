use crate::feed::Item;
use anyhow::Context;
use anyhow::Result as AnyResult;
use clap::{App, Arg, ArgMatches};
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Opts {
    pub output_file: Option<PathBuf>,
    pub browser: Option<PathBuf>,
    pub yesterday: bool,
    pub debug: Option<Item>,
}

impl Opts {
    pub fn parse<I, T>(iter: I) -> AnyResult<Opts>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let matches = App::new("the-daily-stallman")
            .version(env!("CARGO_PKG_VERSION"))
            .author("Karl Lindström <kalind@posteo.se>")
            .arg(
                Arg::with_name("output")
                    .short("o")
                    .long("output")
                    .takes_value(true)
                    .help(
                        "A path (relative or absolute) to where the HTML output is written to. \
                        If the path is an existing directory, the file is placed in the directory \
                        and named tds.html",
                    ),
            )
            .arg(
                Arg::with_name("browser")
                    .short("b")
                    .long("browser")
                    .takes_value(true)
                    .conflicts_with("output")
                    .help(
                        "The name of a browser executable to open the HTML output in. If this \
                        option is set, the output is written to a temporary file that is opened \
                        automatically in the browser. The temp file is removed (or replaced) the \
                        next time `tds` is run, or when the system temp file cleaner removes it.",
                    ),
            )
            .arg(
                Arg::with_name("yesterday")
                    .long("yesterday")
                    .help("Fetches yesterday's articles instead of today's."),
            )
            .arg(
                Arg::with_name("debug")
                    .long("debug")
                    .takes_value(true)
                    .help("Prints extraction information given an article URL."),
            )
            .get_matches_from(iter);

        Ok(Opts {
            output_file: output_file(&matches)?,
            browser: browser(&matches)?,
            yesterday: matches.is_present("yesterday"),
            debug: debug(&matches),
        })
    }
}

fn output_file(matches: &ArgMatches) -> AnyResult<Option<PathBuf>> {
    if let Some(output) = matches.value_of("output") {
        let mut path = output.parse::<PathBuf>()?;
        if path.is_relative() {
            path = std::env::current_dir()?.join(path);
        }
        path = if path.is_dir() {
            path.join("tds.html")
        } else {
            path
        };
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

fn browser(matches: &ArgMatches) -> AnyResult<Option<PathBuf>> {
    if let Some(browser) = matches.value_of("browser") {
        let exe =
            which::which(browser).with_context(|| format!("is `{}` in your PATH?", browser))?;
        Ok(Some(exe))
    } else {
        Ok(None)
    }
}

fn debug(matches: &ArgMatches) -> Option<Item> {
    if let Some(url) = matches.value_of("debug") {
        Some(Item {
            title: "".to_string(),
            date: None,
            description: "".to_string(),
            links: vec![url.to_string()],
        })
    } else {
        None
    }
}
