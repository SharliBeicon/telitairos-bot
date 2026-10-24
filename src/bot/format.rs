//! Convert CommonMark to Telegram entities, then split text and entity ranges together.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use teloxide::types::{MessageEntity, MessageEntityKind};

#[derive(Default)]
pub(super) struct Rendered {
    pub text: String,
    pub entities: Vec<MessageEntity>,
}

impl Rendered {
    pub fn plain(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            entities: vec![],
        }
    }

    pub fn markdown(input: &str) -> Self {
        let mut output = Self::default();
        let mut offset = 0;
        let mut spans = Vec::new();
        let mut lists = Vec::new();
        for event in Parser::new_ext(input, Options::ENABLE_STRIKETHROUGH) {
            match event {
                Event::Start(tag) => {
                    let end = tag.to_end();
                    let kind = match tag {
                        Tag::Strong => Some(MessageEntityKind::Bold),
                        Tag::Emphasis => Some(MessageEntityKind::Italic),
                        Tag::Strikethrough => Some(MessageEntityKind::Strikethrough),
                        Tag::Heading { .. } => {
                            output.newline(&mut offset, 2);
                            Some(MessageEntityKind::Bold)
                        }
                        Tag::Link { dest_url, .. } => reqwest::Url::parse(&dest_url)
                            .ok()
                            .filter(|url| {
                                matches!(url.scheme(), "http" | "https")
                                    && url.username().is_empty()
                                    && url.password().is_none()
                            })
                            .map(|url| MessageEntityKind::TextLink { url }),
                        Tag::CodeBlock(kind) => {
                            output.newline(&mut offset, 1);
                            let language = match kind {
                                CodeBlockKind::Fenced(language) => language
                                    .split_whitespace()
                                    .next()
                                    .filter(|name| name.len() <= 64)
                                    .map(str::to_owned),
                                CodeBlockKind::Indented => None,
                            };
                            Some(MessageEntityKind::Pre { language })
                        }
                        Tag::List(first) => {
                            lists.push(first);
                            None
                        }
                        Tag::Item => {
                            output.newline(&mut offset, 1);
                            let prefix = match lists.last_mut() {
                                Some(Some(number)) => {
                                    let prefix = format!("{number}. ");
                                    *number = number.saturating_add(1);
                                    prefix
                                }
                                _ => "• ".to_owned(),
                            };
                            output.append(&mut offset, &prefix);
                            None
                        }
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        spans.push((end, kind, offset));
                    }
                }
                Event::End(end) => {
                    if let Some(index) = spans.iter().rposition(|(tag, _, _)| *tag == end) {
                        let (_, kind, start) = spans.remove(index);
                        if offset > start {
                            output
                                .entities
                                .push(MessageEntity::new(kind, start, offset - start));
                        }
                    }
                    match end {
                        TagEnd::Paragraph | TagEnd::Heading(_) => {
                            output.newline(&mut offset, if lists.is_empty() { 2 } else { 1 })
                        }
                        TagEnd::CodeBlock | TagEnd::Item => output.newline(&mut offset, 1),
                        TagEnd::List(_) => {
                            lists.pop();
                            output.newline(&mut offset, 1);
                        }
                        _ => {}
                    }
                }
                Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                    output.append(&mut offset, &text)
                }
                Event::Code(text) => {
                    let start = offset;
                    output.append(&mut offset, &text);
                    if offset > start {
                        output
                            .entities
                            .push(MessageEntity::code(start, offset - start));
                    }
                }
                Event::SoftBreak | Event::HardBreak => output.append(&mut offset, "\n"),
                Event::Rule => {
                    output.newline(&mut offset, 1);
                    output.append(&mut offset, "────\n");
                }
                _ => {}
            }
        }
        if output.entities.is_empty()
            && output.text.trim_end_matches('\n') == input.trim_end_matches('\n')
        {
            return Self::plain(input);
        }
        output.text = output.text.trim_end_matches('\n').to_owned();
        let length = output.text.encode_utf16().count();
        output.entities.retain_mut(|entity| {
            entity.length = entity.length.min(length.saturating_sub(entity.offset));
            entity.length > 0
        });
        // Telegram forbids other entities inside code/pre, even if Markdown nests them.
        let code: Vec<_> = output
            .entities
            .iter()
            .filter(|entity| {
                matches!(
                    entity.kind,
                    MessageEntityKind::Code | MessageEntityKind::Pre { .. }
                )
            })
            .map(|entity| entity.offset..entity.offset + entity.length)
            .collect();
        let mut entities = Vec::new();
        for entity in output.entities {
            if matches!(
                entity.kind,
                MessageEntityKind::Code | MessageEntityKind::Pre { .. }
            ) {
                entities.push(entity);
                continue;
            }
            let mut ranges: Vec<_> =
                std::iter::once(entity.offset..entity.offset + entity.length).collect();
            for excluded in &code {
                ranges = ranges
                    .into_iter()
                    .flat_map(|range| {
                        let mut pieces = Vec::new();
                        if range.end <= excluded.start || range.start >= excluded.end {
                            pieces.push(range);
                        } else {
                            if range.start < excluded.start {
                                pieces.push(range.start..excluded.start);
                            }
                            if range.end > excluded.end {
                                pieces.push(excluded.end..range.end);
                            }
                        }
                        pieces
                    })
                    .collect();
            }
            entities.extend(ranges.into_iter().map(|range| {
                MessageEntity::new(entity.kind.clone(), range.start, range.end - range.start)
            }));
        }
        entities.sort_by_key(|entity| (entity.offset, std::cmp::Reverse(entity.length)));
        output.entities = entities;
        output
    }

    fn append(&mut self, offset: &mut usize, text: &str) {
        self.text.push_str(text);
        *offset += text.encode_utf16().count();
    }

    fn newline(&mut self, offset: &mut usize, count: usize) {
        if self.text.is_empty() {
            return;
        }
        let existing = self.text.chars().rev().take_while(|ch| *ch == '\n').count();
        for _ in existing..count {
            self.append(offset, "\n");
        }
    }

    pub fn chunks(&self) -> Vec<Self> {
        let mut offset = 0;
        let mut result = Vec::new();
        for text in super::split_text(&self.text) {
            let length = text.encode_utf16().count();
            if !text.trim().is_empty() {
                let entities = self
                    .entities
                    .iter()
                    .filter_map(|entity| {
                        let start = entity.offset.max(offset);
                        let end = (entity.offset + entity.length).min(offset + length);
                        (start < end).then(|| {
                            MessageEntity::new(entity.kind.clone(), start - offset, end - start)
                        })
                    })
                    .take(100)
                    .collect();
                result.push(Self {
                    text: text.to_owned(),
                    entities,
                });
            }
            offset += length;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_markdown_with_utf16_offsets_and_no_telegram_parse_mode() {
        let rendered = Rendered::markdown("🦀 **Hola** y *adiós* con `a < b && c`.");
        assert_eq!(rendered.text, "🦀 Hola y adiós con a < b && c.");
        assert!(rendered.entities.contains(&MessageEntity::bold(3, 4)));
        assert!(rendered
            .entities
            .iter()
            .any(|entity| entity.kind == MessageEntityKind::Italic));
        assert!(rendered
            .entities
            .iter()
            .any(|entity| entity.kind == MessageEntityKind::Code));
    }

    #[test]
    fn code_is_literal_and_is_never_nested_inside_other_entities() {
        let rendered = Rendered::markdown(
            "**antes `**literal**` después**\n\n```rust\nlet x = \"<b>&\";\n```",
        );
        assert!(rendered.text.contains("**literal**"));
        assert!(rendered.text.contains("<b>&"));
        for code in rendered.entities.iter().filter(|entity| {
            matches!(
                entity.kind,
                MessageEntityKind::Code | MessageEntityKind::Pre { .. }
            )
        }) {
            for other in &rendered.entities {
                if code != other {
                    assert!(
                        other.offset + other.length <= code.offset
                            || other.offset >= code.offset + code.length
                    );
                }
            }
        }
    }

    #[test]
    fn formatting_survives_long_unicode_messages_and_chunk_boundaries() {
        let rendered = Rendered::markdown(&format!("**{}**", "🦀".repeat(5000)));
        let chunks = rendered.chunks();
        assert_eq!(
            chunks
                .iter()
                .map(|chunk| chunk.text.as_str())
                .collect::<String>(),
            "🦀".repeat(5000)
        );
        for chunk in chunks {
            let length = chunk.text.encode_utf16().count();
            assert!(length <= 4096);
            assert_eq!(chunk.entities, vec![MessageEntity::bold(0, length)]);
        }
    }

    #[test]
    fn links_are_validated_and_html_and_plain_transcriptions_remain_literal() {
        let rendered = Rendered::markdown(
            "[Web](https://example.com) [No](javascript:alert(1)) <b>literal</b>",
        );
        assert!(rendered.text.contains("<b>literal</b>"));
        assert_eq!(rendered.entities.len(), 1);
        let literal = "**Palabras literales** y `código`\n";
        let plain = Rendered::plain(literal);
        assert_eq!(plain.text, literal);
        assert!(plain.entities.is_empty());
    }
}
