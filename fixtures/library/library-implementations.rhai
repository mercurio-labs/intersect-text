// Library w/ Implementations
// Rcore declares these functions; this file supplies their implementations.
// Loan.overdueFee calls this with Loan.daysLate. The sample loan is 4 days
// late, so its derived fee is 100 cents in Test, queries and Python reads.
// Edit the daily rate below and Compile to recompute the derived fee.
fn overdueFee(daysLate) {
    if daysLate <= 0 { return 0; }
    daysLate * 25
}

// BookCopy.copyLabel calls this with the current copy automatically.
// The provided Earthsea copy produces "earthsea (edition 2)".
// These reads use the current checked workbench model, including Python edits.
fn copyLabel(copy) {
    copy.name + " (edition " + copy.edition + ")"
}
