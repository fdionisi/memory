use synx_domain::content::{Content, ContentKind};

pub fn extract_text_content(content: &Content) -> Option<String> {
    let text_contents: Vec<String> = content
        .0
        .iter()
        .filter_map(|c| {
            if let ContentKind::Text { text } = c {
                Some(text.clone())
            } else {
                None
            }
        })
        .collect();

    if text_contents.is_empty() {
        None
    } else {
        Some(text_contents.join("\n"))
    }
}
