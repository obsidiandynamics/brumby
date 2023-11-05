`brumby`
===
A fast, allocation-free Monte Carlo model of a top-_N_ podium finish in racing events. Derives probabilities for placing in arbitrary positions given only win probabilities. Also derives joint probability of multiple runners with arbitrary (exact and top-_N_) placings.

[![Crates.io](https://img.shields.io/crates/v/brumby?style=flat-square&logo=rust)](https://crates.io/crates/brumby)
[![docs.rs](https://img.shields.io/badge/docs.rs-brumby-blue?style=flat-square&logo=docs.rs)](https://docs.rs/brumby)
[![Build Status](https://img.shields.io/github/actions/workflow/status/obsidiandynamics/brumby/master.yml?branch=master&style=flat-square&logo=github)](https://github.com/obsidiandynamics/brumby/actions/workflows/master.yml)

# Performance
Circa 15M simulations/sec of a top-4 podium over 14 runners using the [tinyrand](https://github.com/obsidiandynamics/tinyrand) RNG. (Per thread, benchmarked on Apple M2 Pro.) Roughly 70% of time is spent in the RNG routine.

# Example
Sourced from `examples/multi.rs`. To try this example, run `just multi` on the command line. You'll need [just](https://github.com/casey/just) installed.

```rust
use std::error::Error;
use std::path::PathBuf;

use stanza::renderer::console::Console;
use stanza::renderer::Renderer;

use brumby::display::DisplaySlice;
use brumby::file::ReadJsonFile;
use brumby::market::{Market, OverroundMethod};
use brumby::model::{Fitter, FitterConfig, WinPlace, Model};
use brumby::model::cf::Coefficients;
use brumby::model::fit::FitOptions;
use brumby::print;
use brumby::selection::{Rank, Runner};

fn main() -> Result<(), Box<dyn Error>> {
    // prices taken from a popular website
    let win_prices = vec![
        1.65,
        7.0,
        15.0,
        9.5,
        f64::INFINITY, // a scratched runner
        9.0,
        7.0,
        11.0,
        151.0,
    ];
    let place_prices = vec![
        1.12,
        1.94,
        3.2,
        2.3,
        f64::INFINITY, // a scratched runner
        2.25,
        1.95,
        2.55,
        28.0,
    ];

    // load coefficients from a file and create a fitter
    let coefficients = Coefficients::read_json_file(PathBuf::from("config/thoroughbred.cf.json"))?;
    let config = FitterConfig {
        coefficients,
        fit_options: FitOptions::fast() // use the default presents in production; fast presets are used for testing
    };
    let fitter = Fitter::try_from(config)?;

    // fit Win and Place probabilities from the supplied prices, undoing the overrounds
    let wp_markets = WinPlace {
        win: Market::fit(&OverroundMethod::Multiplicative, win_prices, 1.),
        place: Market::fit(&OverroundMethod::Multiplicative, place_prices, 3.),
        places_paying: 3,
    };

    // we have overrounds for Win and Place; extrapolate for Top-2 and Top-4 markets
    let overrounds = wp_markets.extrapolate_overrounds()?;

    // fit a model using the Win/Place prices and extrapolated overrounds
    let model = fitter.fit(&wp_markets, &overrounds)?.value;
    
    // nicely format the derived price matrix
    let table = print::tabulate_derived_prices(&model.prices().as_price_matrix());
    println!("\n{}", Console::default().render(&table));
    
    // simulate a same-race multi for a chosen selection vector using the previously fitted model
    let selections = vec![
        Runner::number(6).top(Rank::number(1)),
        Runner::number(7).top(Rank::number(2)),
        Runner::number(8).top(Rank::number(3)),
    ];
    let multi_price = model.derive_multi(&selections)?.value;
    println!(
        "{} with probability {:.6} is priced at {:.2}",
        DisplaySlice::from(&*selections),
        multi_price.probability,
        multi_price.price
    );

    Ok(())
}
```

# How it works
Brumby is based on a regression-fitted, weighted Monte Carlo (MC) simulation.

Ideally, one would start with a set of fair probabilities. But in less ideal scenarios, one might have prices with an overround applied; it is first necessary to remove the overround to obtain a set of fair probabilities. Brumby supports a range of overround methods. (Detailed in a later section.)

We start with a naive MC model. It simulates a podium finish using the supplied set of win probabilities. The naive model takes input a vector of probabilities _P_ and runs a series of random trials using a seeded pseudo-RNG. _P<sub>j</sub>_ is the probability of runner _j_ finishing first. In each trial, a uniformly distributed random number is mapped to an interval proportional to the supplied probabilities. This yields the winner of that trial. In the same trial, we then eliminate the winner's interval and adjust the output range of the RNG accordingly. The subsequent random number points to the second place getter, and so forth, until the desired podium is established. The podium is recorded for that trial and the next trial begins. As trials progress, the system establishes a matrix of top-1.._N_ placement probabilities for each runner. (Row per rank and column per runner.) We have observed that 100K trials are generally sufficient to populate a probabilities matrix with sufficient precision to be used in wagering applications, such as pricing derivatives. Accuracy, however, is another matter.

The model above is unaware of the systemic biases such as the favourite-longshot bias, nor does it realise that longshots are more likely to place than their win probability might imply. (Conversely, favourites are less likely to place.) We don't go into details here, suffice it to say that real-life competitors are motivated differently depending on their position in the race. A naive MC model assumes a field of runners with constant exertion and no motives other than the ambition to come first.

The naive model is clearly insufficient. The assumptions it makes are simply untrue in any competitive sport that we are aware of. While the accuracy may be palatable in a field of approximately equally-capable runners, it breaks down in fields of diverse runner strengths. Since the target applications mostly comprise the latter, we must modify the model to skew the probabilities of a runner coming in positions 2 through to _N_ (for an _N_-place podium) without affecting their win probability. We introduce a fine-grained biasing layer capable of targeting individual runners in specific finishing position. Where the naive model uses a vector of probabilities _P_, the biased model uses a matrix _W_ — with one row per finishing rank. _W<sub>i,j</sub>_ is the relative probability of runner _j_ taking position _i_. (_i_ ∈ ℕ ∩ [1, _N_], _j_ ∈ ℕ ∩ [1, _M_].) The values of _W_ in row 1 are equal to _P_. Rows 2–_N_ are adjusted according to the changes in the relative ranking probability of each runner.

Note, when all rows are identical, the biased model behaves identically to the naive one. I.e., _M_<sub>naive</sub> ≡ _M_<sub>biased</sub> ⇔ ∀ _i_, _k_ ∈ ℕ ∩ [1, _N_], _j_ ∈ ℕ ∩ [1, _M_] : _W<sub>i,j</sub>_ = _W<sub>k,j</sub>_.

Take, for example, a field of 6 with win probabilities _P_ = (0.05, 0.1, 0.25, 0.1, 0.35, 0.15). For a two-place podium, _W_ might resemble the following:

_W_<sub>1,_</sub> = (0.05, 0.1, 0.25, 0.1, 0.35, 0.15) = _P_;

_W_<sub>2,_</sub> = (0.09, 0.13, 0.22, 0.13, 0.28, 0.15).

In other words, the high-probability runners have had their relative ranking probabilities suppressed, while low-probability runners were instead boosted. This reflects our updated assumption that low(/high)-probability runners are under(/over)estimated to place by a naive model.

A pertinent questions is how to assign the relative probabilities in rows 2–_N_, given _P_ and possibly other data. An intuitive approach is to fit the probabilities based on historical data. Brumby uses a linear regression model with a configurable set of regressors. For example, a third degree polynomial comprising runner prices and the field size. (Which we found to be a reasonably effective predictor.) Distinct models may be used for different race types, competitor classes, track conditions, and so forth. The fitting process is performed offline; its output is a set of regression factors and corresponding coefficients.

The offline-fitted model does not cater to specific biases present in individual races and, crucially, it does not protect the operator of the model against _internal arbitrage_ opportunities. Let the Place market be paying _X_ places, where _X_ is typically 2 or 3. When deriving the Top-1.._N_ price matrix solely from Win prices, it is possible that the Top-_X_ prices differ from the Places price when the latter are sourced from an alternate model. This creates an internal price incoherency, where a semi-rational bettor will select the higher of the two prices, all other terms being equal. In the extreme case, the price difference may expose value in the bet and even enable rational bettors to take a risk-free position across a pair of incoherent markets.

This problem is ideally solved by unifying the models so that the Place prices are taken directly from the Top-1.._N_ matrix. Often this is not viable, particularly when the operator sources its Win and Place markets from a commodity pricing supplier and/or trades them manually. As such, Brumby allows the fitting of the Top-_X_ prices to the offered Place prices. The fitting is entirely online, typically following a price update, iterating while adjusting _W_<sub>_X_, _</sub> until the Top-_X_ prices match the Place prices within some acceptable margin of error. 

Fitting of the Top-_X_ market to the Place market is a _closed loop_ process, using the fitted residuals to moderate subsequent adjustments and eventually terminate the fitting process. In each iteration, for every rank _i_ and every runner _j_, a price is fitted and compared with the sample price. The difference is used to scale the probability at _W_<sub>_i_,_j_</sub>. For example, let the fitted price _f_ be 2.34 and the sample price _s_ be 2.41 for runner 5 in rank 3. The adjustment factor is _s_ / _f_ = 1.03. _W′_<sub>3,5</sub> = _W_<sub>3,5</sub> × 1.03.

In addition to the closed-loop fitting of the Place rank, Brumby supports (and enables by default) the _open-loop_ fitting of the other markets. The rational is as follows: if there are errors in the Place rank, attributable to some specific but unknown bias, there are likely similar errors in other ranks also attributable to that bias. In other words, any specific bias is unlikely to be confined to just one rank. So in addition to the closed-loop fitting of the Top-_X_ price, the online model also translates the same adjustments to the other ranks. This assumption appears to be sound in practice; tested on historical prices, open loop fitting consistently demonstrates lower errors.

Open-loop fitting isn't a binary on/off setting; instead, it is represented as an exponent: a real in the interval [0, 1]. The formula for adjusting the probabilities in _W_ is _W′_<sub>_i_,_j_</sub> = _W_<sub>_i_,_j_</sub> × (_s_<sub>_i_,_j_</sub> / _f_<sub>_i_,_j_</sub>)<sup>_t_</sup>, where _t_ is the open-loop exponent. When _t_ = 0, the process is purely a closed-loop one: no adjustment is made to ranks other than the Place rank. When _t_ = 1, the magnitude of adjustment for the other ranks equals the magnitude of adjustment to the Place rank. When _t_ takes some intermediate value, open-loop adjustments are made, albeit to a lesser magnitude than the corresponding Place rank ones. For example, when _t_ = 0.5, the adjustments to the other ranks are equal to the square root of the adjustments to the Place rank. The default setting is _t_ = 1.

## Overrounds
Brumby includes both fitting and framing support for several overround methods, including _multiplicative_, _power_ and _odds ratio_. Fitting and framing capabilities are complementary: by 'fitting' it is meant the removal of overrounds from an existing market offering; by 'framing' it is meant the application of overrounds to a set of fair probabilities to obtain a corresponding set of market prices.

### Multiplicative method
This is among the simplest and most commonly used methods, wherein each fair price is multiplied by a constant to achieve the desired overround. The fitting operation is the reverse of framing; in both cases the scaling coefficient is trivially closed-form obtainable. For framing:

_m_<sub>_j_</sub> = _p_<sub>_j_</sub><sup>-1</sup> / _v_,

where _m_ is the market price, _p_ is the fair probability and _v_ is the desired overround. Fitting is the inverse:

_p_<sub>_j_</sub> = _m_<sub>_j_</sub><sup>-1</sup> / _v_, where _v_ is obtained by summing the implied probabilities _m_<sub>1</sub><sup>-1</sup> to _m_<sub>_M_</sub><sup>-1</sup>.

Where the resulting price is > 1, the multiplicative method has the property of maintaining a constant margin regardless of the distribution of wagered money over the offered outcomes — every outcome offers the same return to player. When the method is applied naively, it is possible that the resulting price is less than or equal to 1 on high-probability runners. Therefore, its output must be capped to ensure that prices are greater than 1. In the capped case, the margin on the affected runners is reduced.

### Power method
This method is loosely based on the account of Stephen Clarke in [Adjusting Bookmaker’s Odds to Allow for Overround](https://www.researchgate.net/publication/326510904_Adjusting_Bookmaker's_Odds_to_Allow_for_Overround). The original work has limited application in the sports betting industry, being suited to games with equal-chance outcomes, such as Roulette. We augment Clarke's method by using the calculation of _k_ only as the initial estimate; thereafter iteratively optimising _k_ to minimise the relative error between the fitted and the ideal overrounds.

Given the exponent _k_, the market price is obtained by _m_ = _p_<sup>-_k_</sup>. The initial estimate of _k_ is given by _k̂_<sub>0</sub> = 1 + _log_(1/_v_) / _log_(_N_). 

Using Lagrange multipliers it can be shown that the overround is at its maximum when the probabilities are equal; any departure from equality yields a lower overround. Therefore, we conclude that the initial search direction is towards decreasing _k_. The fitting process is in the same vein: we take an initial estimate of _k_ using the observed overround and iterate, initially stepping in the direction of decreasing _k_, until the probabilities sum to 1.

The power method has the property of skewing the margin towards (i.e., overcharging) low-probability outcomes, enacting the favourite-longshot bias.

### Odds Ratio method
This method is based on the account of Keith Cheung in [Fixed-odds betting and traditional odds](https://www.sportstradingnetwork.com/article/fixed-odds-betting-traditional-odds/). Rather than dealing with conventional probabilities, it translates probabilities to odds, where _o_<sub>_j_</sub> =  _p_<sub>_j_</sub> / (1 - _p_<sub>_j_</sub>). This crux of this method is in ensuring that every pair of fair odds and corresponding market odds remain in a constant ratio after the application of the overround. _m_<sub>_j_</sub> = ((1 / _p_<sub>_j_</sub>) - 1) / _d_ + 1, where _d_ is a constant.

An optimiser is used to obtain _d_ for fields with arbitrary number of outcomes. The same is done in reverse during fitting.

Like the Power method, the Odds Ratio method skews the margin towards low-probability outcomes.

## Monte Carlo engine
Brumby relies on a custom-built MC engine that utilises pooled memory buffers to avoid costly allocations — `malloc` and `free` syscalls. It also uses a custom matrix data structure that flattens all rows into a contiguous vector, thereby avoiding memory sparsity and taking advantage of CPU-level caching to update and retrieve nearby data points. This provides a reasonably performant, allocation-free MC simulator, taking ~65 ns to perform one trial of a 4-place podium over a field of 14.

## Optimiser
The optimiser used for fitting and framing overrounds is a form of univariate search that descends the residual curve in fixed steps for as long as the residual decreases. Conversely, following the step where the residual increases, the search direction is reversed and the step size is halved. The search terminates when either the residual is within the acceptable value, or the number of steps or the number of reversals is exhausted. Each of these parameters, including the initial value and the initial search direction, are configurable.

## Linear regression
Brumby contains a linear regression (LR) model fitter that can also be driven as a predictor. This enables offline fitting to take place directly within Brumby, rather than relegating to a separate statistical package, such as _R_, to fit the coefficients. Our LR model is based on [linregress](https://github.com/n1m3/linregress), credited to the Computational Systems Medicine group of Technische Universität München, but with some important enhancements. The main one is to support the fitting of models without intercepts. Additionally, we amended the calculation of R-squared for models in such cases. Briefly, conventional R-squared is effectively a comparison to a reference model that predicts using only the sample mean (i.e., a linear model comprising only the intercept), where the _Sum of Squares Total_ (SST) term subtracts the mean from the sample value, squaring the difference. The elimination of the intercept makes such a comparison less meaningful. For intercept-free models, we use the same calculation as _R_ — wherein the SST term is noise — squaring of the sample values. We also amend the degrees of freedom in the calculation of the adjusted R-squared to mirror _R_'s approach.