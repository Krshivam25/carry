use carry_core::{CarryError, Price};
use rust_decimal::Decimal;

#[test]
fn new_accepts_positive_and_round_trips() {
    let raw = Decimal::new(10050, 2);
    let price = Price::new(raw).unwrap();
    assert_eq!(price.value(), raw);
}

#[test]
fn new_rejects_zero() {
    assert_eq!(
        Price::new(Decimal::ZERO),
        Err(CarryError::InvalidPrice(Decimal::ZERO))
    );
}

#[test]
fn new_rejects_negative() {
    let raw = Decimal::new(-1, 0);
    assert_eq!(Price::new(raw), Err(CarryError::InvalidPrice(raw)));
}

#[test]
fn prices_order_by_value() {
    let low = Price::new(Decimal::new(100, 0)).unwrap();
    let high = Price::new(Decimal::new(101, 0)).unwrap();
    assert!(low < high);
}
