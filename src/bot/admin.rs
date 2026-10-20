use crate::bot::*;
use chrono::{DateTime, Duration, Utc};
use teloxide::types::{Chat, ChatMemberKind, ChatPermissions, Me, UserId};

#[derive(BotCommands, Clone, PartialEq)]
#[command(
    rename_rule = "lowercase",
    description = "Comandos de administración (ayuda disponible para todos)",
    parse_with = "split"
)]
pub enum AdminCommand {
    #[command(description = "Mostrar esta ayuda.")]
    Help,
    #[command(
        description = "Silenciar en un supergrupo: responde con /mute N h/m/s/p (p = permanente; duración mínima de 60 segundos)."
    )]
    Mute(types::TimeAmount, types::UnitOfTime),
    #[command(
        description = "Expulsar: responde con /ban N h/m/s/p (p = permanente; temporal: solo supergrupos, mínimo 60 segundos)."
    )]
    Ban(types::TimeAmount, types::UnitOfTime),
    #[command(
        description = "Permitir volver a entrar: responde con /unban en un supergrupo; no añade al usuario automáticamente."
    )]
    Unban,
    #[command(
        description = "Restaurar los permisos predeterminados del supergrupo: responde con /unmute."
    )]
    Unmute,
}

pub async fn handle_admin_commands(
    bot: Bot,
    me: Me,
    telitairo_bot: TelitairoBot,
    msg: Message,
    cmd: AdminCommand,
) -> ResponseResult<()> {
    if let Some(usage) = msg.text().and_then(|text| {
        super::malformed_command_usage(text, me.username(), telitairo_bot.buffer_size)
    }) {
        return send_text(&bot, &msg, &usage).await;
    }
    if matches!(cmd, AdminCommand::Unban | AdminCommand::Unmute) {
        return reverse_moderation(&bot, &me, &msg, cmd == AdminCommand::Unmute).await;
    }
    let (amount, unit, mute) = match cmd {
        AdminCommand::Help => return send_text(&bot, &msg, &all_command_descriptions()).await,
        AdminCommand::Mute(amount, unit) => (amount, unit, true),
        AdminCommand::Ban(amount, unit) => (amount, unit, false),
        AdminCommand::Unban | AdminCommand::Unmute => unreachable!("handled above"),
    };
    if !mute {
        if let Err(reason) = validate_ban_duration(&msg.chat, &unit) {
            return send_text(&bot, &msg, reason).await;
        }
    }
    let (caller, target) = match moderation_target(&msg, mute, me.id) {
        Ok(ids) => ids,
        Err(reason) => return send_text(&bot, &msg, reason).await,
    };
    if let Err(reason) = authorize(&bot, msg.chat.id, caller).await {
        return send_text(&bot, &msg, reason).await;
    }
    // Compute immediately before sending, not from the incoming message's potentially stale date.
    let expiry = match calc_time(amount, unit, Utc::now()) {
        Ok(expiry) => expiry,
        Err(reason) => return send_text(&bot, &msg, reason).await,
    };
    let result = if mute {
        let mut request = bot.restrict_chat_member(msg.chat.id, target, ChatPermissions::empty());
        if let Expiry::Until(until) = expiry {
            request = request.until_date(until);
        }
        request.await
    } else {
        let mut request = bot.ban_chat_member(msg.chat.id, target);
        if let Expiry::Until(until) = expiry {
            request = request.until_date(until);
        }
        request.await
    };
    let feedback = match result {
        Ok(_) if mute => "Usuario silenciado.",
        Ok(_) => "Usuario expulsado.",
        Err(_) => {
            log::warn!("Telegram moderation request failed");
            "No he podido aplicar esa acción. Comprueba que soy administrador con permiso para restringir miembros y que el usuario seleccionado no es administrador."
        }
    };
    send_text(&bot, &msg, feedback).await
}

async fn authorize(bot: &Bot, chat: ChatId, caller: UserId) -> Result<(), &'static str> {
    match bot.get_chat_member(chat, caller).await {
        Ok(member) if can_restrict(&member.kind) => Ok(()),
        Ok(_) => {
            Err("Solo los administradores con permiso para restringir miembros pueden hacer eso.")
        }
        Err(_) => {
            log::warn!("Could not verify moderation caller permissions");
            Err("No he podido verificar tus permisos de administrador. No se ha realizado ninguna acción.")
        }
    }
}

fn unban_request(bot: &Bot, chat: ChatId, target: UserId) -> <Bot as Requester>::UnbanChatMember {
    // Without this flag Telegram can kick an existing member.
    bot.unban_chat_member(chat, target).only_if_banned(true)
}

fn unmute_request(
    bot: &Bot,
    chat: ChatId,
    target: UserId,
    defaults: Option<ChatPermissions>,
) -> Result<<Bot as Requester>::RestrictChatMember, &'static str> {
    let defaults = defaults.ok_or("No he podido obtener los permisos predeterminados del supergrupo. No se ha realizado ninguna acción.")?;
    Ok(bot
        .restrict_chat_member(chat, target, defaults)
        .use_independent_chat_permissions(true))
}

fn unmute_target(kind: &ChatMemberKind) -> Result<(), &'static str> {
    match kind {
        ChatMemberKind::Restricted(member) if member.is_member => Ok(()),
        ChatMemberKind::Banned(_) => {
            Err("Ese usuario está expulsado. Usa /unban para permitirle volver a entrar.")
        }
        _ => Err("Ese usuario no es un miembro con restricciones que pueda retirar con /unmute."),
    }
}

async fn reverse_moderation(bot: &Bot, me: &Me, msg: &Message, unmute: bool) -> ResponseResult<()> {
    if !msg.chat.is_supergroup() {
        return send_text(
            bot,
            msg,
            "Para usar /unban o /unmute se necesita un supergrupo.",
        )
        .await;
    }
    let (caller, target) = match moderation_target(msg, true, me.id) {
        Ok(ids) => ids,
        Err(reason) => return send_text(bot, msg, reason).await,
    };
    if let Err(reason) = authorize(bot, msg.chat.id, caller).await {
        return send_text(bot, msg, reason).await;
    }
    let result = if unmute {
        let member = match bot.get_chat_member(msg.chat.id, target).await {
            Ok(member) => member,
            Err(_) => return send_text(bot, msg, "No he podido comprobar las restricciones del usuario. No se ha realizado ninguna acción.").await,
        };
        if let Err(reason) = unmute_target(&member.kind) {
            return send_text(bot, msg, reason).await;
        }
        let defaults = match bot.get_chat(msg.chat.id).await {
            Ok(chat) => chat.permissions(),
            Err(_) => None,
        };
        let request = match unmute_request(bot, msg.chat.id, target, defaults) {
            Ok(request) => request,
            Err(reason) => return send_text(bot, msg, reason).await,
        };
        request.await
    } else {
        unban_request(bot, msg.chat.id, target).await
    };
    let feedback = match result {
        Ok(_) if unmute => "Se han restaurado los permisos predeterminados del supergrupo para ese usuario.",
        Ok(_) => "Si el usuario estaba expulsado, ya puede volver a entrar mediante una invitación o enlace. No se le ha añadido automáticamente.",
        Err(_) => {
            log::warn!("Telegram reverse moderation request failed");
            "No he podido aplicar esa acción. Comprueba que soy administrador con permiso para restringir miembros y que el usuario seleccionado no es administrador."
        }
    };
    send_text(bot, msg, feedback).await
}

fn validate_ban_duration(chat: &Chat, unit: &types::UnitOfTime) -> Result<(), &'static str> {
    if chat.is_group() && *unit != types::UnitOfTime::Permanent {
        return Err("Las expulsiones temporales requieren un supergrupo. Telegram no permite que las expulsiones caduquen en grupos básicos. Usa /ban 0 p para una expulsión permanente.");
    }
    Ok(())
}

fn moderation_target(
    msg: &Message,
    mute: bool,
    bot_id: UserId,
) -> Result<(UserId, UserId), &'static str> {
    if !(msg.chat.is_supergroup() || !mute && msg.chat.is_group()) {
        return Err(if mute {
            "Para silenciar usuarios se necesita un supergrupo."
        } else {
            "Para expulsar usuarios se necesita un grupo o supergrupo."
        });
    }
    if msg.sender_chat.is_some() {
        return Err(
            "Los administradores anónimos deben enviar este comando desde su cuenta personal.",
        );
    }
    let caller = msg
        .from
        .as_ref()
        .ok_or("No he podido identificar a quien envió el comando.")?;
    let reply = msg
        .reply_to_message()
        .ok_or("Responde al mensaje de un usuario con este comando.")?;
    if reply.sender_chat.is_some() || reply.forum_topic_created().is_some() {
        return Err("Responde a un mensaje enviado por un usuario, no por un canal ni a un mensaje de creación de tema.");
    }
    let target = reply
        .from
        .as_ref()
        .ok_or("Ese mensaje no tiene un usuario identificable al que moderar.")?;
    if target.id == bot_id {
        return Err("No puedo moderarme a mí mismo.");
    }
    Ok((caller.id, target.id))
}

fn can_restrict(kind: &ChatMemberKind) -> bool {
    match kind {
        ChatMemberKind::Owner(_) => true,
        ChatMemberKind::Administrator(admin) => admin.can_restrict_members,
        _ => false,
    }
}

#[derive(Debug, PartialEq)]
enum Expiry {
    Forever,
    Until(DateTime<Utc>),
}

fn calc_time(
    amount: types::TimeAmount,
    unit: types::UnitOfTime,
    now: DateTime<Utc>,
) -> Result<Expiry, &'static str> {
    let seconds = i64::from(amount)
        * match unit {
            types::UnitOfTime::Permanent => return Ok(Expiry::Forever),
            types::UnitOfTime::Seconds => 1,
            types::UnitOfTime::Minutes => 60,
            types::UnitOfTime::Hours => 3600,
        };
    // Telegram interprets <30 seconds and >366 days as permanent. Keep a network-delay margin.
    if !(60..=366 * 24 * 3600).contains(&seconds) {
        return Err("Las duraciones temporales deben ser de al menos 60 segundos y como máximo 366 días. Usa N (0–255) y h (horas), m (minutos), s (segundos) o p (permanente).");
    }
    now.checked_add_signed(Duration::seconds(seconds))
        .map(Expiry::Until)
        .ok_or("Esa fecha de caducidad está fuera del intervalo permitido.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::tests::message;
    use serde_json::json;
    use teloxide::requests::HasPayload;

    #[test]
    fn reversal_payloads_never_kick_or_grant_extra_permissions() {
        let bot = Bot::new("offline-placeholder");
        let unban = unban_request(&bot, ChatId(-1001), UserId(2));
        let value = serde_json::to_value(unban.payload_ref()).unwrap();
        assert_eq!(value["only_if_banned"], true);
        assert_eq!(value["user_id"], 2);
        let defaults = ChatPermissions::SEND_MESSAGES | ChatPermissions::SEND_POLLS;
        let unmute =
            unmute_request(&bot, ChatId(-1001), UserId(2), Some(defaults.clone())).unwrap();
        let value = serde_json::to_value(unmute.payload_ref()).unwrap();
        assert_eq!(
            value["permissions"],
            serde_json::to_value(&defaults).unwrap()
        );
        assert_eq!(value["use_independent_chat_permissions"], true);
        assert!(value.get("until_date").is_none());
        assert_ne!(defaults, ChatPermissions::all());
        let empty = unmute_request(
            &bot,
            ChatId(-1001),
            UserId(2),
            Some(ChatPermissions::empty()),
        )
        .unwrap();
        assert_eq!(empty.payload_ref().permissions, ChatPermissions::empty());
        assert!(unmute_request(&bot, ChatId(-1001), UserId(2), None).is_err());
    }

    #[test]
    fn unmute_only_changes_restricted_current_members() {
        for (status, member, allowed) in [
            ("restricted", true, true),
            ("restricted", false, false),
            ("kicked", false, false),
            ("member", true, false),
            ("left", false, false),
            ("creator", true, false),
        ] {
            let kind = serde_json::from_value(json!({"status":status,"is_member":member,
                "is_anonymous":false,"until_date":0,"can_send_messages":false,
                "can_send_audios":false,"can_send_documents":false,"can_send_photos":false,
                "can_send_videos":false,"can_send_video_notes":false,"can_send_voice_notes":false,
                "can_send_polls":false,"can_send_other_messages":false,"can_add_web_page_previews":false,
                "can_change_info":false,"can_invite_users":false,"can_pin_messages":false,"can_manage_topics":false})).unwrap();
            assert_eq!(unmute_target(&kind).is_ok(), allowed, "{status}");
        }
    }

    #[test]
    fn reversal_commands_have_no_arguments_and_respect_mentions() {
        for command in ["/unban", "/unmute"] {
            assert!(AdminCommand::parse(command, "mybot").is_ok());
            assert!(AdminCommand::parse(&format!("{command}@MYBOT"), "mybot").is_ok());
            assert!(AdminCommand::parse(&format!("{command}@otherbot"), "mybot").is_err());
            assert!(super::super::malformed_command_usage(command, "mybot", 200).is_none());
            assert!(super::super::malformed_command_usage(
                &format!("{command} extra"),
                "mybot",
                200
            )
            .unwrap()
            .contains("sin argumentos"));
        }
    }

    #[test]
    fn basic_groups_allow_only_permanent_bans() {
        let basic = message(json!({"chat":{"id":-1,"type":"group","title":"Group"}}));
        for unit in [
            types::UnitOfTime::Seconds,
            types::UnitOfTime::Minutes,
            types::UnitOfTime::Hours,
        ] {
            assert!(validate_ban_duration(&basic.chat, &unit)
                .unwrap_err()
                .contains("Las expulsiones temporales requieren un supergrupo"));
            assert!(validate_ban_duration(&message(json!({})).chat, &unit).is_ok());
        }
        assert!(validate_ban_duration(&basic.chat, &types::UnitOfTime::Permanent).is_ok());
    }

    #[test]
    fn durations_are_explicit_and_checked() {
        let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        for amount in [0, 1, 29, 30, 59] {
            assert!(calc_time(amount, types::UnitOfTime::Seconds, now)
                .unwrap_err()
                .starts_with("Las duraciones temporales deben ser de al menos 60 segundos"));
        }
        for (amount, unit, seconds) in [
            (60, types::UnitOfTime::Seconds, 60),
            (255, types::UnitOfTime::Seconds, 255),
            (10, types::UnitOfTime::Minutes, 600),
            (255, types::UnitOfTime::Hours, 918_000),
        ] {
            assert_eq!(
                calc_time(amount, unit, now),
                Ok(Expiry::Until(now + Duration::seconds(seconds)))
            );
        }
        assert_eq!(
            calc_time(0, types::UnitOfTime::Permanent, now),
            Ok(Expiry::Forever)
        );
        assert_eq!(
            calc_time(255, types::UnitOfTime::Hours, DateTime::<Utc>::MAX_UTC).unwrap_err(),
            "Esa fecha de caducidad está fuera del intervalo permitido."
        );
    }

    #[test]
    fn moderation_rejects_unsafe_targets_and_callers() {
        let mut msg = message(json!({"text":"/ban 1 h", "reply_to_message": {
            "message_id":2, "date":0, "chat":{"id":-1001,"type":"supergroup","title":"Group"},
            "from":{"id":2,"is_bot":false,"first_name":"Target"}, "text":"hi"}}));
        assert_eq!(
            moderation_target(&msg, false, UserId(3)),
            Ok((UserId(1), UserId(2)))
        );
        assert_eq!(
            moderation_target(&msg, false, UserId(2)).unwrap_err(),
            "No puedo moderarme a mí mismo."
        );
        msg.sender_chat = Some(msg.chat.clone());
        assert_eq!(
            moderation_target(&msg, false, UserId(3)).unwrap_err(),
            "Los administradores anónimos deben enviar este comando desde su cuenta personal."
        );
        msg.sender_chat = None;
        msg.from = None;
        assert_eq!(
            moderation_target(&msg, false, UserId(3)).unwrap_err(),
            "No he podido identificar a quien envió el comando."
        );
        for extra in [
            json!({}),
            json!({"from":null}),
            json!({"sender_chat":{"id":-1002,"type":"channel","title":"Channel"}}),
            json!({"forum_topic_created":{"name":"Topic","icon_color":0}}),
        ] {
            let mut reply = json!({"message_id":2,"date":0,"chat":{"id":-1001,"type":"supergroup","title":"Group"},
                "from":{"id":2,"is_bot":false,"first_name":"Target"},"text":"hi"});
            for (key, value) in extra.as_object().unwrap() {
                reply[key] = value.clone();
            }
            if extra.get("forum_topic_created").is_some() {
                reply.as_object_mut().unwrap().remove("text");
            }
            let msg = message(json!({"text":"/mute 1 h", "reply_to_message":reply}));
            assert_eq!(
                moderation_target(&msg, true, UserId(3)).is_ok(),
                extra == json!({})
            );
        }
        let msg = message(json!({"text":"command"}));
        assert_eq!(
            moderation_target(&msg, false, UserId(3)).unwrap_err(),
            "Responde al mensaje de un usuario con este comando."
        );
        for (kind, mute, allowed) in [
            ("private", false, false),
            ("group", true, false),
            ("group", false, true),
            ("channel", false, false),
        ] {
            let msg = message(
                json!({"text":"/ban 1 h", "chat":{"id":-1,"type":kind,"title":"Group","first_name":"Person"},
                "reply_to_message":{"message_id":2,"date":0,"chat":{"id":1,"type":"private","first_name":"Target"},
                "from":{"id":2,"is_bot":false,"first_name":"Target"},"text":"hi"}}),
            );
            assert_eq!(moderation_target(&msg, mute, UserId(3)).is_ok(), allowed);
        }
    }

    #[test]
    fn only_owner_or_admin_with_restrict_rights() {
        let owner =
            serde_json::from_value(json!({"status":"creator","is_anonymous":false})).unwrap();
        assert!(can_restrict(&owner));
        for status in ["member", "left"] {
            assert!(!can_restrict(
                &serde_json::from_value(json!({"status":status})).unwrap()
            ));
        }
        for allowed in [true, false] {
            let admin = serde_json::from_value(json!({"status":"administrator", "is_anonymous":false,
                "can_be_edited":false,"can_manage_chat":true,"can_change_info":false,"can_delete_messages":false,
                "can_manage_video_chats":false,"can_invite_users":false,"can_restrict_members":allowed,"can_promote_members":false})).unwrap();
            assert_eq!(can_restrict(&admin), allowed);
        }
    }

    #[test]
    fn command_parsing_reports_invalid_units_and_amounts() {
        assert!(AdminCommand::parse("/mute 1 h", "bot").is_ok());
        for input in [
            "/mute",
            "/ban -1 h",
            "/ban 256 s",
            "/mute 2 x",
            "/ban 1 h extra",
        ] {
            assert!(AdminCommand::parse(input, "bot").is_err());
        }
    }
}
