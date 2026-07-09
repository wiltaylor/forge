//! GET /api/events — Server-Sent Events endpoint.
//!
//! SSE frames: `event:` = topic, `data:` = JSON payload. Optional
//! `?topics=a,b` filter. Heartbeat comment `: ping` every 15 s.
//! Lagged consumers silently drop missed events (live telemetry, not a queue).

use std::collections::HashSet;
use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{Query, State};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use serde::Deserialize;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

use crate::events::EventBus;

#[derive(Debug, Deserialize)]
pub(crate) struct EventsQuery {
    /// Comma-separated topic filter; absent/empty = all topics.
    topics: Option<String>,
}

pub(crate) fn parse_topics(raw: Option<&str>) -> Option<HashSet<String>> {
    let raw = raw?;
    let set: HashSet<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect();
    if set.is_empty() {
        None
    } else {
        Some(set)
    }
}

pub(crate) async fn sse_handler(
    State(bus): State<EventBus>,
    Query(query): Query<EventsQuery>,
) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    let topics = parse_topics(query.topics.as_deref());
    let rx = bus.subscribe();

    let stream = BroadcastStream::new(rx).filter_map(move |item| match item {
        Ok(ev) => {
            if topics.as_ref().is_none_or(|t| t.contains(&ev.topic)) {
                Some(Ok(SseEvent::default().event(&ev.topic).data(&ev.json)))
            } else {
                None
            }
        }
        // Lagged: SSE just drops missed events.
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}
