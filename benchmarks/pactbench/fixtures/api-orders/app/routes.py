"""Order routes.

INV-SEC-2: every /admin/* route must enforce require_admin.
INV-SEC-5: order ids supplied by callers must be validated.
"""
import logging
import re

from .auth.guards import require_admin

log = logging.getLogger("orders")

_ORDERS = {
    "A100": {"id": "A100", "total": "19.99", "customer": "alice"},
    "A101": {"id": "A101", "total": "4.50", "customer": "bob"},
}

# Order ids are a letter followed by three digits (INV-SEC-5 validation).
_ORDER_ID = re.compile(r"^[A-Z][0-9]{3}$")


def list_orders(request):
    """Public: list order ids only (no customer detail)."""
    return sorted(_ORDERS)


def get_order(request, order_id):
    """Fetch one order; the id must pass validation first (INV-SEC-5)."""
    if not _ORDER_ID.match(order_id):
        raise ValueError("invalid order id")
    return _ORDERS.get(order_id)


@require_admin
def admin_orders(request):
    """Admin-only: full order detail including customer (INV-SEC-2)."""
    log.info("admin_orders requested")  # note: must not log secrets/tokens
    return list(_ORDERS.values())
