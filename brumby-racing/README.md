`brumby-racing`
===
A fast, allocation-free Monte Carlo model of a top-_N_ podium finish in racing events. Derives probabilities for placing in arbitrary positions given only win probabilities. Also derives the joint probability of multiple runners with arbitrary (exact and top-_N_) placings.

[![Crates.io](https://img.shields.io/crates/v/brumby?style=flat-square&logo=rust)](https://crates.io/crates/brumby)
[![docs.rs](https://img.shields.io/badge/docs.rs-brumby-blue?style=flat-square&logo=docs.rs)](https://docs.rs/brumby)
[![Build Status](https://img.shields.io/github/actions/workflow/status/obsidiandynamics/brumby/master.yml?branch=master&style=flat-square&logo=github)](https://github.com/obsidiandynamics/brumby/actions/workflows/master.yml)

# Performance
Circa 15M simulations/sec of a top-4 podium over 14 runners using the [tinyrand](https://github.com/obsidiandynamics/tinyrand) RNG. (Per thread, benchmarked on Apple M2 Pro.) Roughly 70% of execution time is spent in the RNG routine.

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
use brumby::selection::{Rank, Runner};
use brumby_racing::model::{Fitter, FitterConfig, WinPlace, Model};
use brumby_racing::model::cf::Coefficients;
use brumby_racing::model::fit::FitOptions;
use brumby_racing::print;

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
    let coefficients = Coefficients::read_json_file(PathBuf::from("../config/thoroughbred.cf.json"))?;
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

## Offline-online vs offline-only APIs
Brumby uses a combination of offline and online fitting to determine the initial weighted probabilities (the offline phase) and, thereafter, fine-tune these weights (the online phase) to align the output of the derived Top-3 (or Top-2, depending on the number of places payable) market prices to the supplied Place market prices. The online phase ensures that the model provides arbitrage-free prices relative to some other model that was used to generate the Place prices. In other words, the offline phase generalises in removing the bulk of systemic biases present in the sport; for example, the favourite-longshot bias and the win-place bias. The online phases zeroes in on the specific bias(es) present in a given race; these bias(es) are largely unknown but exist in the Place model.

Although it is more computationally intensive, running the online phase is strongly recommended when the Place prices are produced by an external model. The example above performs the online fit using the `Fitter` API. The result is a `FittedModel` struct that can subsequently be used to obtain the Top-_N_ (singles) prices and the multiple prices.

In the case lacking a Place model, Brumby may be used to derive the Place prices. Here, online fitting is no longer applicable; instead, use the `Primer` API to instantiate a model struct, bypassing the online fitting step.

# How it works
Brumby is based on a regression-fitted, weighted Monte Carlo (MC) simulation.

Ideally, one would start with a set of fair probabilities. But in less ideal scenarios, one might have prices with an overround applied; it is first necessary to remove the overround to obtain a set of fair probabilities. Brumby supports a range of overround methods. (Detailed in a later section.)

We start with a naive MC model. It simulates a podium finish using the supplied set of win probabilities. The naive model takes as input a vector of probabilities _P_ and runs a series of random trials using a seeded pseudo-RNG. _P<sub>j</sub>_ is the probability of runner _j_ finishing first. In each trial, a uniformly distributed random number is mapped to an interval proportional to the supplied probabilities. This yields the winner of that trial. In the same trial, we then eliminate the winner's interval and adjust the output range of the RNG accordingly. The subsequent random number points to the second place getter, and so forth, until the desired podium is established. The podium is recorded for that trial and the next trial begins. As trials progress, the system establishes a matrix of top-1.._N_ placement probabilities for each runner. (Row per rank and column per runner.) We have observed that 100K trials are generally sufficient to populate a probabilities matrix with sufficient precision to be used in wagering applications, such as pricing derivatives. Accuracy, however, is another matter.

It is easy to see that the naive MC model is the stochastic equivalent of the [Harville method](https://doi.org/10.1080/01621459.1973.10482425). It determines the probability of a runner coming second by summing the probabilities of the runner beating the remaining field, conditional on every other winning. The 'runner beating the field' probability in each case is obtained by normalising the residual probabilities after the removal of the winner. The Harville method generalises to all finishing ranks.

The model above, both in its stochastic and analytic guises, is unaware of the systemic biases such as the favourite-longshot bias, nor does it realise that longshots are more likely to place than their win probability might imply. (Conversely, favourites are less likely to place.) We don't go into details here, suffice it to say that real-life competitors are motivated differently depending on their position in the race. They may also act under instruction. A naive MC (or Harville) model assumes a field of runners with constant exertion and no motives other than coming first.

The naive model is clearly insufficient. The assumptions it makes are simply untrue in any competitive sport that we are aware of. While the accuracy may be palatable in a field of approximately equally capable runners, it breaks down in fields of diverse runner strengths and competitive strategies. Since the target applications mostly comprise the latter, we must modify the model to skew the probabilities of a runner coming in positions 2 through to _N_ (for an _N_-place podium) without affecting their win probability. We introduce a fine-grained biasing layer capable of targeting individual runners in specific finishing positions. Where the naive model uses a vector of probabilities _P_, the biased model uses a matrix _W_ — with one row per finishing rank. _W<sub>i,j</sub>_ is the relative probability of runner _j_ taking position _i_. (_i_ ∈ ℕ ∩ [1, _N_], _j_ ∈ ℕ ∩ [1, _M_].) The values of _W_ in row 1 are equal to _P_. Rows 2–_N_ are adjusted according to the changes in the relative ranking probability of each runner.

Note, when all rows are identical, the biased model behaves identically to the naive one. I.e., _M_<sub>_naive_</sub> ≡ _M_<sub>_biased_</sub> ⇔ ∀ _i_, _k_ ∈ ℕ ∩ [1, _N_], _j_ ∈ ℕ ∩ [1, _M_] : _W<sub>i,j</sub>_ = _W<sub>k,j</sub>_.

Take, for example, a field of 6 with win probabilities _P_ = (0.05, 0.1, 0.25, 0.1, 0.35, 0.15). For a two-place podium, the destructured _W_ might resemble the following:

_W_<sub>1,_</sub> = (0.05, 0.1, 0.25, 0.1, 0.35, 0.15) = _P_;

_W_<sub>2,_</sub> = (0.09, 0.13, 0.22, 0.13, 0.28, 0.15).

In other words, the high-probability runners had their relative ranking probabilities suppressed, while low-probability runners were instead boosted. This reflects our updated assumption that low(/high)-probability runners are under(/over)estimated to place by a naive model.

A pertinent question is how to assign the relative probabilities in rows 2–_N_, given _P_ and possibly other data. An intuitive approach is to fit the probabilities based on historical data. Brumby uses a linear regression model with a configurable set of regressors. For example, a third-degree polynomial comprising runner prices and the field size. (Which we found to be a reasonably effective predictor.) Distinct models may be used for different race types, competitor classes, track conditions, and so forth. The fitting process is performed offline; its output is a set of regression factors and corresponding coefficients.

The offline-fitted model does not cater to specific biases present in individual races and, crucially, it does not protect the model's user against _internal arbitrage_ opportunities. Let the Place market be paying _X_ places, where _X_ is typically 2 or 3. When deriving the Top-1.._N_ price matrix solely from Win prices, the Top-X prices may differ from the Place prices when the latter are sourced from an alternate model. This creates an internal price incoherency, where a semi-rational bettor will select the higher of the two prices, all other terms being equal. In the extreme case, the price difference may expose value in the bet and even enable rational bettors to take a risk-free position across a pair of incoherent markets.

This problem is ideally solved by unifying the models so that the Place prices are taken directly from the Top-1.._N_ matrix. Often this is not viable, particularly when the operator sources its Win and Place markets from a commodity pricing supplier and/or trades them manually. As such, Brumby allows the fitting of the Top-_X_ prices to the offered Place prices. The fitting is entirely online, typically following a price update, iterating while adjusting _W_<sub>_X_, _</sub> until the Top-_X_ prices match the Place prices within some acceptable margin of error.

Fitting of the Top-_X_ market to the Place market is a _closed-loop_ process, using the fitted residuals to moderate subsequent adjustments and eventually terminate the fitting process. In each iteration, for every rank _i_ and every runner _j_, a price is fitted and compared with the sample price. The difference is used to scale the probability at _W_<sub>_i_,_j_</sub>. For example, let the fitted price _f_ be 2.34 and the sample price _s_ be 2.41 for runner 5 in rank 3. The adjustment factor is _s_ / _f_ = 1.03. _W′_<sub>3,5</sub> = _W_<sub>3,5</sub> × 1.03.

In addition to the closed-loop fitting of the Place rank, Brumby supports (and enables by default) the _open-loop_ fitting of the other markets. The rationale is as follows: if there are errors in the Place rank, attributable to some specific but unknown bias, there are likely similar errors in other ranks also attributable to that bias. In other words, any specific bias is unlikely to be confined to just one rank. So in addition to the closed-loop fitting of the Top-_X_ price, the online model also translates the same adjustments to the other ranks. This assumption appears to be sound in practice; tested on historical prices, open loop fitting consistently demonstrates lower errors.

Open-loop fitting isn't a binary on/off setting; instead, it is represented as an exponent: a real in the interval [0, 1]. The formula for adjusting the probabilities in _W_ is _W′_<sub>_i_,_j_</sub> = _W_<sub>_i_,_j_</sub> × (_s_<sub>_i_,_j_</sub> / _f_<sub>_i_,_j_</sub>)<sup>_t_</sup>, where _t_ is the open-loop exponent. When _t_ = 0, the process is purely a closed-loop one: no adjustment is made to ranks other than the Place rank. When _t_ = 1, the magnitude of adjustment for the other ranks equals the magnitude of adjustment to the Place rank. When _t_ takes some intermediate value, open-loop adjustments are made, albeit to a lesser magnitude than the corresponding Place rank ones. For example, when _t_ = 0.5, the adjustments to the other ranks are equal to the square root of the adjustments to the Place rank. The default setting is _t_ = 1.

## Overrounds
Brumby includes both _fitting_ and _framing_ support for several overround methods. Fitting and framing capabilities are complementary: 'fitting' refers to the removal of overrounds from an existing market offering (where the overround value and the overround method are known but the parameters to the method are concealed); 'framing' refers to the application of overrounds to a set of fair probabilities to obtain a corresponding set of market prices.

### Multiplicative method
This is among the simplest and most commonly used methods, wherein each fair price is multiplied by a constant to achieve the desired overround. The fitting operation is the reverse of framing; in both cases, the scaling coefficient is trivially closed-form obtainable. For framing:

_m_<sub>_j_</sub> = _p_<sub>_j_</sub><sup>-1</sup> / _v_,

where _m_ is the market price, _p_ is the fair probability and _v_ is the desired overround. Fitting is the inverse:

_p_<sub>_j_</sub> = _m_<sub>_j_</sub><sup>-1</sup> / _v_, where _v_ is obtained by summing the implied probabilities _m_<sub>1</sub><sup>-1</sup> to _m_<sub>_M_</sub><sup>-1</sup>.

Where the resulting price is > 1, the multiplicative method has the property of maintaining a constant margin regardless of the distribution of wagered money over the offered outcomes — every outcome offers the same return to player. When the method is applied naively, the resulting price may be ≤ 1 on high-probability runners. Therefore, its output must be capped to ensure that prices are greater than 1. In the capped case, the margin on the affected runners is reduced.

### Power method
This method is loosely based on the account of Stephen Clarke in [Adjusting Bookmaker’s Odds to Allow for Overround](https://www.researchgate.net/publication/326510904_Adjusting_Bookmaker's_Odds_to_Allow_for_Overround). The original work has limited application in the sports betting industry, being suited to games with equal-chance outcomes, such as Roulette. We augment Clarke's method by using the calculation of _k_ only as the initial estimate and, thereafter, iteratively optimising _k_ to minimise the relative error between the fitted and the ideal overrounds.

Given the exponent _k_, the market price is obtained by _m_ = _p_<sup>-_k_</sup>. The initial estimate of _k_ is given by _k̂_<sub>0</sub> = 1 + _log_(1/_v_) / _log_(_N_).

Using Lagrange multipliers it can be shown that the overround is at its maximum when the probabilities are equal; any departure from equality yields a lower overround. Therefore, we conclude that the initial search direction is towards decreasing _k_. The fitting process is in the same vein: we take an initial estimate of _k_ using the observed overround and iterate, initially stepping in the direction of decreasing _k_, until the probabilities sum to 1.

The power method has the property of skewing the margin towards (i.e., overcharging) low-probability outcomes, enacting the favourite-longshot bias.

### Odds Ratio method
This method is based on the account of Keith Cheung in [Fixed-odds betting and traditional odds](https://www.sportstradingnetwork.com/article/fixed-odds-betting-traditional-odds/). Rather than operating in terms of conventional probabilities, it translates probabilities to odds, where _o_<sub>_j_</sub> = _p_<sub>_j_</sub> / (1 - _p_<sub>_j_</sub>). The crux of this method is in ensuring that every pair of fair odds and corresponding market odds remain in a constant ratio after the application of the overround. _m_<sub>_j_</sub> = ((1 / _p_<sub>_j_</sub>) - 1) / _d_ + 1, where _d_ is constant for all _j_.

An optimiser is used to obtain _d_ for fields with an arbitrary number of outcomes. The same is done in reverse during fitting.

Like the Power method, the Odds Ratio method skews the margin towards low-probability outcomes. Anecdotally, the Odds Ratio method is still occasionally used by bookmakers to set Place and Each-Way prices from Win prices — by coercing the overrounds.

## Monte Carlo engine
Brumby relies on a custom-built MC engine that utilises pooled memory buffers to avoid costly allocations — `malloc` and `free` syscalls. It also uses a custom matrix data structure that flattens all rows into a contiguous vector, thereby avoiding memory sparsity and taking advantage of CPU-level caching to update and retrieve nearby data points. This provides a reasonably performant, allocation-free MC simulator, taking ~65 ns to perform one trial of a 4-place podium over a field of 14.

## Optimiser
The optimiser used for fitting and framing overrounds is a form of univariate search that descends the residual curve in fixed steps for as long as the residual decreases. Conversely, following the step where the residual increases, the search direction is reversed and the step size is halved. The search terminates when either the residual is within the acceptable value, or the number of steps or the number of reversals is exhausted. Each of these parameters, including the initial value and the initial search direction is configurable.

## Linear regression
Brumby contains a linear regression (LR) model fitter that can also be driven as a predictor. This enables offline fitting to take place directly within Brumby, rather than relegating to a separate statistical package, such as _R_, to fit the coefficients. Our LR model is based on [linregress](https://github.com/n1m3/linregress), credited to the Computational Systems Medicine group of Technische Universität München, but with some important enhancements. The main one is to support the fitting of models without intercepts. Additionally, we amended the calculation of R-squared for models in such cases. Briefly, conventional R-squared is effectively a comparison to a reference model that predicts using only the sample mean (i.e., a linear model comprising only the intercept), where the _Sum of Squares Total_ (SST) term subtracts the mean from the sample value, squaring the difference. The elimination of the intercept makes such a comparison less meaningful. For intercept-free models, we use the same calculation as _R_ — wherein the SST term is noise — squaring the sample values. We also amend the degrees of freedom in the calculation of the adjusted R-squared to mirror _R_'s approach.

The model fitter can operate on regressor formulas defined in a `.r.json` file. A regressor formula is structurally similar to the _R_'s `lm` formula but expressed as an abstract syntax tree. Each top-level node corresponds to the terms of the linear sum. In the simplest case, a top-level node may correspond directly to an independent variable or an intercept term. Lower-level nodes are used to compose regressor terms from multiple independent variables. Assuming independent variables _a_ and _b_, consider the examples below.

The formula ~ _a_ + _b_:

```json
[
    { "Variable": "a" },
    { "Variable": "b" },
    "Intercept"
]
```

The formula ~ _a_ + _b_ + 0 (no intercept):

```json
[
    { "Variable": "a" },
    { "Variable": "b" },
    "Origin"
]
```

The formula ~ _a_<sup>2</sup> + _b_:

```json
[
    { "Exp": [{ "Variable": "a" }, 2] },
    { "Variable": "b" },
    "Intercept"
]
```

The formula ~ _a_<sup>2</sup> + _b_<sup>2</sup> + _ab_:

```json
[
    { "Exp": [{ "Variable": "a" }, 2] },
    { "Exp": [{ "Variable": "b" }, 2] },
    { "Product": [{ "Variable": "a" }, { "Variable": "b" }] },
    "Intercept"
]
```

Exactly one constant term must be supplied: either `Intercept` or `Origin`. This is unlike _R_'s formulas, which enable intercepts by default.

# Training
Here we describe the offline fitting process. It comprises three steps: 1) extracting a suitable training dataset, 2) selecting and fitting the regressor coefficients, and 3) evaluating the model's performance against a test set. The steps are decoupled. And while Brumby has the tooling to perform all steps, one may equally use statistical packages, such as _R_, to fit the coefficients. The `linear.regression` module of Brumby mirrors the behaviour of R_'s `lm` function.

## Dataset extraction
Use the `datadump` binary to produce a training set.

A training set is produced by iterating over historical race data snapshots, optimising the weighted MC probabilities until the derived (singles) prices match the snapshot prices within some margin of error. Brumby uses the [racing-scraper](https://github.com/anil0906/racing-scraper) format, which may be tailored to a range of market feed providers.

For every race, the weights are written to a specified CSV file — one row per runner. The CSV file starts with a header row — it must be stripped if using _R_ to analyse the data.

Depending on the source, historical data may contain significant pricing aberrations. Many sources don't bother aligning the outputs of their internal pricing models, creating ample room for internal arbitrage. Poor model coherency results in anomalies in the relationships of prices across finishing ranks; attempting to fit a regression model to such data will harm the model's generalisability.

Brumby's dataset extractor has a basic quality control filter — the _departure cutoff_, activated using the `-c` flag. Departure is a measure of the relative difference between the snapshot Place prices and the snapshot Top-2/3 prices, obtained by taking the absolute difference among the price pair and dividing by the largest of the two prices. The _worst-case departure_ is the largest of the departure values in the race. In an ideally cohesive model, this value is zero. The departure cutoff flag drops all races where the worst-case departure is above a set value.

The following example extracts a thoroughbred dataset from historical data residing in `~/archive`, writing the output to `data/thoroughbred.csv`. A departure cutoff filter of 0.3 is used.

```shell
just datadump -d 0.3 -r thoroughbred ~/archive data/thoroughbred.csv
```
## Selecting and fitting regressors
Use the `backfit` binary to fit the regressor coefficients to the dataset produced by `datadump`.

The application takes as input the training dataset and the regressor formulas defined in a `.r.json` file. There are three formulas — one for each non-winning rank in the podium. By default, the regression coefficients and summary statistics are outputted to the console. The output format is, again, similar to _R_'s `summary` function. Outputting to the console lets you perform a dry-run without saving the coefficients to file, perhaps going back and trying a different combination of regressors.

Once the appropriate regressors have been selected, use the `-o` (output) flag to save both regressors and their corresponding coefficients to a `.cf.json` file.

```shell
just backfit data/thoroughbred.csv config/thoroughbred.r.json -o config/thoroughbred.cf.json
```

The above example persists the fitted coefficients to `config/thoroughbred.cf.json` and prints the summary statistics, including the standard errors, p-values and R-squared values for each of the three predictors. Below is an output sample for one predictor.

```txt
╔═══════════════════════════════╤════════════╤═══════════╤═════════╤═════╗
║Regressor                      │ Coefficient│ Std. error│  P-value│     ║
╠═══════════════════════════════╪════════════╪═══════════╪═════════╪═════╣
║Variable(Weight0)              │  1.13191021│   0.017757│ 0.000000│***  ║
╟───────────────────────────────┼────────────┼───────────┼─────────┼─────╢
║Exp(Variable(Weight0), 2)      │ -3.49457066│   0.110952│ 0.000000│***  ║
╟───────────────────────────────┼────────────┼───────────┼─────────┼─────╢
║Exp(Variable(Weight0), 3)      │  3.82793135│   0.180875│ 0.000000│***  ║
╟───────────────────────────────┼────────────┼───────────┼─────────┼─────╢
║Variable(ActiveRunners)        │  0.02373357│   0.000701│ 0.000000│***  ║
╟───────────────────────────────┼────────────┼───────────┼─────────┼─────╢
║Exp(Variable(ActiveRunners), 2)│ -0.00308830│   0.000119│ 0.000000│***  ║
╟───────────────────────────────┼────────────┼───────────┼─────────┼─────╢
║Exp(Variable(ActiveRunners), 3)│  0.00010562│   0.000005│ 0.000000│***  ║
╟───────────────────────────────┼────────────┼───────────┼─────────┼─────╢
║Origin                         │  0.00000000│   0.000000│ 1.000000│     ║
╚═══════════════════════════════╧════════════╧═══════════╧═════════╧═════╝
r_squared:     0.991777
r_squared_adj: 0.991741
```

The asterisks mimic _R_'s significance codes: 0 ‘&ast;&ast;&ast;’ 0.001 ‘&ast;&ast;’ 0.01 ‘&ast;’ 0.05 ‘.’ 0.1 ‘ ’ 1.

## Evaluating the model
Use the `evaluate` binary to assess the model's predictive power against a historical test set.

Like its `datadump` counterpart, `evaluate` takes an optional departure cutoff flag to ignore incohesive data. The result is a set of ranked RMSRE (root mean squared relative error) scores, summarised by quantile. More detailed statistics are provided for the top and bottom 25 ranked races.

```shell
just evaluate -d 0.3 ~/archive
```