// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, 
// you can obtain one at https://mozilla.org/MPL/2.0/.


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
use std::error::Error;

use futures::{Future, future};
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
    unit: Option<String>,
    cmd: Option<String>,
}

// Acquires the configuration
fn get_cfg() -> Result<Config, String> {
    // Try to look at the arguments
    let args: Vec<_> = args().collect();
    if args.len() != 2 {
        return Err("Not enough arguments.".into());
    }
    
    // Attempt to open the given config file & read config from it
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

fn main() {
    println!("harf - HTTP Watchdog v.{}\n", env!("CARGO_PKG_VERSION"));
    // If unable to get the config, just print out the usage
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
            println!("\t - unit: The systemd unit to restart when this service is down. (Optional)");
            println!("\t - cmd: An arbitrary command to run when this service is down. (Optional)");
            exit(1);
        }
    };

    // Start up the core reactor, which executes futures
    let mut core = Core::new().expect("Could not start async reactor");
    // Create a HTTP client based on the reactor
    let client = Client::new(&core.handle());

    // Contains all the individual request/response futures
    let mut work = Vec::new();

    // Construct a future for each site
    for site in cfg.sites {
        // Acquire the URI of the service to be tested.
        let uri: hyper::Uri = match site.url.parse() {
            Ok(uri) => uri,
            Err(e) => {
                println!("Unable to parse '{}': {}", site.url, e);
                continue;
            }
        };

        // Perform a HTTP GET request for the URL
        let client_request = client.get(uri);

        // If the GET request failed, restart the systemd service and/or run the specified command
        let f = client_request.then(move |res| {
            let (succeeded, msg) = match res {
                Ok(res) => (res.status() == StatusCode::Ok, format!("{}", res.status())),
                Err(e) => (false, format!("{}", e.description()))
            };

            if succeeded {
                println!("[OK]     {}", site.name)
            } else {
                println!("[NOT OK] {}", site.name);
                println!("\t |{}", msg);
                if let Some(unit) = site.unit {
                    println!("\t |Restarting unit '{}'", unit);
                    Command::new("systemctl")
                        .arg("restart")
                        .arg(&unit)
                        .output()
                        .expect(&format!("Could not restart systemd unit {}", &unit));
                }
                if let Some(cmd) = site.cmd {
                    println!("\t |Running command '{}'", cmd);
                    Command::new("sh")
                        .arg("-c")
                        .arg(&cmd)
                        .output()
                        .expect(&format!("Could not execute command '{}'", &cmd));
                }
                println!("\t +------------\n");
            };
            future::ok::<(), ()>(())
        });

        work.push(f);
    }

    core.run(future::join_all(work)).expect("Could not run primary enjoinment");
}
