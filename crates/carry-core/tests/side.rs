use carry_core::Side;

#[test]
fn opposite_flips_side() {
    assert_eq!(Side::Bid.opposite(), Side::Ask);
    assert_eq!(Side::Ask.opposite(), Side::Bid);
}

#[test]
fn opposite_twice_is_identity() {
    for side in [Side::Bid, Side::Ask] {
        assert_eq!(side.opposite().opposite(), side);
    }
}
