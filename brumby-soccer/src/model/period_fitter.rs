use brumby::opt::HypergridSearchConfig;
use crate::model::Model;

pub struct FitterConfig<'a> {
    poisson_search: HypergridSearchConfig<'a>,
    binomial_search: HypergridSearchConfig<'a>,
}
impl FitterConfig<'_> {
    fn validate(&self) -> Result<(), anyhow::Error> {
        self.poisson_search.validate()?;
        self.binomial_search.validate()?;
        Ok(())
    }
}

pub struct PeriodFitter<'a> {
    config: FitterConfig<'a>
}
impl PeriodFitter<'_> {
    pub fn fit(model: &mut Model) -> Result<(), anyhow::Error> {
        todo!()
    }
}

impl<'a> TryFrom<FitterConfig<'a>> for PeriodFitter<'a> {
    type Error = anyhow::Error;

    fn try_from(config: FitterConfig<'a>) -> Result<Self, Self::Error> {
        config.validate()?;
        Ok(Self {
            config,
        })
    }
}