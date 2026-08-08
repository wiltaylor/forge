"""In-process event bus: publish a topic, fan it out to every subscriber.

Live-telemetry semantics, not a durable queue: each subscriber has a bounded
``asyncio.Queue`` (64); on overflow the oldest message is dropped and the
subscription is marked lagged. Who is told about the drop is the transport's
business — the WS route sends ``{"type": "lagged"}``, SSE just drops.
"""

from __future__ import annotations

import asyncio
from typing import Any

QUEUE_SIZE = 64


class Subscription:
    __slots__ = ("queue", "lagged")

    def __init__(self) -> None:
        self.queue: asyncio.Queue[tuple[str, Any]] = asyncio.Queue(maxsize=QUEUE_SIZE)
        self.lagged = False


class EventBus:
    def __init__(self) -> None:
        self.subscribers: set[Subscription] = set()
        self._loop: asyncio.AbstractEventLoop | None = None

    def subscribe(self) -> Subscription:
        self._loop = asyncio.get_running_loop()
        sub = Subscription()
        self.subscribers.add(sub)
        return sub

    def unsubscribe(self, sub: Subscription) -> None:
        self.subscribers.discard(sub)

    def publish(self, topic: str, data: Any = None) -> None:
        """Fan ``(topic, data)`` out to every subscriber. Thread-safe: when
        called off the loop that owns the subscribers, delivery is scheduled
        with ``call_soon_threadsafe``."""
        try:
            running = asyncio.get_running_loop()
        except RuntimeError:
            running = None
        if self._loop is not None and running is not self._loop:
            self._loop.call_soon_threadsafe(self._deliver, topic, data)
        else:
            self._deliver(topic, data)

    def _deliver(self, topic: str, data: Any) -> None:
        for sub in list(self.subscribers):
            try:
                sub.queue.put_nowait((topic, data))
            except asyncio.QueueFull:
                try:
                    sub.queue.get_nowait()  # drop oldest
                except asyncio.QueueEmpty:
                    pass
                sub.lagged = True
                try:
                    sub.queue.put_nowait((topic, data))
                except asyncio.QueueFull:
                    pass
