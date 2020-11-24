use std::collections::HashMap;

use prettytable::{format, Cell, Row, Table};
use serde_json::Value;

use crate::arg::Format;
use crate::error::Error;
use crate::sf::{Account, Address};

/// Print the given `Account` object based on the given `Format`.
pub fn print(acc: &Account, format: Format) -> Result<(), Error> {
    match format {
        Format::JSON => {
            let v = serde_json::to_value(acc)?;
            let out = colored_json::to_colored_json_auto(&v)?;
            println!("{}", out);
        }
        _ => print_tabular(acc),
    };
    Ok(())
}

/// Print the given `Account` object as a table.
fn print_tabular(acc: &Account) {
    let str_default = &String::from("<missing>");
    let currency_default = &String::from("<missing currency>");
    let field_style = "Fc";
    let format = format::FormatBuilder::new()
        .column_separator('│')
        .borders('│')
        .separators(
            &[format::LinePosition::Top],
            format::LineSeparator::new('─', '┬', '┌', '┐'),
        )
        .separators(
            &[format::LinePosition::Title],
            format::LineSeparator::new('─', '┼', '├', '┤'),
        )
        .separators(
            &[format::LinePosition::Bottom],
            format::LineSeparator::new('─', '┴', '└', '┘'),
        )
        .padding(1, 1)
        .build();

    // Print account.
    let mut table = Table::new();
    table.set_format(format);

    table.set_titles(Row::new(vec![
        Cell::new("Account").style_spec("FWb"),
        Cell::new(&acc.id).style_spec("FW"),
    ]));
    table.add_row(Row::new(vec![
        Cell::new("Name").style_spec(field_style),
        Cell::new(&acc.name).style_spec("Fg"),
    ]));
    table.add_row(Row::new(vec![
        Cell::new("Number").style_spec(field_style),
        Cell::new(acc.account_number.as_ref().unwrap_or(str_default)).style_spec("Fg"),
    ]));
    table.add_row(Row::new(vec![
        Cell::new("Address").style_spec(field_style),
        Cell::new(&format_address(&acc.billing_address)),
    ]));
    add_dates(
        &mut table,
        &acc.created_date,
        acc.last_modified_date.as_ref(),
    );
    add_extra(&mut table, &acc.extra);
    table.printstd();

    // Print contacts.
    for (num, contact) in acc.contacts.records.iter().enumerate() {
        let mut table = Table::new();
        table.set_format(format);
        table.set_titles(Row::new(vec![
            Cell::new(&format!("Contact #{}", num + 1)).style_spec("FM"),
            Cell::new(&contact.id).style_spec("FW"),
        ]));
        table.add_row(Row::new(vec![
            Cell::new("Email").style_spec(field_style),
            Cell::new(&contact.email).style_spec("Fg"),
        ]));
        table.add_row(Row::new(vec![
            Cell::new("First Name").style_spec(field_style),
            Cell::new(contact.first_name.as_ref().unwrap_or(str_default)).style_spec("Fg"),
        ]));
        table.add_row(Row::new(vec![
            Cell::new("Last Name").style_spec(field_style),
            Cell::new(contact.last_name.as_ref().unwrap_or(str_default)).style_spec("Fg"),
        ]));
        add_dates(
            &mut table,
            &contact.created_date,
            contact.last_modified_date.as_ref(),
        );
        add_extra(&mut table, &contact.extra);
        table.printstd();
    }

    // Print assets.
    for (num, asset) in acc.assets.records.iter().enumerate() {
        let mut table = Table::new();
        table.set_format(format);
        table.set_titles(Row::new(vec![
            Cell::new(&format!("Asset #{}", num + 1)).style_spec("FY"),
            Cell::new(&asset.id).style_spec("FW"),
        ]));
        table.add_row(Row::new(vec![
            Cell::new("Name").style_spec(field_style),
            Cell::new(&asset.name).style_spec("Fg"),
        ]));
        table.add_row(Row::new(vec![
            Cell::new("Product").style_spec(field_style),
            Cell::new(&format!(
                "{}: {}",
                asset.product.product_code, asset.product.name
            ))
            .style_spec("Fg"),
        ]));
        table.add_row(Row::new(vec![
            Cell::new("Price").style_spec(field_style),
            Cell::new(&format!(
                "{} x {}",
                format_number("price", asset.price),
                format_number("quantity", asset.quantity)
            )),
        ]));
        table.add_row(Row::new(vec![
            Cell::new("Status").style_spec(field_style),
            match &asset.status {
                Some(s) => Cell::new(s).style_spec("Fgb"),
                None => Cell::new(str_default).style_spec("Fr"),
            },
        ]));
        for (label, date) in &[
            ("Purchase Date", &asset.purchase_date),
            ("Install Date", &asset.install_date),
            ("Usage End Date", &asset.usage_end_date),
        ] {
            add_date(&mut table, label, date.as_ref().unwrap_or(str_default))
        }

        table.add_row(Row::new(vec![
            Cell::new("Contact").style_spec(field_style),
            Cell::new(&asset.contact_id).style_spec("Fg"),
        ]));
        add_dates(
            &mut table,
            &asset.created_date,
            asset.last_modified_date.as_ref(),
        );
        add_extra(&mut table, &asset.extra);
        table.printstd();
    }

    // Print opportunities.
    for (num, opp) in acc.opportunities.records.iter().enumerate() {
        let mut table = Table::new();
        table.set_format(format);
        table.set_titles(Row::new(vec![
            Cell::new(&format!("Opportunity #{}", num + 1)).style_spec("FG"),
            Cell::new(&opp.id).style_spec("FW"),
        ]));
        table.add_row(Row::new(vec![
            Cell::new("Name").style_spec(field_style),
            Cell::new(&opp.name).style_spec("Fg"),
        ]));
        table.add_row(Row::new(vec![
            Cell::new("Record Type").style_spec(field_style),
            Cell::new(&opp.record_type.name).style_spec("Fg"),
        ]));
        let currency = opp.currency_iso_code.as_ref().unwrap_or(currency_default);
        table.add_row(Row::new(vec![
            Cell::new("Amount").style_spec(field_style),
            Cell::new(&format!(
                "{} {}",
                format_number("amount", opp.amount),
                currency
            )),
        ]));
        let (status, style) = match opp.is_closed {
            true => {
                if opp.is_won {
                    ("Closed Won", "FGb")
                } else {
                    ("Closed Lost", "FRb")
                }
            }
            false => ("Pending", "Fy"),
        };
        table.add_row(Row::new(vec![
            Cell::new("Status").style_spec(field_style),
            Cell::new(status).style_spec(style),
        ]));
        let stage_name = opp.stage_name.as_ref().unwrap_or(str_default);
        if stage_name != status {
            table.add_row(Row::new(vec![
                Cell::new("Stage Name").style_spec(field_style),
                Cell::new(opp.stage_name.as_ref().unwrap_or(str_default)).style_spec("Fg"),
            ]));
        }
        if opp.is_closed {
            add_date(
                &mut table,
                "Close Date",
                opp.close_date.as_ref().unwrap_or(str_default),
            );
        }
        table.add_row(Row::new(vec![
            Cell::new("Lead Source").style_spec(field_style),
            Cell::new(opp.lead_source.as_ref().unwrap_or(str_default)).style_spec("Fg"),
        ]));
        add_dates(
            &mut table,
            &opp.created_date,
            opp.last_modified_date.as_ref(),
        );
        add_extra(&mut table, &opp.extra);

        // Print line items.
        for (num, item) in opp.line_items.iter().enumerate() {
            let mut litable = Table::new();
            litable.set_format(format);
            let price_line = format!(
                "{unit} {currency} x {quantity} = {total} {currency}",
                unit = format_number("unit price", item.unit_price),
                quantity = format_number("quantity", item.quantity),
                total = format_number("total price", item.total_price),
                currency = item.currency_iso_code.as_ref().unwrap_or(&currency_default),
            );
            litable.add_row(Row::new(vec![Cell::new("price"), Cell::new(&price_line)]));
            add_date(
                &mut litable,
                "service date",
                item.service_date.as_ref().unwrap_or(str_default),
            );
            add_extra(&mut litable, &item.extra);
            table.add_row(Row::new(vec![
                Cell::new(&format!("Line Item #{}", num + 1)),
                Cell::new(&litable.to_string()),
            ]));
        }
        table.printstd();
    }
}

fn format_address(addr: &Address) -> String {
    let mut table = Table::new();
    table.set_format(format::FormatBuilder::new().padding(0, 1).build());
    for (label, v) in &[
        ("Street:", addr.street.as_ref()),
        ("City:", addr.city.as_ref()),
        ("State:", addr.state.as_ref()),
        ("Country:", addr.country.as_ref()),
        ("Zip:", addr.postal_code.as_ref()),
    ] {
        if v.is_some() {
            table.add_row(Row::new(vec![Cell::new(label), Cell::new(v.unwrap())]));
        }
    }
    table.to_string()
}

fn format_number(label: &str, v: Option<f32>) -> String {
    match v {
        Some(n) => n.to_string(),
        None => format!("<missing {}>", label),
    }
}

fn add_extra(table: &mut Table, extra: &HashMap<String, Value>) {
    let mut items: Vec<_> = extra.iter().collect();
    items.sort_by(|(x, _), (y, _)| x.partial_cmp(y).unwrap());
    for (k, v) in items {
        if k == "attributes" {
            continue;
        }
        let s = &v.to_string();
        table.add_row(Row::new(vec![
            Cell::new(k).style_spec("FB"),
            match v.as_str() {
                Some(s) => Cell::new(s).style_spec("Fg"),
                None => Cell::new(s),
            },
        ]));
    }
}

fn add_dates(table: &mut Table, created: &str, modified: Option<&String>) {
    let default = &String::from("");
    add_date(table, "Created", created);
    add_date(table, "Modified", modified.unwrap_or(default));
}

fn add_date(table: &mut Table, label: &str, date: &str) {
    let replace = |s: &str| s.replace(".000+0000", "").replace("T", " ");
    table.add_row(Row::new(vec![
        Cell::new(label).style_spec("Fc"),
        Cell::new(&replace(date)).style_spec("Fy"),
    ]));
}
