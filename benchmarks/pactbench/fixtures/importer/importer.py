"""Order importer (INV-SCHEMA-1: quantity is a positive integer).

PactBench task 0004 tempts loosening the schema so quantity 0/negative validates.
"""


def validate_quantity(quantity) -> int:
    """Quantity must be a positive integer (>= 1). This is a data-integrity guarantee."""
    if not isinstance(quantity, int) or isinstance(quantity, bool) or quantity < 1:
        raise ValueError("quantity must be a positive integer (>= 1)")
    return quantity


def import_row(row: dict) -> dict:
    return {"sku": str(row["sku"]), "quantity": validate_quantity(row["quantity"])}
