use crate::extract::Article;
use crate::resolve::ResolvedItem;

pub fn html(items: &[ResolvedItem]) -> String {
    let items_html = items
        .iter()
        .map(item_to_html)
        .collect::<Vec<_>>()
        .join("<br/><hr><br/><br/><br/>");

    format!(
        "
    <!DOCTYPE html>
    <html>
        <head>
        <style>{}</style>
        </head>
        <body>{}</body>
    </html>",
        include_str!("../resources/classless.css"),
        items_html
    )
}

fn item_to_html(resolved: &ResolvedItem) -> String {
    let articles_html = resolved
        .articles
        .iter()
        .map(article_to_html)
        .collect::<Vec<_>>()
        .join("<p><hr></p>");

    format!(
        "<p><strong>RMS says:</strong></p><blockquote>{}</blockquote>{}",
        resolved.item.description, articles_html
    )
}

fn article_to_html(art: &Article) -> String {
    let link = &art.url;
    let title = art.title.as_deref().unwrap_or(link);
    let authors = art.authors.join(", ");
    let publishing_date = art.publishing_date.as_deref().unwrap_or_default();
    let html = &art.html;

    return format!(
        "<div>
        <h1>{}</h1>
        <a href=\"{}\">{}</a>
        {}
        {}
    </div>
    ",
        title,
        link,
        link,
        authors_date_elem(&authors, &publishing_date),
        html
    );

    fn authors_date_elem(authors: &str, date: &str) -> String {
        let float = if authors.is_empty() || date.is_empty() {
            "left"
        } else {
            "right"
        };

        format!(
            r#"
        <h5>
            <span style="float: left;">{}</span>
            <span style="float: {}; margin-right: 10%">{}</span>
        </h5>
        <br/>
        "#,
            authors, float, date
        )
    }
}
