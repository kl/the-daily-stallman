use anyhow::Result as AnyResult;
use clap::{App, Arg, ArgMatches};
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Opts {
    pub output_file: PathBuf,
    pub yesterday: bool,
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
                Arg::with_name("output_file")
                    .short("o")
                    .takes_value(true)
                    .default_value(".")
                    .help(
                        "A path (relative or absolute) to where the HTML output is written to. \
                    If the path is an existing directory, the file is placed in the directory \
                    and named tds.html",
                    ),
            )
            .arg(
                Arg::with_name("yesterday")
                    .long("yesterday")
                    .help("Fetches yesterday's articles instead of today's."),
            )
            .get_matches_from(iter);

        Ok(Opts {
            output_file: output_file(&matches)?,
            yesterday: matches.is_present("yesterday"),
        })
    }
}

fn output_file(matches: &ArgMatches) -> AnyResult<PathBuf> {
    let mut path = matches
        .value_of("output_file")
        .unwrap()
        .parse::<PathBuf>()?;
    if path.is_relative() {
        path = std::env::current_dir()?.join(path)
    }
    Ok(if path.is_dir() {
        path.join("tds.html")
    } else {
        path
    })
}
