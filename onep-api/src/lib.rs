#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]

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
    type Error;

    fn totp(&self, uuid: &str) -> Result<String, Self::Error>;
    fn account(&self) -> Result<AccountMetadata, Self::Error>;
    fn vaults(&self) -> Result<Vec<VaultMetadata>, Self::Error>;
    fn search(&self, terms: Option<&str>) -> Result<Vec<ItemMetadata>, Self::Error>;
    fn get(&self, uuid: &str) -> Result<Option<Item>, Self::Error>;
    fn generate(
        &self,
        name: &str,
        username: Option<&str>,
        url: Option<&str>,
        tags: Option<&str>,
    ) -> Result<Item, Self::Error>;
}
