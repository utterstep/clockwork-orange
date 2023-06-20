use std::borrow::Cow;

use once_cell::sync::Lazy;
use pulldown_cmark::{Event, Options as DeOptions, Parser, Tag};
use pulldown_cmark_to_cmark::Options as SerOptions;
use regex::{Captures, Regex};

macro_rules! regex {
    ($re:literal $(,)?) => {
        Lazy::new(|| regex::Regex::new($re).unwrap())
    };
}

#[macro_export]
macro_rules! md_message {
    ($($args:tt)*) => {
        $crate::md!(&$crate::message!($($args)*))
    };
}

#[macro_export]
macro_rules! md {
    ($message:expr) => {
        $crate::tg_escape($message)
    };
}

#[macro_export]
macro_rules! message {
    ($message_path:literal) => {{
        format!(include_str!(concat!(::std::env!("CARGO_MANIFEST_DIR"), "/messages/", $message_path)))
    }};
    ($message_path:literal, $($args:tt)*) => {{
        format!(include_str!(concat!(::std::env!("CARGO_MANIFEST_DIR"), "/messages/", $message_path)), $($args)*)
    }}
}

static TG_MD_ESCAPE_REGEX: Lazy<Regex> = regex!(r"[_*\[\]()~`>#+\-=|{}\.!\\]");
static TG_MD_CODE_ESCAPE_REGEX: Lazy<Regex> = regex!(r"[`\\]");
static TG_MD_SERIALIZE_OPTIONS: Lazy<SerOptions> = Lazy::new(|| SerOptions {
    code_block_token_count: 3,
    ..Default::default()
});

/// Escapes given text, abiding Telegram flavoured Markdown
/// [rules](https://core.telegram.org/bots/api#formatting-options).
pub fn tg_escape(text: &str) -> String {
    let mut options = DeOptions::empty();
    options.insert(DeOptions::ENABLE_STRIKETHROUGH);

    let mut inside_code = false;

    let parser = Parser::new_ext(text, options).map(|event| {
        dbg!(&event);

        match &event {
            Event::Start(Tag::CodeBlock(_)) => {
                inside_code = true;

                event
            }
            Event::End(Tag::CodeBlock(_)) => {
                inside_code = false;

                event
            }
            Event::Text(text) | Event::Code(text) => {
                let re = if inside_code {
                    &TG_MD_CODE_ESCAPE_REGEX
                } else {
                    &TG_MD_ESCAPE_REGEX
                };

                // manual COW implementation...
                let replaced = re.replace_all(text, |caps: &Captures| {
                    dbg!(&caps);
                    format!("\x5C{}", &caps[0])
                });
                dbg!(&replaced);
                match replaced {
                    Cow::Borrowed(_) => event,
                    Cow::Owned(text) => match event {
                        Event::Text(_) => Event::Text(text.into()),
                        Event::Code(_) => Event::Code(text.into()),
                        _ => unreachable!(),
                    },
                }
            }
            _ => event,
        }
    });

    let mut res = String::with_capacity(text.len());

    pulldown_cmark_to_cmark::cmark_with_options(parser, &mut res, TG_MD_SERIALIZE_OPTIONS.clone())
        .expect("writing to string failed!");

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let graph = "123";

        assert_eq!(
            message!("test/stats_for_today.md", graph = graph),
            "Присылаю статистику по ответам всех за сегодня:\n\n```\n  %\n123\n```"
        );
        assert_eq!(
            md_message!("test/stats_for_today.md", graph = graph),
            "Присылаю статистику по ответам всех за сегодня:\n\n```\n  %\n123\n```"
        );
    }

    #[test]
    fn test_md_escape() {
        assert_eq!(
            tg_escape("Скоро тебе придёт статистика за сегодня, а в целом – доступную стату можно посмотреть по запросу /get_stat :)"),
            r#"Скоро тебе придёт статистика за сегодня, а в целом – доступную стату можно посмотреть по запросу /get\_stat :\)"#
        );
    }

    #[test]
    #[ignore = "Need to debug this double-quotation issue"]
    fn test_nausicaa() {
        assert_eq!(
            tg_escape(
                "https://en.wikipedia.org/wiki/Nausica%C3%A4_of_the_Valley_of_the_Wind_(film)"
            ),
            r#"https://en\.wikipedia\.org/wiki/Nausica%C3%A4\_of\_the\_Valley\_of\_the\_Wind\_\(film\)"#
        );
    }
}
