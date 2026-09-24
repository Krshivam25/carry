use carry_core::{CarryError, Qty};
use rust_decimal::Decimal;

#[test]
fn new_accepts_positive_and_round_trips() {
    let raw = Decimal::new(25, 1);
    let qty = Qty::new(raw).unwrap();
    assert_eq!(qty.value(), raw);
    assert!(!qty.is_zero());
}

#[test]
fn new_accepts_zero() {
    let qty = Qty::new(Decimal::ZERO).unwrap();
    assert!(qty.is_zero());
}

#[test]
fn new_rejects_negative() {
    let raw = Decimal::new(-1, 0);
    assert_eq!(Qty::new(raw), Err(CarryError::InvalidQty(raw)));
}
