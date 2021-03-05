#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

mod convert;
mod extract;
mod feed;
mod filter;
mod options;
mod resolve;
mod util;

use crate::options::Opts;
use anyhow::Result as AnyResult;
use anyhow::*;
use chrono::{Duration, Local};
use feed::Item;
use std::path::PathBuf;
use std::process;
use std::process::Command;

lazy_static! {
    static ref TEMP_FILE: PathBuf =
        std::env::temp_dir().join("123679816239the-daily-stallman.html");
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {:?}\n", err);
        process::exit(1);
    }
}

fn run() -> AnyResult<()> {
    remove_temp_file_if_exists();
    let mut opts = Opts::parse(std::env::args())?;

    if let Some(debug) = opts.debug.take() {
        let resolved = resolve::resolve_items(vec![debug]);
        if let Some(article) = resolved.first().and_then(|r| r.articles.first()) {
            println!("{:#?}", article);
        }
    } else {
        let mut items = feed::items().context("failed to get items from RSS feed")?;
        filter_items(&mut items, &opts);
        let resolved = resolve::resolve_items(items);
        let html = convert::html(&resolved);
        output_html(&html, &opts)?;
    }
    Ok(())
}

fn remove_temp_file_if_exists() {
    if TEMP_FILE.is_file() {
        let _ = std::fs::remove_file(TEMP_FILE.as_path());
    }
}

fn filter_items(items: &mut Vec<Item>, opts: &Opts) {
    let target_date = if opts.yesterday {
        Local::now() - Duration::days(1)
    } else {
        Local::now()
    }
    .date();

    items.retain(|item| item.date.map(|d| d.date()) == Some(target_date));
}

fn output_html(html: &str, opts: &Opts) -> AnyResult<()> {
    match (opts.output_file.as_ref(), opts.browser.as_ref()) {
        (Some(output), _) => {
            std::fs::write(&output, &html)?;
        }
        (_, Some(browser)) => {
            std::fs::write(TEMP_FILE.as_path(), html)?;
            Command::new(browser).arg(TEMP_FILE.as_path()).spawn()?;
        }
        _ => {}
    }
    Ok(())
}
