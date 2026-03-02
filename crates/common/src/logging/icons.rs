use tracing::Level;

use super::colors;

pub struct LevelIcon {
    pub symbol: &'static str,
    pub color: &'static str,
}

pub fn for_level(level: &Level) -> LevelIcon {
    match *level {
        Level::ERROR => LevelIcon {
            symbol: "✗",
            color: colors::BRIGHT_RED,
        },
        Level::WARN => LevelIcon {
            symbol: "⚠",
            color: colors::YELLOW,
        },
        Level::INFO => LevelIcon {
            symbol: "✓",
            color: colors::GREEN,
        },
        Level::DEBUG => LevelIcon {
            symbol: "→",
            color: colors::GRAY,
        },
        Level::TRACE => LevelIcon {
            symbol: "·",
            color: colors::GRAY,
        },
    }
}
