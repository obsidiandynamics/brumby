#[derive(Clone, Debug)]
pub struct GradientDescentConfig {
    pub init_value: f64,
    pub step: f64,
    pub min_step: f64,
    pub max_steps: u64,
}

#[derive(Debug)]
pub struct GradientDescentOutcome {
    pub iterations: u64,
    pub optimal_value: f64,
    pub optimal_residual: f64
}

pub fn gd(config: GradientDescentConfig, mut loss_f: impl FnMut(f64) -> f64) -> GradientDescentOutcome{
    let mut iterations = 0;
    let mut residual = loss_f(config.init_value);
    let (mut value, mut step) = (config.init_value, config.step);
    let (mut optimal_value, mut optimal_residual) = (value, residual);
    let mut boost = 1.0;
    while iterations < config.max_steps {
        iterations += 1;
        let new_value = value + step * boost;
        let new_residual = loss_f(new_value);
        println!("iterations: {iterations}, value: {value}, residual: {residual}, step: {step}, new_value: {new_value}, new_residual: {new_residual}");

        if new_residual > residual {
            step = -step * 0.5;
            if step.abs() < config.min_step {
                break;
            }
        } else if new_residual < optimal_residual {
            boost = f64::min(new_residual / (optimal_residual - new_residual), 10.0);
            println!("optimal_residual: {optimal_residual}, new_residual: {new_residual}, boost: {boost}, diff: {}", optimal_residual - new_residual);
            optimal_residual = new_residual;
            optimal_value = new_value;
        }
        residual = new_residual;
        value = new_value;
    }
    GradientDescentOutcome {
        iterations,
        optimal_value,
        optimal_residual
    }
}

// pub fn gd(config: GradientDescentConfig, mut loss_f: impl FnMut(f64) -> f64) -> GradientDescentOutcome{
//     let mut iterations = 0;
//     let mut residual = loss_f(config.init_value);
//     let mut value = config.init_value;
//     let learn_rate = config.learn_rate;
//     let mut gradient = -1.0;
//     while iterations < config.max_steps {
//         iterations += 1;
//         let new_value = value - gradient * learn_rate;
//         println!("iterations: {iterations}, value: {value}, residual: {residual}, gradient: {gradient}, new_value: {new_value}");
//         let step = new_value - value;
//         if step.abs() < config.min_step {
//             break;
//         }
//         let new_residual = loss_f(new_value);
//         gradient = (new_residual - residual) / step;
//         residual = new_residual;
//         value = new_value;
//     }
//     GradientDescentOutcome {
//         iterations: iterations, value, residual
//     }
// }

#[cfg(test)]
mod tests {
    use assert_float_eq::*;
    use super::*;

    #[test]
    fn gd_sqrt() {
        let config = GradientDescentConfig {
            init_value: 0.0,
            step: 0.1,
            min_step: 0.00001,
            max_steps: 100,
        };
        let outcome = gd(config.clone(), |value| (81.0 - value.powi(2)).powi(2));
        assert_float_absolute_eq!(9.0, outcome.optimal_value, config.min_step);
    }
}