use super::*;
use crate::testing::assert_slice_f64_relative;
use assert_float_eq::*;

const BOUNDS: PriceBounds = 1.04..=10_001.0;

#[test]
fn fit_multiplicative() {
    {
        let prices = vec![10.0, 5.0, 3.333, 2.5];
        let market = Market::fit(&OverroundMethod::Multiplicative, prices, 1.0);
        assert_slice_f64_relative(&[0.1, 0.2, 0.3, 0.4], &market.probs, 0.001);
        assert_float_absolute_eq!(1.0, market.overround.value, 0.001);
    }
    {
        let prices = vec![9.0909, 4.5454, 3.0303, 2.273];
        let market = Market::fit(&OverroundMethod::Multiplicative, prices, 1.0);
        println!("market: {:?}", market);
        assert_slice_f64_relative(&[0.1, 0.2, 0.3, 0.4], &market.probs, 0.001);
        assert_float_absolute_eq!(1.1, market.overround.value, 0.001);
    }
    {
        let prices = vec![9.0909, 4.5454, 3.0303, 2.273, f64::INFINITY];
        let market = Market::fit(&OverroundMethod::Multiplicative, prices, 1.0);
        println!("market: {:?}", market);
        assert_slice_f64_relative(&[0.1, 0.2, 0.3, 0.4, 0.0], &market.probs, 0.001);
        assert_float_absolute_eq!(1.1, market.overround.value, 0.001);
    }
    {
        let prices = vec![4.5454, 2.2727, 1.5152, 1.1364];
        let market = Market::fit(&OverroundMethod::Multiplicative, prices, 2.0);
        println!("market: {:?}", market);
        assert_slice_f64_relative(&[0.2, 0.4, 0.6, 0.8], &market.probs, 0.001);
        assert_float_absolute_eq!(1.1, market.overround.value, 0.001);
    }
    {
        let prices = vec![
            23.,
            6.5,
            8.,
            10.,
            5.5,
            11.,
            13.,
            3.7,
            27.,
            251.,
            16.,
            91.,
            126.,
            8.5,
            126.,
            201.,
            f64::INFINITY,
            f64::INFINITY,
        ];
        let market = Market::fit(&OverroundMethod::Multiplicative, prices, 1.0);
        println!("market: {:?}", market);
        assert_slice_f64_relative(
            &[
                0.03356745745810524,
                0.11877715715944932,
                0.09650644019205257,
                0.07720515215364206,
                0.14037300391571284,
                0.07018650195785642,
                0.05938857857972466,
                0.20866257338822172,
                0.028594500797645205,
                0.0030759024762407193,
                0.048253220096026284,
                0.00848408265424638,
                0.006127393028066829,
                0.09082959076899065,
                0.006127393028066829,
                0.0038410523459523407,
                0.0,
                0.0,
            ],
            &market.probs,
            0.001,
        );
        assert_float_absolute_eq!(1.29525, market.overround.value, 0.001);
    }
}

#[test]
fn fit_power() {
    {
        let prices = vec![10.0, 5.0, 3.333, 2.5];
        let market = Market::fit(&OverroundMethod::Power, prices, 1.0);
        assert_slice_f64_relative(&[0.1, 0.2, 0.3, 0.4], &market.probs, 0.001);
        assert_float_absolute_eq!(1.0, market.overround.value, 0.001);
    }
    {
        let prices = vec![8.4319, 4.4381, 3.0489, 2.3359];
        let market = Market::fit(&OverroundMethod::Power, prices, 1.0);
        println!("market: {:?}", market);
        assert_slice_f64_relative(&[0.1, 0.2, 0.3, 0.4], &market.probs, 0.001);
        assert_float_absolute_eq!(1.1, market.overround.value, 0.001);
    }
    {
        let prices = vec![8.4319, 4.4381, 3.0489, 2.3359, f64::INFINITY];
        let market = Market::fit(&OverroundMethod::Power, prices, 1.0);
        println!("market: {:?}", market);
        assert_slice_f64_relative(&[0.1, 0.2, 0.3, 0.4, 0.0], &market.probs, 0.001);
        assert_float_absolute_eq!(1.1, market.overround.value, 0.001);
    }
    {
        let prices = vec![4.2159, 2.219, 1.5244, 1.168];
        let market = Market::fit(&OverroundMethod::Power, prices, 2.0);
        println!("market: {:?}", market);
        assert_slice_f64_relative(&[0.2, 0.4, 0.6, 0.8], &market.probs, 0.001);
        assert_float_absolute_eq!(1.1, market.overround.value, 0.001);
    }
}

#[test]
fn fit_odds_ratio() {
    {
        let prices = vec![10.0, 5.0, 3.333, 2.5];
        let market = Market::fit(&OverroundMethod::OddsRatio, prices, 1.0);
        assert_slice_f64_relative(&[0.1, 0.2, 0.3, 0.4], &market.probs, 0.001);
        assert_float_absolute_eq!(1.0, market.overround.value, 0.001);
    }
    {
        let prices = vec![8.8335, 4.4816, 3.0309, 2.3056];
        let market = Market::fit(&OverroundMethod::OddsRatio, prices, 1.0);
        println!("market: {:?}", market);
        assert_slice_f64_relative(&[0.1, 0.2, 0.3, 0.4], &market.probs, 0.001);
        assert_float_absolute_eq!(1.1, market.overround.value, 0.001);
    }
    {
        let prices = vec![8.8335, 4.4816, 3.0309, 2.3056, f64::INFINITY];
        let market = Market::fit(&OverroundMethod::OddsRatio, prices, 1.0);
        println!("market: {:?}", market);
        assert_slice_f64_relative(&[0.1, 0.2, 0.3, 0.4, 0.0], &market.probs, 0.001);
        assert_float_absolute_eq!(1.1, market.overround.value, 0.001);
    }
    {
        let prices = vec![4.1132, 2.1675, 1.5189, 1.1946];
        let market = Market::fit(&OverroundMethod::OddsRatio, prices, 2.0);
        println!("market: {:?}", market);
        assert_slice_f64_relative(&[0.2, 0.4, 0.6, 0.8], &market.probs, 0.001);
        assert_float_absolute_eq!(1.1, market.overround.value, 0.001);
    }
    {
        let prices = vec![1.2494, 1.1109, 1.0647, 1.0416, f64::INFINITY];
        let market = Market::fit(&OverroundMethod::OddsRatio, prices, 1.0);
        println!("market: {:?}", market);
        assert_slice_f64_relative(&[0.1, 0.2, 0.3, 0.4, 0.0], &market.probs, 0.005);
        assert_float_absolute_eq!(3.6, market.overround.value, 0.001);
    }
}

#[test]
fn frame_fair() {
    let probs = vec![0.1, 0.2, 0.3, 0.4];
    let market = Market::frame(&Overround::fair(),
                               probs,
                               &BOUNDS
    );
    assert_slice_f64_relative(&[10.0, 5.0, 3.333, 2.5], &market.prices, 0.001);
}

#[test]
fn frame_multiplicative() {
    {
        let probs = vec![0.1, 0.2, 0.3, 0.4];
        let market = Market::frame(
            &Overround {
                method: OverroundMethod::Multiplicative,
                value: 1.0,
            },
            probs,
            &BOUNDS
        );
        assert_slice_f64_relative(&[10.0, 5.0, 3.333, 2.5], &market.prices, 0.001);
    }
    {
        let probs = vec![0.1, 0.2, 0.3, 0.4];
        let market = Market::frame(
            &Overround {
                method: OverroundMethod::Multiplicative,
                value: 1.1,
            },
            probs,
            &BOUNDS
        );
        assert_slice_f64_relative(&[9.0909, 4.5454, 3.0303, 2.273], &market.prices, 0.001);
    }
    {
        let probs = vec![0.1, 0.2, 0.3, 0.4, 0.0];
        let market = Market::frame(
            &Overround {
                method: OverroundMethod::Multiplicative,
                value: 1.1,
            },
            probs,
            &BOUNDS
        );
        assert_slice_f64_relative(
            &[9.0909, 4.5454, 3.0303, 2.273, f64::INFINITY],
            &market.prices,
            0.001,
        );
    }
    {
        let probs = vec![0.2, 0.4, 0.6, 0.8];
        let market = Market::frame(
            &Overround {
                method: OverroundMethod::Multiplicative,
                value: 1.1,
            },
            probs,
            &BOUNDS
        );
        assert_slice_f64_relative(&[4.5454, 2.2727, 1.5152, 1.1364], &market.prices, 0.001);
    }
}

#[test]
fn frame_power() {
    {
        let probs = vec![0.1, 0.2, 0.3, 0.4];
        let market = Market::frame(
            &Overround {
                method: OverroundMethod::Power,
                value: 1.0,
            },
            probs,
            &BOUNDS
        );
        println!("market: {:?}", market);
        assert_slice_f64_relative(&[10.0, 5.0, 3.333, 2.5], &market.prices, 0.001);
    }
    {
        let probs = vec![0.1, 0.2, 0.3, 0.4];
        let market = Market::frame(
            &Overround {
                method: OverroundMethod::Power,
                value: 1.1,
            },
            probs,
            &BOUNDS
        );
        println!("market: {:?}", market);
        assert_slice_f64_relative(&[8.4319, 4.4381, 3.0489, 2.3359], &market.prices, 0.001);
    }
    {
        let probs = vec![0.1, 0.2, 0.3, 0.4, 0.0];
        let market = Market::frame(
            &Overround {
                method: OverroundMethod::Power,
                value: 1.1,
            },
            probs,
            &BOUNDS
        );
        println!("market: {:?}", market);
        assert_slice_f64_relative(
            &[8.4319, 4.4381, 3.0489, 2.3359, f64::INFINITY],
            &market.prices,
            0.001,
        );
    }
    {
        let probs = vec![0.2, 0.4, 0.6, 0.8];
        let market = Market::frame(
            &Overround {
                method: OverroundMethod::Power,
                value: 1.1,
            },
            probs,
            &BOUNDS
        );
        println!("market: {:?}", market);
        assert_slice_f64_relative(&[4.2159, 2.219, 1.5244, 1.168], &market.prices, 0.001);
    }
}

#[test]
fn frame_odds_ratio() {
    {
        let probs = vec![0.1, 0.2, 0.3, 0.4];
        let market = Market::frame(
            &Overround {
                method: OverroundMethod::OddsRatio,
                value: 1.0,
            },
            probs,
            &BOUNDS
        );
        println!("market: {:?}", market);
        assert_slice_f64_relative(&[10.0, 5.0, 3.333, 2.5], &market.prices, 0.001);
    }
    {
        let probs = vec![0.1, 0.2, 0.3, 0.4];
        let market = Market::frame(
            &Overround {
                method: OverroundMethod::OddsRatio,
                value: 1.1,
            },
            probs,
            &BOUNDS
        );
        println!("market: {:?}", market);
        assert_slice_f64_relative(&[8.8335, 4.4816, 3.0309, 2.3056], &market.prices, 0.001);
    }
    {
        let probs = vec![0.1, 0.2, 0.3, 0.4, 0.0];
        let market = Market::frame(
            &Overround {
                method: OverroundMethod::OddsRatio,
                value: 1.1,
            },
            probs,
            &BOUNDS
        );
        println!("market: {:?}", market);
        assert_slice_f64_relative(
            &[8.8335, 4.4816, 3.0309, 2.3056, f64::INFINITY],
            &market.prices,
            0.001,
        );
    }
    {
        let probs = vec![0.2, 0.4, 0.6, 0.8];
        let market = Market::frame(
            &Overround {
                method: OverroundMethod::OddsRatio,
                value: 1.1,
            },
            probs,
            &BOUNDS
        );
        println!("market: {:?}", market);
        assert_slice_f64_relative(&[4.1132, 2.1675, 1.5189, 1.1946], &market.prices, 0.001);
    }
    {
        let probs = vec![0.1, 0.2, 0.3, 0.4, 0.0];
        let market = Market::frame(
            &Overround {
                method: OverroundMethod::OddsRatio,
                value: 3.6,
            },
            probs,
            &BOUNDS
        );
        println!("market: {:?}", market);
        assert_slice_f64_relative(
            &[1.2494, 1.1109, 1.0647, 1.0416, f64::INFINITY],
            &market.prices,
            0.001,
        );
    }
}

#[test]
fn booksum() {
    let probs = vec![0.1, 0.2, 0.3, 0.4, 0.0];
    let market = Market::frame(&Overround {
        method: OverroundMethod::Multiplicative,
        value: 1.1,
    }, probs, &BOUNDS);
    assert_eq!(1.0, market.fair_booksum());
    assert_eq!(1.1, market.offered_booksum());
}