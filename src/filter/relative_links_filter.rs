use kuchiki::{ElementData, NodeDataRef, NodeRef};
use selectors::Element;
use url::Url;

// Links and images that are relative must be made absolute.
pub struct RelativeLinksFilter<'a> {
    base: &'a Url,
}

impl<'a> RelativeLinksFilter<'a> {
    pub(crate) fn new(base: &'a Url) -> Self {
        Self { base }
    }

    pub fn run(&self, node: &NodeRef) {
        if let Ok(select) = node.select("img,a,area") {
            for elem in select {
                if elem.is_link() {
                    self.resolve_elem(elem, "href")
                } else {
                    self.resolve_elem(elem, "src")
                }
            }
        }
    }

    fn resolve_elem(&self, elem: NodeDataRef<ElementData>, attribute: &str) {
        let mut attrs = elem.attributes.borrow_mut();
        if let Some(attr) = attrs.get_mut(attribute) {
            if attr.is_empty() || attr.starts_with("http://") || attr.starts_with("https://") {
                return;
            }
            if let Ok(absolute) = self.base.join(attr) {
                *attr = absolute.into_string();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kuchiki::traits::TendrilSink;
    use std::str::FromStr;

    #[test]
    fn can_resolve_relative_links() {
        assert_eq!(
            resolve("<a href='img.png'/>", "http://example.com/folder/"),
            "http://example.com/folder/img.png"
        );

        assert_eq!(
            resolve("<img src='img.png'/>", "http://example.com/file"),
            "http://example.com/img.png"
        );

        assert_eq!(
            resolve(
                "<a href='http://site.com/img.png'/>",
                "http://example.com/file"
            ),
            "http://site.com/img.png"
        );

        assert_eq!(
            resolve(
                "<img src='../img.png'/>",
                "http://example.com/first/second/"
            ),
            "http://example.com/first/img.png"
        );

        assert_eq!(
            resolve("<a href='/img.png'/>", "http://example.com/first/second/"),
            "http://example.com/img.png"
        );

        fn resolve(elem: &str, base: &str) -> String {
            let node = kuchiki::parse_html().one(elem);
            let url = Url::from_str(base).unwrap();

            RelativeLinksFilter::new(&url).run(&node);

            let first = node.select("img,a,area").unwrap().next().unwrap();
            let attr = if first.is_link() { "href" } else { "src" };
            let attrs = first.attributes.borrow();
            attrs.get(attr).map(str::to_string).unwrap()
        }
    }
}
