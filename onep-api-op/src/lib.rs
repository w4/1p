use serde::Deserialize;
use serde_json::Value;
use onep_api::OnePasswordApiError;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct GetAccount {
    name: String,
    domain: String,
}

#[derive(Debug, Deserialize)]
struct ListVault {
    uuid: String,
    name: String,
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

impl Into<onep_api::ItemField> for GetItemDetailsField {
    fn into(self) -> onep_api::ItemField {
        onep_api::ItemField {
            name: self.field_type,
            value: match self.value {
                Value::Null => String::new(),
                Value::String(v) => v,
                Value::Number(v) => format!("{}", v),
                Value::Bool(v) => if v { "true" } else { "false" }.to_string(),
                _ => panic!("unknown item field type for {}", self.name),
            },
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

impl Into<onep_api::ItemField> for GetItemSectionField {
    fn into(self) -> onep_api::ItemField {
        onep_api::ItemField {
            name: self.field_type,
            value: match self.value {
                Value::Null => String::new(),
                Value::String(v) => v,
                Value::Number(v) => format!("{}", v),
                Value::Bool(v) => if v { "true" } else { "false" }.to_string(),
                _ => panic!("unknown item field type for {}", self.name),
            },
        }
    }
}

pub struct OnepasswordOp {}

impl OnepasswordOp {
    fn exec(&self, args: &[&str]) -> Result<Vec<u8>, OnePasswordApiError> {
        let cmd = Command::new("op").args(args).output().map_err(OnePasswordApiError::Exec)?;

        if cmd.status.success() {
            Ok(cmd.stdout)
        } else {
            Err(OnePasswordApiError::Backend(
                std::str::from_utf8(&cmd.stderr).unwrap().to_string()
            ))
        }
    }
}

impl onep_api::OnePassword for OnepasswordOp {
    fn totp(&self, uuid: &str) -> Result<String, OnePasswordApiError> {
        Ok(std::str::from_utf8(&self.exec(&["get", "totp", uuid])?).unwrap().to_string())
    }

    fn account(&self) -> Result<onep_api::AccountMetadata, OnePasswordApiError> {
        let ret: GetAccount = serde_json::from_slice(&self.exec(&["get", "account"])?).unwrap();

        Ok(onep_api::AccountMetadata {
            name: ret.name,
            domain: ret.domain,
        })
    }

    fn vaults(&self) -> Result<Vec<onep_api::VaultMetadata>, OnePasswordApiError> {
        let ret: Vec<ListVault> = serde_json::from_slice(&self.exec(&["list", "vaults"])?).unwrap();

        Ok(ret.into_iter()
            .map(|v| onep_api::VaultMetadata {
                uuid: v.uuid,
                name: v.name,
            })
            .collect())
    }

    fn search(&self, terms: Option<&str>) -> Result<Vec<onep_api::ItemMetadata>, OnePasswordApiError> {
        let ret: Vec<ListItem> = serde_json::from_slice(&self.exec(&["list", "items"])?).unwrap();

        Ok(ret.into_iter()
            .filter(|v| {
                if let Some(terms) = terms {
                    v.uuid == terms
                        || v.vault_uuid == terms
                        || v.overview.urls.iter().any(|v| v.url.contains(terms))
                        || v.overview.title.contains(terms)
                        || v.overview.account_info.contains(terms)
                        || v.overview.tags.iter().any(|v| v.contains(terms))
                } else {
                    true
                }
            })
            .map(|v| onep_api::ItemMetadata {
                title: v.overview.title,
                account_info: v.overview.account_info,
                uuid: v.uuid,
                vault_uuid: v.vault_uuid,
            })
            .collect())
    }

    fn get(&self, uuid: &str) -> Result<Option<onep_api::Item>, OnePasswordApiError> {
        let ret: GetItem = serde_json::from_slice(&self.exec(&["get", "item", uuid])?).unwrap();

        Ok(Some(onep_api::Item {
            title: ret.overview.title,
            fields: ret
                .details
                .fields
                .into_iter()
                .map(|f| f.into())
                .filter(|f: &onep_api::ItemField| !f.value.is_empty())
                .collect(),
            sections: ret
                .details
                .sections
                .into_iter()
                .map(|v| onep_api::ItemSection {
                    name: v.title,
                    fields: v
                        .fields
                        .into_iter()
                        .map(|f| f.into())
                        .filter(|f: &onep_api::ItemField| !f.value.is_empty())
                        .collect(),
                })
                .collect(),
        }))
    }
}
