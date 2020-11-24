use std::env;
use std::process;

mod arg;
mod config;
mod environ;
mod error;
mod finder;
mod output;
mod sf;

#[tokio::main]
async fn main() {
    // Parse arguments.
    let (action, format) = arg::parse(env::args().collect());
    let query = match action {
        arg::Action::Find(id) => id,
        arg::Action::Config => match config::Config::edit() {
            Ok(_) => {
                eprintln!("config saved successfully");
                process::exit(0);
            }
            Err(err) => {
                eprintln!("cannot edit config: {}", err);
                process::exit(1);
            }
        },
        arg::Action::Help => {
            arg::usage();
            process::exit(1);
        }
        arg::Action::Err(err) => {
            eprintln!("cannot parse args: {}", err);
            process::exit(1);
        }
    };

    // Fetch the environment variables.
    let e = match environ::Env::new() {
        Ok(v) => v,
        Err(err) => {
            eprintln!("cannot retrieve environment info: {}", err);
            process::exit(1);
        }
    };

    // Parse config.
    let conf = match config::Config::parse() {
        Err(err) => {
            eprintln!("cannot parse config: {}", err);
            process::exit(1);
        }
        Ok(conf) => conf,
    };

    // Instantiate the Salesforce client.
    let client = match sf::client(e).await {
        Err(err) => {
            eprintln!("cannot instantiate sf client: {}", err);
            process::exit(1);
        }
        Ok(v) => v,
    };

    // Start looking for stuff!
    match finder::run(client, &query, conf).await {
        Err(err) => {
            eprintln!("cannot find sf entities: {}", err);
            process::exit(1);
        }
        Ok(acc) => {
            if let Err(err) = output::print(&acc, format) {
                eprintln!("cannot serialize account: {}", err);
                process::exit(1);
            }
        }
    };
}
