use crate::filter;
use crate::filter::{remove_all, remove_all_class};
use anyhow::Result as AnyResult;
use anyhow::*;
use extrablatt::date::Date;
use extrablatt::select::document::Document;
use extrablatt::select::node::Node;
use extrablatt::select::predicate::{Attr, Class, Name, Predicate};
use extrablatt::{Extractor, Language};
use html5ever::{local_name, namespace_url, ns, QualName};
use kuchiki::traits::*;
use kuchiki::NodeRef;
use regex::Regex;
use std::collections::HashMap;
use std::str;
use url::Url;

#[derive(Debug)]
pub struct Article {
    pub url: String,
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub publishing_date: Option<String>,
    pub html: String,
}

#[derive(Debug)]
struct ExtractionParts {
    title: Option<String>,
    authors: Vec<String>,
    publishing_date: Option<String>,
    article_node: Option<NodeRef>,
}

impl ExtractionParts {
    fn empty() -> Self {
        Self {
            title: None,
            authors: Vec::new(),
            publishing_date: None,
            article_node: None,
        }
    }

    fn with_article(article: NodeRef) -> Self {
        let mut parts = Self::empty();
        parts.article_node = Some(article);
        parts
    }
}

pub struct ArticleExtractor<E: Extractor> {
    pub extractor: E,
    pub url: Url,
    pub doc: Document,
    pub print_warnings: bool,
}

impl<E: Extractor> ArticleExtractor<E> {

    pub fn extract(&self) -> AnyResult<Article> {
        let parts = self.extract_article_parts();

        let html = self.article_html(parts.article_node)?;

        let url = self.url.to_string();

        let title = parts.title.or_else(|| self.default_title());

        let publishing_date = parts
            .publishing_date
            .or_else(|| self.default_publishing_date());

        let authors = if parts.authors.is_empty() {
            self.default_authors()
        } else {
            parts.authors
        };

        Ok(Article {
            url,
            title,
            authors,
            publishing_date,
            html,
        })
    }

    fn article_html(&self, article_node: Option<NodeRef>) -> AnyResult<String> {
        let node = article_node
            .or_else(|| self.default_article_node())
            .ok_or_else(|| anyhow!("failed to extract article html"))?;

        filter::do_global_filtering(&node, &self.url);

        Ok(node_to_html(node))
    }

    fn default_title(&self) -> Option<String> {
        self.extractor.title(&self.doc).map(|t| t.to_string())
    }

    fn default_publishing_date(&self) -> Option<String> {
        self.extractor
            .publishing_date(&self.doc, Some(&self.url))
            .map(|date| match date.published {
                Date::Date(d) => d.to_string(),
                Date::DateTime(d) => d.date().to_string(),
            })
    }

    fn default_authors(&self) -> Vec<String> {
        self.extractor
            .authors(&self.doc)
            .iter()
            .map(|a| a.to_string())
            .collect()
    }

    /// Try to extract the article node with extrablatt or if that fails try to extract it
    /// using the score_divs function.
    fn default_article_node(&self) -> Option<NodeRef> {
        self.extractor
            .article_node(&self.doc, Language::English)
            .map(|n| select_to_kuchiki(&n))
            .or_else(|| self.score_divs().get(0).map(|n| select_to_kuchiki(&n.0)))
    }

    /// Finds all <p> tags in the document and then finds their first <div>
    /// ancestor (if there is one). These divs are then scored based on how much text their
    /// descendant <p> tags contains, and sorted in descending order.
    fn score_divs(&self) -> Vec<(Node, usize)> {
        let ptags = self.doc.find(Name("p"));
        let mut text_count: HashMap<usize, (Node, usize)> = HashMap::new();

        for p in ptags {
            if let Some(div) = div_ancestor(p) {
                let mut entry = text_count.entry(div.index()).or_insert((div, 0));
                (*entry).1 += p.text().len();
            }
        }

        let mut ret = text_count.values().cloned().collect::<Vec<_>>();
        ret.sort_unstable_by_key(|v| -(v.1 as i64));
        return ret;

        fn div_ancestor(mut node: Node) -> Option<Node> {
            while let Some(parent) = node.parent() {
                if parent.name() == Some("div") {
                    return Some(parent);
                }
                node = parent;
            }
            None
        }
    }

    fn warn<T>(&self, option: Option<T>, msg: &str) -> Option<T> {
        if option.is_none() && self.print_warnings {
            println!(
                "WARNING: ({}) {} - falling back on default extractor",
                self.url, msg
            );
        }
        option
    }

    fn extract_article_parts(&self) -> ExtractionParts {
        let site_domain = site_domain(&self.url).unwrap_or_default();

        let parts = match site_domain.as_str() {
            "commondreams.org" => self.commondreams_article(),
            "theguardian.com" => self.theguardian_article(),
            "theintercept.com" => self.theintercept_article(),
            "gnu.org" => self.gnu_article(),
            "cnn.com" => self.cnn_article(),
            "theatlantic.com" => self.theatlantic_article(),
            "vice.com" => self.vice_article(),
            "dailykos.com" => self.dailykos_article(),
            "france24.com" => self.france24_article(),
            _ => None,
        };

        parts.unwrap_or_else(ExtractionParts::empty)
    }

    //
    // --- CUSTOM ARTICLE EXTRACTORS ---
    //

    fn commondreams_article(&self) -> Option<ExtractionParts> {
        let article = self.default_article_node()?;
        remove_all(&article, &["div.block-inject", "div.newswire-end"]);
        Some(ExtractionParts::with_article(article))
    }

    fn theguardian_article(&self) -> Option<ExtractionParts> {
        let article = self.default_article_node()?;

        remove_all(
            &article,
            &[
                "div.submeta",
                "div.block-share",
                "div[id^='rich-link-']",
                "div[data-component='rich-link']",
                "div[id^='guide-']",
                "div[class^='youtube-']",
                "*[class*='creditStyling']",
                "*[class*='footerStyling']",
                "*[class*='plusStyling']",
            ],
        );

        // Remove this class to prevent the entire element from being removed later.
        remove_all_class(&article, &["fig--has-shares"]);

        Some(ExtractionParts::with_article(article))
    }

    fn theintercept_article(&self) -> Option<ExtractionParts> {
        let article = self.warn(
            self.doc.find(Name("div").and(Class("PostContent"))).next(),
            "could not extract article node",
        )?;
        let article = select_to_kuchiki(&article);

        remove_all(
            &article,
            &[
                "div.NewsletterEmbed-container",
                "div.PromoteRelatedPost-promo",
            ],
        );

        Some(ExtractionParts::with_article(article))
    }

    fn gnu_article(&self) -> Option<ExtractionParts> {
        let article = self.warn(
            self.doc.find(Name("div").and(Attr("id", "content"))).next(),
            "could not extract article node",
        )?;

        let title = self.warn(
            article.find(Name("h2")).next().map(|n| n.text()),
            "could not extract title",
        );

        let article = select_to_kuchiki(&article);

        let mut parts = ExtractionParts::with_article(article);
        parts.title = title;
        Some(parts)
    }

    fn cnn_article(&self) -> Option<ExtractionParts> {
        let article = self.default_article_node()?;
        remove_all(
            &article,
            &["div.el__article--embed", "section#story-bottom"],
        );

        let wrapper = NodeRef::new_element(QualName::new(None, ns!(html), local_name!("p")), None);
        filter::wrap_all(&article, "div.zn-body__paragraph", wrapper);
        //remove_all(&article, &["div.zn-body__paragraph"]);

        //dbg!("{:#?}", article.to_string());

        Some(ExtractionParts::with_article(article))
    }

    fn theatlantic_article(&self) -> Option<ExtractionParts> {
        let article = self.warn(
            self.doc
                .find(Name("div").and(Class("l-article__container")))
                .next(),
            "could not extract article node",
        )?;
        let article = select_to_kuchiki(&article);

        Some(ExtractionParts::with_article(article))
    }

    fn vice_article(&self) -> Option<ExtractionParts> {
        let article = self.default_article_node()?;
        fix_picture_source_scaling(&article);

        return Some(ExtractionParts::with_article(article));

        fn fix_picture_source_scaling(article: &NodeRef) {
            lazy_static! {
                static ref RESIZE: Regex = Regex::new(r"resize=\d+").unwrap();
            }

            if let Ok(sel) = article.select("picture source") {
                for source in sel {
                    let mut borrow = source.attributes.borrow_mut();
                    if let Some(srcset) = borrow.get_mut("srcset") {
                        *srcset = RESIZE.replace_all(srcset, "resize=1000").to_string();
                    }
                }
            }
        }
    }

    fn dailykos_article(&self) -> Option<ExtractionParts> {
        let article = self.warn(
            self.doc
                .find(
                    Name("div")
                        .and(Attr("class", "story-column"))
                        .child(Name("noscript")),
                )
                .next(),
            "could not extract article node",
        )?;
        let article = kuchiki::parse_html().one(article.text());
        Some(ExtractionParts::with_article(article))
    }

    fn france24_article(&self) -> Option<ExtractionParts> {
        let article = self.default_article_node()?;
        remove_all(&article, &[r#"[class*="o-self-promo"]"#]);
        Some(ExtractionParts::with_article(article))
    }
}

fn site_domain(url: &Url) -> Option<String> {
    url.domain().and_then(|d| {
        let mut split = d.split('.').collect::<Vec<_>>();
        split.reverse();
        let tld = split.get(0)?;
        let site = split.get(1)?;
        Some(format!("{}.{}", site, tld))
    })
}

fn node_to_html(node: NodeRef) -> String {
    let mut html = node.to_string();

    // This is added by kuchiki so remove it. Might be a way to not make kuchiki emit this?
    let start_offset = "<html><head></head><body>".len();
    let end_offset = "</body></html>".len();

    html.replace_range(..start_offset, "");
    html.replace_range((html.len() - end_offset)..html.len(), "");

    html
}

fn select_to_kuchiki(node: &extrablatt::select::node::Node) -> NodeRef {
    kuchiki::parse_html().one(node.html())
}

#[cfg(test)]
mod tests {
    use super::*;
    use extrablatt::DefaultExtractor;

    #[test]
    fn converts_node_to_html_correctly() {
        let node = kuchiki::parse_html().one("<div>hello</div>");
        assert_eq!("<div>hello</div>", node_to_html(node));
    }

    #[test]
    fn scores_divs_correctly() {
        let html = "<html>
            <head />
            <body>
                <div><p><strong>strong text</strong></p></div>
                <div>
                    <p>Some text.</p>
                    <p>And some more.</p>
                </div>
                <div>no p</div>
            </body>
        </html>";

        let extractor = ArticleExtractor {
            extractor: DefaultExtractor::default(),
            url: Url::parse("http://www.example.com").unwrap(),
            doc: Document::from(html),
            print_warnings: false,
        };

        let score = extractor.score_divs();

        assert_eq!(score.len(), 2);
        assert_eq!(score[0].1, 24);
        assert_eq!(score[1].1, 11);
    }
}
