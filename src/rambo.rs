use crate::util::FloatRandom;
use num_traits::{Float, FloatConst};

pub enum Scale<F: Float> {
    Fixed(F),
    Uniform { min: F, max: F },
}

/// Generic RAMBO phase-space generator (10.1016/0010-4655(86)90119-0) based on the Fortran implementation of GoSam.
pub(crate) fn rambo<F: Float + FloatConst>(
    s: Scale<F>,
    masses: &[F],
    n_in: usize,
    rng: &mut impl FloatRandom<F>,
) -> (F, Vec<[F; 4]>) {
    let s = match s {
        Scale::Fixed(s) => s,
        Scale::Uniform { min, max } => rng.range(min, max),
    };
    let two: F = F::one() + F::one();
    let mut vecs = vec![[F::zero(); 4]; masses.len()];
    if n_in == 2 {
        let m1_sq = masses[0] * masses[0];
        let m2_sq = masses[1] * masses[1];
        let two_sqrt_s = two * s.sqrt();
        let a = (s + m1_sq - m2_sq) / two_sqrt_s;
        let b = (s - m1_sq + m2_sq) / two_sqrt_s;
        vecs[0][0] = a;
        vecs[0][3] = (a * a - m1_sq).sqrt();
        vecs[1][0] = b;
        vecs[1][3] = -(b * b - m2_sq).sqrt();
    } else {
        vecs[0][0] = masses[0];
    }
    let n = masses.len() - n_in;
    if n > 1 {
        let mut q = Vec::with_capacity(n);
        let mut u: [F; 4];
        for _ in 0..n {
            u = [
                rng.generate(),
                rng.generate(),
                rng.generate(),
                rng.generate(),
            ];
            let sin_theta = two * (u[0] * (F::one() - u[0])).sqrt();
            let phi = two * F::PI() * u[1];
            let e = -(u[2] * u[3]).ln();
            let (sin_phi, cos_phi) = phi.sin_cos();
            q.push([
                e,
                e * cos_phi * sin_theta,
                e * sin_phi * sin_theta,
                e * (two * u[0] - F::one()),
            ]);
        }
        let mut v = [F::zero(); 4];
        for i in 0..=3 {
            v[i] = q.iter().map(|q| q[i]).fold(F::zero(), |a, b| a + b);
        }
        let v_sq = v[0] * v[0] - v[1] * v[1] - v[2] * v[2] - v[3] * v[3];
        let m_inv = v_sq.sqrt().recip();
        let b = [-m_inv * v[1], -m_inv * v[2], -m_inv * v[3]];
        let gamma = (F::one() + b[0] * b[0] + b[1] * b[1] + b[2] * b[2]).sqrt();
        let a = (F::one() + gamma).recip();
        let x = m_inv * s.sqrt();
        for i in 0..n {
            let bq = b[0] * q[i][1] + b[1] * q[i][2] + b[2] * q[i][3];
            let m = n_in + i;
            let c = q[i][0] + a * bq;
            vecs[m][0] = x * (gamma * q[i][0] + bq);
            vecs[m][1] = x * (b[0] * c + q[i][1]);
            vecs[m][2] = x * (b[1] * c + q[i][2]);
            vecs[m][3] = x * (b[2] * c + q[i][3]);
        }
        let x = newton(s, &vecs[n_in..], &masses[n_in..]);
        for i in n_in..vecs.len() {
            vecs[i][0] = (masses[i] * masses[i] + x * x * vecs[i][0] * vecs[i][0]).sqrt();
            vecs[i][1] = x * vecs[i][1];
            vecs[i][2] = x * vecs[i][2];
            vecs[i][3] = x * vecs[i][3];
        }
    } else {
        for i in 0..3 {
            vecs[masses.len()][i] = vecs[0..vecs.len() - 1]
                .iter()
                .map(|v| v[i])
                .fold(F::zero(), |a, b| a + b);
        }
    }

    return (s, vecs);
}

#[inline]
fn newton<F: Float>(s: F, vecs: &[[F; 4]], masses: &[F]) -> F {
    let two: F = F::one() + F::one();
    let prec = two.powi(10) * F::epsilon();
    let neg_sqrt_s = -s.sqrt();
    let n = vecs.len();
    let mut fx = neg_sqrt_s;

    let mut x = two.recip();
    let mut n_iter = 0;
    while fx.abs() > prec && n_iter < 50 {
        fx = neg_sqrt_s;
        let mut fpx = F::zero();
        let x_sq = x * x;
        for i in 0..n {
            let p_sq = vecs[i][0] * vecs[i][0];
            let tmp = (masses[i] * masses[i] + x_sq * p_sq).sqrt();
            fx = fx + tmp;
            fpx = fpx + p_sq / tmp;
        }
        fpx = fpx + x;
        x = x - fx / fpx;
        n_iter += 1;
    }
    return x;
}

#[cfg(test)]
mod tests {
    use super::*;
    use fastrand::Rng;
    const N_ITER_TEST: usize = 1_000_000;
    #[test]
    fn rambo_test() {
        let mut rng = Rng::new();
        for n in 0..N_ITER_TEST {
            let masses = vec![0.0, 0.0, 125.0, 125.0, 0.0, 0.0];
            let (_, vecs) = rambo(Scale::Fixed(500.0_f64.powi(2)), &masses, 2, &mut rng);
            let e_ref = 0.5 * vecs.iter().map(|p| p[0].abs()).sum::<f64>();
            for i in 1..=3 {
                let prec = vecs.iter().map(|p| p[i]).sum::<f64>().abs() / e_ref;
                if prec > 1E-7 {
                    println!("{n}:{i}: {} ({} digits)", prec, -prec.log10());
                }
                assert!(prec < 1E-7);
            }
        }
    }
}
