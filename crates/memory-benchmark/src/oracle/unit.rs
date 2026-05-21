#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitVec(pub [i8; 7]);

impl UnitVec {
    pub fn pow(self, n: i8) -> Self {
        let mut out = [0; 7];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = self.0[i] * n;
        }
        Self(out)
    }
}

impl std::ops::Mul for UnitVec {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: Self) -> Self::Output {
        let mut out = [0; 7];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = self.0[i] + rhs.0[i];
        }
        Self(out)
    }
}

impl std::ops::Div for UnitVec {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        let mut out = [0; 7];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = self.0[i] - rhs.0[i];
        }
        Self(out)
    }
}
