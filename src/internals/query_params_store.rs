use anyhow::Result;
use serde::Serialize;
use smallvec::SmallVec;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

#[derive(Debug, Clone, PartialEq)]
pub struct QueryParamsStore {
    query_params: SmallVec<[String; 0]>,
}

impl QueryParamsStore {
    pub fn new() -> Self {
        Self {
            query_params: SmallVec::new(),
        }
    }

    pub fn add<V>(&mut self, query_params: V) -> Result<()>
    where
        V: Serialize,
    {
        let value_raw = ::serde_urlencoded::to_string(query_params)?;
        self.add_raw(value_raw);

        Ok(())
    }

    pub fn add_raw(&mut self, value_raw: String) {
        self.query_params.push(value_raw);
    }

    pub fn clear(&mut self) {
        self.query_params.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.query_params.is_empty()
    }

    pub fn has_content(&self) -> bool {
        !self.is_empty()
    }
}

impl Display for QueryParamsStore {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut is_joining = false;
        for query in &self.query_params {
            if is_joining {
                write!(f, "&")?;
            }

            write!(f, "{query}")?;
            is_joining = true;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test_add {
    use super::*;

    #[test]
    fn it_should_add_multiple_key_values() {
        let mut params = QueryParamsStore::new();

        params
            .add(&[("key", "value"), ("another", "value")])
            .unwrap();

        assert_eq!("key=value&another=value", params.to_string());
    }

    #[test]
    fn it_should_add_multiple_calls() {
        let mut params = QueryParamsStore::new();

        params.add(&[("key", "value")]).unwrap();
        params.add(&[("another", "value")]).unwrap();

        assert_eq!("key=value&another=value", params.to_string());
    }

    #[test]
    fn it_should_reject_raw_string() {
        let mut params = QueryParamsStore::new();

        let result = params.add("key=value");

        assert!(result.is_err());
    }

    #[test]
    fn it_should_add_query_param_strings_deserialized() {
        let mut params = QueryParamsStore::new();

        params.add(&[("key", "value&another=value")]).unwrap();

        assert_eq!("key=value%26another%3Dvalue", params.to_string());
    }
}

#[cfg(test)]
mod test_add_raw {
    use crate::internals::QueryParamsStore;

    #[test]
    fn it_should_add_key_value_pairs_correctly() {
        let mut params = QueryParamsStore::new();

        params.add_raw("key=value".to_string());
        params.add_raw("another=value".to_string());

        assert_eq!("key=value&another=value", params.to_string());
    }

    #[test]
    fn it_should_add_single_keys_correctly() {
        let mut params = QueryParamsStore::new();

        params.add_raw("key".to_string());
        params.add_raw("another".to_string());

        assert_eq!("key&another", params.to_string());
    }

    #[test]
    fn it_should_add_query_param_strings_correctly() {
        let mut params = QueryParamsStore::new();

        params.add_raw("key=value&another=value".to_string());
        params.add_raw("more=value".to_string());

        assert_eq!("key=value&another=value&more=value", params.to_string());
    }
}
