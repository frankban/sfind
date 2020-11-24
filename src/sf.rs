use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use async_trait::async_trait;
use rustforce::response::QueryResponse;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::environ;

/// Create and return a Salesforce client.
pub async fn client(e: environ::Env) -> Result<rustforce::Client, Error> {
    let mut client = rustforce::Client::new(e.client_id, e.client_secret);
    client.set_login_endpoint(if e.is_sandbox {
        "https://test.salesforce.com"
    } else {
        "https://login.salesforce.com"
    });
    client.login_with_credential(e.username, e.password).await?;
    Ok(client)
}

/// A client for interacting with Salesforce.
#[async_trait]
pub trait Client {
    /// Return the `Account` with the given Salesforce account id, including all
    /// specified additional fields.
    async fn get_account(
        &self,
        id: &str,
        additional_fields: Vec<EntityField>,
    ) -> Result<Account, Error>;

    // Return an account id given an entity field and its value.
    async fn get_account_id_by_field(&self, ef: &EntityField, value: &str)
        -> Result<String, Error>;
}

#[async_trait]
impl Client for rustforce::Client {
    async fn get_account(
        &self,
        id: &str,
        additional_fields: Vec<EntityField>,
    ) -> Result<Account, Error> {
        let mut account_fields = vec![
            "Id",
            "Name",
            "AccountNumber",
            "BillingAddress",
            "CreatedDate",
            "LastModifiedDate",
        ];
        let mut asset_fields = vec![
            "Id",
            "Name",
            "Product2.ProductCode",
            "Product2.Name",
            "Product2.LastModifiedDate",
            "Price",
            "Quantity",
            "Status",
            "ContactId",
            "InstallDate",
            "PurchaseDate",
            "UsageEndDate",
            "CreatedDate",
            "LastModifiedDate",
        ];
        let mut contact_fields = vec![
            "Id",
            "Email",
            "FirstName",
            "LastName",
            "CreatedDate",
            "LastModifiedDate",
        ];
        let mut opportunity_fields = vec![
            "Id",
            "Name",
            "RecordType.Name",
            "StageName",
            "Amount",
            "CurrencyIsoCode",
            "IsWon",
            "IsClosed",
            "CloseDate",
            "LeadSource",
            "CreatedDate",
            "LastModifiedDate",
        ];
        let mut opportunity_line_item_fields = vec![
            "UnitPrice",
            "Quantity",
            "TotalPrice",
            "CurrencyISOCode",
            "ServiceDate",
        ];
        for ef in additional_fields.iter() {
            match ef.entity {
                Entity::Account => account_fields.push(&ef.field),
                Entity::Asset => asset_fields.push(&ef.field),
                Entity::Contact => contact_fields.push(&ef.field),
                Entity::Opportunity => opportunity_fields.push(&ef.field),
                Entity::OpportunityLineItem => opportunity_line_item_fields.push(&ef.field),
            }
        }
        let q = format!(
            "SELECT
                {account_fields},
                (SELECT {asset_fields} FROM assets),
                (SELECT {contact_fields} FROM contacts),
                (SELECT {opportunity_fields} FROM opportunities)
            FROM {account} WHERE Id = '{id}'",
            account = Entity::Account,
            account_fields = account_fields.join(", "),
            asset_fields = asset_fields.join(", "),
            contact_fields = contact_fields.join(", "),
            opportunity_fields = opportunity_fields.join(", "),
            id = id,
        );
        let res = self.query(&q).await?;
        let mut acc: Account = get_one(res)?;
        // Salesforce allows querying only one level of related objects.
        // TODO(frankban): rather than one query per opportunity, this is doable
        // with only one query for getting all line items, mapped in code.
        let fields = opportunity_line_item_fields.join(", ");
        for opp in acc.opportunities.records.iter_mut() {
            let q = format!(
                "SELECT {fields} FROM OpportunityLineItem
                WHERE OpportunityId = '{id}'",
                fields = fields,
                id = opp.id,
            );
            let res: QueryResponse<LineItem> = self.query(&q).await?;
            opp.line_items = res.records;
        }
        Ok(acc)
    }

    async fn get_account_id_by_field(
        &self,
        ef: &EntityField,
        value: &str,
    ) -> Result<String, Error> {
        match ef.entity {
            // Just return the provided value if we already have an Account.Id.
            Entity::Account if ef.field == "Id" => Ok(value.to_string()),
            Entity::Account => {
                let q = format!(
                    "SELECT Id FROM {} WHERE {} = '{}' ORDER BY LastModifiedDate DESC",
                    ef.entity, ef.field, value
                );
                let res: QueryResponse<ObjectWithID> = self.query(&q).await?;
                let acc = get_one(res)?;
                Ok(acc.id)
            }
            // Assume all other entities are account children.
            _ => {
                let q = format!(
                    "SELECT AccountId FROM {} WHERE {} = '{}' ORDER BY LastModifiedDate DESC",
                    ef.entity, ef.field, value
                );
                let res: QueryResponse<AccountChild> = self.query(&q).await?;
                let child = get_one(res)?;
                Ok(child.account_id)
            }
        }
    }
}

/// Fetch the first result from the given query response.
fn get_one<T: DeserializeOwned>(res: QueryResponse<T>) -> Result<T, Error> {
    match res.records.into_iter().next() {
        Some(record) => Ok(record),
        None => Err(Error::NotFound),
    }
}

/// The top level object returned when querying Salesforce.
/// The account includes its own fields but also related contacts, assets and
/// opportunities.
#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Account {
    pub id: String,
    pub name: String,
    pub account_number: Option<String>,
    pub billing_address: Address,

    pub created_date: String,
    pub last_modified_date: Option<String>,

    pub assets: Related<Asset>,
    pub contacts: Related<Contact>,
    pub opportunities: Related<Opportunity>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub city: Option<String>,
    pub country: Option<String>,
    pub postal_code: Option<String>,
    pub state: Option<String>,
    pub street: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Related<T> {
    pub records: Vec<T>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Asset {
    pub id: String,
    pub name: String,
    #[serde(rename = "Product2")]
    pub product: Product,
    pub price: Option<f32>,
    pub quantity: Option<f32>,
    pub status: Option<String>,
    pub contact_id: String,

    pub install_date: Option<String>,
    pub purchase_date: Option<String>,
    pub usage_end_date: Option<String>,

    pub created_date: String,
    pub last_modified_date: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Product {
    pub name: String,
    pub product_code: String,
    pub last_modified_date: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Contact {
    pub id: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,

    pub created_date: String,
    pub last_modified_date: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Opportunity {
    pub id: String,
    pub name: String,
    pub record_type: RecordType,
    pub stage_name: Option<String>,
    pub amount: Option<f32>,
    pub currency_iso_code: Option<String>,
    pub is_won: bool,
    pub is_closed: bool,
    pub close_date: Option<String>,
    pub lead_source: Option<String>,

    pub created_date: String,
    pub last_modified_date: Option<String>,

    #[serde(skip_deserializing)]
    pub line_items: Vec<LineItem>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct LineItem {
    pub unit_price: Option<f32>,
    pub quantity: Option<f32>,
    pub total_price: Option<f32>,
    pub currency_iso_code: Option<String>,
    pub service_date: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct RecordType {
    pub name: String,
}

/// Identifiers for Salesforce entities.
#[derive(Copy, Clone, Debug)]
pub enum Entity {
    Account,
    Asset,
    Contact,
    Opportunity,
    OpportunityLineItem,
}

impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl FromStr for Entity {
    type Err = Error;

    /// Create an `Entity` from its string representation.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Account" => Ok(Self::Account),
            "Asset" => Ok(Self::Asset),
            "Contact" => Ok(Self::Contact),
            "Opportunity" => Ok(Self::Opportunity),
            "OpportunityLineItem" => Ok(Self::OpportunityLineItem),
            _ => Err(Error::Message(format!("invalid entity {:?}", s))),
        }
    }
}

impl Entity {
    /// Create an entity from its id in Salesforce.
    pub fn from_id(id: &str) -> Option<Self> {
        match id.len() {
            15 | 18 => match &id[..3] {
                "001" => Some(Self::Account),
                "02i" => Some(Self::Asset),
                "003" => Some(Self::Contact),
                "006" => Some(Self::Opportunity),
                // OpportunityLineItem entities are not supported for id search.
                _ => None,
            },
            _ => None,
        }
    }

    /// Return an EntityField built from the entity and the given field name.
    pub fn to_field(&self, name: &str) -> EntityField {
        EntityField {
            entity: *self,
            field: name.to_string(),
        }
    }
}

/// A Salesforce entity field.
#[derive(Debug)]
pub struct EntityField {
    entity: Entity,
    field: String,
}

impl fmt::Display for EntityField {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.entity, self.field)
    }
}

impl FromStr for EntityField {
    type Err = Error;

    /// Create an `EntityField` from its string representation, for instance
    /// "Contact.Birthday".
    fn from_str(s: &str) -> Result<Self, Error> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 2 {
            return Err(Error::Message(format!("invalid entity field {:?}", s)));
        }
        match parts[0].parse::<Entity>() {
            Ok(entity) => Ok(Self {
                entity,
                field: parts[1].to_string(),
            }),
            Err(err) => Err(Error::Message(format!(
                "cannot parse entity field {:?}: {}",
                s, err
            ))),
        }
    }
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectWithID {
    id: String,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AccountChild {
    account_id: String,
}

/// A failure when communicating with salesforce.
#[derive(Debug)]
pub enum Error {
    Message(String),
    NotFound,
    SFError(rustforce::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(msg) => write!(f, "{}", msg),
            Error::NotFound => write!(f, "salesforce entity not found"),
            Error::SFError(err) => write!(f, "salesforce error: {}", err),
        }
    }
}

impl From<rustforce::Error> for Error {
    fn from(err: rustforce::Error) -> Error {
        Error::SFError(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_display() {
        assert_eq!(Entity::Account.to_string(), "Account");
        assert_eq!(Entity::Opportunity.to_string(), "Opportunity");
    }

    #[test]
    fn entity_from_str() {
        let ent: Entity = "Account".parse().unwrap();
        assert!(matches!(ent, Entity::Account));
        let ent: Entity = "Contact".parse().unwrap();
        assert!(matches!(ent, Entity::Contact));
    }

    #[test]
    fn entity_from_str_error() {
        let err = "BadWolf".parse::<Entity>().unwrap_err();
        assert_eq!(err.to_string(), "invalid entity \"BadWolf\"");
    }

    #[test]
    fn entity_from_id() {
        let ent = Entity::from_id("001012345678901").unwrap();
        assert!(matches!(ent, Entity::Account));
        let ent = Entity::from_id("02i012345678901234").unwrap();
        assert!(matches!(ent, Entity::Asset));
    }

    #[test]
    fn entity_from_id_none() {
        assert!(Entity::from_id("bad-length").is_none());
        assert!(Entity::from_id("bad012345678901").is_none());
    }

    #[test]
    fn entity_to_field() {
        let ef = Entity::Asset.to_field("MyField");
        assert!(matches!(ef.entity, Entity::Asset));
        assert_eq!(ef.field, "MyField");
    }

    #[test]
    fn entity_field_display() {
        assert_eq!(
            EntityField {
                entity: Entity::Account,
                field: String::from("Id"),
            }
            .to_string(),
            "Account.Id"
        );
        assert_eq!(
            EntityField {
                entity: Entity::Contact,
                field: String::from("AccountId"),
            }
            .to_string(),
            "Contact.AccountId"
        );
    }

    #[test]
    fn entity_field_from_str() {
        let ef: EntityField = "Account.Address__c".parse().unwrap();
        assert!(matches!(ef.entity, Entity::Account));
        assert_eq!(ef.field, "Address__c");

        let ef: EntityField = "Contact.Id".parse().unwrap();
        assert!(matches!(ef.entity, Entity::Contact));
        assert_eq!(ef.field, "Id");
    }

    #[test]
    fn entity_field_from_str_error() {
        let tests = vec![
            ("", "invalid entity field \"\""),
            ("BadWolf", "invalid entity field \"BadWolf\""),
            (
                "Account.Id.BadWolf",
                "invalid entity field \"Account.Id.BadWolf\"",
            ),
            (
                "Badwolf.Id",
                "cannot parse entity field \"Badwolf.Id\": invalid entity \"Badwolf\"",
            ),
        ];
        for (input, want_err) in tests {
            let err = input.parse::<EntityField>().unwrap_err();
            assert_eq!(err.to_string(), want_err);
        }
    }
}

// TODO(frankban): test the actual client trait implementation.
