"""Payment webhook handler — demo file for Becket multi-language indexing."""

from typing import Any


def handle_payment_event(payload: dict[str, Any]) -> None:
    """Process an incoming payment webhook."""
    event_type = payload.get("type", "")
    if event_type == "payment.captured":
        notify_fulfillment(payload)


def notify_fulfillment(payload: dict[str, Any]) -> None:
    order_id = payload.get("order_id")
    print(f"fulfillment queued for order {order_id}")
