use crate::config::Config;
use crate::error::Error;
use crate::sf::{self, Entity, EntityField};

/// Find an account based on the given query on Salesforce.
pub async fn run<T: sf::Client>(client: T, q: &str, conf: Config) -> Result<sf::Account, Error> {
    let err_not_found = Error {
        message: format!("nothing found for query {:?}", q),
    };
    let id = match from_id(&client, q).await {
        IDResult::Ok(id) => id,
        IDResult::Err(err) => return Err(err),
        IDResult::None => match from_extra(&client, q, conf.search_fields).await {
            IDResult::Ok(id) => id,
            IDResult::Err(err) => return Err(err),
            IDResult::None => return Err(err_not_found),
        },
    };
    match client.get_account(&id, conf.additional_fields).await {
        Ok(acc) => Ok(acc),
        Err(sf::Error::NotFound) => Err(err_not_found),
        Err(err) => Err(Error::from(err)),
    }
}

/// Return an account id from the given generic Salesforce id.
async fn from_id<T: sf::Client>(client: &T, id: &str) -> IDResult {
    if let Some(entity) = Entity::from_id(id) {
        let ef = entity.to_field("Id");
        return match client.get_account_id_by_field(&ef, id).await {
            Ok(aid) => IDResult::Ok(aid),
            Err(sf::Error::NotFound) => IDResult::None,
            Err(err) => IDResult::Err(Error::from(err)),
        };
    }
    IDResult::None
}

/// Return an account id from the given extra field query.
async fn from_extra<T: sf::Client>(
    client: &T,
    q: &str,
    search_fields: Vec<EntityField>,
) -> IDResult {
    // First always check for contact email if the value looks like an email.
    if q.contains('@') {
        let ef = Entity::Contact.to_field("email");
        match client.get_account_id_by_field(&ef, q).await {
            Ok(aid) => return IDResult::Ok(aid),
            Err(sf::Error::NotFound) => (),
            Err(err) => return IDResult::Err(Error::from(err)),
        };
    }
    // Then search over additional fields provided in the configuration.
    for ef in search_fields.iter() {
        match client.get_account_id_by_field(ef, q).await {
            Ok(aid) => return IDResult::Ok(aid),
            Err(sf::Error::NotFound) => (),
            Err(err) => return IDResult::Err(Error::from(err)),
        }
    }
    IDResult::None
}

/// A result of trying to fetch an account id.
enum IDResult {
    Ok(String),
    Err(Error),
    None,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use async_trait::async_trait;

    use super::*;

    #[tokio::test]
    async fn run_from_id_ok_get_account_ok() {
        let q = "0012500001Lhk3hAAB";
        let config = Config::empty();
        // TODO(frankban): is this better than a RefCell in the mock client?
        let client = TestClient::new(|args| match args {
            MockArgs::GetAccountIDByField("Account.Id", "0012500001Lhk3hAAB") => {
                MockResult::ID(q.to_string())
            }
            MockArgs::GetAccount("0012500001Lhk3hAAB") => {
                MockResult::Account(sf::Account::new_for_tests())
            }
            _ => panic!("unhandled request/response: {:?}", args),
        });
        let acc = run(client, q, config).await.unwrap();
        assert_eq!(acc.id, "id-for-tests");
    }

    #[tokio::test]
    async fn run_from_id_ok_get_account_not_found() {
        let q = "0012500001Lhk3hAAB";
        let config = Config::empty();
        let client = TestClient::new(|args| match args {
            MockArgs::GetAccountIDByField("Account.Id", "0012500001Lhk3hAAB") => {
                MockResult::ID(q.to_string())
            }
            MockArgs::GetAccount("0012500001Lhk3hAAB") => MockResult::Err(sf::Error::NotFound),
            _ => panic!("unhandled request/response: {:?}", args),
        });
        let err = run(client, q, config).await.unwrap_err();
        assert_eq!(
            err.message,
            "nothing found for query \"0012500001Lhk3hAAB\""
        );
    }

    #[tokio::test]
    async fn run_from_id_ok_get_account_error() {
        let q = "0012500001Lhk3hAAB";
        let config = Config::empty();
        let client = TestClient::new(|args| match args {
            MockArgs::GetAccountIDByField("Account.Id", "0012500001Lhk3hAAB") => {
                MockResult::ID(q.to_string())
            }
            MockArgs::GetAccount("0012500001Lhk3hAAB") => {
                MockResult::Err(sf::Error::Message(String::from("bad wolf")))
            }
            _ => panic!("unhandled request/response: {:?}", args),
        });
        let err = run(client, q, config).await.unwrap_err();
        assert_eq!(err.message, "bad wolf");
    }

    #[tokio::test]
    async fn run_from_id_error() {
        let q = "02i2500000HTaW9AAL";
        let config = Config::empty();
        let client = TestClient::new(|args| match args {
            MockArgs::GetAccountIDByField("Asset.Id", "02i2500000HTaW9AAL") => {
                MockResult::Err(sf::Error::Message(String::from("bad wolf")))
            }
            _ => panic!("unhandled request/response: {:?}", args),
        });
        let err = run(client, q, config).await.unwrap_err();
        assert_eq!(err.message, "bad wolf");
    }

    #[tokio::test]
    async fn run_from_extra_ok_get_account_ok() {
        let q = "02i2500000HTaW9AAL";
        let config = Config {
            additional_fields: vec![],
            search_fields: vec![
                "Account.SomeField".parse::<sf::EntityField>().unwrap(),
                "Opportunity.AnotherField"
                    .parse::<sf::EntityField>()
                    .unwrap(),
            ],
        };
        let client = TestClient::new(|args| match args {
            MockArgs::GetAccountIDByField("Asset.Id", "02i2500000HTaW9AAL") => {
                MockResult::Err(sf::Error::NotFound)
            }
            MockArgs::GetAccountIDByField("Account.SomeField", "02i2500000HTaW9AAL") => {
                MockResult::Err(sf::Error::NotFound)
            }
            MockArgs::GetAccountIDByField("Opportunity.AnotherField", "02i2500000HTaW9AAL") => {
                MockResult::ID(String::from("0012500001Lhk3hAAB"))
            }
            MockArgs::GetAccount("0012500001Lhk3hAAB") => {
                MockResult::Account(sf::Account::new_for_tests())
            }
            _ => panic!("unhandled request/response: {:?}", args),
        });
        let acc = run(client, q, config).await.unwrap();
        assert_eq!(acc.id, "id-for-tests");
    }

    #[tokio::test]
    async fn run_from_extra_ok_get_account_not_found() {
        let q = "some-query";
        let config = Config {
            additional_fields: vec![],
            search_fields: vec!["Account.SomeField".parse::<sf::EntityField>().unwrap()],
        };
        let client = TestClient::new(|args| match args {
            MockArgs::GetAccountIDByField("Account.SomeField", "some-query") => {
                MockResult::ID(String::from("0012500001Lhk3hAAB"))
            }
            MockArgs::GetAccount("0012500001Lhk3hAAB") => MockResult::Err(sf::Error::NotFound),
            _ => panic!("unhandled request/response: {:?}", args),
        });
        let err = run(client, q, config).await.unwrap_err();
        assert_eq!(err.message, "nothing found for query \"some-query\"");
    }

    #[tokio::test]
    async fn run_from_extra_ok_get_account_error() {
        let q = "some-query";
        let config = Config {
            additional_fields: vec![],
            search_fields: vec!["Asset.OpportunityId__c".parse::<sf::EntityField>().unwrap()],
        };
        let client = TestClient::new(|args| match args {
            MockArgs::GetAccountIDByField("Asset.OpportunityId__c", "some-query") => {
                MockResult::ID(String::from("0012500001Lhk3hAAA"))
            }
            MockArgs::GetAccount("0012500001Lhk3hAAA") => {
                MockResult::Err(sf::Error::Message(String::from("bad wolf")))
            }
            _ => panic!("unhandled request/response: {:?}", args),
        });
        let err = run(client, q, config).await.unwrap_err();
        assert_eq!(err.message, "bad wolf");
    }

    #[tokio::test]
    async fn run_from_extra_not_found() {
        let q = "some-query";
        let config = Config {
            additional_fields: vec![],
            search_fields: vec![
                "Account.SomeField".parse::<sf::EntityField>().unwrap(),
                "Opportunity.AnotherField"
                    .parse::<sf::EntityField>()
                    .unwrap(),
            ],
        };
        let client = TestClient::new(|args| match args {
            MockArgs::GetAccountIDByField("Account.SomeField", "some-query") => {
                MockResult::Err(sf::Error::NotFound)
            }
            MockArgs::GetAccountIDByField("Opportunity.AnotherField", "some-query") => {
                MockResult::Err(sf::Error::NotFound)
            }
            _ => panic!("unhandled request/response: {:?}", args),
        });
        let err = run(client, q, config).await.unwrap_err();
        assert_eq!(err.message, "nothing found for query \"some-query\"");
    }

    #[tokio::test]
    async fn run_from_extra_not_found_no_fields() {
        let q = "some-query";
        let config = Config::empty();
        let client = TestClient::new(|args| match args {
            _ => panic!("unhandled request/response: {:?}", args),
        });
        let err = run(client, q, config).await.unwrap_err();
        assert_eq!(err.message, "nothing found for query \"some-query\"");
    }

    #[tokio::test]
    async fn run_from_extra_error() {
        let q = "some-query";
        let config = Config {
            additional_fields: vec![],
            search_fields: vec![
                "Account.SomeField".parse::<sf::EntityField>().unwrap(),
                "Opportunity.AnotherField"
                    .parse::<sf::EntityField>()
                    .unwrap(),
            ],
        };
        let client = TestClient::new(|args| match args {
            MockArgs::GetAccountIDByField("Account.SomeField", "some-query") => {
                MockResult::Err(sf::Error::Message(String::from("bad wolf")))
            }
            _ => panic!("unhandled request/response: {:?}", args),
        });
        let err = run(client, q, config).await.unwrap_err();
        assert_eq!(err.message, "bad wolf");
    }

    #[tokio::test]
    async fn run_from_email_ok_get_account_ok() {
        let q = "who@example.com";
        let config = Config {
            additional_fields: vec![],
            search_fields: vec!["Account.SomeField".parse::<sf::EntityField>().unwrap()],
        };
        let client = TestClient::new(|args| match args {
            MockArgs::GetAccountIDByField("Contact.email", "who@example.com") => {
                MockResult::ID(String::from("0012500001Lhk3hAAB"))
            }
            MockArgs::GetAccount("0012500001Lhk3hAAB") => {
                MockResult::Account(sf::Account::new_for_tests())
            }
            _ => panic!("unhandled request/response: {:?}", args),
        });
        let acc = run(client, q, config).await.unwrap();
        assert_eq!(acc.id, "id-for-tests");
    }

    #[tokio::test]
    async fn run_from_email_not_found_get_account_ok() {
        let q = "who@example.com";
        let config = Config {
            additional_fields: vec![],
            search_fields: vec!["Account.SomeField".parse::<sf::EntityField>().unwrap()],
        };
        let client = TestClient::new(|args| match args {
            MockArgs::GetAccountIDByField("Contact.email", "who@example.com") => {
                MockResult::Err(sf::Error::NotFound)
            }
            MockArgs::GetAccountIDByField("Account.SomeField", "who@example.com") => {
                MockResult::ID(String::from("0012500001Lhk3hAAB"))
            }
            MockArgs::GetAccount("0012500001Lhk3hAAB") => {
                MockResult::Account(sf::Account::new_for_tests())
            }
            _ => panic!("unhandled request/response: {:?}", args),
        });
        let acc = run(client, q, config).await.unwrap();
        assert_eq!(acc.id, "id-for-tests");
    }

    #[tokio::test]
    async fn run_from_email_error() {
        let q = "who@example.com";
        let config = Config {
            additional_fields: vec![],
            search_fields: vec!["Account.SomeField".parse::<sf::EntityField>().unwrap()],
        };
        let client = TestClient::new(|args| match args {
            MockArgs::GetAccountIDByField("Contact.email", "who@example.com") => {
                MockResult::Err(sf::Error::Message(String::from("bad wolf")))
            }
            _ => panic!("unhandled request/response: {:?}", args),
        });
        let err = run(client, q, config).await.unwrap_err();
        assert_eq!(err.message, "bad wolf");
    }

    /// A Salesforce client implementing the sf::Client trait for testing.
    #[derive(Debug)]
    struct TestClient<T: Fn(MockArgs) -> MockResult> {
        request: T,
    }

    impl<'a, T: Fn(MockArgs) -> MockResult> TestClient<T> {
        fn new(f: T) -> Self {
            Self { request: f }
        }
    }

    #[async_trait]
    impl<'a, T: Fn(MockArgs) -> MockResult + Sync> sf::Client for TestClient<T> {
        async fn get_account(
            &self,
            id: &str,
            _additional_fields: Vec<EntityField>,
        ) -> Result<sf::Account, sf::Error> {
            match (self.request)(MockArgs::GetAccount(id)) {
                MockResult::Account(acc) => Ok(acc),
                MockResult::Err(err) => Err(err),
                _ => panic!("invalid mock result for account"),
            }
        }

        async fn get_account_id_by_field(
            &self,
            ef: &EntityField,
            value: &str,
        ) -> Result<String, sf::Error> {
            match (self.request)(MockArgs::GetAccountIDByField(&ef.to_string(), value)) {
                MockResult::ID(id) => Ok(id),
                MockResult::Err(err) => Err(err),
                _ => panic!("invalid mock result for {}", ef),
            }
        }
    }

    #[derive(Debug)]
    enum MockArgs<'a> {
        GetAccount(&'a str),
        GetAccountIDByField(&'a str, &'a str),
    }

    #[derive(Debug)]
    enum MockResult {
        Account(sf::Account),
        Err(sf::Error),
        ID(String),
    }

    impl sf::Account {
        /// Return an account for testing.
        fn new_for_tests() -> Self {
            Self {
                id: String::from("id-for-tests"),
                name: String::from("name"),
                account_number: None,
                billing_address: Default::default(),
                created_date: String::from("name"),
                last_modified_date: Some(String::from("name")),
                assets: None,
                contacts: None,
                opportunities: None,
                extra: HashMap::new(),
            }
        }
    }

    impl Config {
        /// Return an empty config.
        fn empty() -> Self {
            return Self {
                additional_fields: vec![],
                search_fields: vec![],
            };
        }
    }
}
