// Hyper performs HTTP actions
extern crate hyper;
// Futures provides async programming functionality
extern crate futures;
// Tokio provides a method for actually executing futures code
extern crate tokio_core;
// Serde allows configuration via JSON file
extern crate serde_json;
#[macro_use] extern crate serde_derive;

// For file IO
use std::io::prelude::*;
use std::fs::File;
// Arguments to the process
use std::env::args;
use std::process::Command;

use futures::{Future, Stream};
use hyper::{Client, StatusCode};
use tokio_core::reactor::Core;

// The structure of a configuration file
#[derive(Deserialize)]
struct Config {
    sites: Vec<Site>
}

// The structure of each item in the config list
#[derive(Deserialize)]
struct Site {
    name: String,
    url: String,
    unit: String
}

// Acquires the configuration
fn get_cfg() -> Result<Config, String> {
    let args: Vec<_> = args().collect();
    if args.len() != 2 {
        Err("Not enough arguments.".into())
    } else {
        use std::error::Error;
        match File::open(&args[1]) {
            Ok(mut f) => {
                let mut contents = String::new();
                if let Err(e) = f.read_to_string(&mut contents) {
                    return Err(format!("Could not read from file {}: {}", args[1], e.description()))
                };
                match serde_json::from_str(&contents) {
                    Ok(v) => Ok(v),
                    Err(e) => Err(format!("Could not understand configuration in file {}: {}  ({:?})", args[1], e, e))
                }
            }
            Err(e) => { Err(format!("Could not open file {}: {}", args[1], e.description())) }
        }
    }
}

fn main() {
    use std::process::exit;
    let cfg = match get_cfg() {
        Ok(c) => c,
        Err(e) => {
            println!("Error: {}", e);
            println!("Usage: harf CONFIG");
            println!("\tCONFIG must be a valid JSON file containing an object with a single item, 'sites', which is a list of objects.");
            println!("\tEach of these objects must have the following keys:");
            println!("\t - name: The human-readable descriptive name of the service.");
            println!("\t - url: The service's URL.");
            println!("\t - unit: The systemd unit to restart when this service is down.");
            exit(1);
        }
    };

    let mut core = Core::new().unwrap();
    let client = Client::new(&core.handle());

    let mut work = Vec::new();

    for site in cfg.sites {
        let uri = site.url.parse().unwrap();
        work.push(client.get(uri).map(move |res| {
            if res.status() == StatusCode::Ok {
                println!("{}: OK", site.name)
            } else {
                println!("{}: Down! Restarting...", site.name);
                let output = Command::new("systemctl").arg("restart")
                    .arg(&site.unit).output().unwrap();
            }
            
        }));
    }

    core.run(futures::future::join_all(work)).unwrap();
}
