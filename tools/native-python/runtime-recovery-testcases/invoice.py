"""Small invoice application with an incorrect unit-price conversion."""
import csv
import io
from decimal import Decimal


# ------------------------=
# FUNC: total_invoice
# DESC: Totals quantity and unit_price columns and returns a two-decimal monetary amount.
# ------------------=
def total_invoice(csv_text):
    total = Decimal("0")
    for row in csv.DictReader(io.StringIO(csv_text)):
        quantity = int(row["quantity"])
        unit_price = Decimal(int(row["unit_price"]))
        total += quantity * unit_price
    return total.quantize(Decimal("0.01"))


if __name__ == "__main__":
    DEMO_TOTAL = total_invoice(
        "description,quantity,unit_price\n"
        "Notebooks,2,12.50\n"
        "Pens,3,1.25\n")
    print(DEMO_TOTAL)
