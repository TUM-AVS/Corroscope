#[derive(PartialEq, PartialOrd)]
pub struct Finite<Floating>(Floating);

pub type Finite32 = Finite<f32>;

#[allow(unused)]
pub type Finite64 = Finite<f64>;

impl Finite<f32> {
    pub fn value(x: f32) -> Finite<f32> {
        match Self::try_value(x) {
            Some(x) => x,
            None => {
                panic!("tried to pass non-finite value to Finite::value");
            }
        }
    }

    // Can be const when const_float_classify is stabilized
    pub fn try_value(x: f32) -> Option<Finite<f32>> {
        if x.is_finite() {
            Some(Finite(x))
        } else {
            None
        }
    }

    pub const MAX: Finite<f32> = Finite(f32::MAX);
    pub const MIN: Finite<f32> = Finite(f32::MIN);
}

impl Eq for Finite<f32> {}

impl Ord for Finite<f32> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Eq for Finite<f64> {}

impl Ord for Finite<f64> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl From<i32> for Finite<f32> {
    fn from(value: i32) -> Self {
        Finite(value as f32)
    }
}

impl From<i32> for Finite<f64> {
    fn from(value: i32) -> Self {
        Finite(value as f64)
    }
}

impl From<Finite<f32>> for f32 {
    fn from(value: Finite<f32>) -> Self {
        value.0
    }
}

impl From<Finite<f64>> for f64 {
    fn from(value: Finite<f64>) -> Self {
        value.0
    }
}

impl TryFrom<f32> for Finite<f32> {
    type Error = ();

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if value.is_finite() {
            Ok(Finite(value))
        } else {
            Err(())
        }
    }
}

impl TryFrom<f64> for Finite<f64> {
    type Error = ();

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_finite() {
            Ok(Finite(value))
        } else {
            Err(())
        }
    }
}

impl<Floating: std::ops::Neg> std::ops::Neg for Finite<Floating> {
    type Output = Finite<<Floating as std::ops::Neg>::Output>;

    fn neg(self) -> Self::Output {
        Finite(-(self.0))
    }
}
