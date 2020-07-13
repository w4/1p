#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]

use async_trait::async_trait;

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
    pub field_type: ItemFieldType,
    pub value: String,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum ItemFieldType {
    Totp,
    Unknown,
}

#[derive(Debug)]
pub struct ItemSection {
    pub name: String,
    pub fields: Vec<ItemField>,
}

#[async_trait]
pub trait Backend {
    type Error;

    async fn account(&self) -> Result<AccountMetadata, Self::Error>;
    async fn vaults(&self) -> Result<Vec<VaultMetadata>, Self::Error>;
    async fn search(&self, terms: Option<&str>) -> Result<Vec<ItemMetadata>, Self::Error>;
    async fn get(&self, uuid: &str) -> Result<Option<Item>, Self::Error>;
    async fn generate(
        &self,
        name: &str,
        username: Option<&str>,
        url: Option<&str>,
        tags: Option<&str>,
    ) -> Result<Item, Self::Error>;
}
