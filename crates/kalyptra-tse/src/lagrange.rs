//! Lagrange interpolation at zero over the ristretto scalar field.

use curve25519_dalek::scalar::Scalar;

/// Coefficients `l_i = prod_{j != i} x_j / (x_j - x_i)` so that `sum l_i * f(x_i) = f(0)`
/// for any polynomial of degree below `ids.len()`. Node ids are the x-coordinates.
///
/// Returns `None` if an id is zero or repeated (interpolation would be undefined).
pub fn coefficients_at_zero(ids: &[u16]) -> Option<Vec<Scalar>> {
    if ids.contains(&0) {
        return None;
    }
    let mut out = Vec::with_capacity(ids.len());
    for (a, &i) in ids.iter().enumerate() {
        let xi = Scalar::from(i as u64);
        let mut num = Scalar::ONE;
        let mut den = Scalar::ONE;
        for (b, &j) in ids.iter().enumerate() {
            if a == b {
                continue;
            }
            if i == j {
                return None;
            }
            let xj = Scalar::from(j as u64);
            num *= xj;
            den *= xj - xi;
        }
        out.push(num * den.invert());
    }
    Some(out)
}
