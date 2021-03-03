use kuchiki::{Attributes, NodeRef};
use rayon::prelude::*;
use std::cell::RefMut;
use std::str::FromStr;
use std::time::Duration;
use ureq::{Agent, AgentBuilder};
use url::Url;

// Some sites put a dummy 1x1 image in the src attribute and put the url of the real image in the
// data-src attribute and then lazy load it with javascript. This filter checks if that appears
// to be the case and replaces the fake image with the real one.
pub struct ImgDataSrcFilter<'a> {
    base: &'a Url,
    agent: Agent,
}

impl<'a> ImgDataSrcFilter<'a> {
    pub fn new(base: &'a Url) -> Self {
        Self {
            base,
            agent: AgentBuilder::new().timeout(Duration::from_secs(20)).build(),
        }
    }

    pub fn run(&self, node: &NodeRef) {
        if let Ok(select) = node.select("img") {
            for img in select {
                let mut attrs = img.attributes.borrow_mut();
                if has_data_attr(&attrs) && self.is_likely_placeholder(&attrs) {
                    self.try_resolve_src(&mut attrs);
                }
            }
        }
    }

    fn try_resolve_src(&self, attrs: &mut RefMut<Attributes>) {
        let src = if let Some(srcset) = attrs.get("srcset").or_else(|| attrs.get("data-srcset")) {
            image_from_srcset(srcset)
        } else {
            let images = self.resolve_images(attrs);
            pick_image(images).map(|image| image.url.into_string())
        };

        if let Some(src) = src {
            if let Some(current_src) = attrs.get_mut("src") {
                *current_src = src;
            } else {
                attrs.insert("src", src);
            }
        }
    }

    fn resolve_images(&self, attrs: &RefMut<Attributes>) -> Vec<ImageResource> {
        attrs
            .map
            .iter()
            .filter(|a| a.0.local.starts_with("data-"))
            .map(|a| &a.1.value)
            .filter_map(|val| {
                Url::from_str(val)
                    .or_else(|_| self.base.join(val))
                    .ok()
                    .filter(|url| url.scheme().starts_with("http"))
            })
            .collect::<Vec<Url>>()
            .into_par_iter()
            .filter_map(|url| self.load_image_resource(url))
            .collect()
    }

    fn is_likely_placeholder(&self, attrs: &RefMut<Attributes>) -> bool {
        let src = match attrs.get("src") {
            Some(src) => src,
            None => return true,
        };

        let is_inline = src.starts_with("data:");
        if is_inline {
            return true;
        }

        let url = match Url::from_str(src) {
            Ok(url) => url,
            Err(_) => return true,
        };

        println!("loading to check if placeholder: {}", url.as_str());
        self.load_image_resource(url)
            .map(|res| res.size_bytes < 2000)
            .unwrap_or(true)
    }

    fn load_image_resource(&self, url: Url) -> Option<ImageResource> {
        self.agent.head(url.as_str()).call().ok().and_then(|resp| {
            Some(ImageResource {
                url,
                size_bytes: resp.header("Content-Length")?.parse().ok()?,
                mime: resp
                    .header("Content-Type")
                    .filter(|mime| mime.starts_with("image"))?
                    .to_string(),
            })
        })
    }
}

#[derive(Debug)]
struct ImageResource {
    url: Url,
    size_bytes: usize,
    mime: String,
}

fn data_src(attrs: &RefMut<Attributes>) -> Option<String> {
    attrs.get("data-src").map(str::to_string)
}

fn image_from_srcset(attr_val: &str) -> Option<String> {
    let parts = attr_val.split_whitespace().collect::<Vec<_>>();
    let mut links = parts.chunks(2).map(parse_chunk).collect::<Vec<_>>();

    links.sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    links.last().map(|l| l.0.to_string())
}

fn parse_chunk<'a>(chunk: &'a [&str]) -> (&'a str, Option<f32>) {
    let link = chunk[0];
    let size = chunk.last().and_then(|size| {
        let mut first_num = String::new();
        for c in size.chars() {
            if c.is_digit(10) || c == '.' {
                first_num.push(c);
            } else {
                break;
            }
        }
        f32::from_str(&first_num).ok()
    });
    (link, size)
}

fn has_data_attr(attrs: &RefMut<Attributes>) -> bool {
    attrs.map.iter().any(|a| a.0.local.starts_with("data-"))
}

fn pick_image(mut images: Vec<ImageResource>) -> Option<ImageResource> {
    if images.is_empty() {
        None
    } else {
        images.sort_unstable_by_key(|i| i.size_bytes);
        Some(images.remove(images.len() - 1))
    }
}
