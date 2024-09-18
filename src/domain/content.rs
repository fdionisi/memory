use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentKind {
    Text { text: String },
    Image { url: String },
}

#[derive(Clone, Serialize)]
#[serde(untagged)]
pub enum Content {
    Single(ContentKind),
    Multiple(Vec<ContentKind>),
}

impl From<String> for Content {
    fn from(text: String) -> Self {
        Content::Single(ContentKind::Text { text })
    }
}

impl From<Vec<String>> for Content {
    fn from(texts: Vec<String>) -> Self {
        Content::Multiple(
            texts
                .into_iter()
                .map(|text| ContentKind::Text { text })
                .collect(),
        )
    }
}

impl<'de> serde::Deserialize<'de> for Content {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ContentHelper {
            Single(String),
            Multiple(Vec<String>),
            SingleObject(ContentKind),
            MultipleObjects(Vec<ContentKind>),
        }

        let helper = ContentHelper::deserialize(deserializer)?;
        match helper {
            ContentHelper::Single(text) => Ok(Content::Single(ContentKind::Text { text })),
            ContentHelper::Multiple(texts) => Ok(Content::Multiple(
                texts
                    .into_iter()
                    .map(|text| ContentKind::Text { text })
                    .collect(),
            )),
            ContentHelper::SingleObject(content_type) => Ok(Content::Single(content_type)),
            ContentHelper::MultipleObjects(content_types) => Ok(Content::Multiple(content_types)),
        }
    }
}
