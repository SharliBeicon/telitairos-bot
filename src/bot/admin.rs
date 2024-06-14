use crate::bot::*;
use chrono::Duration;
use teloxide::{
    payloads::RestrictChatMemberSetters,
    types::{ChatPermissions, ParseMode},
};

#[derive(BotCommands, Clone, PartialEq)]
#[command(
    rename_rule = "lowercase",
    description = "Supported ADMIN Commands",
    parse_with = "split"
)]
pub enum AdminCommand {
    #[command(description = "Display this text\\.")]
    Help,

    #[command(
        description = "`/mute X {h/m/s/p}` \\-\\> Mute an User from the Chat Group the selected time\\. 'p' is for 'permanent'"
    )]
    Mute(types::TimeAmount, types::UnitOfTime),

    #[command(
        description = "`/ban X {h/m/s/p}` \\-\\> Ban an User from the Chat Group the selected time\\. 'p' is for 'permanent'"
    )]
    Ban(types::TimeAmount, types::UnitOfTime),
}

pub async fn handle_admin_commands(
    bot: Bot,
    msg: Message,
    cmd: AdminCommand,
) -> ResponseResult<()> {
    match cmd {
        AdminCommand::Help => {
            bot.send_message(msg.chat.id, all_command_descriptions())
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
        }
        AdminCommand::Mute(time_amount, unit_of_time) => {
            mute_user(bot, msg, calc_time(time_amount, unit_of_time)).await?;
        }
        AdminCommand::Ban(time_amount, unit_of_time) => {
            ban_user(bot, msg, calc_time(time_amount, unit_of_time)).await?;
        }
    };

    Ok(())
}

async fn mute_user(bot: Bot, msg: Message, time: Option<Duration>) -> ResponseResult<()> {
    let duration = match time {
        Some(d) => d,
        None => {
            bot.send_message(msg.chat.id, "Send a properly formatted time span")
                .await?;
            return Ok(());
        }
    };

    match msg.reply_to_message() {
        Some(replied) => {
            bot.restrict_chat_member(
                msg.chat.id,
                replied.from().expect("Must be MessageKind::Common").id,
                ChatPermissions::empty(),
            )
            .until_date(msg.date + duration)
            .await?;
        }
        None => {
            bot.send_message(
                msg.chat.id,
                "Use this command in a reply to another message!",
            )
            .await?;
        }
    }

    Ok(())
}

async fn ban_user(bot: Bot, msg: Message, time: Option<Duration>) -> ResponseResult<()> {
    let duration = match time {
        Some(d) => d,
        None => {
            bot.send_message(msg.chat.id, "Send a properly formatted time span")
                .await?;
            return Ok(());
        }
    };

    match msg.reply_to_message() {
        Some(replied) => {
            bot.kick_chat_member(
                msg.chat.id,
                replied.from().expect("Must be MessageKind::Common").id,
            )
            .until_date(msg.date + duration)
            .await?;
        }
        None => {
            bot.send_message(
                msg.chat.id,
                "Use this command in a reply to another message!",
            )
            .await?;
        }
    }

    Ok(())
}

fn calc_time(time_amount: types::TimeAmount, unit_of_time: types::UnitOfTime) -> Option<Duration> {
    match unit_of_time {
        types::UnitOfTime::Seconds => Duration::try_seconds(time_amount.into()),
        types::UnitOfTime::Minutes => Duration::try_minutes(time_amount.into()),
        types::UnitOfTime::Hours => Duration::try_hours(time_amount.into()),
        types::UnitOfTime::Permanent => Some(Duration::max_value()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use chrono::Duration;

    #[test]
    fn calc_time_seconds() {
        let result = calc_time(10, UnitOfTime::Seconds);
        assert_eq!(result, Some(Duration::seconds(10)));
    }

    #[test]
    fn calc_time_minutes() {
        let result = calc_time(10, UnitOfTime::Minutes);
        assert_eq!(result, Some(Duration::seconds(600)));
    }

    #[test]
    fn calc_time_hours() {
        let result = calc_time(2, UnitOfTime::Hours);
        assert_eq!(result, Some(Duration::seconds(7200)));
    }

    #[test]
    fn calc_time_permanent() {
        let result = calc_time(0, UnitOfTime::Permanent);
        assert_eq!(result, Some(Duration::max_value()));
    }
}
