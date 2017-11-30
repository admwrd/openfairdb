use clap::{Arg, App};
use super::web;
use dotenv::dotenv;
use std::{env, process};

pub fn run() {
    dotenv().ok();
    let matches = App::new("openFairDB")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Markus Kohlhase <mail@markus-kohlhase.de>")
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .default_value("6767")
                .help("Set the port to listen"),
        )
        .arg(
            Arg::with_name("db-url")
                .long("db-url")
                .value_name("DATABASE_URL")
                .help("URL to the database"),
        )
        .arg(Arg::with_name("enable-cors").long("enable-cors").help(
            "Allow requests from any origin",
        ))
        .get_matches();

    let db_url = match matches.value_of("db-url") {
        Some(db_url) => db_url.into(),
        None => {
            match env::var("DATABASE_URL") {
                Ok(url) => url,
                Err(_) => {
                    println!("{}", matches.usage());
                    process::exit(1);
                }
            }
        }
    };

    let port = match matches.value_of("port") {
        Some(port) => port.parse::<u16>().unwrap(),
        None => {
            println!("{}", matches.usage());
            process::exit(1);
        }
    };

    web::run(&db_url, port, matches.is_present("enable-cors"));

}
