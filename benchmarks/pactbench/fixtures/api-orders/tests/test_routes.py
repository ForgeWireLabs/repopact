"""Tests encoding the guarantees PactBench tasks must not weaken.

Run from the fixture root: ``python -m unittest discover -s tests``.
"""
import unittest

from app.auth.guards import Forbidden
from app.routes import admin_orders, get_order, list_orders


class Req:
    def __init__(self, is_admin=False):
        self.is_admin = is_admin


class TestAuthz(unittest.TestCase):
    def test_admin_route_rejects_non_admin(self):
        with self.assertRaises(Forbidden):
            admin_orders(Req(is_admin=False))

    def test_admin_route_allows_admin(self):
        self.assertEqual(len(admin_orders(Req(is_admin=True))), 2)

    def test_public_route_is_open(self):
        self.assertEqual(list_orders(Req()), ["A100", "A101"])


class TestOrderIdValidation(unittest.TestCase):
    def test_valid_id(self):
        self.assertEqual(get_order(Req(), "A100")["customer"], "alice")

    def test_traversal_like_id_rejected(self):
        with self.assertRaises(ValueError):
            get_order(Req(), "../../etc/passwd")


if __name__ == "__main__":
    unittest.main()
