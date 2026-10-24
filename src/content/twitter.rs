//! Read shared FixupX and FxTwitter status links through FxEmbed API v2.

use reqwest::Url;
use serde_json::Value;

const UNAVAILABLE: &str = "Contenido no leído: FxTwitter no ha devuelto un tuit público válido. Puede estar borrado, ser privado o no estar disponible temporalmente.";

pub(super) fn post_id(url: &Url) -> Option<&str> {
    if !matches!(
        url.host_str()?,
        "fixupx.com" | "www.fixupx.com" | "fxtwitter.com" | "www.fxtwitter.com"
    ) {
        return None;
    }
    let path: Vec<_> = url.path_segments()?.collect();
    let id = match path.as_slice() {
        [_, "status", id, ..] | ["i", "web", "status", id, ..] | ["status", id, ..] => *id,
        _ => return None,
    };
    ((2..=20).contains(&id.len()) && id.bytes().all(|byte| byte.is_ascii_digit())).then_some(id)
}

fn append_post(output: &mut String, post: &Value) {
    let name = post["author"]["name"]
        .as_str()
        .unwrap_or("Autor desconocido");
    let handle = post["author"]["screen_name"].as_str().unwrap_or("");
    output.push_str(&format!(
        "Autor del tuit: {name} (@{handle})\nTexto del tuit:\n{}\n",
        post["text"].as_str().unwrap_or("[Sin texto disponible]")
    ));
    if let Some(choices) = post["poll"]["choices"].as_array() {
        output.push_str("Opciones de la encuesta:\n");
        for choice in choices {
            if let Some(label) = choice["label"].as_str() {
                output.push_str(&format!("- {label}\n"));
            }
        }
    }
    if let Some(article) = post.get("article").filter(|article| article.is_object()) {
        output.push_str(&format!(
            "Artículo adjunto: {}\n",
            article["title"].as_str().unwrap_or("Sin título")
        ));
        if let Some(blocks) = article["content"]["blocks"]
            .as_array()
            .filter(|blocks| !blocks.is_empty())
        {
            for block in blocks {
                if let Some(text) = block["text"].as_str() {
                    output.push_str(text);
                    output.push('\n');
                }
            }
        } else {
            if let Some(preview) = article["preview_text"].as_str() {
                output.push_str(preview);
                output.push('\n');
            }
            output.push_str("[Contenido parcial] Solo está disponible la vista previa del artículo, no su contenido completo.\n");
        }
    }
    let photos = post["media"]["photos"].as_array();
    let videos = post["media"]["videos"].as_array();
    if photos.is_some_and(|items| !items.is_empty())
        || videos.is_some_and(|items| !items.is_empty())
        || post["media"]["external"].is_object()
        || post["media"]["broadcast"].is_object()
    {
        output.push_str("[Contenido parcial] El tuit incluye imágenes o vídeos que no se han analizado. No se conoce su contenido visual o sonoro.\n");
        for photo in photos.into_iter().flatten() {
            if let Some(alt) = photo["altText"]
                .as_str()
                .filter(|text| !text.trim().is_empty())
            {
                output.push_str(&format!("Texto alternativo publicado para una imagen (no verificado visualmente): {alt}\n"));
            }
        }
    }
}

pub(super) fn post_text(bytes: &[u8], expected_id: &str) -> Result<String, &'static str> {
    let response: Value = serde_json::from_slice(bytes).map_err(|_| UNAVAILABLE)?;
    if response["code"].as_u64() != Some(200) {
        return Err(UNAVAILABLE);
    }
    let post = &response["status"];
    if post["id"].as_str() != Some(expected_id)
        || post["type"].as_str() != Some("status")
        || !post["text"].is_string()
    {
        return Err(UNAVAILABLE);
    }
    let mut output = String::from("Tuit público obtenido mediante la API de FxTwitter. No se ha leído el hilo completo ni los enlaces incluidos en el tuit.\n");
    append_post(&mut output, post);
    if let Some(quote) = post.get("quote").filter(|quote| !quote.is_null()) {
        output.push_str("\nTuit citado (otro autor; no confundir con el texto anterior):\n");
        if quote["type"].as_str() == Some("status") && quote["text"].is_string() {
            append_post(&mut output, quote);
        } else {
            output.push_str("[Contenido parcial] El tuit citado no está disponible.\n");
        }
    }
    Ok(super::bounded_text(&output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_recognizes_numeric_status_ids_on_exact_supported_hosts() {
        for raw in [
            "https://www.fixupx.com/user/status/12345?s=20",
            "https://fixupx.com/i/web/status/12345/photo/1",
            "https://fxtwitter.com/status/12345",
            "https://www.fxtwitter.com/user/status/12345/es",
        ] {
            assert_eq!(post_id(&Url::parse(raw).unwrap()), Some("12345"));
        }
        for raw in [
            "https://fixupx.com.evil.test/user/status/12345",
            "https://evil.test/fixupx.com/user/status/12345",
            "https://fixupx.com/user/status/not-a-number",
            "https://fixupx.com/user/status/123%2f45",
            "https://fixupx.com/user",
            "https://fixupx.com/user/status/123456789012345678901",
        ] {
            assert_eq!(post_id(&Url::parse(raw).unwrap()), None, "{raw}");
        }
    }

    #[test]
    fn extracts_post_quotes_and_poll_without_confusing_authors_or_viewing_media() {
        let value = json!({"code":200,"status":{"type":"status","id":"12345","text":"Texto original", "author":{"name":"Ana","screen_name":"ana"}, "poll":{"choices":[{"label":"Sí"},{"label":"No"}]}, "media":{"photos":[{"altText":"Un cartel"}]}, "quote":{"type":"status","text":"Texto citado","author":{"name":"Luis","screen_name":"luis"}}}});
        let text = post_text(&serde_json::to_vec(&value).unwrap(), "12345").unwrap();
        for expected in [
            "Texto original",
            "Ana (@ana)",
            "Texto citado",
            "Luis (@luis)",
            "Opciones de la encuesta",
            "no se han analizado",
            "no verificado visualmente",
            "[Contenido parcial]",
        ] {
            assert!(text.contains(expected), "{expected}");
        }
    }

    #[test]
    fn rejects_provider_errors_tombstones_mismatched_ids_and_invalid_data() {
        for value in [
            json!({"code":404,"status":null}),
            json!({"code":401}),
            json!({"code":200,"status":{"type":"tombstone","id":"12345","text":"unavailable"}}),
            json!({"code":200,"status":{"type":"status","id":"67890","text":"Other post"}}),
            json!({"code":200,"status":{"type":"status","id":"12345"}}),
        ] {
            assert!(post_text(&serde_json::to_vec(&value).unwrap(), "12345").is_err());
        }
        assert!(post_text(b"<html>Login</html>", "12345").is_err());
    }

    #[test]
    fn article_previews_and_long_posts_are_explicitly_partial() {
        let value = json!({"code":200,"status":{"type":"status","id":"12345","text":"Mira este artículo", "article":{"title":"Título","preview_text":"Vista previa"}}});
        let text = post_text(&serde_json::to_vec(&value).unwrap(), "12345").unwrap();
        assert!(text.contains("Vista previa"));
        assert!(text.contains("no su contenido completo"));
        assert!(text.contains("[Contenido parcial]"));
        let value =
            json!({"code":200,"status":{"type":"status","id":"12345","text":"🦀".repeat(10_000)}});
        let text = post_text(&serde_json::to_vec(&value).unwrap(), "12345").unwrap();
        assert!(text.ends_with("[Contenido truncado]"));
        assert!(text.chars().count() < 6_100);
    }
}
