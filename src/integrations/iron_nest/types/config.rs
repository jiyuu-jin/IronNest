use {
    super::FullAction,
    serde::{Deserialize, Serialize},
};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Config {
    pub actions: Vec<FullAction>,
}
