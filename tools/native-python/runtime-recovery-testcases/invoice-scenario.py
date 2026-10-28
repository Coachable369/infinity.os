"""Trusted arithmetic checks, kept separate from model-editable invoice source."""
from decimal import Decimal

MINIMUM_CHECKS = 9


# ------------------------=
# FUNC: verify
# DESC: Checks exact monetary results, CSV parsing, empty input, invalid input and ordinary main execution against candidate functions.
# ------------------=
def verify(candidate, check):
    check(candidate["__name__"] == "__main__")
    total = candidate["total_invoice"]
    check(callable(total))
    check(candidate["DEMO_TOTAL"] == Decimal("28.75"))
    check(total("description,quantity,unit_price\n") == Decimal("0.00"))
    check(total("description,quantity,unit_price\nPaper,3,2\n") == Decimal("6.00"))
    check(total("description,quantity,unit_price\nA,3,0.10\nB,1,0.20\n") == Decimal("0.50"))
    check(total('description,quantity,unit_price\n"Binder, blue",2,4.95\n') == Decimal("9.90"))
    check(isinstance(total("description,quantity,unit_price\nA,1,1.25\n"), Decimal))
    try:
        total("description,quantity,unit_price\nA,many,1.25\n")
    except ValueError:
        check(True)
    else:
        check(False)
