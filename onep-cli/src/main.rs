#![deny(clippy::pedantic)]

use clap::Clap;
use colored::Colorize;
use itertools::Itertools;
use onep_backend_api as api;
use onep_backend_op as backend;
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
        #[clap(long, short = 'i')]
        show_uuids: bool,
        #[clap(long, short = 'n')]
        show_account_names: bool,
    },
    /// Grab a two-factor authentication code for the given item
    Totp { uuid: String },
    /// Search for an item
    Search { terms: String },
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

fn main() {
    if let Err(e) = run(&backend::OpBackend {}) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

#[allow(clippy::non_ascii_literal)]
fn run<T: api::Backend>(imp: &T) -> anyhow::Result<()>
where
    T::Error: 'static + std::error::Error + Send + Sync,
{
    match Opt::parse() {
        Opt::List {
            show_uuids,
            show_account_names,
        } => {
            let account = imp.account()?;
            let vaults = imp.vaults()?;
            let results = imp.search(None)?;

            let mut results_grouped: Vec<(_, Vec<_>)> = Vec::new();
            for (key, group) in &results.into_iter().group_by(|v| v.vault_uuid.clone()) {
                results_grouped.push((key, group.collect()));
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
        }
        Opt::Totp { uuid } => println!("{}", imp.totp(&uuid)?.trim()),
        Opt::Search { terms } => {
            for result in imp.search(Some(&terms))? {
                println!("[{}]", result.title.green());
                println!("{}", result.account_info);
                println!("{}", result.uuid);
                println!();
            }
        }
        Opt::Show { uuid } => {
            let result = imp.get(&uuid)?.ok_or(Error::NotFound)?;
            show(result);
        }
        Opt::Generate {
            name,
            username,
            url,
            tags,
        } => {
            let result =
                imp.generate(&name, username.as_deref(), url.as_deref(), tags.as_deref())?;
            show(result);
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
            table.add_row(Row::new(vec![
                TableCell::new(field.name),
                TableCell::new_with_alignment(field.value, 1, Alignment::Right),
            ]));
        }

        println!("{}", table.render());
    }
}
