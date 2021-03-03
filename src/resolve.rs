use crate::feed::Item;
use anyhow::Result as AnyResult;
use anyhow::*;
use extrablatt::select::document::Document;
use extrablatt::DefaultExtractor;
use rayon::prelude::*;
use std::str::FromStr;
use crate::extract::{Article, ArticleExtractor};
use url::Url;
use ureq::{AgentBuilder, Agent};
use std::time::Duration;

#[derive(Debug)]
pub struct ResolvedItem {
    pub item: Item,
    pub articles: Vec<Article>,
}

pub fn resolve_items(items: Vec<Item>) -> Vec<ResolvedItem> {
    let agent = AgentBuilder::new().timeout(Duration::from_secs(20)).build();
    items
        .into_par_iter()
        .map(|item: Item| {
            let articles = fetch_articles(&agent, &item.links);
            ResolvedItem { item, articles }
        })
        .collect()
}

fn fetch_articles(agent: &Agent, links: &[String]) -> Vec<Article> {
    links
        .iter()
        .filter_map(|link| {
            let res = fetch_article(&agent, link);
            match &res {
                Err(err) => println!("{} ... Error: {} - skipping article", link, err),
                Ok(_) => println!("{} ... Ok", link),
            }
            res.ok()
        })
        .collect()
}

fn fetch_article(agent: &Agent, link: &str) -> AnyResult<Article> {
    // TODO: does this follow redirects?
    let resp = agent.get(link).call().context("failed to get article")?;
    let doc = Document::from_read(resp.into_reader())?;
    extract_article(doc, Url::from_str(link)?)
}

fn extract_article(doc: Document, url: Url) -> AnyResult<Article> {
    let article_extractor = ArticleExtractor {
        extractor: DefaultExtractor::default(),
        url,
        doc,
        print_warnings: true,
    };

    article_extractor.extract()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::path::PathBuf;

    #[test]
    fn can_extract_article() {
        let doc = get_test_article("commondreams1.html");
        let url = Url::from_str(
            "https://www.commondreams.org/news/2020/11/22/\
                vaccine-access-advocates-cautiously-optimistic-g20-summit-ends-pledge-spare-no",
        )
        .unwrap();

        let article = extract_article(doc, url).unwrap();

        assert_eq!(
            article.title.as_deref(),
            Some(
                "Vaccine Access Advocates Cautiously Optimistic \
            As G20 Summit Ends With Pledge to 'Spare No Effort' to Ensure Widespread Distribution"
            )
        );

        assert!(article
            .authors
            .contains(&"Julia Conley, staff writer".to_string()));

        // TODO: fix date being None
        assert_eq!(
            article.publishing_date.as_deref(),
            None, //Some("Sunday, November 22, 2020")
        );
    }

    fn get_test_article(file_name: &str) -> Document {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests");
        path.push("articles");
        path.push(file_name);

        let file = File::open(&path).unwrap();
        Document::from_read(&file).unwrap()
    }
}
