# Python consumes the same derived properties as the inspector and queries.
# Their implementations live in library-implementations.rhai, executed in WASM.
from intersect_api import BookCopy, Loan, model

loan = model.find(Loan, name="first")
copy = model.find(BookCopy, name="earthsea")
print("Fee before (cents):", loan.overdueFee["value"])
print("Label before:", copy.copyLabel)

loan.daysLate = {"kind": "int64", "value": "7"}
copy.edition = 3
print("Fee after (cents):", loan.overdueFee["value"])
print("Label after:", copy.copyLabel)
print("Validation:", model.validate())
