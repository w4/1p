//! A backend for the [op] tool distributed by `AgileBits`. This crate uses
//! `onep_backend_api::Backend` to provide an implementation of a 1password
//! backend for use 1p-cli.
//!
//! [op]: https://1password.com/downloads/command-line/

#![deny(clippy::pedantic)]
#![allow(clippy::used_underscore_binding)]

use async_trait::async_trait;
use onep_backend_api as api;
use serde::Deserialize;
use serde_json::Value;
use std::borrow::Cow;
use tokio::process::Command;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("op backend returned an error:\n{0}")]
    Backend(String),
    #[error("failed to exec backend:\n{0}")]
    Exec(std::io::Error),
    #[error("failed to parse json from op:\n{0}")]
    Json(#[from] serde_json::error::Error),
    #[error("failed to convert op response to utf-8:\n{0}")]
    Utf8(#[from] std::str::Utf8Error),
}

#[derive(Debug, Deserialize)]
struct GetAccount {
    name: String,
    domain: String,
}

impl Into<api::AccountMetadata> for GetAccount {
    fn into(self) -> api::AccountMetadata {
        api::AccountMetadata {
            name: self.name,
            domain: self.domain,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ListVault {
    uuid: String,
    name: String,
}

impl Into<api::VaultMetadata> for ListVault {
    fn into(self) -> api::VaultMetadata {
        api::VaultMetadata {
            uuid: self.uuid,
            name: self.name,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListItem {
    uuid: String,
    vault_uuid: String,
    created_at: String,
    updated_at: String,
    overview: ItemOverview,
}

impl Into<api::ItemMetadata> for ListItem {
    fn into(self) -> api::ItemMetadata {
        api::ItemMetadata {
            title: self.overview.title,
            account_info: self.overview.account_info,
            uuid: self.uuid,
            vault_uuid: self.vault_uuid,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ItemOverview {
    #[serde(rename = "URLs", default)]
    urls: Vec<ItemOverviewUrl>,
    title: String,
    url: Option<String>,
    #[serde(rename = "ainfo")]
    account_info: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ItemOverviewUrl {
    #[serde(rename = "l")]
    label: String,
    #[serde(rename = "u")]
    url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetItem {
    details: GetItemDetails,
    overview: ItemOverview,
}

impl Into<api::Item> for GetItem {
    fn into(self) -> api::Item {
        api::Item {
            title: self.overview.title,
            fields: self
                .details
                .fields
                .into_iter()
                .map(|f| f.into())
                .filter(|f: &api::ItemField| !f.value.is_empty())
                .collect(),
            sections: self
                .details
                .sections
                .into_iter()
                .map(|v| api::ItemSection {
                    name: v.title,
                    fields: v
                        .fields
                        .into_iter()
                        .map(|f| f.into())
                        .filter(|f: &api::ItemField| !f.value.is_empty())
                        .collect(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GetItemDetails {
    #[serde(default)]
    fields: Vec<GetItemDetailsField>,
    #[serde(default)]
    sections: Vec<GetItemSection>,
}

#[derive(Debug, Deserialize)]
struct GetItemDetailsField {
    name: String,
    #[serde(rename = "designation")]
    field_type: String,
    value: Value,
}

impl Into<api::ItemField> for GetItemDetailsField {
    fn into(self) -> api::ItemField {
        api::ItemField {
            name: if self.field_type.is_empty() {
                self.name.clone()
            } else {
                self.field_type
            },
            value: match self.value {
                Value::Null => String::new(),
                Value::String(v) => v,
                Value::Number(v) => format!("{}", v),
                Value::Bool(v) => if v { "true" } else { "false" }.to_string(),
                _ => panic!("unknown item field type for {}", self.name),
            },
            field_type: api::ItemFieldType::Unknown,
        }
    }
}

#[derive(Debug, Deserialize)]
struct GetItemSection {
    title: String,
    #[serde(default)]
    fields: Vec<GetItemSectionField>,
}

#[derive(Debug, Deserialize)]
struct GetItemSectionField {
    #[serde(rename = "k")]
    kind: String,
    #[serde(rename = "n")]
    name: String,
    #[serde(rename = "t")]
    field_type: String,
    #[serde(rename = "v", default)]
    value: Value,
}

impl Into<api::ItemField> for GetItemSectionField {
    fn into(self) -> api::ItemField {
        api::ItemField {
            name: self.field_type,
            value: match self.value {
                Value::Null => String::new(),
                Value::String(v) => v,
                Value::Number(v) => format!("{}", v),
                Value::Bool(v) => if v { "true" } else { "false" }.to_string(),
                _ => panic!("unknown item field type for {}", self.name),
            },
            field_type: if self.name.starts_with("TOTP_") {
                api::ItemFieldType::Totp
            } else {
                api::ItemFieldType::Unknown
            },
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateItem {
    uuid: String,
    vault_uuid: String,
}

pub struct OpBackend {}

async fn exec<I, S>(args: I) -> Result<Vec<u8>, Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let cmd = Command::new("op")
        .args(args)
        .output()
        .await
        .map_err(Error::Exec)?;

    if cmd.status.success() {
        Ok(cmd.stdout)
    } else {
        Err(Error::Backend(
            std::str::from_utf8(&cmd.stderr)?.to_string(),
        ))
    }
}

#[async_trait]
impl api::Backend for OpBackend {
    type Error = Error;

    async fn account(&self) -> Result<api::AccountMetadata, Self::Error> {
        let ret: GetAccount = serde_json::from_slice(&exec(&["get", "account"]).await?)?;

        Ok(ret.into())
    }

    async fn vaults(&self) -> Result<Vec<api::VaultMetadata>, Self::Error> {
        let ret: Vec<ListVault> = serde_json::from_slice(&exec(&["list", "vaults"]).await?)?;

        Ok(ret.into_iter().map(|v| v.into()).collect())
    }

    #[allow(clippy::filter_map)]
    async fn search(&self, terms: Option<&str>) -> Result<Vec<api::ItemMetadata>, Self::Error> {
        let ret: Vec<ListItem> = serde_json::from_slice(&exec(&["list", "items"]).await?)?;

        let terms = terms.map(str::to_lowercase);

        Ok(ret
            .into_iter()
            .filter(|v| {
                if let Some(terms) = &terms {
                    let terms = terms.as_ref();

                    v.uuid == terms
                        || v.vault_uuid == terms
                        || v.overview
                            .urls
                            .iter()
                            .any(|v| v.url.to_lowercase().contains(terms))
                        || v.overview.title.to_lowercase().contains(terms)
                        || v.overview.account_info.to_lowercase().contains(terms)
                        || v.overview
                            .tags
                            .iter()
                            .any(|v| v.to_lowercase().contains(terms))
                } else {
                    true
                }
            })
            .map(|v| v.into())
            .collect())
    }

    async fn get(&self, uuid: &str) -> Result<Option<api::Item>, Self::Error> {
        let ret: GetItem = serde_json::from_slice(&exec(&["get", "item", uuid]).await?)?;
        Ok(Some(ret.into()))
    }

    async fn generate(
        &self,
        name: &str,
        username: Option<&str>,
        url: Option<&str>,
        tags: Option<&str>,
    ) -> Result<api::Item, Self::Error> {
        let mut args = Vec::with_capacity(12);

        args.push(Cow::Borrowed("create"));
        args.push(Cow::Borrowed("item"));
        args.push(Cow::Borrowed("Login"));
        args.push(Cow::Borrowed("--generate-password"));
        args.push(Cow::Borrowed("--title"));
        args.push(Cow::Borrowed(name));

        if let Some(url) = url {
            args.push(Cow::Borrowed("--url"));
            args.push(Cow::Borrowed(url));
        }

        if let Some(tags) = tags {
            args.push(Cow::Borrowed("--tags"));
            args.push(Cow::Borrowed(tags));
        }

        if let Some(username) = username {
            args.push(Cow::Owned(format!("username={}", username)));
        }

        let ret: CreateItem = serde_json::from_slice(&exec(args.iter().map(Cow::as_ref)).await?)?;

        Ok(self.get(&ret.uuid).await?.unwrap_or_else(|| unreachable!()))
    }
}
