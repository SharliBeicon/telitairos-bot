//! Bounded, on-demand reading of explicitly shared links and Telegram attachments.

mod twitter;

use crate::{gpt::Ai, types::BufferedMessage};
use futures_util::StreamExt;
use linkify::{LinkFinder, LinkKind};
use reqwest::{redirect::Policy, Url};
use scraper::{Html, Selector};
use std::{collections::VecDeque, net::IpAddr, time::Duration};
use teloxide::{
    net::Download,
    prelude::*,
    types::{FileMeta, MessageEntityKind},
};

const MAX_BYTES: usize = 20_000_000;
const MAX_WEB_BYTES: usize = 2_000_000;
const MAX_SOURCES: usize = 8;
const MAX_TEXT: usize = 6_000;
const UNREAD: &str = "Contenido no leído: descarga, formato o análisis no disponible.";

#[derive(Default)]
pub(crate) struct ReadReport {
    pub unread: usize,
    pub partial: bool,
    pub history_truncated: bool,
}

impl ReadReport {
    pub(crate) fn with_notice(&self, mut answer: String) -> String {
        if self.unread > 0 {
            if self.unread == 1 {
                answer.push_str("\n\nNota de lectura: 1 recurso no se ha podido leer o se ha omitido por los límites de lectura.");
            } else {
                answer.push_str(&format!("\n\nNota de lectura: {} recursos no se han podido leer o se han omitido por los límites de lectura.", self.unread));
            }
        }
        if self.partial {
            answer.push_str(
                "\nNota de lectura: parte del contenido externo es parcial o se ha truncado.",
            );
        }
        if self.history_truncated {
            answer.push_str("\nNota de lectura: el contexto supera el límite de caracteres; solo se incluye el historial más reciente que cabe.");
        }
        answer
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum FileKind {
    Image(&'static str),
    Pdf,
    Text,
    Audio(String),
    Unsupported,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Source {
    Url(String),
    File { meta: FileMeta, kind: FileKind },
    Omitted,
}

pub(crate) fn urls(text: &str) -> Vec<Source> {
    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkKind::Url]);
    finder.url_must_have_scheme(false);
    let mut result: Vec<_> = finder
        .links(text)
        .filter(|link| {
            let raw = link.as_str();
            raw.get(..4)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("www."))
                || Url::parse(raw).is_ok()
        })
        .take(MAX_SOURCES + 1)
        .map(|link| Source::Url(link.as_str().to_owned()))
        .collect();
    if result.len() > MAX_SOURCES {
        result[MAX_SOURCES] = Source::Omitted;
    }
    result
}

fn document_kind(mime: &str, filename: &str) -> FileKind {
    match mime {
        "image/jpeg" => FileKind::Image("image/jpeg"),
        "image/png" => FileKind::Image("image/png"),
        "image/webp" => FileKind::Image("image/webp"),
        "application/pdf" => FileKind::Pdf,
        "application/json" | "application/xml" | "application/yaml" => FileKind::Text,
        _ => {
            let extension = filename
                .rsplit('.')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if mime.starts_with("text/")
                || matches!(
                    extension.as_str(),
                    "txt"
                        | "csv"
                        | "json"
                        | "md"
                        | "rs"
                        | "py"
                        | "js"
                        | "ts"
                        | "toml"
                        | "yaml"
                        | "yml"
                        | "xml"
                        | "log"
                )
            {
                FileKind::Text
            } else if matches!(mime, "" | "application/octet-stream") {
                match extension.as_str() {
                    "pdf" => FileKind::Pdf,
                    "jpg" | "jpeg" => FileKind::Image("image/jpeg"),
                    "png" => FileKind::Image("image/png"),
                    "webp" => FileKind::Image("image/webp"),
                    _ => FileKind::Unsupported,
                }
            } else {
                FileKind::Unsupported
            }
        }
    }
}

pub(crate) fn sources(message: &Message) -> Vec<Source> {
    let mut result = Vec::new();
    let file = if let Some(photo) = message.photo().and_then(|photos| photos.last()) {
        Some((photo.file.clone(), FileKind::Image("image/jpeg")))
    } else if let Some(document) = message.document() {
        Some((
            document.file.clone(),
            document_kind(
                document
                    .mime_type
                    .as_ref()
                    .map_or("", |mime| mime.essence_str()),
                document.file_name.as_deref().unwrap_or(""),
            ),
        ))
    } else if let Some(voice) = message.voice() {
        let kind = crate::bot::transcript::audio_filename(
            voice.mime_type.as_ref().map(|mime| mime.essence_str()),
            Some("voice.ogg"),
        )
        .map(FileKind::Audio)
        .unwrap_or(FileKind::Unsupported);
        Some((voice.file.clone(), kind))
    } else if let Some(audio) = message.audio() {
        let kind = crate::bot::transcript::audio_filename(
            audio.mime_type.as_ref().map(|mime| mime.essence_str()),
            audio.file_name.as_deref(),
        )
        .map(FileKind::Audio)
        .unwrap_or(FileKind::Unsupported);
        Some((audio.file.clone(), kind))
    } else {
        None
    };
    if let Some((meta, kind)) = file {
        result.push(Source::File { meta, kind });
    }
    for entity in message
        .entities()
        .or_else(|| message.caption_entities())
        .unwrap_or(&[])
    {
        if let MessageEntityKind::TextLink { url } = &entity.kind {
            if result.len() < MAX_SOURCES {
                let source = Source::Url(url.to_string());
                if !result.contains(&source) {
                    result.push(source);
                }
            } else if !result.contains(&Source::Omitted) {
                result.push(Source::Omitted);
            }
        }
    }
    for source in urls(message.text().or_else(|| message.caption()).unwrap_or("")) {
        if result.len() >= MAX_SOURCES {
            if !result.contains(&source) && !result.contains(&Source::Omitted) {
                result.push(Source::Omitted);
            }
            break;
        }
        if !result.contains(&source) {
            result.push(source);
        }
    }
    result
}

fn bounded_text(text: &str) -> String {
    let mut chars = text.chars();
    let mut result: String = chars.by_ref().take(MAX_TEXT).collect();
    if chars.next().is_some() {
        result.push_str("\n[Contenido truncado]");
    }
    result
}

fn append_bytes(bytes: &mut Vec<u8>, chunk: &[u8], limit: usize) -> Result<(), &'static str> {
    if chunk.len() > limit.saturating_sub(bytes.len()) {
        return Err("Contenido no leído: supera el límite de tamaño.");
    }
    bytes.extend_from_slice(chunk);
    Ok(())
}

async fn telegram_bytes(bot: &Bot, meta: &FileMeta) -> Result<Vec<u8>, &'static str> {
    if meta.size != u32::MAX && meta.size as usize > MAX_BYTES {
        return Err("Contenido no leído: el archivo supera 20 MB.");
    }
    let file = bot.get_file(meta.id.clone()).await.map_err(|_| UNREAD)?;
    let stream = bot.download_file_stream(&file.path);
    tokio::pin!(stream);
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        append_bytes(&mut bytes, &chunk.map_err(|_| UNREAD)?, MAX_BYTES)?;
    }
    if bytes.is_empty() {
        return Err(UNREAD);
    }
    Ok(bytes)
}

// Allow only ordinary public unicast addresses. Reject mixed public/private DNS answers too.
fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || a == 0
                || a >= 224
                || (a == 100 && (64..=127).contains(&b))
                || (a == 192 && b == 0)
                || (a == 192 && b == 168)
                || (a == 192 && b == 88 && c == 99)
                || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(ip) => {
            let segments = ip.segments();
            // Exclude transition, special-purpose and documentation ranges.
            segments[0] & 0xe000 == 0x2000
                && segments[0] != 0x2002
                && !(segments[0] == 0x2001 && (segments[1] < 0x0200 || segments[1] == 0x0db8))
                && !(segments[0] == 0x3fff && segments[1] < 0x1000)
        }
    }
}

fn validate_url(url: &Url) -> Result<(), &'static str> {
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || !matches!(url.port_or_known_default(), Some(80 | 443))
    {
        return Err("Contenido no leído: solo se permiten URLs HTTP/HTTPS públicas sin credenciales, en puertos 80 o 443.");
    }
    if url
        .host_str()
        .unwrap_or("")
        .trim_matches(['[', ']'])
        .parse::<IpAddr>()
        .is_ok_and(|ip| !public_ip(ip))
    {
        return Err("Contenido no leído: dirección de red privada o reservada bloqueada.");
    }
    Ok(())
}

fn web_url(raw: &str) -> Result<Url, &'static str> {
    if raw
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("www."))
    {
        Url::parse(&format!("https://{raw}")).map_err(|_| UNREAD)
    } else {
        Url::parse(raw).map_err(|_| UNREAD)
    }
}

async fn web_bytes(raw: &str) -> Result<(String, Vec<u8>), &'static str> {
    let mut url = web_url(raw)?;
    for _ in 0..4 {
        validate_url(&url)?;
        let host = url.host_str().ok_or(UNREAD)?.trim_matches(['[', ']']);
        let port = url.port_or_known_default().ok_or(UNREAD)?;
        let addresses: Vec<_> = tokio::net::lookup_host((host, port))
            .await
            .map_err(|_| UNREAD)?
            .collect();
        if addresses.is_empty() || addresses.iter().any(|address| !public_ip(address.ip())) {
            return Err("Contenido no leído: dirección de red privada o reservada bloqueada.");
        }
        // Pin the validated resolution; no proxies, automatic redirects, cookies or ambient credentials.
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .resolve_to_addrs(host, &addresses)
            .build()
            .map_err(|_| UNREAD)?;
        let response = client
            .get(url.clone())
            .header(
                reqwest::header::USER_AGENT,
                "TelitairosBot/0.2 (shared-link reader)",
            )
            .send()
            .await
            .map_err(|_| UNREAD)?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or(UNREAD)?;
            url = url.join(location).map_err(|_| UNREAD)?;
            continue;
        }
        if !response.status().is_success() {
            return Err(match response.status().as_u16() {
                401 | 403 => {
                    "Contenido no leído: el sitio deniega el acceso o requiere autenticación."
                }
                404 => "Contenido no leído: el sitio indica que el recurso no está disponible.",
                429 => "Contenido no leído: el sitio ha limitado temporalmente las peticiones.",
                500..=599 => {
                    "Contenido no leído: el servicio de origen está fallando temporalmente."
                }
                _ => UNREAD,
            });
        }
        let mime = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if response
            .content_length()
            .is_some_and(|size| size > MAX_WEB_BYTES as u64)
        {
            return Err("Contenido no leído: la URL supera 2 MB.");
        }
        let mut stream = response.bytes_stream();
        let mut bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            append_bytes(&mut bytes, &chunk.map_err(|_| UNREAD)?, MAX_WEB_BYTES)?;
        }
        return Ok((mime, bytes));
    }
    Err("Contenido no leído: demasiadas redirecciones.")
}

fn html_text(bytes: &[u8]) -> Result<String, &'static str> {
    let text = std::str::from_utf8(bytes).map_err(|_| UNREAD)?;
    let document = Html::parse_document(text);
    let selector = Selector::parse("body").map_err(|_| UNREAD)?;
    let body = document.select(&selector).next().ok_or(UNREAD)?;
    let mut result = String::new();
    let mut count = 0;
    for node in body.descendants() {
        let Some(text) = node.value().as_text() else {
            continue;
        };
        let mut ancestors = node.ancestors();
        if ancestors.by_ref().take(128).any(|ancestor| {
            ancestor.value().as_element().is_some_and(|element| {
                matches!(element.name(), "script" | "style" | "noscript" | "template")
            })
        }) {
            continue;
        }
        if ancestors.next().is_some() {
            return Err("Contenido no leído: HTML demasiado anidado.");
        }
        for word in text.split_whitespace() {
            result.push_str(word);
            result.push(' ');
            count += word.chars().count() + 1;
            if count >= MAX_TEXT {
                return Ok(format!(
                    "{}\n[Contenido truncado]",
                    result.chars().take(MAX_TEXT).collect::<String>()
                ));
            }
        }
    }
    if result.trim().is_empty() {
        return Err(UNREAD);
    }
    Ok(result)
}

async fn read_source(bot: &Bot, ai: &Ai, source: &Source) -> Result<String, &'static str> {
    let (kind, bytes) =
        match source {
            Source::Omitted => return Err(
                "Contenido no leído: se han omitido recursos adicionales del mensaje (máximo 8).",
            ),
            Source::File {
                kind: FileKind::Unsupported,
                ..
            } => return Err("Contenido no leído: formato de archivo no compatible."),
            Source::File { meta, kind } => (kind.clone(), telegram_bytes(bot, meta).await?),
            Source::Url(url) => {
                let parsed = web_url(url)?;
                validate_url(&parsed)?;
                if let Some(id) = twitter::post_id(&parsed) {
                    // Use the documented data endpoint, not the browser/embed redirect.
                    let (_, bytes) =
                        web_bytes(&format!("https://api.fxtwitter.com/2/status/{id}")).await?;
                    return twitter::post_text(&bytes, id);
                }
                let (mime, bytes) = web_bytes(url).await?;
                if matches!(mime.as_str(), "text/html" | "application/xhtml+xml") {
                    return html_text(&bytes);
                }
                let kind = document_kind(&mime, "");
                if kind == FileKind::Unsupported {
                    return Err(
                    "Contenido no leído: la URL no devuelve HTML, texto, imagen o PDF compatible.",
                );
                }
                (kind, bytes)
            }
        };
    match kind {
        FileKind::Text => {
            let text = std::str::from_utf8(&bytes)
                .map_err(|_| "Contenido no leído: se requiere texto UTF-8.")?;
            if text.contains('\0') {
                return Err(UNREAD);
            }
            Ok(bounded_text(text))
        }
        FileKind::Audio(filename) => ai
            .transcribe(filename, bytes)
            .await
            .map(|text| bounded_text(&text)),
        FileKind::Image(mime) => {
            let valid = match mime {
                "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
                "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
                "image/webp" => bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"),
                _ => false,
            };
            if !valid {
                return Err("Contenido no leído: imagen no válida para el formato declarado.");
            }
            ai.inspect(&bytes, Some(mime)).await
        }
        FileKind::Pdf => {
            if !bytes.starts_with(b"%PDF-") {
                return Err("Contenido no leído: PDF no válido.");
            }
            ai.inspect(&bytes, None).await
        }
        FileKind::Unsupported => Err(UNREAD),
    }
}

/// Work on a snapshot: never hold the history lock across downloads or provider calls.
pub(crate) async fn enrich(
    bot: &Bot,
    ai: &Ai,
    messages: &mut VecDeque<BufferedMessage>,
) -> ReadReport {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    let mut seen: Vec<(Source, Result<String, &'static str>)> = Vec::new();
    let mut remaining_text = 24_000usize;
    let mut report = ReadReport::default();
    // Spend the bounded reading budget on the newest messages first.
    for message in messages.iter_mut().rev() {
        for source in &message.sources {
            let result = if remaining_text < 200 || tokio::time::Instant::now() >= deadline {
                Err("Contenido no leído: presupuesto de texto o tiempo de lectura agotado.")
            } else if let Some((_, text)) = seen.iter().find(|(previous, _)| previous == source) {
                text.clone()
            } else if seen.len() >= MAX_SOURCES {
                Err("Contenido no leído: límite de 8 recursos, texto extraído o tiempo de lectura alcanzado.")
            } else {
                let result = tokio::time::timeout_at(
                    deadline.min(tokio::time::Instant::now() + Duration::from_secs(120)),
                    read_source(bot, ai, source),
                )
                .await
                .unwrap_or(Err("Contenido no leído: tiempo de lectura agotado."));
                seen.push((source.clone(), result.clone()));
                result
            };
            report.unread += usize::from(result.is_err());
            let result = result.unwrap_or_else(str::to_owned);
            report.partial |= result.chars().count() > MAX_TEXT
                || result.contains("[Contenido truncado]")
                || result.contains("[Contenido parcial]");
            let text = bounded_text(&result);
            let text = if text.chars().count() > remaining_text && remaining_text >= 200 {
                report.partial = true;
                format!(
                    "{}\n[Contenido truncado]",
                    text.chars()
                        .take(remaining_text.saturating_sub(30))
                        .collect::<String>()
                )
            } else {
                text
            };
            remaining_text = remaining_text.saturating_sub(text.chars().count());
            let label = match source {
                Source::Omitted => "Recursos adicionales omitidos".to_owned(),
                Source::Url(url) => format!("URL {}", crate::types::escape_transcript_field(url)),
                Source::File {
                    kind: FileKind::Audio(_),
                    ..
                } => "Transcripción automática de audio; puede contener errores".to_owned(),
                Source::File {
                    kind: FileKind::Image(_) | FileKind::Pdf,
                    ..
                } => "Descripción automática de adjunto; puede contener errores".to_owned(),
                Source::File { .. } => "Archivo adjunto".to_owned(),
            };
            message.text.push_str(&format!(
                "\n[Contenido externo no fiable: {label}]\n{text}\n[Fin del contenido externo]"
            ));
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::tests::message;
    use serde_json::json;

    #[test]
    fn rejects_private_reserved_and_transition_addresses() {
        for ip in [
            "127.0.0.1",
            "10.0.0.1",
            "169.254.169.254",
            "100.100.100.200",
            "192.0.2.1",
            "198.18.0.1",
            "203.0.113.1",
            "224.0.0.1",
            "::1",
            "::ffff:127.0.0.1",
            "fc00::1",
            "fe80::1",
            "2001:db8::1",
            "2002:7f00:1::",
            "64:ff9b::7f00:1",
        ] {
            assert!(!public_ip(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"] {
            assert!(public_ip(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn urls_reject_credentials_schemes_and_nonstandard_ports() {
        for url in [
            "file:///etc/passwd",
            "ftp://example.com/a",
            "https://user:pass@example.com",
            "http://example.com:8080",
            "http://127.0.0.1/",
            "http://2130706433/",
            "http://169.254.169.254/",
            "http://[::ffff:127.0.0.1]/",
        ] {
            assert!(validate_url(&Url::parse(url).unwrap()).is_err());
        }
        assert!(validate_url(&Url::parse("https://example.com").unwrap()).is_ok());
    }

    #[test]
    fn schemeless_www_links_use_https_without_weakening_url_validation() {
        let raw = "www.fixupx.com/user/status/12345?s=20";
        assert_eq!(urls(raw), vec![Source::Url(raw.into())]);
        assert!(
            urls("Adjunto main.rs y config.json; ejemplo.com no es un enlace explícito.")
                .is_empty()
        );
        let parsed = web_url(raw).unwrap();
        assert_eq!(parsed.scheme(), "https");
        assert_eq!(twitter::post_id(&parsed), Some("12345"));
        for raw in [
            "www.fixupx.com@127.0.0.1/user/status/12345",
            "file:///etc/passwd",
            "https://user:password@fixupx.com/user/status/12345",
        ] {
            assert!(validate_url(&web_url(raw).unwrap()).is_err(), "{raw}");
        }
    }

    #[tokio::test]
    async fn twitter_adapter_validates_the_original_url_before_any_fetch() {
        let bot = Bot::new("offline-placeholder");
        let ai = Ai::fixture();
        let source = Source::Url("https://user:password@fixupx.com/user/status/12345".into());
        assert!(read_source(&bot, &ai, &source)
            .await
            .unwrap_err()
            .contains("sin credenciales"));
    }

    #[test]
    fn extracts_attachments_and_hidden_links_without_network() {
        let photo = message(
            json!({"photo":[{"file_id":"photo","file_unique_id":"unique","width":10,"height":10}]}),
        );
        assert!(matches!(
            sources(&photo).as_slice(),
            [Source::File {
                kind: FileKind::Image(_),
                ..
            }]
        ));
        assert!(BufferedMessage::from_message(&photo).is_some());
        let link = message(
            json!({"text":"web", "entities":[{"type":"text_link","offset":0,"length":3,"url":"https://example.com"}]}),
        );
        assert_eq!(
            sources(&link),
            vec![Source::Url("https://example.com/".into())]
        );
        let voice = message(
            json!({"voice":{"file_id":"voice","file_unique_id":"unique","duration":1,"mime_type":"audio/ogg"}}),
        );
        assert!(matches!(
            sources(&voice).as_slice(),
            [Source::File {
                kind: FileKind::Audio(_),
                ..
            }]
        ));
    }

    #[test]
    fn html_ignores_scripts_and_text_and_downloads_are_bounded() {
        assert_eq!(html_text(b"<html><body><h1>Hola</h1><script>secreto</script><style>css</style><p>mundo</p></body></html>").unwrap(), "Hola mundo ");
        let mut bytes = vec![1];
        assert!(append_bytes(&mut bytes, &[2, 3], 2).is_err());
        assert_eq!(bytes, vec![1]);
        assert!(bounded_text(&"a".repeat(MAX_TEXT + 1)).ends_with("[Contenido truncado]"));
        assert_eq!(document_kind("application/pdf", "a.pdf"), FileKind::Pdf);
        assert_eq!(document_kind("application/json", "a.json"), FileKind::Text);
        assert_eq!(
            document_kind("application/zip", "a.zip"),
            FileKind::Unsupported
        );
    }

    #[test]
    fn too_many_links_leave_an_explicit_omission_marker() {
        let text = (0..20)
            .map(|n| format!("https://example.com/{n} "))
            .collect::<String>();
        let sources = urls(&text);
        assert_eq!(sources.len(), MAX_SOURCES + 1);
        assert_eq!(sources.last(), Some(&Source::Omitted));
    }

    #[tokio::test]
    async fn enrichment_is_bounded_and_reports_failures_without_network() {
        let mut messages = (0..10)
            .map(|n| BufferedMessage {
                author: format!("Author {n}"),
                text: "original".into(),
                sources: vec![Source::Url(format!("file:///forbidden/{n}"))],
            })
            .collect();
        let report = enrich(
            &Bot::new("offline-placeholder"),
            &Ai::fixture(),
            &mut messages,
        )
        .await;
        assert_eq!(report.unread, 10);
        assert!(messages
            .front()
            .unwrap()
            .text
            .contains("límite de 8 recursos"));
        assert!(messages
            .back()
            .unwrap()
            .text
            .contains("solo se permiten URLs"));
        assert!(messages
            .iter()
            .all(|message| message.text.starts_with("original")));
        assert!(report.with_notice("Resumen".into()).contains("10 recursos"));
        assert_eq!(
            ReadReport::default().with_notice("Resumen".into()),
            "Resumen"
        );
    }
}
