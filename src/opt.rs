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
    pub optimal_residual: f64,
}

pub fn gd(
    config: GradientDescentConfig,
    mut loss_f: impl FnMut(f64) -> f64,
) -> GradientDescentOutcome {
    let mut iterations = 0;
    let mut residual = loss_f(config.init_value);
    let (mut value, mut step) = (config.init_value, config.step);
    let (mut optimal_value, mut optimal_residual) = (value, residual);
    let mut boost = 1.0;
    // let mut gradient: f64 = 1.0;
    while iterations < config.max_steps {
        iterations += 1;
        let new_value = value + step * boost; // * f64::min(gradient.abs(), 100.0);
        let new_residual = loss_f(new_value);
        let gradient = (new_residual - residual) / (new_value - value);
        println!("iterations: {iterations}, value: {value}, residual: {residual}, step: {step}, new_value: {new_value}, new_residual: {new_residual}, gradient: {gradient}");

        if new_residual > residual {
            step = -step * 0.5;
            if step.abs() < config.min_step {
                break;
            }
        } else if new_residual < optimal_residual {
            // boost = f64::min(new_residual / (optimal_residual - new_residual), 10.0);
            boost = f64::min(gradient.abs(), 10.0);
            println!("optimal_residual: {optimal_residual}, new_residual: {new_residual}, boost: {boost}, diff: {}", optimal_residual - new_residual);
            optimal_residual = new_residual;
            optimal_value = new_value;
        } else if (new_residual - residual).abs() <= f64::EPSILON {
            break;
        }
        residual = new_residual;
        value = new_value;
    }
    GradientDescentOutcome {
        iterations,
        optimal_value,
        optimal_residual,
    }
}

#[derive(Clone, Debug)]
pub struct MvGdConfig {
    pub init_value: f64,
    pub step: f64,
    pub min_step: f64,
}

#[derive(Default, Clone)]
struct MvGdState {
    value: f64,
    step: f64,
    active: bool,
}

pub fn mv_gd(configs: &[MvGdConfig], max_steps: u64, mut loss_f: impl FnMut(&[f64]) -> f64) {
    // let mut optimal_values = vec![0.0; configs.len()];
    let mut states: Vec<_> = configs
        .iter()
        .map(|config| MvGdState {
            value: config.init_value,
            step: config.step,
            active: true,
        })
        .collect();
    let mut values = vec![0.0; configs.len()];
    let mut residual;
    let mut iterations = 0;
    while iterations < max_steps {
        iterations += 1;
        for (i, state) in states.iter().enumerate() {
            values[i] = state.value;
        }
        residual = loss_f(&values);
        println!("initial values: {values:?}, residual: {residual}");
        let mut at_least_one = false;
        for (i, state) in states.iter_mut().enumerate() {
            if state.active {
                at_least_one = true;
            }
            let step = state.step;
            state.value += step;
            values[i] = state.value;
            let new_residual = loss_f(&values);
            println!("iterations: {iterations}, values: {values:?}, step: {step}, residual: {residual}, new_residual: {new_residual}");

            values[i] -= step;
            if new_residual > residual {
                state.step = -state.step * 0.5;
                let config = &configs[i];
                if state.step.abs() < config.min_step {
                    state.active = false;
                }
            }
        }
        if !at_least_one {
            break;
        }
    }
}

// pub fn mv_gd(configs: &[MvGdConfig], max_steps: u64, mut loss_f: impl FnMut(&[f64]) -> f64) {
//     // let mut optimal_values = vec![0.0; configs.len()];
//     let mut states: Vec<_> = configs
//         .iter()
//         .map(|config| MvGdState {
//             value: config.init_value,
//             step: config.step,
//             active: true,
//         })
//         .collect();
//     let mut values = vec![0.0; configs.len()];
//     let mut residual;
//     let mut iterations = 0;
//     while iterations < max_steps {
//         iterations += 1;
//         for (i, state) in states.iter().enumerate() {
//             values[i] = state.value;
//         }
//         residual = loss_f(&values);
//         let mut best_residual = f64::MAX;
//         let mut best_direction = usize::MAX;
//         println!("initial values: {values:?}, residual: {residual}");
//         let mut at_least_one = false;
//         for (i, state) in states.iter_mut().enumerate() {
//             if state.active {
//                 at_least_one = true;
//             }
//             let step = state.step;
//             // state.value += step;
//             values[i] += step;
//             let new_residual = loss_f(&values);
//             println!("iterations: {iterations}, values: {values:?}, step: {step}, residual: {residual}, new_residual: {new_residual}");
//             values[i] -= step;
//             if new_residual > residual {
//                 state.step = -state.step * 0.5;
//                 let config = &configs[i];
//                 if state.step.abs() < config.min_step {
//                     state.active = false;
//                 }
//             }
//             if new_residual < best_residual {
//                 best_residual = new_residual;
//                 best_direction = i;
//             }
//         }
//         states[best_direction].value += states[best_direction].step;
//         if !at_least_one {
//             break;
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use assert_float_eq::*;

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

    #[test]
    /// Fitting of y = ax + b, where target (a, b) = (2, 1).
    fn mv_gd_line() {
        mv_gd(
            &[
                MvGdConfig {
                    init_value: 0.0,
                    step: 0.1,
                    min_step: 0.000001,
                },
                MvGdConfig {
                    init_value: 0.0,
                    step: 0.1,
                    min_step: 0.000001,
                },
            ],
            100,
            |values| {
                let (x1, x2) = (3.0, 4.0);
                let fitted_y1 = values[0] * x1 + values[1];
                let fitted_y2 = values[0] * x1 + values[1];
                let ideal_y1 = 2.0 * x1 + 1.0;
                let ideal_y2 = 2.0 * x2 + 1.0;
                println!("values: {values:?} ideal_y1={ideal_y1}, ideal_y2={ideal_y2}, fitted_y1={fitted_y1}, fitted_y2={fitted_y2}");
                // let error = (ideal_y1 - fitted_y1) * (ideal_y2 - fitted_y2);
                let error = (ideal_y1 - fitted_y1).powi(2) + (ideal_y2 - fitted_y2).powi(2);
                error.powi(2)
            },
        );

        //TODO needs assertions
    }
}
