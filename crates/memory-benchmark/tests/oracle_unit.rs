use memory_benchmark::oracle::unit::UnitVec;

#[test]
fn unit_vec_algebra_is_integer_exact() {
    let velocity = UnitVec([1, 0, -1, 0, 0, 0, 0]);
    let time = UnitVec([0, 0, 1, 0, 0, 0, 0]);
    assert_eq!(velocity.mul(time), UnitVec([1, 0, 0, 0, 0, 0, 0]));
    assert_eq!(velocity.div(time), UnitVec([1, 0, -2, 0, 0, 0, 0]));
    assert_eq!(velocity.pow(2), UnitVec([2, 0, -2, 0, 0, 0, 0]));
}
