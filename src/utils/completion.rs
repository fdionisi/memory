use std::sync::Arc;

use anyhow::Result;
use ferrochain::{
    completion::{Completion, CompletionRequest, StreamEvent},
    futures::StreamExt,
    message::{Content, Message},
};
use ferrochain_anthropic_completion::Model;
use indoc::indoc;

const SUMMARY_SYSTEM_PROMPT: &str = indoc! {"
    You are an AI assistant tasked with summarizing conversations. Your goal is to provide
    concise yet comprehensive summaries that capture the main points, key ideas, and overall
    context of the discussion.

    Guidelines:
    - Be clear, concise, and objective in your summaries.
    - Focus on the most important information and key takeaways.
    - Maintain the original tone and intent of the conversation.
    - Avoid including unnecessary details or tangential information.
    - Use neutral language and avoid editorializing.
    - Organize the summary in a logical and coherent manner.
    - Ensure the summary can stand alone and be understood without the full context.

    Your summaries should give readers a quick but thorough understanding of the conversation's
    content and progression. Adjust your level of detail based on the length and complexity of
    the original conversation.
"};

const SUMMARY_PROMPT: &str = indoc! {"
    Summarize the conversation, incorporating the new message

    <current_summary>
    {{CURRENT_SUMMARY}}
    </current_summary>

    <new_message>
    {{NEW_MESSAGE}}
    </new_message>
    "};

pub async fn generate_summary(
    completion: Arc<dyn Completion>,
    summary: String,
    content: String,
) -> Result<String> {
    let mut stream = completion
        .complete(CompletionRequest {
            model: Model::ClaudeThreeDotFiveSonnet.to_string(),
            system: Some(vec![SUMMARY_SYSTEM_PROMPT.into()]),
            messages: vec![Message {
                content: vec![SUMMARY_PROMPT
                    .replace("{{CURRENT_SUMMARY}}", &summary)
                    .replace("{{NEW_MESSAGE}}", &content)
                    .into()],
                ..Default::default()
            }],
            temperature: Some(0.2),
        })
        .await?;

    let mut summary = String::new();
    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::Start { content, .. } | StreamEvent::Delta { content, .. } => {
                match content {
                    Content::Text { text } => summary.push_str(&text),
                    Content::Image { .. } => continue,
                }
            }
            _ => continue,
        }
    }

    Ok(summary)
}
