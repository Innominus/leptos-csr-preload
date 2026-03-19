use crate::ResolvedPreloads;
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct IndexHtmlTemplate {
    head_prefix: Arc<str>,
    tail: Arc<str>,
    existing_hrefs: BTreeSet<String>,
}

impl IndexHtmlTemplate {
    pub fn parse(index_html: impl AsRef<str>) -> Self {
        let index_html = index_html.as_ref();
        let head_close = find_head_close(index_html).unwrap_or(index_html.len());

        Self {
            head_prefix: Arc::from(&index_html[..head_close]),
            tail: Arc::from(&index_html[head_close..]),
            existing_hrefs: collect_hrefs(index_html),
        }
    }

    pub fn render(&self, asset_base: &str, preloads: Option<&ResolvedPreloads>) -> String {
        let Some(preloads) = preloads else {
            return format!("{}{}", self.head_prefix, self.tail);
        };

        let tags = preloads.preload_tags(asset_base, &self.existing_hrefs);
        if tags.is_empty() {
            return format!("{}{}", self.head_prefix, self.tail);
        }

        let mut html = String::with_capacity(self.head_prefix.len() + tags.len() + self.tail.len());
        html.push_str(&self.head_prefix);
        html.push_str(&tags);
        html.push_str(&self.tail);
        html
    }
}

pub trait HtmlResponseBackend {
    type Response;

    fn respond_html(&self, html: String) -> Self::Response;
}

pub fn render_html_with_preloads<B: HtmlResponseBackend>(
    backend: &B,
    template: &IndexHtmlTemplate,
    asset_base: &str,
    preloads: Option<&ResolvedPreloads>,
) -> B::Response {
    backend.respond_html(template.render(asset_base, preloads))
}

pub fn asset_href(asset_base: &str, file: &str) -> String {
    let base = asset_base.trim();
    if base.is_empty() || base == "/" {
        format!("/{file}")
    } else {
        format!("{}/{}", base.trim_end_matches('/'), file)
    }
}

impl ResolvedPreloads {
    pub fn preload_tags(&self, asset_base: &str, existing_hrefs: &BTreeSet<String>) -> String {
        let mut tags = String::new();

        let loader_href = asset_href(asset_base, &self.loader);
        if !self.loader.is_empty() && !existing_hrefs.contains(&loader_href) {
            tags.push_str(&format!(
                "<link rel=\"modulepreload\" href=\"{loader_href}\">"
            ));
        }

        for wasm in &self.wasm_chunks {
            let wasm_href = asset_href(asset_base, wasm);
            if existing_hrefs.contains(&wasm_href) {
                continue;
            }
            tags.push_str(&format!(
                "<link rel=\"preload\" href=\"{wasm_href}\" as=\"fetch\" type=\"application/wasm\" crossorigin>"
            ));
        }

        tags
    }
}

fn find_head_close(index_html: &str) -> Option<usize> {
    index_html.to_ascii_lowercase().find("</head>")
}

fn collect_hrefs(index_html: &str) -> BTreeSet<String> {
    let mut hrefs = BTreeSet::new();
    let mut rest = index_html;

    while let Some(position) = rest.find("href=") {
        rest = &rest[position + 5..];
        let Some(quote) = rest
            .chars()
            .next()
            .filter(|quote| *quote == '\'' || *quote == '"')
        else {
            continue;
        };
        rest = &rest[quote.len_utf8()..];
        if let Some(end) = rest.find(quote) {
            hrefs.insert(rest[..end].to_string());
            rest = &rest[end + quote.len_utf8()..];
        } else {
            break;
        }
    }

    hrefs
}
