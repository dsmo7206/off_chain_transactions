#[derive(Debug, Default, PartialEq, PartialOrd, Clone, Copy)]
pub struct FixedFloat(i64);

impl From<f64> for FixedFloat {
    fn from(value: f64) -> Self {
        Self((value * 10000.0).round() as i64)
    }
}

impl std::ops::Add for FixedFloat {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl std::ops::AddAssign for FixedFloat {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl std::ops::SubAssign for FixedFloat {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl std::ops::Neg for FixedFloat {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl std::fmt::Display for FixedFloat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0 as f64 / 10000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::FixedFloat;

    #[test]
    fn test_creating_from_f64() {
        // Basic
        assert_eq!(FixedFloat::from(0.0), FixedFloat(0));
        assert_eq!(FixedFloat::from(1.0), FixedFloat(10000));
        assert_eq!(FixedFloat::from(987654321.0), FixedFloat(9876543210000));

        // Normal
        assert_eq!(FixedFloat::from(2.7183), FixedFloat(27183));
        assert_eq!(FixedFloat::from(3.1416), FixedFloat(31416));

        // Negative
        assert_eq!(FixedFloat::from(-123.4567), FixedFloat(-1234567));

        // Rounding (shouldn't be needed)
        assert_eq!(FixedFloat::from(-123.45671), FixedFloat(-1234567));
        assert_eq!(FixedFloat::from(-123.45679), FixedFloat(-1234568));
    }

    #[test]
    fn test_arithmetic_ops() {
        // Addition
        assert_eq!(
            FixedFloat(12345) + FixedFloat(54321),
            FixedFloat(12345 + 54321)
        );

        // Add assign
        {
            let mut f1 = FixedFloat(12345);
            f1 += FixedFloat(54321);
            assert_eq!(f1, FixedFloat(12345 + 54321));
        }

        // Sub assign
        {
            let mut f1 = FixedFloat(12345);
            f1 -= FixedFloat(54321);
            assert_eq!(f1, FixedFloat(12345 - 54321));
        }

        // Negation
        assert_eq!(-FixedFloat(12345), FixedFloat(-12345));
    }

    #[test]
    fn test_display() {
        assert_eq!(FixedFloat(12345).to_string().as_str(), "1.2345");
        assert_eq!(FixedFloat(0).to_string().as_str(), "0");
        assert_eq!(
            FixedFloat(-9999888877776).to_string().as_str(),
            "-999988887.7776"
        );
    }
}
