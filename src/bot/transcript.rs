use super::*;
use crate::gpt::Ai;
use futures_util::StreamExt;
use std::time::Duration;
use teloxide::{net::Download, types::FileMeta};

const MAX_AUDIO_BYTES: usize = 20_000_000;
const TOO_LARGE: &str =
    "El audio supera el límite de 20 MB (20.000.000 bytes). Envía un archivo más pequeño.";
const DOWNLOAD_FAILURE: &str =
    "No he podido descargar el audio de Telegram. Inténtalo de nuevo más tarde.";
const USAGE: &str = "No he encontrado un audio o una nota de voz compatibles, o no he podido leer sus metadatos. Usa /transcript sin argumentos como respuesta a un audio o una nota de voz. La transcripción conserva el idioma original y el audio se envía a OpenAI.";

struct Source<'a> {
    message: &'a Message,
    file: &'a FileMeta,
    filename: String,
}

fn source(command: &Message) -> Result<Source<'_>, &'static str> {
    let message = command.reply_to_message().ok_or(USAGE)?;
    let (file, filename) = if let Some(voice) = message.voice() {
        (
            &voice.file,
            audio_filename(
                voice.mime_type.as_ref().map(|mime| mime.essence_str()),
                // Voice has no filename. Only absent/generic MIME falls back to Ogg.
                Some("voice.ogg"),
            )?,
        )
    } else if let Some(audio) = message.audio() {
        (
            &audio.file,
            audio_filename(
                audio.mime_type.as_ref().map(|mime| mime.essence_str()),
                audio.file_name.as_deref(),
            )?,
        )
    } else {
        return Err(USAGE);
    };
    check_size(file.size)?;
    Ok(Source {
        message,
        file,
        filename,
    })
}

fn check_size(size: u32) -> Result<(), &'static str> {
    // Telegram uses this sentinel when the size is unknown. The stream is still bounded.
    if size != u32::MAX && u64::from(size) > MAX_AUDIO_BYTES as u64 {
        Err(TOO_LARGE)
    } else {
        Ok(())
    }
}

fn audio_filename(mime: Option<&str>, filename: Option<&str>) -> Result<String, &'static str> {
    let extension = match mime.map(str::to_ascii_lowercase).as_deref() {
        Some("audio/flac" | "audio/x-flac") => Some("flac"),
        Some("audio/mpeg" | "audio/mp3") => Some("mp3"),
        Some("audio/mpga") => Some("mpga"),
        Some("audio/mp4" | "audio/m4a" | "audio/x-m4a") => Some("m4a"),
        Some("audio/ogg" | "application/ogg") => Some("ogg"),
        Some("audio/wav" | "audio/wave" | "audio/x-wav" | "audio/vnd.wave") => Some("wav"),
        Some("audio/webm") => Some("webm"),
        // Generic/absent MIME metadata can be resolved from the filename extension.
        None | Some("application/octet-stream") => None,
        Some(_) => return Err("Ese formato de audio no es compatible. Usa FLAC, MP3, MP4, MPEG, MPGA, M4A, OGG, WAV o WebM."),
    };
    let extension = extension.map(str::to_owned).or_else(|| {
        let extension = filename?.rsplit_once('.')?.1.to_ascii_lowercase();
        matches!(extension.as_str(), "flac" | "mp3" | "mp4" | "mpeg" | "mpga" | "m4a" | "ogg" | "wav" | "webm").then_some(extension)
    }).ok_or("No he podido identificar un formato de audio compatible. Usa un archivo con una extensión FLAC, MP3, MP4, MPEG, MPGA, M4A, OGG, WAV o WebM.")?;
    // Never forward user-provided paths or filenames to the provider.
    Ok(format!("audio.{extension}"))
}

fn append_chunk(bytes: &mut Vec<u8>, chunk: &[u8]) -> Result<(), &'static str> {
    if chunk.len() > MAX_AUDIO_BYTES.saturating_sub(bytes.len()) {
        return Err(TOO_LARGE);
    }
    bytes.extend_from_slice(chunk);
    Ok(())
}

async fn download(bot: &Bot, metadata: &FileMeta) -> Result<Vec<u8>, &'static str> {
    let file = bot
        .get_file(metadata.id.clone())
        .await
        .map_err(|_| DOWNLOAD_FAILURE)?;
    check_size(file.meta.size)?;
    let stream = bot.download_file_stream(&file.path);
    tokio::pin!(stream);
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| DOWNLOAD_FAILURE)?;
        append_chunk(&mut bytes, &chunk)?;
    }
    if bytes.is_empty() {
        return Err("El audio está vacío. Envía un audio que contenga voz.");
    }
    Ok(bytes)
}

pub(super) async fn handle(bot: &Bot, ai: &Ai, command: &Message) -> ResponseResult<()> {
    let source = match source(command) {
        Ok(source) => source,
        Err(reason) => return send_text(bot, command, reason).await,
    };
    let bytes = match tokio::time::timeout(Duration::from_secs(60), download(bot, source.file)).await {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(reason)) => return send_text(bot, command, reason).await,
        Err(_) => return send_text(bot, command, "La descarga del audio ha superado el límite de 60 segundos. Inténtalo de nuevo más tarde.").await,
    };
    match ai.transcribe(source.filename, bytes).await {
        Ok(text) => super::send_text_at(bot, source.message, &text, Some(source.message.id)).await,
        Err(reason) => send_text(bot, command, reason).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::tests::message;
    use serde_json::json;

    fn command(media: serde_json::Value) -> Message {
        let mut reply = json!({"message_id":2,"date":0,"chat":{"id":-1001,"type":"supergroup","title":"Group"}});
        for (key, value) in media.as_object().unwrap() {
            reply[key] = value.clone();
        }
        message(json!({"text":"/transcript","reply_to_message":reply}))
    }

    #[test]
    fn extracts_only_replied_voice_and_audio() {
        let voice = command(
            json!({"voice":{"file_id":"voice-id","file_unique_id":"unique","duration":2,"mime_type":"audio/ogg"}}),
        );
        let source = source(&voice).unwrap();
        assert_eq!(source.message.id, MessageId(2));
        assert_eq!(source.file.id.0, "voice-id");
        assert_eq!(source.filename, "audio.ogg");
        assert_eq!(source.file.size, u32::MAX);
        let audio = command(
            json!({"audio":{"file_id":"audio-id","file_unique_id":"unique","duration":2,"file_size":42,"file_name":"song.MP3","mime_type":null}}),
        );
        let source = super::source(&audio).unwrap();
        assert_eq!(source.file.id.0, "audio-id");
        assert_eq!(source.filename, "audio.mp3");
        assert_eq!(source.file.size, 42);
        for media in [
            json!({"text":"hi"}),
            json!({"document":{"file_id":"id","file_unique_id":"unique","file_name":"song.mp3"}}),
        ] {
            assert!(super::source(&command(media))
                .err()
                .unwrap()
                .contains("no he podido leer sus metadatos"));
        }
        assert!(super::source(&message(json!({"text":"/transcript"}))).is_err());
        let oversized = command(
            json!({"voice":{"file_id":"id","file_unique_id":"unique","duration":2,"file_size":20_000_001,"mime_type":"audio/ogg"}}),
        );
        assert_eq!(super::source(&oversized).err(), Some(TOO_LARGE));
    }

    #[test]
    fn voice_filenames_follow_mime_and_only_unknown_metadata_defaults_to_ogg() {
        for (mime, extension) in [
            (Some("audio/ogg"), "ogg"),
            (Some("audio/mpeg"), "mp3"),
            (Some("audio/mp3"), "mp3"),
            (Some("audio/mp4"), "m4a"),
            (Some("audio/x-m4a"), "m4a"),
            (Some("audio/m4a"), "m4a"),
            (None, "ogg"),
            (Some("application/octet-stream"), "ogg"),
        ] {
            let message = command(json!({"voice":{"file_id":"id","file_unique_id":"unique",
                "duration":2,"mime_type":mime}}));
            assert_eq!(
                source(&message).unwrap().filename,
                format!("audio.{extension}")
            );
        }
        let unsupported = command(json!({"voice":{"file_id":"id","file_unique_id":"unique",
            "duration":2,"mime_type":"audio/aac"}}));
        assert!(source(&unsupported)
            .err()
            .unwrap()
            .contains("no es compatible"));
    }

    #[test]
    fn filenames_are_safe_and_formats_are_explicit() {
        for (mime, expected) in [
            ("audio/flac", "flac"),
            ("audio/mpeg", "mp3"),
            ("audio/mpga", "mpga"),
            ("audio/mp4", "m4a"),
            ("audio/m4a", "m4a"),
            ("audio/x-m4a", "m4a"),
            ("audio/ogg", "ogg"),
            ("audio/wav", "wav"),
            ("audio/webm", "webm"),
            ("AUDIO/MPEG", "mp3"),
        ] {
            assert_eq!(
                audio_filename(Some(mime), Some("../../unsafe.exe")).unwrap(),
                format!("audio.{expected}")
            );
        }
        for extension in [
            "flac", "mp3", "mp4", "mpeg", "mpga", "m4a", "ogg", "wav", "webm",
        ] {
            for mime in [None, Some("application/octet-stream")] {
                assert_eq!(
                    audio_filename(
                        mime,
                        Some(&format!("../../secret.{}", extension.to_uppercase()))
                    )
                    .unwrap(),
                    format!("audio.{extension}")
                );
            }
        }
        for filename in [
            None,
            Some("no-extension"),
            Some("audio.exe"),
            Some("audio.mp3/path"),
            Some("audio.mp3\n"),
        ] {
            assert!(audio_filename(None, filename).is_err());
        }
        assert!(audio_filename(Some("audio/aac"), Some("audio.mp3")).is_err());
    }

    #[test]
    fn known_unknown_and_actual_sizes_are_bounded() {
        for size in [0, 1, MAX_AUDIO_BYTES as u32, u32::MAX] {
            assert!(check_size(size).is_ok());
        }
        for size in [MAX_AUDIO_BYTES as u32 + 1, u32::MAX - 1] {
            assert_eq!(check_size(size), Err(TOO_LARGE));
        }
        let mut bytes = vec![0; MAX_AUDIO_BYTES - 2];
        append_chunk(&mut bytes, &[1, 2]).unwrap();
        assert_eq!(bytes.len(), MAX_AUDIO_BYTES);
        assert_eq!(append_chunk(&mut bytes, &[3]), Err(TOO_LARGE));
        assert_eq!(bytes.len(), MAX_AUDIO_BYTES);
        assert_eq!(&bytes[MAX_AUDIO_BYTES - 2..], &[1, 2]);
    }
}
