pub mod admin;
pub mod ai;
pub(crate) mod transcript;

use crate::{types, TelitairoBot};
pub use teloxide::{prelude::*, utils::command::BotCommands};
use teloxide::{
    types::{MessageId, ReplyParameters, ThreadId},
    RequestError,
};

pub async fn handle_messages(
    bot: Bot,
    me: teloxide::types::Me,
    buffer_store: types::BufferStore,
    telitairo_bot: TelitairoBot,
    msg: Message,
) -> ResponseResult<()> {
    if let Some(usage) = msg
        .text()
        .and_then(|text| malformed_command_usage(text, me.username(), telitairo_bot.buffer_size))
    {
        return send_text(&bot, &msg, &usage).await;
    }
    if let Some(message) = types::BufferedMessage::from_message(&msg) {
        let mut store = buffer_store.write().await;
        types::push_message(
            store.entry(msg.chat.id).or_default(),
            message,
            telitairo_bot.buffer_size,
        );
    }
    Ok(())
}

fn malformed_command_usage(text: &str, username: &str, capacity: usize) -> Option<String> {
    let command = text.split_whitespace().next()?;
    let (name, mention) = command
        .split_once('@')
        .map_or((command, None), |(name, mention)| (name, Some(mention)));
    if mention.is_some_and(|mention| !mention.eq_ignore_ascii_case(username)) {
        return None;
    }
    if matches!(
        name,
        "/ask"
            | "/mediate"
            | "/summary"
            | "/mute"
            | "/ban"
            | "/unban"
            | "/unmute"
            | "/transcript"
            | "/help"
    ) && text.trim_start()[command.len()..]
        .starts_with(|ch: char| ch.is_whitespace() && ch != ' ')
    {
        return Some(format!(
            "Separa {name} de sus argumentos con un espacio normal, no con un salto de línea ni una tabulación. Usa /help para consultar la sintaxis."
        ));
    }
    match name {
        "/unban" | "/unmute" | "/transcript" if !text.trim_start()[command.len()..].trim().is_empty() => Some(format!("Usa {name} sin argumentos, como respuesta al mensaje {}.", if name == "/transcript" { "de audio o voz que quieres transcribir" } else { "del usuario" })),
        "/ask" if text.trim_start()[command.len()..].trim().is_empty() => {
            Some("Usa /ask seguido de una pregunta.".to_owned())
        }
        "/mute" | "/ban" if admin::AdminCommand::parse(text, username).is_err() => Some(
            "Usa /mute N h/m/s/p o /ban N h/m/s/p como respuesta a un usuario. N debe estar entre 0 y 255; las duraciones temporales deben ser de al menos 60 segundos. Unidades: h (horas), m (minutos), s (segundos), p (permanente). Ejemplo: /mute 10 m; permanente: /ban 0 p.".to_owned(),
        ),
        "/mediate" | "/summary" => ai::history_args(text.trim_start()[command.len()..].trim(), name == "/summary", capacity).err(),
        _ => None,
    }
}

pub fn all_command_descriptions() -> String {
    format!(
        concat!(
            "👮🚨{}\n\n🦀 🤖{}\n\n",
            "Las respuestas se dan en español, salvo /transcript, que conserva el idioma del audio sin traducir ni aplicar la personalidad. Cada fragmento responde al audio original; si se ha borrado, se intenta entregar sin respuesta. La descarga de /transcript tiene un límite exterior de 60 segundos, pero el cliente HTTP puede interrumpir antes (17 segundos por petición de forma predeterminada). La transcripción tiene un límite de 120 segundos.\n\n",
            "/ask lee enlaces compartidos y el mensaje respondido. /summary y /mediate también leen enlaces y adjuntos del historial seleccionado: páginas públicas, imágenes JPEG/PNG/WebP, PDF, archivos de texto UTF-8 y audio o voz mediante transcripción. El contenido se descarga al ejecutar el comando y se puede enviar a OpenAI, con coste adicional. Informa antes a los participantes. No se analizan vídeos ni se ejecuta JavaScript.\n\n",
            "Se intentan leer hasta 8 recursos distintos por comando. Límites: 20 MB por archivo de Telegram, 2 MB por URL, 6.000 caracteres extraídos por recurso y 24.000 en total. La lectura dispone de hasta 180 segundos en total antes de generar la respuesta. Se avisa del contenido no leído u omitido. Se bloquean URLs a redes privadas o reservadas.\n\n",
            "/mediate [N] [objetivo] y /summary [N] usan por defecto todo el historial guardado disponible. N debe estar entre 1 y la capacidad del historial configurada; si hay menos mensajes, se usan los disponibles. No se recupera historial de Telegram; el contexto enviado se limita a 50.000 caracteres. El primer elemento con apariencia numérica se reserva para N. Los objetivos complementan los criterios de mediación configurados. La mediación representa las posturas con imparcialidad; el resumen usa la personalidad configurada sin inventar información. Ejemplos: /mediate 50 Ayúdanos a acordar un plazo realista; /summary 50. /summary no admite texto adicional. Unidades de moderación: h (horas), m (minutos), s (segundos), p (permanente)."
        ),
        admin::AdminCommand::descriptions(),
        ai::AiCommand::descriptions()
    )
}

/// Split conservatively by UTF-16 units; every byte of the original is retained.
fn split_text(text: &str) -> Vec<&str> {
    let mut chunks = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let mut units = 0;
        let mut end = 0;
        let mut newline = None;
        let mut space = None;
        for (index, ch) in rest.char_indices() {
            if units + ch.len_utf16() > 4096 {
                break;
            }
            units += ch.len_utf16();
            end = index + ch.len_utf8();
            if ch == '\n' {
                newline = Some(end);
            } else if ch.is_whitespace() {
                space = Some(end);
            }
        }
        if end < rest.len() {
            end = newline.or(space).unwrap_or(end);
        }
        chunks.push(&rest[..end]);
        rest = &rest[end..];
    }
    chunks
}

pub async fn send_text(bot: &Bot, message: &Message, text: &str) -> ResponseResult<()> {
    send_text_at(bot, message, text, None).await
}

async fn send_text_at(
    bot: &Bot,
    message: &Message,
    text: &str,
    reply: Option<MessageId>,
) -> ResponseResult<()> {
    for chunk in sendable_chunks(text) {
        send_chunk(bot, message.chat.id, reply_thread(message), reply, chunk).await?;
    }
    Ok(())
}

fn reply_thread(message: &Message) -> Option<ThreadId> {
    message
        .is_topic_message
        .then_some(message.thread_id)
        .flatten()
}

fn sendable_chunks(text: &str) -> impl Iterator<Item = &str> {
    split_text(text)
        .into_iter()
        .filter(|chunk| !chunk.trim().is_empty())
}

fn retry_wait(delay: teloxide::types::Seconds) -> Option<std::time::Duration> {
    (delay.seconds() <= 30).then_some(delay.duration())
}

async fn send_chunk(
    bot: &Bot,
    chat: ChatId,
    thread: Option<ThreadId>,
    reply: Option<MessageId>,
    text: &str,
) -> ResponseResult<()> {
    // Retry the same chunk once on Telegram's explicit rate-limit response.
    for attempt in 0..2 {
        let request = text_request(bot, chat, thread, reply, text);
        match request.await {
            Err(RequestError::RetryAfter(delay)) if attempt == 0 => {
                let Some(wait) = retry_wait(delay) else {
                    return Err(RequestError::RetryAfter(delay));
                };
                tokio::time::sleep(wait).await;
            }
            result => return result.map(|_| ()),
        }
    }
    unreachable!("the final send attempt returns")
}

fn text_request(
    bot: &Bot,
    chat: ChatId,
    thread: Option<ThreadId>,
    reply: Option<MessageId>,
    text: &str,
) -> <Bot as Requester>::SendMessage {
    let mut request = bot.send_message(chat, text);
    if let Some(thread) = thread {
        request = request.message_thread_id(thread);
    }
    if let Some(reply) = reply {
        request =
            request.reply_parameters(ReplyParameters::new(reply).allow_sending_without_reply());
    }
    request
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::tests::message;
    use serde_json::json;

    #[test]
    fn transcript_chunks_reply_to_original_audio_not_command() {
        use teloxide::requests::HasPayload;
        let command = message(json!({"text":"/transcript", "message_id":50,
            "message_thread_id":99,"is_topic_message":true,
            "reply_to_message":{"message_id":2,"date":0,
            "chat":{"id":-1001,"type":"supergroup","title":"Group"},
            "message_thread_id":42,"is_topic_message":true,
            "voice":{"file_id":"audio","file_unique_id":"unique","duration":1}}}));
        let original = command.reply_to_message().unwrap();
        let text = "🦀 hello\n".repeat(2000);
        let bot = Bot::new("offline-placeholder");
        let mut count = 0;
        for chunk in sendable_chunks(&text) {
            let request = text_request(
                &bot,
                original.chat.id,
                reply_thread(original),
                Some(original.id),
                chunk,
            );
            let payload = request.payload_ref();
            let serialized = serde_json::to_value(payload).unwrap();
            assert_eq!(
                serialized["reply_parameters"]["allow_sending_without_reply"],
                true
            );
            assert_eq!(
                payload.reply_parameters.as_ref().unwrap().message_id,
                original.id
            );
            assert_ne!(
                payload.reply_parameters.as_ref().unwrap().message_id,
                command.id
            );
            assert_eq!(payload.message_thread_id, Some(ThreadId(MessageId(42))));
            assert!(payload.parse_mode.is_none());
            assert!(payload.text.encode_utf16().count() <= 4096);
            count += 1;
        }
        assert!(count > 1);
        let request = text_request(&bot, command.chat.id, None, None, "ordinary reply");
        assert!(request.payload_ref().reply_parameters.is_none());
        assert!(request.payload_ref().message_thread_id.is_none());
    }

    #[test]
    fn transcript_rejects_arguments_but_not_other_bots_commands() {
        assert!(ai::AiCommand::parse("/transcript", "mybot").is_ok());
        assert!(ai::AiCommand::parse("/transcript@MYBOT", "mybot").is_ok());
        assert!(ai::AiCommand::parse("/transcript@otherbot", "mybot").is_err());
        for command in ["/unban", "/unmute", "/transcript"] {
            for extra in [" extra", " 1 h", "\nextra", "\textra"] {
                assert!(
                    malformed_command_usage(&format!("{command}{extra}"), "mybot", 200).is_some()
                );
                assert!(malformed_command_usage(
                    &format!("{command}@otherbot{extra}"),
                    "mybot",
                    200
                )
                .is_none());
            }
        }
    }

    #[test]
    fn forwards_only_topic_thread_ids() {
        for (kind, topic) in [
            ("group", false),
            ("supergroup", false),
            ("supergroup", true),
        ] {
            let message = message(
                json!({"text":"reply", "message_thread_id":42, "is_topic_message":topic,
                "chat":{"id":-1001,"type":kind,"title":"Group","is_forum":topic}}),
            );
            assert_eq!(
                reply_thread(&message),
                if topic {
                    Some(ThreadId(teloxide::types::MessageId(42)))
                } else {
                    None
                }
            );
        }
        assert_eq!(reply_thread(&message(json!({"text":"plain"}))), None);
    }

    #[test]
    fn retry_wait_is_bounded() {
        for seconds in [0, 1, 29, 30, 31, u32::MAX] {
            let delay = teloxide::types::Seconds::from_seconds(seconds);
            assert_eq!(
                retry_wait(delay),
                if seconds <= 30 {
                    Some(delay.duration())
                } else {
                    None
                }
            );
        }
    }

    #[test]
    fn skips_only_whitespace_chunks_at_send_stage() {
        let text = format!("{}\n", "x".repeat(4096));
        assert_eq!(split_text(&text).concat(), text);
        assert_eq!(
            sendable_chunks(&text).collect::<Vec<_>>(),
            vec!["x".repeat(4096)]
        );
        assert_eq!(sendable_chunks(&" \n\t".repeat(5000)).count(), 0);
        assert_eq!(
            sendable_chunks("  meaningful \n").collect::<Vec<_>>(),
            ["  meaningful \n"]
        );
    }

    #[test]
    fn malformed_command_usage_respects_mentions_and_unknown_commands() {
        for text in ["/mute", "/ban 256 h", "/mute 1 x", "/ban@mybot -1 h"] {
            assert!(malformed_command_usage(text, "mybot", 200)
                .unwrap()
                .contains("N debe estar entre 0 y 255"));
        }
        for text in [
            "/mediate 0",
            "/summary extra",
            "/summary 50 extra",
            "/mediate -1",
        ] {
            assert!(malformed_command_usage(text, "mybot", 200).is_some());
        }
        for text in ["/ask", "/ask   "] {
            assert_eq!(
                malformed_command_usage(text, "mybot", 200).unwrap(),
                "Usa /ask seguido de una pregunta."
            );
        }
        for text in [
            "/mute 1 h",
            "/ask una pregunta",
            "/ask@otherbot",
            "/ban@otherbot",
            "/unknown",
            "/mediate",
            "/mediate extra",
            "/mediate 50 goal",
            "/summary 50",
            "/summary@otherbot bad",
            "/mediate@otherbot -1",
            "ordinary text",
            "",
        ] {
            assert_eq!(malformed_command_usage(text, "mybot", 200), None);
        }
    }

    #[test]
    fn non_space_command_separators_get_spanish_feedback() {
        for command in [
            "/ask",
            "/mediate",
            "/summary",
            "/mute",
            "/ban",
            "/unban",
            "/unmute",
            "/transcript",
            "/help",
        ] {
            for separator in ["\n", "\t"] {
                for mention in ["", "@MYBOT"] {
                    let text = format!("{command}{mention}{separator}50 objetivo");
                    assert!(
                        malformed_command_usage(&text, "mybot", 37)
                            .unwrap()
                            .contains("con un espacio normal"),
                        "{text:?}"
                    );
                }
                let text = format!("{command}@otherbot{separator}50 objetivo");
                assert_eq!(malformed_command_usage(&text, "mybot", 37), None);
            }
        }
        for text in [
            "/mediate",
            "/summary",
            "/help",
            "/mediate 20 objetivo\ncon\ttabulación",
            "/summary 20",
            "/ask pregunta\nen varias líneas",
            "/mute 1 h",
            "/ban 0 p",
            "/unknown\n50",
        ] {
            assert_eq!(malformed_command_usage(text, "mybot", 37), None, "{text:?}");
        }
        assert!(ai::AiCommand::parse("/mediate\n50 objetivo", "mybot").is_err());
        assert!(ai::AiCommand::parse("/summary\t20", "mybot").is_err());
    }

    #[test]
    fn command_descriptions_are_plain_text() {
        let help = all_command_descriptions();
        for command in [
            "/help",
            "/mute",
            "/ban",
            "/unban",
            "/unmute",
            "/transcript",
            "/ask",
            "/mediate",
            "/summary",
        ] {
            assert!(help.contains(command));
        }
        assert!(!help.contains('\\'));
        assert!(!help.contains("buffer_size"));
        assert!(!help.contains("200"));
        assert!(help.contains("capacidad del historial configurada"));
        for expected in [
            "Comandos de administración",
            "Mostrar esta ayuda",
            "Comandos de IA",
            "Preguntar al bot",
            "Mediar",
            "Resumir",
            "Las respuestas se dan en español",
            "horas",
            "minutos",
            "segundos",
            "permanente",
        ] {
            assert!(help.contains(expected), "{expected}");
        }
    }

    #[test]
    fn splits_unicode_without_loss_or_empty_chunks() {
        for text in [
            String::new(),
            "a".repeat(4096),
            "x".repeat(8193),
            "🦀".repeat(5000),
            "hello 🦀 world\n".repeat(1000),
            format!("{}🦀z", "x".repeat(4095)),
        ] {
            let chunks = split_text(&text);
            assert_eq!(chunks.concat(), text);
            assert!(chunks
                .iter()
                .all(|chunk| !chunk.is_empty() && chunk.encode_utf16().count() <= 4096));
        }
    }

    #[test]
    fn prefers_newline_then_space() {
        let text = format!("first\n{}", "x".repeat(4096));
        assert_eq!(split_text(&text)[0], "first\n");
        let text = format!("first {}", "x".repeat(4096));
        assert_eq!(split_text(&text)[0], "first ");
    }
}
