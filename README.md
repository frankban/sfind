# sfind

Quickly find entities in Salesforce, and show the matching account, assets,
opportunities and contacts.

## Installation

Run `cargo install sfind`.

## Usage

Find Salesforce entities by id:
```
sfind 0012500001Lhk3hAAB
```

Find Salesforce entities by contact email:
```
sfind who@example.com
```

Use JSON output:
```
sfind 0012500001Lhk3hAAB --json
```

Get help:
```
sfind help
```

## Configuration

By running `sfind config` the default editor is used to open the configuration
file. By editing the configuration we can declare additional object fields that
must be reported or even string fields that must be matched when searching:
```
fields = [
    'Account.Foo__c',
    'Contact.Birthdate',
]
search = [
    'Account.Name',
    'Opportunity.LeadSource',
]
```

## Supported entities

sfind works with accounts, assets, opportunities and contacts.

## Note

This must be intended mostly as a rust learning exercise.
