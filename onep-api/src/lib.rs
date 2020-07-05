use thiserror::Error;

#[derive(Error, Debug)]
pub enum OnePasswordApiError {
    #[error("1password backend returned an error:\n{0}")]
    Backend(String),
    #[error("failed to exec backend:\n{0}")]
    Exec(std::io::Error),
}

#[derive(Debug)]
pub struct AccountMetadata {
    pub name: String,
    pub domain: String,
}

#[derive(Debug)]
pub struct VaultMetadata {
    pub uuid: String,
    pub name: String,
}

#[derive(Debug)]
pub struct ItemMetadata {
    pub uuid: String,
    pub vault_uuid: String,
    pub title: String,
    pub account_info: String,
}

#[derive(Debug)]
pub struct Item {
    pub title: String,
    pub fields: Vec<ItemField>,
    pub sections: Vec<ItemSection>,
}

#[derive(Debug)]
pub struct ItemField {
    pub name: String,
    pub value: String,
}

#[derive(Debug)]
pub struct ItemSection {
    pub name: String,
    pub fields: Vec<ItemField>,
}

pub trait OnePassword {
    fn totp(&self, uuid: &str) -> Result<String, OnePasswordApiError>;
    fn account(&self) -> Result<AccountMetadata, OnePasswordApiError>;
    fn vaults(&self) -> Result<Vec<VaultMetadata>, OnePasswordApiError>;
    fn search(&self, terms: Option<&str>) -> Result<Vec<ItemMetadata>, OnePasswordApiError>;
    fn get(&self, uuid: &str) -> Result<Option<Item>, OnePasswordApiError>;
}
