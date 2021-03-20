mod img_data_src_filter;
mod relative_links_filter;

use html5ever::QualName;
use img_data_src_filter::ImgDataSrcFilter;
use kuchiki::NodeRef;
use relative_links_filter::RelativeLinksFilter;
use url::Url;

pub fn do_global_filtering(node: &NodeRef, url: &Url) {
    remove_all(
        node,
        &[
            "style",
            "iframe",
            "script",
            "button",
            "form",
            "aside",
            "*.ad",
            r#"[class*="share"]"#,
            r#"[class*="sharing"]"#,
            r#"[class*="video"]"#,
            r#"[class*="social"]"#,
            r#"[class*="outbrain"]"#,
        ],
    );
    remove_all_attr(node, &["style"]);

    ImgDataSrcFilter::new(url).run(node);
    RelativeLinksFilter::new(url).run(node);
}

pub fn remove_all(node: &NodeRef, selectors: &[&str]) -> usize {
    let selector = selectors.join(",");
    if let Ok(selection) = node.select(&selector) {
        // This needs to be collected to a vec first before removing nodes.
        let targets: Vec<_> = selection.collect();
        for target in &targets {
            target.as_node().detach();
        }
        targets.len()
    } else {
        0
    }
}

pub fn remove_all_attr(node: &NodeRef, attributes: &[&str]) {
    if let Ok(selection) = node.select("*") {
        for tag in selection {
            for attr in attributes {
                let mut borrow = tag.attributes.borrow_mut();
                borrow.remove(*attr);
            }
        }
    }
}

pub fn remove_all_class(node: &NodeRef, classes: &[&str]) {
    if let Ok(selection) = node.select("*") {
        for tag in selection {
            let mut borrow = tag.attributes.borrow_mut();
            if let Some(class_attr) = borrow.get_mut("class") {
                let keep = class_attr
                    .split_whitespace()
                    .filter(|class| !classes.contains(class))
                    .collect::<Vec<_>>()
                    .join(" ");
                *class_attr = keep;
            }
        }
    }
}

/// Replaces all nodes matched by `selector` with a new node created with `name`.
/// The children of the replaced node are appended to the new node.
pub fn replace_all(base: &NodeRef, selector: &str, name: &QualName) {
    if let Ok(selection) = base.select(&selector) {
        for target in selection {
            let node = target.as_node();
            match (node.next_sibling(), node.parent()) {
                (Some(sibling), _) => {
                    let new = replace_node(node, name);
                    sibling.insert_before(new);
                }
                (_, Some(parent)) => {
                    let new = replace_node(node, name);
                    parent.append(new);
                }
                _ => {}
            }
        }
    }

    fn replace_node(node: &NodeRef, name: &QualName) -> NodeRef {
        node.detach();
        let new = NodeRef::new_element(name.clone(), None);
        for child in node.children() {
            new.append(child);
        }
        new
    }
}
