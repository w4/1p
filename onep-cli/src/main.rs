use clap::Clap;
use colored::*;
use itertools::Itertools;
use onep_api::OnePassword;
use term_table::{
    row::Row,
    table_cell::{Alignment, TableCell},
    Table, TableStyle,
};

#[derive(Clap, Debug)]
#[clap(author, version)]
/// 1password cli for humans
enum Opt {
    /// List all items
    #[clap(alias = "list")]
    Ls {
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
}

fn main() {
    let imp = onep_api_op::OnepasswordOp {};

    match Opt::parse() {
        Opt::Ls {
            show_uuids,
            show_account_names,
        } => {
            let account = imp.account();
            let vaults = imp.vaults();
            let results = imp.search(None);

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
                    .map(|v| v.name.clone())
                    .unwrap_or_else(|| format!("Unknown Vault ({})", vault));

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
        Opt::Totp { uuid } => println!("{}", imp.totp(&uuid).trim()),
        Opt::Search { terms } => {
            for result in imp.search(Some(&terms)) {
                println!("[{}]", result.title.green());
                println!("{}", result.account_info);
                println!("{}", result.uuid);
                println!();
            }
        }
        Opt::Show { uuid } => {
            let result = imp.get(&uuid).unwrap();

            let mut table = Table::new();
            table.style = TableStyle::extended();

            table.add_row(Row::new(vec![TableCell::new_with_alignment(
                result.title,
                2,
                Alignment::Center,
            )]));

            for field in result.fields {
                table.add_row(Row::new(vec![
                    TableCell::new(field.name),
                    TableCell::new_with_alignment(field.value, 1, Alignment::Right),
                ]));
            }

            println!("{}", table.render());

            for section in result.sections {
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
    }
}
