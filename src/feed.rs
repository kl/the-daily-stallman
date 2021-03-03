use anyhow::Result as AnyResult;
use anyhow::*;
use chrono::{DateTime, Local};
use kuchiki::traits::TendrilSink;
use rss::Channel;
use std::io::Read;
use std::time::Duration;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A stallman.org news feed item.
#[derive(Debug)]
pub struct Item {
    /// The news item title.
    pub title: String,
    /// The news item date. Note that this is not the date the article was written but when
    /// it was added to the feed.
    pub date: Option<DateTime<Local>>,
    /// RMS's comment on the article. Also contains the actual link(s) to the article(s).
    pub description: String,
    /// The links to all articles mentioned in the description.
    /// May also be plain file names (without http:// or https://) in which case a test file
    /// with that name will be used.
    pub links: Vec<String>,
}

static FEED_URL: &str = "https://stallman.org/rss/rss.xml";

/// Returns all items in the stallman.org news feed.
pub fn items() -> AnyResult<Vec<Item>> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .build();
    let resp = agent
        .get(&FEED_URL)
        .call()
        .context("failed to get RSS feed")?;

    let mut reader = resp.into_reader();
    let mut bytes = vec![];
    reader.read_to_end(&mut bytes)?;
    parse_feed(&bytes)
}

fn parse_feed(feed: &[u8]) -> AnyResult<Vec<Item>> {
    let channel = Channel::read_from(feed)?;

    channel
        .items()
        .iter()
        .map(|rss_item| {
            Ok(Item {
                title: rss_item.title().unwrap_or("<Untitled>").to_string(),
                date: parse_date(&rss_item),
                description: rss_item
                    .description()
                    .unwrap_or("<No description>")
                    .to_string(),
                links: parse_article_links(rss_item)?,
            })
        })
        .collect()
}

fn parse_date(item: &rss::Item) -> Option<DateTime<Local>> {
    item.pub_date()
        .and_then(|date| DateTime::parse_from_rfc2822(date).ok().map(DateTime::from))
}

// Stallman puts the article links in the description so we get the links from there. Any links
// that link to stallman.org are ignored.
fn parse_article_links(rss_item: &rss::Item) -> AnyResult<Vec<String>> {
    let desc = rss_item.description().unwrap_or("");

    let html = kuchiki::parse_html().one(desc);

    let mut links = html
        .select("a")
        .map_err(|_| anyhow!("failed to parse item description as html"))?
        .filter_map(|a| {
            let attrs = a.attributes.borrow();
            attrs.get("href").map(str::to_string)
        })
        .filter(|a| !a.starts_with("https://stallman.org") && !a.starts_with("http://stallman.org"))
        .collect::<Vec<String>>();

    // Sometimes RMS gets a bit silly and puts multiple links to the same article so we use
    // we remove any duplicated links while retaining the link order here.
    let mut seen = Vec::new();
    links.retain(|link| {
        let mut hasher = DefaultHasher::new();
        link.hash(&mut hasher);
        let hash = hasher.finish();

        let retain = !seen.contains(&hash);
        if retain {
            seen.push(hash);
        }
        retain
    });

    Ok(links)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_parse_article_links() {
        let desc = "<p>\nThe United States, European Union, United Kingdom, Canada, \
            and\nAustralia oppose the push for the WTO to waive patent restrictions and\n<a \
            href=\"https://www.commondreams.org/news/2020/11/20/rejecting-wto-drug-patent-waivers-\
            amid-pandemic-richest-nations-put-big-pharma\">allow all countries to make and use \
            Covid-19 vaccines without paying\nfor the privilege</a>.\n\n<p>\nIt is worth reminding \
            people that the WTO is the reason why most\ncountries allow medicines to be patented.  \
            That was a scheme to enrich\nbig pharma companies at the expense of people who can\'t \
            afford\nmonopolistic prices for drugs.  This system represents a decision to\nkill \
            millions of people, and is one of the reasons why we ought to\nabolish the WTO.\n\n\
            <p>\nCovid-19 vaccine developers are keeping the techniques of making them\nsecret \
            and <a href=\"https://www.theguardian.com/world/2020/nov/22/hackers-try-to-steal-covid\
            -vaccine-secrets-in-intellectual-property-war\">have the gall to criticize people for \
            trying to get those\nsecrets</a>.\n\n<p>\nThis information should be made available to \
            every would-be vaccine\nmanufacturer.\n\n<p>\nBoth of these articles used the \
            misleading term <a href=\"https://gnu.org/philosophy/not-ipr.html\">\"intellectual \
            property.\"</a>\n  The first uses it to mean patents.  The second uses it to mean \
            trade\nsecrets.  Patents and trade secrets are totally different and have\nnothing \
            whatsoever in common.\n\n<p>\nThe term lumps together patents with copyrights with \
            trade secrets\nwith trademarks, and some other things as well.  These laws are\n\
            totally different, so the term is sophisticated-sounding confusion.\n\n<p>\nWhen \
            someone uses the term \"intellectual property\", understand it to\nmean, \"I don\'t \
            know what I am talking about.\"\n\n\n\n";

        let mut item = rss::Item::default();
        item.set_description(desc.to_string());

        assert_eq!(
            parse_article_links(&item).unwrap(),
            vec![
                "https://www.commondreams.org/news/2020/11/20/rejecting-wto-drug-patent-\
                        waivers-amid-pandemic-richest-nations-put-big-pharma",
                "https://www.theguardian.com/world/2020/nov/22/hackers-try-to-steal-covid-\
                        vaccine-secrets-in-intellectual-property-war",
                "https://gnu.org/philosophy/not-ipr.html"
            ]
        )
    }

    #[test]
    fn stallman_org_links_are_removed() {
        let desc = r#"I'm a <a href="https://stallman.org/archives/2018-sep-dec.html#26_October_
            2018_(Khashoggi_admission)">bad</a> link."#;

        let mut item = rss::Item::default();
        item.set_description(desc.to_string());

        assert!(parse_article_links(&item).unwrap().is_empty());
    }
}
