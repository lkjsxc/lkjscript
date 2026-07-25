use crate::semantic::schema::{TriviaAttachment, TriviaRecord};
use crate::source::SourceNode;

pub(super) fn records(node: &SourceNode) -> Vec<TriviaRecord> {
    vec![
        TriviaRecord {
            attachment: TriviaAttachment::Leading,
            lines: node.leading_trivia.clone(),
        },
        TriviaRecord {
            attachment: TriviaAttachment::BeforeClose,
            lines: node.before_close_trivia.clone(),
        },
    ]
}
