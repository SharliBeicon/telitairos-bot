use crate::{
    bot::*,
    gpt::{Ai, CompletionKind},
    types, TelitairoBot,
};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Comandos de IA")]
pub enum AiCommand {
    #[command(
        description = "Preguntar al bot: /ask <pregunta>; lee enlaces y el mensaje respondido (incluidos adjuntos)"
    )]
    Ask(String),
    #[command(description = "Mediar: /mediate [N] [objetivo]; últimos N mensajes guardados")]
    Mediate(String),
    #[command(description = "Resumir: /summary [N]; últimos N mensajes guardados")]
    Summary(String),
    #[command(
        description = "Transcribir en el idioma original: responde a un audio o nota de voz con /transcript (máximo 20 MB; se envía a OpenAI)."
    )]
    Transcript,
}

#[derive(Debug, PartialEq)]
pub(super) struct HistoryArgs<'a> {
    count: Option<usize>,
    goal: Option<&'a str>,
}

pub(super) fn history_args(
    args: &str,
    summary: bool,
    capacity: usize,
) -> Result<HistoryArgs<'_>, String> {
    let usage = || {
        format!(
        "{} N debe ser un entero positivo de 1 a {capacity} (capacidad del historial configurada). Solo se usan los mensajes guardados disponibles. {}",
        if summary { "Usa /summary o /summary N." } else { "Usa /mediate, /mediate N, /mediate objetivo o /mediate N objetivo." },
        if summary { "Ejemplo: /summary 50. No se permite texto adicional." } else { "Ejemplos: /mediate Ayúdanos a acordar un plazo realista; /mediate 50 Ayúdanos a acordar un plazo realista. El primer elemento con apariencia numérica se reserva para N; empieza los objetivos numéricos con palabras." }
    )
    };
    let args = args.trim();
    let Some(first) = args.split_whitespace().next() else {
        return Ok(HistoryArgs {
            count: None,
            goal: None,
        });
    };
    // Reserve digit/sign-prefixed tokens, including overflow and malformed numbers.
    let numeric = first.starts_with(|ch: char| ch.is_numeric() || ch == '+' || ch == '-');
    if numeric {
        if !first.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(usage());
        }
        let count = first
            .parse::<usize>()
            .ok()
            .filter(|n| *n > 0 && *n <= capacity)
            .ok_or_else(usage)?;
        let rest = args[first.len()..].trim();
        if summary && !rest.is_empty() {
            return Err(usage());
        }
        Ok(HistoryArgs {
            count: Some(count),
            goal: (!rest.is_empty()).then_some(rest),
        })
    } else if summary {
        Err(usage())
    } else {
        Ok(HistoryArgs {
            count: None,
            goal: Some(args),
        })
    }
}

pub async fn handle_ai_commands(
    bot: Bot,
    me: teloxide::types::Me,
    buffer_store: types::BufferStore,
    telitairo_bot: TelitairoBot,
    ai: Ai,
    msg: Message,
    cmd: AiCommand,
) -> ResponseResult<()> {
    if let Some(usage) = msg.text().and_then(|text| {
        super::malformed_command_usage(text, me.username(), telitairo_bot.buffer_size)
    }) {
        return send_text(&bot, &msg, &usage).await;
    }
    if matches!(cmd, AiCommand::Transcript) {
        return super::transcript::handle(&bot, &ai, &msg).await;
    }
    let pending = super::start_pending(&bot, &msg).await?;
    let answer = match cmd {
        AiCommand::Transcript => return super::transcript::handle(&bot, &ai, &msg).await,
        AiCommand::Ask(question) if question.trim().is_empty() => {
            "Usa /ask seguido de una pregunta.".to_owned()
        }
        AiCommand::Ask(question) => {
            let mut context = std::collections::VecDeque::new();
            if let Some(reply) = msg
                .reply_to_message()
                .and_then(types::BufferedMessage::from_message)
            {
                context.push_back(reply);
            }
            let sources = crate::content::sources(&msg);
            if !sources.is_empty() {
                context.push_back(types::BufferedMessage {
                    author: "Pregunta actual".into(),
                    text: String::new(),
                    sources,
                });
            }
            let mut report = crate::content::enrich(&bot, &ai, &mut context).await;
            report.history_truncated =
                types::transcript(&context, None).lines().count() < context.len();
            let input = if context.is_empty() {
                question
            } else {
                format!("{question}\n\nContexto citado no fiable del mensaje respondido y recursos de la pregunta:\n{}", types::transcript(&context, None))
            };
            report.with_notice(
                ai.complete(&telitairo_bot, &input, CompletionKind::Ask)
                    .await,
            )
        }
        AiCommand::Mediate(ref args) | AiCommand::Summary(ref args) => {
            let summary = matches!(cmd, AiCommand::Summary(_));
            match history_args(args, summary, telitairo_bot.buffer_size) {
                Err(usage) => usage,
                Ok(args) => {
                    let mut report = crate::content::ReadReport::default();
                    match history_input(
                        &buffer_store,
                        msg.chat.id,
                        args.count,
                        summary,
                        Some((&bot, &ai, &mut report)),
                    )
                    .await
                    {
                        Ok(conversation) => report.with_notice(
                            ai.complete(
                                &telitairo_bot,
                                &conversation,
                                if summary {
                                    CompletionKind::Summary
                                } else {
                                    CompletionKind::Mediate { goal: args.goal }
                                },
                            )
                            .await,
                        ),
                        Err(explanation) => explanation.to_owned(),
                    }
                }
            }
        }
    };
    super::finish_pending(&bot, &msg, pending, &answer).await
}

async fn history_input(
    store: &types::BufferStore,
    chat: ChatId,
    count: Option<usize>,
    summary: bool,
    content: Option<(&Bot, &Ai, &mut crate::content::ReadReport)>,
) -> Result<String, &'static str> {
    let mut messages = {
        let store = store.read().await;
        store
            .get(&chat)
            .map(|buffer| {
                buffer
                    .iter()
                    .rev()
                    .take(count.unwrap_or(buffer.len()))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    messages.reverse();
    let mut messages: std::collections::VecDeque<_> = messages.into();
    // Do not download resources from messages already excluded by the transcript cap.
    let retained = types::transcript(&messages, None).lines().count();
    let original_count = messages.len();
    while messages.len() > retained {
        messages.pop_front();
    }
    if let Some((bot, ai, report)) = content {
        *report = crate::content::enrich(bot, ai, &mut messages).await;
        report.history_truncated = retained < original_count
            || types::transcript(&messages, None).lines().count() < messages.len();
    }
    let conversation = types::transcript(&messages, None);
    if conversation.is_empty() {
        Err(if summary {
            "Todavía no hay conversación guardada para resumir. Solo puedo usar los mensajes y adjuntos que recibo mientras estoy activo."
        } else {
            "Todavía no hay conversación guardada para mediar. Solo puedo usar los mensajes y adjuntos que recibo mientras estoy activo."
        })
    } else {
        Ok(conversation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::{HashMap, VecDeque},
        sync::Arc,
    };
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn empty_history_explains_command_without_network() {
        let store: types::BufferStore = Arc::new(RwLock::new(HashMap::new()));
        for summary in [false, true] {
            assert!(history_input(&store, ChatId(1), None, summary, None)
                .await
                .unwrap_err()
                .contains(if summary { "resumir" } else { "mediar" }));
            store.write().await.insert(ChatId(1), VecDeque::new());
            assert!(history_input(&store, ChatId(1), Some(50), summary, None)
                .await
                .is_err());
        }
        store
            .write()
            .await
            .get_mut(&ChatId(1))
            .unwrap()
            .push_back(types::BufferedMessage {
                author: "A".into(),
                text: "hello".into(),
                sources: vec![],
            });
        assert!(history_input(&store, ChatId(1), None, false, None)
            .await
            .unwrap()
            .contains("hello"));
        assert!(history_input(&store, ChatId(2), None, true, None)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn reads_history_attachments_on_a_snapshot_without_overwriting_the_buffer() {
        let original = types::BufferedMessage {
            author: "A".into(),
            text: "Texto original".into(),
            sources: vec![crate::content::Source::Url("file:///forbidden".into())],
        };
        let store: types::BufferStore = Arc::new(RwLock::new(HashMap::from([(
            ChatId(1),
            VecDeque::from([original.clone()]),
        )])));
        let bot = Bot::new("offline-placeholder");
        let ai = Ai::fixture();
        let mut report = crate::content::ReadReport::default();
        let context = history_input(
            &store,
            ChatId(1),
            None,
            true,
            Some((&bot, &ai, &mut report)),
        )
        .await
        .unwrap();
        assert!(context.contains("Contenido externo no fiable"));
        assert!(context.contains("Contenido no leído"));
        assert_eq!(report.unread, 1);
        assert_eq!(store.read().await[&ChatId(1)][0], original);
    }

    #[tokio::test]
    async fn does_not_read_resources_from_history_excluded_by_the_character_limit() {
        let store: types::BufferStore = Arc::new(RwLock::new(HashMap::from([(
            ChatId(1),
            VecDeque::from([
                types::BufferedMessage {
                    author: "Old".into(),
                    text: "old".into(),
                    sources: vec![crate::content::Source::Url("file:///forbidden".into())],
                },
                types::BufferedMessage {
                    author: "New".into(),
                    text: "x".repeat(types::TRANSCRIPT_LIMIT),
                    sources: vec![],
                },
            ]),
        )])));
        let bot = Bot::new("offline-placeholder");
        let ai = Ai::fixture();
        let mut report = crate::content::ReadReport::default();
        let context = history_input(
            &store,
            ChatId(1),
            None,
            true,
            Some((&bot, &ai, &mut report)),
        )
        .await
        .unwrap();
        assert_eq!(report.unread, 0);
        assert!(report.history_truncated);
        assert!(!context.contains("Old"));
        assert_eq!(context.chars().count(), types::TRANSCRIPT_LIMIT);
    }

    #[test]
    fn parses_all_history_command_forms_and_mentions() {
        for (text, count, goal) in [
            ("/mediate", None, None),
            ("/mediate 50", Some(50), None),
            (
                "/mediate Help us agree on a realistic deadline",
                None,
                Some("Help us agree on a realistic deadline"),
            ),
            (
                "/mediate 50 Help us agree on a realistic deadline",
                Some(50),
                Some("Help us agree on a realistic deadline"),
            ),
            (
                "/mediate@MYBOT 50 2026 deadline",
                Some(50),
                Some("2026 deadline"),
            ),
            ("/mediate 50   ", Some(50), None),
            ("/summary", None, None),
            ("/summary@mybot 50", Some(50), None),
        ] {
            let cmd = AiCommand::parse(text, "mybot").unwrap();
            let (args, summary) = match cmd {
                AiCommand::Mediate(args) => (args, false),
                AiCommand::Summary(args) => (args, true),
                _ => panic!("unexpected command"),
            };
            assert_eq!(
                history_args(&args, summary, 200).unwrap(),
                HistoryArgs { count, goal },
                "{text}"
            );
        }
        for text in ["/mediate@otherbot 50", "/summary@otherbot 50", "/unknown"] {
            assert!(AiCommand::parse(text, "mybot").is_err());
        }
        assert_eq!(history_args(" \n\t ", false, 200).unwrap().goal, None);
    }

    #[test]
    fn history_usage_reports_configured_capacity_without_internal_names() {
        for summary in [false, true] {
            let error = history_args("38", summary, 37).unwrap_err();
            assert!(error.contains("1 a 37 (capacidad del historial configurada)"));
            assert!(!error.contains("buffer_size"));
            assert!(!error.contains("200"));
        }
    }

    #[test]
    fn rejects_invalid_or_ambiguous_counts_and_summary_goals() {
        for summary in [false, true] {
            for args in [
                "0",
                "-1",
                "+1",
                "201",
                "999999999999999999999999999999999999999",
                "50x",
                "1.5",
                "1e2",
                "５０",
                "50,",
                "0 goal",
                "-20 goal",
            ] {
                let error = history_args(args, summary, 200).unwrap_err();
                assert!(error.contains("1 a 200"), "{args}");
                assert!(error.contains(if summary { "/summary" } else { "/mediate" }));
            }
        }
        for args in ["goal", "50 extra", "50 20"] {
            assert!(history_args(args, true, 200).is_err());
        }
        assert!(history_args("51", false, 50).is_err());
        assert_eq!(history_args("200", false, 200).unwrap().count, Some(200));
        assert_eq!(
            history_args("Discuss 2026", false, 200).unwrap().goal,
            Some("Discuss 2026")
        );
    }

    #[tokio::test]
    async fn selects_newest_available_history_in_order_for_both_commands() {
        let buffer = (1..=4)
            .map(|n| types::BufferedMessage {
                author: "A".into(),
                text: format!("message {n}"),
                sources: vec![],
            })
            .collect();
        let store: types::BufferStore = Arc::new(RwLock::new(HashMap::from([(ChatId(1), buffer)])));
        for summary in [false, true] {
            let newest = history_input(&store, ChatId(1), Some(2), summary, None)
                .await
                .unwrap();
            assert_eq!(
                newest,
                "author=\"A\"; text=\"message 3\"\nauthor=\"A\"; text=\"message 4\"\n"
            );
            let all = history_input(&store, ChatId(1), None, summary, None)
                .await
                .unwrap();
            assert_eq!(all.lines().count(), 4);
            assert_eq!(
                history_input(&store, ChatId(1), Some(50), summary, None)
                    .await
                    .unwrap(),
                all
            );
        }
    }
}
