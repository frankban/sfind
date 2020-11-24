/// Parse the given args and returns the action to be taken by the tool, and the
/// output format.
pub fn parse(args: Vec<String>) -> (Action, Format) {
    let mut args = args.into_iter().skip(1);
    let err = Action::Err(String::from("usage: sfind <arg>: see sfind help"));

    let arg = match args.next() {
        None => return (err, Format::Tabular),
        Some(arg) => arg,
    };
    let action = match &arg[..] {
        "config" => Action::Config,
        "help" => Action::Help,
        _ => Action::Find(arg),
    };
    let format = match args.next() {
        None => Format::Tabular,
        Some(arg) if arg == *"--json" => Format::JSON,
        _ => return (err, Format::Tabular),
    };
    (action, format)
}

/// An action to be executed by the tool.
#[derive(Debug, PartialEq)]
pub enum Action {
    /// Find something in Salesforce.
    Find(String),
    /// Open the config file.
    Config,
    /// Print help end exit.
    Help,
    /// Print an error and exit.
    Err(String),
}

/// Format represents how to format the returned information.
#[derive(Debug, PartialEq)]
pub enum Format {
    Tabular,
    JSON,
}

/// Print the help for the tool.
pub fn usage() {
    eprintln!(
        "
sfind

Quickly find entities in Salesforce, and show the matching account, assets,
opportunities and contacts.

Usage:
    sfind <id or key> [--json]
    sfind config

Examples:

Find Salesforce entities by id:
    sfind 0012500001Lhk3hAAB

Find Salesforce entities by contact email:
    sfind who@example.com

Use JSON output:
sfind 0012500001Lhk3hAAB --json

Authentication:

Set the following environment variables for authenticating to Salesforce:
SFDC_CLIENT_ID
SFDC_CLIENT_SECRET
SFDC_USERNAME
SFDC_PASSWORD
SFDC_SECRET_TOKEN
SFDC_SANDBOX (optional)

Configuration:

By running `sfind config` the default editor is used to open the configuration
file. By editing the configuration we can declare additional object fields that
must be reported or even string fields that must be matched when searching:

    fields = [
        'Account.Foo__c',
        'Contact.Birthdate',
    ]
    search = [
        'Account.Name',
        'Opportunity.LeadSource',
    ]

sfind works with accounts, assets, opportunities and contacts."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_no_args() {
        let args = vec![String::from("command")];
        let (action, _) = parse(args);
        let msg = String::from("usage: sfind <arg>: see sfind help");
        assert_eq!(action, Action::Err(msg));
    }

    #[test]
    fn parse_error_too_many_args() {
        let args = vec![
            String::from("command"),
            String::from("some-id"),
            String::from("bad-wolf"),
        ];
        let (action, _) = parse(args);
        let msg = String::from("usage: sfind <arg>: see sfind help");
        assert_eq!(action, Action::Err(msg));
    }

    #[test]
    fn parse_config() {
        let args = vec![String::from("command"), String::from("config")];
        let (action, _) = parse(args);
        assert_eq!(action, Action::Config);
    }

    #[test]
    fn parse_help() {
        let args = vec![String::from("command"), String::from("help")];
        let (action, _) = parse(args);
        assert_eq!(action, Action::Help);
    }

    #[test]
    fn parse_find() {
        let args = vec![String::from("command"), String::from("some-id")];
        let (action, format) = parse(args);
        assert_eq!(action, Action::Find(String::from("some-id")));
        assert_eq!(format, Format::Tabular);
    }

    #[test]
    fn parse_find_json() {
        let args = vec![
            String::from("command"),
            String::from("some-id"),
            String::from("--json"),
        ];
        let (action, format) = parse(args);
        assert_eq!(action, Action::Find(String::from("some-id")));
        assert_eq!(format, Format::JSON);
    }
}
