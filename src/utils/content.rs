use domain::content::{Content, ContentKind};

pub fn extract_text_content(content: &Content) -> Option<String> {
    match content {
        Content::Single(ContentKind::Text { text }) => Some(text.clone()),
        Content::Multiple(contents) => {
            let text_contents: Vec<String> = contents
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
        _ => None,
    }
}
