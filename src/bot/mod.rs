pub mod admin;
pub mod ai;
mod format;
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
            "{}\n\n{}\n\n",
            "Idioma y acceso\n",
            "Las respuestas se dan en español. Solo puedes usar el bot en los grupos permitidos.\n",
            "/transcript conserva el idioma del audio. No traduce ni aplica la personalidad del bot.\n\n",
            "Historial\n",
            "En /mediate [N] [objetivo] y /summary [N], N indica cuántos mensajes recientes usar. ",
            "N debe estar entre 1 y la capacidad del historial configurada. Si hay menos mensajes, se usan los disponibles. ",
            "Si omites N, se usa todo el historial guardado disponible. El contexto tiene un límite de 50.000 caracteres. ",
            "El bot no recupera mensajes antiguos de Telegram. El historial se comparte entre los temas del grupo y se borra al reiniciar.\n",
            "Si el primer elemento empieza por un número o un signo, el bot lo interpreta como N. ",
            "Si tu objetivo empieza por un número, escribe una palabra antes. ",
            "El objetivo se añade a los criterios de mediación configurados, no los sustituye. ",
            "La mediación trata las posturas con imparcialidad. El resumen usa la personalidad configurada sin inventar información. ",
            "/summary no admite un objetivo ni texto adicional.\n",
            "Ejemplos:\n/mediate 50 Ayúdanos a acordar un plazo realista\n/summary 50\n\n",
            "Enlaces y archivos\n",
            "/ask lee los enlaces compartidos y el mensaje al que respondes. ",
            "/summary y /mediate también leen enlaces y adjuntos de los mensajes seleccionados. ",
            "Se admiten páginas públicas, imágenes JPEG/PNG/WebP, PDF, archivos de texto UTF-8 y audio o voz mediante transcripción. ",
            "El bot descarga el contenido al ejecutar el comando. No analiza vídeos ni ejecuta JavaScript.\n",
            "Cada comando intenta leer hasta 8 recursos distintos. Límites: 20 MB por archivo de Telegram y 2 MB por URL. ",
            "Se extraen hasta 6.000 caracteres por recurso y 24.000 en total. La lectura tiene un límite total de 180 segundos. ",
            "El bot avisa si no puede leer contenido o si lo omite. Bloquea enlaces a redes privadas o reservadas.\n\n",
            "Audio\n",
            "Responde al audio con /transcript, sin argumentos. Cada parte del resultado responde al audio original. ",
            "Si el audio se borra, el bot intenta enviar el resultado sin enlazarlo al mensaje. ",
            "La descarga tiene un límite total de 60 segundos. El cliente HTTP puede detenerla antes: su límite predeterminado es de 17 segundos por petición. ",
            "La transcripción tiene un límite de 120 segundos.\n\n",
            "Moderación\n",
            "Responde al mensaje del usuario al que quieras aplicar la acción. Necesitas ser propietario o administrador con permiso para restringir miembros. ",
            "El bot también necesita ese permiso. Envía el comando desde tu cuenta personal, no como administrador anónimo ni como canal. ",
            "N debe estar entre 0 y 255. Unidades: h (horas), m (minutos), s (segundos), p (permanente). ",
            "Las acciones temporales duran al menos 60 segundos. Con p, el valor de N no se usa. ",
            "En grupos básicos solo se permiten expulsiones permanentes. /mute, /unmute y /unban requieren un supergrupo. ",
            "/unmute restaura los permisos predeterminados del grupo. /unban permite volver con una invitación, pero no añade al usuario.\n",
            "Ejemplos:\n/mute 10 m\n/ban 0 p\n\n",
            "Privacidad y coste\n",
            "Informa a los participantes antes de usar los comandos de IA. El texto, los nombres de autores y los archivos seleccionados se pueden enviar a OpenAI. ",
            "Para /transcript, informa también a quien habla en el audio. Cualquier miembro de un grupo permitido puede usar estos comandos y generar costes. ",
            "No hay cuotas por usuario ni controles de gasto."
        ),
        admin::AdminCommand::descriptions(),
        ai::AiCommand::descriptions()
    )
}

/// Split text within the UTF-16 limit without losing any original bytes.
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

async fn start_pending(bot: &Bot, message: &Message) -> ResponseResult<MessageId> {
    send_chunk(
        bot,
        message.chat.id,
        reply_thread(message),
        Some(message.id),
        "⏳ Procesando…",
    )
    .await
}

async fn finish_pending(
    bot: &Bot,
    message: &Message,
    pending: MessageId,
    text: &str,
) -> ResponseResult<()> {
    finish_rendered(bot, message, pending, format::Rendered::plain(text)).await
}

async fn finish_markdown(
    bot: &Bot,
    message: &Message,
    pending: MessageId,
    text: &str,
) -> ResponseResult<()> {
    finish_rendered(bot, message, pending, format::Rendered::markdown(text)).await
}

async fn finish_rendered(
    bot: &Bot,
    message: &Message,
    pending: MessageId,
    rendered: format::Rendered,
) -> ResponseResult<()> {
    let mut chunks = rendered.chunks();
    if chunks.is_empty() {
        chunks.push(format::Rendered::plain(
            "No he recibido una respuesta válida. Inténtalo de nuevo.",
        ));
    }
    let edit = edit_chunk(bot, message.chat.id, pending, &chunks[0]).await;
    if let Err(error @ RequestError::RetryAfter(_)) = edit {
        return Err(error);
    }
    if edit.is_err() {
        // Someone can delete the notice during the paid operation.
        // Keep the answer even if Telegram no longer allows edits to that message.
        log::warn!("Could not edit AI placeholder; sending the answer as new messages");
    }
    for chunk in chunks.iter().skip(usize::from(edit.is_ok())) {
        send_chunk_entities(
            bot,
            message.chat.id,
            reply_thread(message),
            Some(message.id),
            &chunk.text,
            &chunk.entities,
        )
        .await?;
    }
    if edit.is_err() && bot.delete_message(message.chat.id, pending).await.is_err() {
        log::warn!("Could not remove AI placeholder after fallback delivery");
    }
    Ok(())
}

fn edit_request(
    bot: &Bot,
    chat: ChatId,
    pending: MessageId,
    text: &str,
) -> <Bot as Requester>::EditMessageText {
    bot.edit_message_text(chat, pending, text)
}

async fn edit_chunk(
    bot: &Bot,
    chat: ChatId,
    pending: MessageId,
    chunk: &format::Rendered,
) -> ResponseResult<()> {
    let mut use_entities = !chunk.entities.is_empty();
    let mut rate_retried = false;
    for _ in 0..3 {
        let mut request = edit_request(bot, chat, pending, &chunk.text);
        if use_entities {
            request = request.entities(chunk.entities.clone());
        }
        match request.await {
            Err(RequestError::Api(teloxide::ApiError::MessageNotModified)) => return Ok(()),
            Err(RequestError::Api(teloxide::ApiError::CantParseEntities(_))) if use_entities => {
                use_entities = false;
            }
            Err(RequestError::RetryAfter(delay)) if !rate_retried => {
                let Some(wait) = retry_wait(delay) else {
                    return Err(RequestError::RetryAfter(delay));
                };
                rate_retried = true;
                tokio::time::sleep(wait).await;
            }
            result => return result.map(|_| ()),
        }
    }
    unreachable!("the final edit attempt returns")
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
) -> ResponseResult<MessageId> {
    send_chunk_entities(bot, chat, thread, reply, text, &[]).await
}

async fn send_chunk_entities(
    bot: &Bot,
    chat: ChatId,
    thread: Option<ThreadId>,
    reply: Option<MessageId>,
    text: &str,
    entities: &[teloxide::types::MessageEntity],
) -> ResponseResult<MessageId> {
    // Retry once after a request limit. Drop formatting only if Telegram rejects the entities.
    let mut use_entities = !entities.is_empty();
    let mut rate_retried = false;
    for _ in 0..3 {
        let mut request = text_request(bot, chat, thread, reply, text);
        if use_entities {
            request = request.entities(entities.to_vec());
        }
        match request.await {
            Err(RequestError::Api(teloxide::ApiError::CantParseEntities(_))) if use_entities => {
                use_entities = false;
            }
            Err(RequestError::RetryAfter(delay)) if !rate_retried => {
                let Some(wait) = retry_wait(delay) else {
                    return Err(RequestError::RetryAfter(delay));
                };
                rate_retried = true;
                tokio::time::sleep(wait).await;
            }
            result => return result.map(|message| message.id),
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

    async fn telegram_fixture(
        responses: Vec<serde_json::Value>,
    ) -> (
        Bot,
        tokio::task::JoinHandle<Vec<(String, serde_json::Value)>>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for response in responses {
                let (socket, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                let (header_end, length) = loop {
                    socket.readable().await.unwrap();
                    let mut chunk = [0u8; 8192];
                    match socket.try_read(&mut chunk) {
                        Ok(0) => panic!("Telegram fixture request ended early"),
                        Ok(n) => bytes.extend_from_slice(&chunk[..n]),
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
                        Err(error) => panic!("{error}"),
                    }
                    if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                        let headers = std::str::from_utf8(&bytes[..end]).unwrap();
                        let length: usize = headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse().unwrap())
                            })
                            .unwrap();
                        if bytes.len() >= end + 4 + length {
                            break (end + 4, length);
                        }
                    }
                };
                let path = std::str::from_utf8(&bytes[..header_end])
                    .unwrap()
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .to_owned();
                let body = serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap();
                requests.push((path, body));
                let body = serde_json::to_string(&response).unwrap();
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                let mut rest = response.as_bytes();
                while !rest.is_empty() {
                    socket.writable().await.unwrap();
                    match socket.try_write(rest) {
                        Ok(n) => rest = &rest[n..],
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
                        Err(error) => panic!("{error}"),
                    }
                }
            }
            requests
        });
        (
            Bot::new("offline-placeholder").set_api_url(url.parse().unwrap()),
            server,
        )
    }

    fn telegram_message(id: i32) -> serde_json::Value {
        json!({"ok":true,"result":crate::types::tests::message(json!({"message_id":id,"text":"fixture"}))})
    }

    #[tokio::test]
    async fn placeholder_is_edited_and_only_remaining_chunks_are_sent_in_the_same_topic() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let command = crate::types::tests::message(json!({"message_id":50,"text":"/ask hola","message_thread_id":42,"is_topic_message":true}));
            let answer = "Respuesta 🦀\n".repeat(1000);
            let count = sendable_chunks(&answer).count();
            let (bot, server) = telegram_fixture((0..=count).map(|n| telegram_message(900 + n as i32)).collect()).await;
            let pending = start_pending(&bot, &command).await.unwrap();
            assert_eq!(pending, MessageId(900));
            finish_pending(&bot, &command, pending, &answer).await.unwrap();
            let requests = server.await.unwrap();
            assert!(requests[0].0.to_ascii_lowercase().ends_with("/sendmessage"));
            assert_eq!(requests[0].1["text"], "⏳ Procesando…");
            assert_eq!(requests[0].1["reply_parameters"]["message_id"], 50);
            assert_eq!(requests[0].1["message_thread_id"], 42);
            assert!(requests[1].0.to_ascii_lowercase().ends_with("/editmessagetext"));
            assert_eq!(requests[1].1["message_id"], 900);
            let mut delivered = requests[1].1["text"].as_str().unwrap().to_owned();
            for (_, payload) in &requests[2..] {
                assert_eq!(payload["reply_parameters"]["message_id"], 50);
                assert_eq!(payload["message_thread_id"], 42);
                delivered.push_str(payload["text"].as_str().unwrap());
            }
            assert_eq!(delivered, answer);
        }).await.unwrap();
    }

    #[tokio::test]
    async fn deleted_placeholder_falls_back_to_a_new_reply_without_losing_the_answer() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let command = crate::types::tests::message(json!({"message_id":50,"text":"/summary"}));
            let (bot, server) = telegram_fixture(vec![
                json!({"ok":false,"error_code":400,"description":"Bad Request: message to edit not found"}),
                telegram_message(901), json!({"ok":true,"result":true}),
            ]).await;
            finish_markdown(&bot, &command, MessageId(900), "**Resultado final**").await.unwrap();
            let requests = server.await.unwrap();
            assert_eq!(requests[1].1["text"], "Resultado final");
            assert_eq!(requests[0].1["entities"], json!([{"type":"bold","offset":0,"length":15}]));
            assert_eq!(requests[1].1["entities"], requests[0].1["entities"]);
            assert_eq!(requests[1].1["reply_parameters"]["message_id"], 50);
            assert!(requests[2].0.to_ascii_lowercase().ends_with("/deletemessage"));
            assert_eq!(requests[2].1["message_id"], 900);
        }).await.unwrap();
    }

    #[tokio::test]
    async fn provider_errors_replace_the_placeholder_instead_of_leaving_it_pending() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let audio = crate::types::tests::message(json!({"message_id":2}));
            let (bot, server) = telegram_fixture(vec![telegram_message(900)]).await;
            finish_pending(
                &bot,
                &audio,
                MessageId(900),
                "No he podido transcribir el audio.",
            )
            .await
            .unwrap();
            let requests = server.await.unwrap();
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].1["message_id"], 900);
            assert_eq!(requests[0].1["text"], "No he podido transcribir el audio.");
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn long_edit_rate_limits_do_not_trigger_an_immediate_fallback_send() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let command = crate::types::tests::message(json!({"message_id":50,"text":"/summary"}));
            let (bot, server) = telegram_fixture(vec![json!({"ok":false,"error_code":429,"description":"Too Many Requests: retry after 60","parameters":{"retry_after":60}})]).await;
            let result = finish_pending(&bot, &command, MessageId(900), "Resultado").await;
            assert!(matches!(result, Err(RequestError::RetryAfter(_))));
            assert_eq!(server.await.unwrap().len(), 1);
        }).await.unwrap();
    }

    #[tokio::test]
    async fn formatted_answers_apply_entities_to_edits_and_continuation_messages() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let command = crate::types::tests::message(json!({"message_id":50,"text":"/ask hola"}));
            let (bot, server) =
                telegram_fixture(vec![telegram_message(900), telegram_message(901)]).await;
            finish_markdown(
                &bot,
                &command,
                MessageId(900),
                &format!("**{}**", "🦀".repeat(3000)),
            )
            .await
            .unwrap();
            let requests = server.await.unwrap();
            for (_, payload) in &requests {
                let length = payload["text"].as_str().unwrap().encode_utf16().count();
                assert_eq!(
                    payload["entities"],
                    json!([{"type":"bold","offset":0,"length":length}])
                );
                assert!(payload.get("parse_mode").is_none());
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn rejected_formatting_retries_the_edit_without_entities() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let command = crate::types::tests::message(json!({"message_id":50,"text":"/ask hola"}));
            let (bot, server) = telegram_fixture(vec![
                json!({"ok":false,"error_code":400,"description":"Bad Request: can't parse entities: invalid entity"}),
                telegram_message(900),
            ]).await;
            finish_markdown(&bot, &command, MessageId(900), "**Hola**").await.unwrap();
            let requests = server.await.unwrap();
            assert_eq!(requests[0].1["text"], "Hola");
            assert_eq!(requests[0].1["entities"], json!([{"type":"bold","offset":0,"length":4}]));
            assert_eq!(requests[1].1["text"], "Hola");
            assert!(requests[1].1.get("entities").is_none());
            assert_eq!(requests[1].1["message_id"], 900);
        }).await.unwrap();
    }

    #[tokio::test]
    async fn transcription_delivery_keeps_markdown_symbols_literal() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let audio = crate::types::tests::message(json!({"message_id":2}));
            let (bot, server) = telegram_fixture(vec![telegram_message(900)]).await;
            finish_pending(&bot, &audio, MessageId(900), "**Literal** y `código`")
                .await
                .unwrap();
            let requests = server.await.unwrap();
            assert_eq!(requests[0].1["text"], "**Literal** y `código`");
            assert!(requests[0].1.get("entities").is_none());
        })
        .await
        .unwrap();
    }

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
    fn spanish_help_explains_access_privacy_and_moderation() {
        let help = all_command_descriptions();
        for expected in [
            "Solo puedes usar el bot en los grupos permitidos",
            "se borra al reiniciar",
            "Informa a los participantes",
            "No hay cuotas por usuario ni controles de gasto",
            "Envía el comando desde tu cuenta personal",
            "En grupos básicos solo se permiten expulsiones permanentes",
            "No traduce ni aplica la personalidad",
        ] {
            assert!(help.contains(expected), "{expected}");
        }
        assert!(!help.contains('👮'));
        assert!(!help.contains('🤖'));
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
