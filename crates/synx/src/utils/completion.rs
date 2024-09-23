use indoc::indoc;

pub const SUMMARY_PROMPT: &str = indoc! {"
    Consider the previous summary in between the <current_summary> tags.
    <current_summary>
    {{CURRENT_SUMMARY}}
    </current_summary>

    Now, consider the new message in between the <new_message> tags.
    <new_message role=\"{{ROLE}}\">
    {{NEW_MESSAGE}}
    </new_message>

    Write a summary of the conversation that captures the main points, key ideas, and overall context of the conversation.
    Ensure that no important information is missed and that the summary is clear, concise, and objective.
    "};
