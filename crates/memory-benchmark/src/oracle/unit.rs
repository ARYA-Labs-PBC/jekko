#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitVec(pub [i8; 7]);

impl UnitVec {
    pub fn mul(self, rhs: Self) -> Self {
        let mut out = [0; 7];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = self.0[i] + rhs.0[i];
        }
        Self(out)
    }

    pub fn div(self, rhs: Self) -> Self {
        let mut out = [0; 7];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = self.0[i] - rhs.0[i];
        }
        Self(out)
    }

    pub fn pow(self, n: i8) -> Self {
        let mut out = [0; 7];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = self.0[i] * n;
        }
        Self(out)
    }
}
