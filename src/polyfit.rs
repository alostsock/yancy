use nalgebra::{DMatrix, DVector};

/// Returns estimated polynomial regression coefficients
/// https://en.wikipedia.org/wiki/Polynomial_regression#Matrix_form_and_calculation_of_estimates
pub fn estimate(x: &DVector<f64>, y: &DVector<f64>, degree: usize) -> DVector<f64> {
    // design matrix
    let mut dm: DMatrix<f64> = DMatrix::zeros(x.len(), degree + 1);
    for (i, xi) in x.iter().enumerate() {
        for j in 0..=degree {
            dm[(i, j)] = xi.powi(j as i32);
        }
    }

    // a = (X^T * X)^-1 * X^t * y
    (dm.transpose() * &dm)
        .try_inverse()
        .expect("matrix should be invertible")
        * &dm.transpose()
        * y
}

/// Evaluates x, given polynomial coefficients
pub fn evaluate(coeffs: &DVector<f64>, x: f64) -> f64 {
  coeffs.iter().enumerate().fold(0.0, |acc, (i, &coeff)| {
    acc + coeff * x.powi(i as i32)
  })
}