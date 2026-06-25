"""Tests encoding INV-CALC-1 (half-even rounding). Run: python -m unittest discover -s tests."""
import unittest
from decimal import Decimal

from calc import discount, round_money


class TestRounding(unittest.TestCase):
    def test_total_rounding_is_half_even(self):
        # half-even rounds the .x5 boundary toward the even neighbor
        self.assertEqual(round_money("2.345"), Decimal("2.34"))
        self.assertEqual(round_money("2.355"), Decimal("2.36"))

    def test_discount_bounds(self):
        self.assertEqual(discount("100", 0), Decimal("100.00"))
        self.assertEqual(discount("100", 100), Decimal("0.00"))

    def test_discount_rejects_out_of_range(self):
        with self.assertRaises(ValueError):
            discount("100", 150)


if __name__ == "__main__":
    unittest.main()
