#![deny(clippy::pedantic)]
#![allow(clippy::used_underscore_binding)]

mod otp;

use clap::Clap;
use colored::Colorize;
use itertools::Itertools;
use onep_backend_api as api;
use onep_backend_op as backend;
use std::{collections::BTreeMap, convert::TryFrom};
use term_table::{
    row::Row,
    table_cell::{Alignment, TableCell},
    Table, TableStyle,
};

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("Couldn't find the requested item.")]
    NotFound,
}

#[derive(Clap, Debug)]
#[clap(author, version)]
/// 1password cli for humans
enum Opt {
    /// List all items
    #[clap(alias = "ls")]
    List {
        #[clap(long, short = 'u')]
        show_uuids: bool,
        #[clap(long, short = 'n')]
        show_account_names: bool,
    },
    /// Search for an item
    Search {
        #[clap(long, short = 'u')]
        show_uuids: bool,
        #[clap(long, short = 'n')]
        show_account_names: bool,
        terms: String,
    },
    /// Show existing password and optionally put it on the clipboard
    #[clap(alias = "get")]
    Show { uuid: String },
    /// Generates a new password and stores it in your password store
    #[clap(alias = "gen")]
    Generate {
        /// Name of the login to create
        name: String,
        /// Username to associate with the login
        #[clap(long, short = 'n')]
        username: Option<String>,
        /// URL to associate with the login
        #[clap(long, short = 'u')]
        url: Option<String>,
        /// Comma-separated list of tags to associate with the login
        #[clap(long, short = 't')]
        tags: Option<String>,
    },
}

#[tokio::main(core_threads = 1)]
async fn main() {
    if let Err(e) = run(&backend::OpBackend {}).await {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

async fn run<T: api::Backend>(backend: &T) -> anyhow::Result<()>
where
    T::Error: 'static + std::error::Error + Send + Sync,
{
    match Opt::parse() {
        Opt::List {
            show_uuids,
            show_account_names,
        } => search(backend, None, show_uuids, show_account_names).await?,
        Opt::Search {
            terms,
            show_uuids,
            show_account_names,
        } => search(backend, Some(terms), show_uuids, show_account_names).await?,
        Opt::Show { uuid } => {
            let result = backend.get(&uuid).await?.ok_or(Error::NotFound)?;
            show(result);
        }
        Opt::Generate {
            name,
            username,
            url,
            tags,
        } => {
            let result = backend
                .generate(&name, username.as_deref(), url.as_deref(), tags.as_deref())
                .await?;
            show(result);
        }
    }

    Ok(())
}

#[allow(clippy::non_ascii_literal)]
async fn search<T: api::Backend>(
    backend: &T,
    terms: Option<String>,
    show_uuids: bool,
    show_account_names: bool,
) -> anyhow::Result<()>
where
    T::Error: 'static + std::error::Error + Send + Sync,
{
    let (account, vaults, results) = tokio::try_join!(
        backend.account(),
        backend.vaults(),
        backend.search(terms.as_deref())
    )?;

    let mut results_grouped: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for (key, group) in &results.into_iter().group_by(|v| v.vault_uuid.clone()) {
        results_grouped.insert(key, group.collect());
    }

    // slow path for when vault is an exact match
    if let Some(terms) = terms {
        if let Some(vault) = vaults
            .iter()
            .find(|v| v.name.to_lowercase() == terms.to_lowercase())
        {
            results_grouped.insert(vault.uuid.clone(), backend.search(Some(&vault.uuid)).await?);
        }
    }

    println!("{} ({})", account.name, account.domain);

    let vault_count = results_grouped.len() - 1;

    for (current_vault_index, (vault, group)) in results_grouped.into_iter().enumerate() {
        let vault = vaults
            .iter()
            .find(|v| v.uuid == vault)
            .map_or_else(|| format!("Unknown Vault ({})", vault), |v| v.name.clone());

        println!(
            "{} {}",
            if current_vault_index < vault_count {
                "├──"
            } else {
                "└──"
            },
            vault.blue()
        );

        let line_start = if current_vault_index < vault_count {
            "│"
        } else {
            " "
        };

        let item_count = group.len() - 1;

        for (current_item_index, result) in group.into_iter().enumerate() {
            println!(
                "{}   {} {}",
                line_start,
                if current_item_index < item_count {
                    "├──"
                } else {
                    "└──"
                },
                result.title.trim()
            );

            let prefix = if current_item_index < item_count {
                "│  "
            } else {
                "   "
            };

            if show_account_names && !result.account_info.trim().is_empty() {
                println!(
                    "{}   {} {}",
                    line_start,
                    prefix,
                    result.account_info.trim().green()
                );
            }

            if show_uuids {
                println!("{}   {} {}", line_start, prefix, result.uuid.yellow());
            }
        }
    }

    Ok(())
}

fn show(item: api::Item) {
    let mut table = Table::new();
    table.style = TableStyle::extended();

    table.add_row(Row::new(vec![TableCell::new_with_alignment(
        item.title,
        2,
        Alignment::Center,
    )]));

    for field in item.fields {
        table.add_row(Row::new(vec![
            TableCell::new(field.name),
            TableCell::new_with_alignment(field.value, 1, Alignment::Right),
        ]));
    }

    println!("{}", table.render());

    for section in item.sections {
        if section.fields.is_empty() {
            continue;
        }

        let mut table = Table::new();
        table.style = TableStyle::extended();

        if !section.name.is_empty() {
            table.add_row(Row::new(vec![TableCell::new_with_alignment(
                section.name,
                2,
                Alignment::Center,
            )]));
        }

        for field in section.fields {
            let mut value = field.value;

            if field.field_type == api::ItemFieldType::Totp {
                if let Ok(tfa) = otp::TwoFactorAuth::try_from(value.as_ref()) {
                    value = tfa.generate().value;
                }
            }

            table.add_row(Row::new(vec![
                TableCell::new(field.name),
                TableCell::new_with_alignment(value, 1, Alignment::Right),
            ]));
        }

        println!("{}", table.render());
    }
}
