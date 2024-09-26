use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentKind {
    Text {
        text: String,
    },
    Image {
        image: String,
        #[serde(rename = "mimeType")]
        mime_type: Option<String>,
    },
}

#[derive(Clone, Debug, Serialize)]
pub struct Content(pub Vec<ContentKind>);

impl From<String> for Content {
    fn from(text: String) -> Self {
        Content(vec![ContentKind::Text { text }])
    }
}

impl From<Vec<String>> for Content {
    fn from(texts: Vec<String>) -> Self {
        Content(
            texts
                .into_iter()
                .map(|text| ContentKind::Text { text })
                .collect(),
        )
    }
}

impl ToString for Content {
    fn to_string(&self) -> String {
        self.0
            .iter()
            .map(|content| match content {
                ContentKind::Text { text } => text.clone(),
                ContentKind::Image { image, .. } => image.clone(),
            })
            .collect::<Vec<String>>()
            .join("\n")
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
            ContentHelper::Single(text) => Ok(Content(vec![ContentKind::Text { text }])),
            ContentHelper::Multiple(texts) => Ok(Content(
                texts
                    .into_iter()
                    .map(|text| ContentKind::Text { text })
                    .collect(),
            )),
            ContentHelper::SingleObject(content_type) => Ok(Content(vec![content_type])),
            ContentHelper::MultipleObjects(content_types) => Ok(Content(content_types)),
        }
    }
}
