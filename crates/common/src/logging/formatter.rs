use std::fmt;

use tracing::Event;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

use super::category::Category;
use super::colors;
use super::icons;
use super::visitor::MessageVisitor;

pub struct SentinelFormatter;

impl<S, N> FormatEvent<S, N> for SentinelFormatter
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let meta = event.metadata();
        let level = meta.level();
        let target = meta.target();

        let time = chrono::Local::now().format("%H:%M:%S");
        let category = Category::from_target(target);
        let icon = icons::for_level(level);

        let mut visitor = MessageVisitor::new();
        event.record(&mut visitor);

        write!(
            writer,
            " {DIM}{time}{R}  {ic}{sym}{R} {B}{cc}{label:<7}{R}  {msg}",
            DIM = colors::DIM,
            R = colors::RESET,
            ic = icon.color,
            sym = icon.symbol,
            B = colors::BOLD,
            cc = category.color(),
            label = category.label(),
            msg = visitor.message,
        )?;

        for (key, value) in &visitor.fields {
            write!(
                writer,
                "  {DIM}{key}{R}={value}",
                DIM = colors::DIM,
                R = colors::RESET,
            )?;
        }

        writeln!(writer)
    }
}
