mod img_data_src_filter;
mod relative_links_filter;

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

pub fn wrap_all(base: &NodeRef, selector: &str, wrapper: NodeRef) -> usize {
    if let Ok(selection) = base.select(&selector) {
        // This needs to be collected to a vec first before removing nodes.
        let targets: Vec<_> = selection.collect();
        for target in &targets {
            let node = target.as_node();
            match (node.next_sibling(), node.parent()) {
                (Some(sibling), _) => {
                    node.detach();
                    let wrapper_clone = wrapper.clone();
                    wrapper_clone.append(node.clone());
                    sibling.insert_before(wrapper_clone);
                }
                (_, Some(parent)) => {
                    node.detach();
                    let wrapper_clone = wrapper.clone();
                    wrapper_clone.append(node.clone());
                    parent.append(wrapper_clone)
                }
                _ => {
                    break;
                }
            }
        }
        targets.len()
    } else {
        0
    }
    // if let Ok(sel) = base.select(selector) {
    //     let targets = sel.iter.collect::<Vec<_>>();
    //     for tag in &targets {
    //         tag.as_node().detach();
    //         // dbg!("WRAPPING");
    //         // let wrapper_clone = wrapper.clone();
    //         // wrapper_clone.append(node.clone());
    //         // match (node.next_sibling(), node.parent()) {
    //         //     (Some(sibling), _ ) => {
    //         //         //sibling.insert_before(wrapper_clone);
    //         //     },
    //         //     (_, Some(parent)) => {
    //         //         //parent.append(wrapper_clone)
    //         //     }
    //         //     _ => {
    //         //         break;
    //         //     },
    //         // }
    //     }
    // }
}
