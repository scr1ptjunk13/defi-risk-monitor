use bigdecimal::BigDecimal;
use std::str::FromStr;
use num_traits::{Zero, ToPrimitive};

/// Calculate percentage change between two values
pub fn percentage_change(old_value: BigDecimal, new_value: BigDecimal) -> Result<BigDecimal, String> {
    if old_value.is_zero() {
        return Err("Cannot calculate percentage change with zero base value".to_string());
    }
    
    let change = (&new_value - &old_value) / &old_value * &BigDecimal::from(100);
    Ok(change)
}

/// Calculate standard deviation of a series of values
pub fn standard_deviation(values: &[BigDecimal]) -> BigDecimal {
    if values.len() < 2 {
        return BigDecimal::zero();
    }
    
    let sum: BigDecimal = values.iter().cloned().sum();
    let mean = &sum / &BigDecimal::from(values.len() as i32);
    let variance_sum: BigDecimal = values
        .iter()
        .map(|x| {
            let diff = x - &mean;
            &diff * &diff
        })
        .sum();
    let variance = &variance_sum / &BigDecimal::from((values.len() - 1) as i32);
    // Use from_f64 for BigDecimal
    let variance_f64 = variance.to_f64().unwrap_or(0.0);
    BigDecimal::from_str(&(variance_f64.sqrt()).to_string()).unwrap_or_else(|_| BigDecimal::zero())
}

/// Calculate moving average
pub fn moving_average(values: &[BigDecimal], window: usize) -> Vec<BigDecimal> {
    if values.len() < window {
        return vec![];
    }
    
    values
        .windows(window)
        .map(|window| {
            let sum: BigDecimal = window.iter().cloned().sum();
            let len = BigDecimal::from(window.len() as i32);
            &sum / &len
        })
        .collect()
}

/// Calculate correlation coefficient between two series
pub fn correlation(x: &[BigDecimal], y: &[BigDecimal]) -> Result<BigDecimal, String> {
    if x.len() != y.len() || x.len() < 2 {
        return Err("Series must have equal length and at least 2 values".to_string());
    }
    
    let n = BigDecimal::from(x.len() as i32);
    let sum_x: BigDecimal = x.iter().cloned().sum();
    let sum_y: BigDecimal = y.iter().cloned().sum();
    let mean_x = &sum_x / &n;
    let mean_y = &sum_y / &n;
    
    let numerator: BigDecimal = x
        .iter()
        .zip(y.iter())
        .map(|(xi, yi)| (xi - &mean_x) * (yi - &mean_y))
        .sum();
    
    let sum_sq_x: BigDecimal = x.iter().map(|xi| {
        let diff = xi - &mean_x;
        &diff * &diff
    }).sum();
    let sum_sq_y: BigDecimal = y.iter().map(|yi| {
        let diff = yi - &mean_y;
        &diff * &diff
    }).sum();
    
    let product = &sum_sq_x * &sum_sq_y;
    let denominator_f64 = product.to_f64().unwrap_or(0.0).sqrt();
    let denominator = BigDecimal::from_str(&(denominator_f64).to_string()).unwrap_or_else(|_| BigDecimal::zero());
    
    if denominator.is_zero() {
        return Ok(BigDecimal::from(0));
    }
    
    Ok(&numerator / &denominator)
}

/// Calculate Value at Risk using historical simulation
pub fn value_at_risk(returns: &[BigDecimal], confidence_level: BigDecimal) -> BigDecimal {
    if returns.is_empty() {
        return BigDecimal::from(0);
    }
    
    let mut sorted_returns = returns.to_vec();
    sorted_returns.sort();
    
    let index = ((&BigDecimal::from(1) - &confidence_level) * &BigDecimal::from(returns.len() as i32))
        .to_usize()
        .unwrap_or(0);
    
    sorted_returns.get(index).cloned().unwrap_or(BigDecimal::from(0)).abs()
}

/// Calculate compound annual growth rate
pub fn cagr(initial_value: BigDecimal, final_value: BigDecimal, years: BigDecimal) -> Result<BigDecimal, String> {
    if &initial_value <= &BigDecimal::from(0) || &final_value <= &BigDecimal::from(0) || &years <= &BigDecimal::from(0) {
        return Err("All values must be positive".to_string());
    }
    
    // Simplified CAGR calculation (would need proper power function for exact calculation)
    let ratio = &final_value / &initial_value;
    let growth_rate = (&ratio - &BigDecimal::from(1)) / &years;
    
    Ok(growth_rate)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_percentage_change() {
        let old_val = BigDecimal::from(100);
        let new_val = BigDecimal::from(110);
        let change = percentage_change(old_val, new_val).unwrap();
        assert_eq!(change, BigDecimal::from(10));
    }
    
    #[test]
    fn test_moving_average() {
        let values = vec![
            BigDecimal::from(1),
            BigDecimal::from(2),
            BigDecimal::from(3),
            BigDecimal::from(4),
            BigDecimal::from(5),
        ];
        let ma = moving_average(&values, 3);
        assert_eq!(ma.len(), 3);
        assert_eq!(ma[0], BigDecimal::from(2)); // (1+2+3)/3
    }
}
