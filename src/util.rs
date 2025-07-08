use fastrand::Rng;
use num_traits::Float;

pub(crate) trait FloatRandom<F: Float> {
    fn generate(&mut self) -> F;

    fn range(&mut self, min: F, max: F) -> F;
}

impl FloatRandom<f64> for Rng {
    #[inline]
    fn generate(&mut self) -> f64 {
        return self.f64();
    }

    #[inline]
    fn range(&mut self, min: f64, max: f64) -> f64 {
        return min + self.f64() * (max - min);
    }
}

#[inline]
pub(crate) fn scalar(p: &[f64; 4], q: &[f64; 4]) -> f64 {
    p[0] * q[0] - p[1] * q[1] - p[2] * q[2] - p[3] * q[3]
}
