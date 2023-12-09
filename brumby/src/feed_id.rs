use std::error::Error;
use std::fmt::Debug;
use std::str::FromStr;

use anyhow::anyhow;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FeedId<P> {
    pub provider: P,
    pub entity_id: String,
}
impl<P> FeedId<P> {
    pub fn new(provider: P, id: String) -> Self {
        Self {
            provider,
            entity_id: id,
        }
    }

    pub fn provider(&self) -> &P {
        &self.provider
    }

    pub fn entity_id(&self) -> &str {
        &self.entity_id
    }

    pub fn take(self) -> (P, String) {
        (self.provider, self.entity_id)
    }
}

impl<P: FromStr> FromStr for FeedId<P> where <P as FromStr>::Err: Error + Send + Sync + 'static {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let index = s.find(':').ok_or(anyhow!("feed ID should be in the form <provider>:<id>"))?;
        let (provider, id) = s.split_at(index);
        let provider = P::from_str(provider)?;
        let (_, id) = id.split_at(1);
        Ok(Self::new(provider, id.into()))
    }
}

#[cfg(test)]
mod tests {
    use thiserror::Error;

    use super::*;

    #[derive(Debug, PartialEq)]
    enum TestProvider {
        Acme, LilBastard
    }
    impl FromStr for TestProvider {
        type Err = TestError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s.to_lowercase().as_str() {
                "acme" => Ok(TestProvider::Acme),
                "lilbastard" => Ok(TestProvider::LilBastard),
                _ => Err(TestError::UnsupportedProvider(format!("unsupported provider '{s}'")))
            }
        }
    }

    #[derive(Debug, Error)]
    enum TestError {
        #[error("{0}")]
        UnsupportedProvider(String)
    }

    #[test]
    fn parse_no_error() {
        let feed_id = FeedId::<TestProvider>::from_str("acme:9").unwrap();
        assert_eq!(&TestProvider::Acme, feed_id.provider());
        assert_eq!("9", feed_id.entity_id());
    }

    #[test]
    fn take() {
        let feed_id = FeedId::new(TestProvider::LilBastard, "foo".into());
        let (provider, entity_id) = feed_id.take();
        assert_eq!(TestProvider::LilBastard, provider);
        assert_eq!("foo".to_owned(), entity_id);
    }

    #[test]
    fn parse_format_error() {
        let feed_id = FeedId::<TestProvider>::from_str("acme");
        assert_eq!(feed_id.unwrap_err().to_string(), "feed ID should be in the form <provider>:<id>");
    }

    #[test]
    fn parse_provider_error() {
        let feed_id = FeedId::<TestProvider>::from_str("anvil:foo");
        assert_eq!(feed_id.unwrap_err().to_string(), "unsupported provider 'anvil'");
    }
}