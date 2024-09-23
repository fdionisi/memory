use std::sync::Arc;

use anyhow::Result;
use ferrochain::{
    completion::{Completion, StreamEvent},
    futures::StreamExt,
    message::{Content, Message},
};
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

    You MUST only answer with a conversation summary.
    NEVER provide additional information or commentary beyond the conversation's content.
"};

const SUMMARY_PROMPT: &str = indoc! {"
    Consider the previous summary in between the <current_summary> tags.

    <current_summary>
    {{CURRENT_SUMMARY}}
    </current_summary>

    Now, consider the new message in between the <new_message> tags.
    <new_message role={{ROLE}}>
    {{NEW_MESSAGE}}
    </new_message>

    Write a summary of the conversation that captures the main points, key ideas, and overall context of the conversation.
    Ensure that no important information is missed and that the summary is clear, concise, and objective.
    "};

// pub async fn generate_summary(
//     completion: Arc<dyn Completion>,
//     completion_model: String,
//     summary: String,
//     role: String,
//     content: String,
// ) -> Result<String> {
//     let mut stream = completion
//         .complete(CompletionRequest {
//             model: completion_model,
//             system: Some(vec![SUMMARY_SYSTEM_PROMPT.into()]),
//             messages: vec![Message {
//                 content: vec![SUMMARY_PROMPT
//                     .replace("{{CURRENT_SUMMARY}}", &summary)
//                     .replace("{{ROLE}}", &role)
//                     .replace("{{NEW_MESSAGE}}", &content)
//                     .into()],
//                 ..Default::default()
//             }],
//             temperature: Some(0.2),
//         })
//         .await?;

//     let mut summary = String::new();
//     while let Some(event) = stream.next().await {
//         match event? {
//             StreamEvent::Start { content, .. } | StreamEvent::Delta { content, .. } => {
//                 match content {
//                     Content::Text { text } => summary.push_str(&text),
//                     Content::Image { .. } => continue,
//                 }
//             }
//             _ => continue,
//         }
//     }

//     Ok(summary)
// }
