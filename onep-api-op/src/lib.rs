use serde::Deserialize;
use serde_json::Value;
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

impl onep_api::OnePassword for OnepasswordOp {
    fn totp(&self, uuid: &str) -> String {
        std::str::from_utf8(
            &Command::new("op")
                .arg("get")
                .arg("totp")
                .arg(uuid)
                .output()
                .expect("failed to exec get totp")
                .stdout,
        )
        .expect("failed to parse get totp output as utf8")
        .to_string()
    }

    fn account(&self) -> onep_api::AccountMetadata {
        let ret: GetAccount = serde_json::from_slice(
            &Command::new("op")
                .arg("get")
                .arg("account")
                .output()
                .expect("failed to exec get account")
                .stdout,
        )
        .expect("failed to parse json");

        onep_api::AccountMetadata {
            name: ret.name,
            domain: ret.domain,
        }
    }

    fn vaults(&self) -> Vec<onep_api::VaultMetadata> {
        let ret: Vec<ListVault> = serde_json::from_slice(
            &Command::new("op")
                .arg("list")
                .arg("vaults")
                .output()
                .expect("failed to exec list vaults")
                .stdout,
        )
        .expect("failed to parse json");

        ret.into_iter()
            .map(|v| onep_api::VaultMetadata {
                uuid: v.uuid,
                name: v.name,
            })
            .collect()
    }

    fn search(&self, terms: Option<&str>) -> Vec<onep_api::ItemMetadata> {
        let ret: Vec<ListItem> = serde_json::from_slice(
            &Command::new("op")
                .arg("list")
                .arg("items")
                .output()
                .expect("failed to exec list items")
                .stdout,
        )
        .expect("failed to parse json");

        ret.into_iter()
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
            .collect()
    }

    fn get(&self, uuid: &str) -> Option<onep_api::Item> {
        let ret: GetItem = serde_json::from_slice(
            &Command::new("op")
                .arg("get")
                .arg("item")
                .arg(uuid)
                .output()
                .expect("failed to exec get item")
                .stdout,
        )
        .expect("failed to parse json");

        Some(onep_api::Item {
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
        })
    }
}
