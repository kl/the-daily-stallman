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

use anyhow::Result as AnyResult;
use anyhow::*;
use feed::Item;
use std::process;

use chrono::{Duration, Local};

use crate::options::Opts;

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {:?}\n", err);
        process::exit(1);
    }
}

fn run() -> AnyResult<()> {
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
        std::fs::write(&opts.output_file, &html)?;
    }
    Ok(())
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
