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
        ctx: &FmtContext<'_, S, N>,
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
            " {DIM}{time}{R}  {ic}{sym}{R} {B}{cc}{label:<8}{R}  {msg}",
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

        if let Some(scope) = ctx.event_scope() {
            let mut span_fields: Vec<(String, String)> = Vec::new();

            for span in scope {
                let extensions = span.extensions();
                if let Some(fields) = extensions.get::<SpanFields>() {
                    for (k, v) in &fields.0 {
                        if !span_fields.iter().any(|(ek, _)| ek == k)
                            && !visitor.fields.iter().any(|(ek, _)| ek == k)
                        {
                            span_fields.push((k.clone(), v.clone()));
                        }
                    }
                }
            }

            for (key, value) in &span_fields {
                write!(
                    writer,
                    "  {DIM}{key}{R}={value}",
                    DIM = colors::DIM,
                    R = colors::RESET,
                )?;
            }
        }

        writeln!(writer)
    }
}

#[derive(Default)]
pub struct SpanFields(pub Vec<(String, String)>);

pub struct SpanFieldLayer;

impl<S> tracing_subscriber::Layer<S> for SpanFieldLayer
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Some(span) = ctx.span(id) {
            let mut fields = SpanFields::default();
            let mut visitor = MessageVisitor::new();
            attrs.record(&mut visitor);
            fields.0 = visitor.fields;
            span.extensions_mut().insert(fields);
        }
    }

    fn on_record(
        &self,
        id: &tracing::span::Id,
        values: &tracing::span::Record<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Some(span) = ctx.span(id) {
            let mut visitor = MessageVisitor::new();
            values.record(&mut visitor);
            let mut extensions = span.extensions_mut();
            if let Some(fields) = extensions.get_mut::<SpanFields>() {
                for (k, v) in visitor.fields {
                    if let Some(existing) = fields.0.iter_mut().find(|(ek, _)| *ek == k) {
                        existing.1 = v;
                    } else {
                        fields.0.push((k, v));
                    }
                }
            }
        }
    }
}
