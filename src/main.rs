#![allow(dead_code)]
extern crate chrono;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate serde;
extern crate serde_json;

mod plib;

use clap::{ArgMatches, App, Arg, SubCommand, value_t};
use std::{
    path::Path,
    env,
    fs::File,
    io::{self, Read, Write},
};
use plib::{
    config::{self, PiServer, PiConfig},
    pihole::Pihole,
};

static LOGGER: GlobalLogger = GlobalLogger;

struct GlobalLogger;

/// This implements the logging to stderr from the `log` crate
impl log::Log for GlobalLogger {
    fn enabled(&self, meta: &log::Metadata) -> bool {
        return meta.level() <= log::max_level();
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let d = chrono::Local::now();
            eprintln!(
                "{} - {} - {}:{} {} - {}",
                d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                record.level(),
                record.file().unwrap(),
                record.line().unwrap(),
                record.target(),
                record.args(),
            );
        }
    }

    fn flush(&self) {}
}

/// Create a set of CLI args via the `clap` crate and return the matches
fn get_args<'a>(def_conf: &'a str) -> ArgMatches<'a> {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author("Jay Deiman")
        .about(crate_description!())
        .set_term_width(80)
        .subcommand(SubCommand::with_name("status")
            .about("Get the current status for your pihole servers \
            (enabled|disabled)")
        )
        .subcommand(SubCommand::with_name("enable")
            .about("Enable the pihole servers")
        )
        .subcommand(SubCommand::with_name("disable")
            .about("Disable the pihole servers")
            .arg(Arg::with_name("time")
                .short("-t")
                .long("--time")
                .value_name("SECS")
                .default_value("300")
                .help("Disable the pihole servers for this many seconds")
            )
        )
        .subcommand(SubCommand::with_name("summary")
            .about("Print a summary for each server")
        )
        .subcommand(SubCommand::with_name("version")
            .about("Print the version for each server")
        )
        .subcommand(SubCommand::with_name("top-domains")
            .about("Print the top N domains")
            .arg(Arg::with_name("topn")
                .short("-n")
                .long("--topn")
                .value_name("INT")
                .default_value("10")
                .help("Print this many domains")
            )
        )
        .subcommand(SubCommand::with_name("top-clients")
            .about("Print the query data for the top N clients")
            .arg(Arg::with_name("topn")
                .short("-n")
                .long("--topn")
                .value_name("INT")
                .default_value("10")
                .help("Print this many clients")
            )
        )
        .subcommand(SubCommand::with_name("upstreams")
            .about("Print the forward destination stats")
        )
        .subcommand(SubCommand::with_name("query-types")
            .about("Print the query type stats")
        )
        .subcommand(SubCommand::with_name("recent-blocked")
            .about("Print the most recently blocked domain")
            .arg(Arg::with_name("num")
                .short("-n")
                .long("--num")
                .value_name("INT")
                .default_value("10")
                .help("Print this many most recent blocked domains")
            )
        )
        .arg(Arg::with_name("config")
            .short("-c")
            .long("--config")
            .default_value(def_conf)
            .value_name("PATH")
            .help("The path to the config file")
        )
        .arg_from_usage("-s, --show-config 'Show the current config and exit'")
        .arg_from_usage("-r, --reconfigure '(Re)configure your pihole servers'")
        .arg_from_usage("-D, --debug 'Turn on debug output'")
        .get_matches();

    return matches;
}

/// Set the global logger from the `log` crate
fn setup_logging(args: &ArgMatches) {
    let l = if args.is_present("debug") {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(l);
}

fn check_yes(ans: &str) -> bool {
    let answers = ["y", "yes", "YES", "Y"];

    return answers.contains(&ans.trim());
}

fn check_num_resp(ans: &str) -> Option<usize> {
    let _ans = ans.trim();
    return match _ans.parse::<usize>() {
        Ok(n) => Some(n),
        _ => None,
    };
}

/// Will return True if user wants to add another, false otherwise
fn get_new_server(ask_another: bool) -> (config::PiServer, bool) {
    print!("Please enter the url for your server:  ");
    io::stdout().flush().unwrap();
    let mut url = String::new();
    io::stdin().read_line(&mut url).unwrap();

    let mut api_key = String::new();
    print!("Now, enter the password (same as the web interface) for that server:  ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut api_key).unwrap();

    let ret = config::PiServer::new(url.trim(), api_key.trim());

    if ask_another {
        let mut ans = String::new();
        print!("Add another [y/N] ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut ans).unwrap();
        return (ret, check_yes(&ans));
    }

    return (ret, false);
}

fn get_modify_delete(server: &config::PiServer) -> Option<usize> {
    let mut resp = String::new();
    println!("Found a config for '{}' with password ******",
        server.base_url);
    println!("Choose an option:");
    println!("  1) modify");
    println!("  2) delete");
    println!("  3) do not modify");
    print!("Select [1-3]:  ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut resp).unwrap();

    let ret = check_num_resp(&resp);
    return match ret {
        Some(1) | Some(2) | Some(3) => ret,
        _ => {
            println!("\nInvalid response, select again");
            get_modify_delete(server)
        },
    };
}

fn add_new_servers(conf: &mut config::PiConfig) {
    loop {
        let (new_srvr, add) = get_new_server(true);
        conf.add_server(new_srvr);
        if !add {
            break;
        }
    }

}

fn configure(conf_path: &Path, cur_config: Option<config::PiConfig>) -> config::PiConfig {
    let mut ret = config::PiConfig::new();
    if let Some(c) = cur_config {
        ret = c.clone();
    }

    if ret.servers.len() < 1 {
        // This is a new configuration
        println!("Welcome to the mpihole configuration!\n");
        println!("We're going to configure some new pihole servers.  For each");
        println!("one, you'll need the base url (http://mypihole.example.com)");
        println!("and the password that you use for the web interface.\n");
        
        add_new_servers(&mut ret);
    } else {
        let mut tmp: Vec<PiServer> = vec![];
        for svr in ret.servers {
            match get_modify_delete(&svr) {
                Some(1) => {
                    // modify, basically, delete and ask for another
                    let (new_srvr, _) = get_new_server(false);
                    tmp.push(new_srvr);
                },
                Some(2) => {
                    // do nothing.  We own this, so we don't have to do anything here
                },
                Some(3) => {
                    // We just re-add the owned server
                    tmp.push(svr);
                },
                _ => {
                    panic!("Something went wrong matching responses!");
                },
            }
        }
        
        // Now we just need to set ret servers to the tmp
        ret.servers = tmp;
        
        // Now we see if we want to add new servers
        let mut ans = String::new();
        print!("Add new servers? [y/N]  ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut ans).unwrap();
        if check_yes(&ans) {
            add_new_servers(&mut ret);
        }
    }

    ret.save_to_path(conf_path).ok();

    return ret;
}

fn show_config(conf_path: &Path) {
    let mut c = String::new();
    let mut fp = File::open(conf_path).expect(
        &format!("Could not open conf file at: {}", conf_path.to_string_lossy())
    );

    fp.read_to_string(&mut c).ok();

    println!("{}", c);
}

fn main() {
    let def_conf = format!("{}/.mpihole", env::var("HOME").ok().unwrap());
    let args = get_args(&def_conf);
    setup_logging(&args);
    let conf_path = Path::new(args.value_of("config").unwrap());

    if args.is_present("show-config") {
        show_config(conf_path);
        std::process::exit(0);
    }


    let conf = match PiConfig::from_path(conf_path) {
        Ok(c) => c,
        Err(config::FromPath::FileNotFound(_)) => 
            configure(conf_path, None),
        Err(config::FromPath::SerError(e))=> {
            error!("Failed to deserialize config: {}", e);
            std::process::exit(1);
        },
        _ => {
            error!("Unknown config error");
            std::process::exit(1);
        }
    };

    if args.is_present("reconfigure") || conf.servers.len() < 1{
        configure(conf_path, Some(conf));
        std::process::exit(0);
    }

    let servers: Vec<Pihole> = conf.servers
        .iter()
        .map(|x| {
            let mut ph = Pihole::from_cfg(x);
            ph.auth().expect(
                &format!("Failed to authenticate with server: {}", x.base_url)
            );
            ph
        })
        .collect();

    // Handle the subcommands
    if let Some(matches) = args.subcommand_matches("disable") {
        let secs = value_t!(matches, "time", usize).ok().unwrap();
        for s in &servers {
            debug!("Disabling '{}' for {} secs", s.base_url, secs);
            s.disable(secs);
        }
    } else if let Some(_) = args.subcommand_matches("enable") {
        for s in &servers {
            debug!("Enabling '{}'", s.base_url);
            s.enable();
        }
    } else if let Some(_) = args.subcommand_matches("summary") {
        for s in &servers {
            println!("Summary for {}", s.base_url);
            match s.summary() {
                None => warn!("Couldn't get a summary for {}", s.base_url),
                Some(v) => println!("{}",
                    serde_json::to_string_pretty(&v).ok().unwrap()
                ),
            }
            println!();
        }
    } else if let Some(_) = args.subcommand_matches("version") {
        for s in &servers {
            match s.version() {
                None => warn!("Couldn't get a version for {}", s.base_url),
                Some(v) => {
                    println!("Version info for {}", s.base_url);
                    println!("{}",
                        serde_json::to_string_pretty(&v).ok().unwrap()
                    );
                }
            }
            println!();
        }
    } else if let Some(matches) = args.subcommand_matches("top-domains") {
        let topn = value_t!(matches, "topn", usize).ok().unwrap();
        for s in &servers {
            println!("The top {} domains for {}", topn, s.base_url);
            match s.top_items(Some(topn)) {
                None => warn!("Couldn't get top domains for {}", s.base_url),
                Some(v) => println!("{}",
                    serde_json::to_string_pretty(&v).ok().unwrap()
                ),
            }
        }
    } else if let Some(matches) = args.subcommand_matches("top-clients") {
        let topn = value_t!(matches, "topn", usize).ok().unwrap();
        for s in &servers {
            println!("The top {} clients for {}", topn, s.base_url);
            match s.top_clients(Some(topn)) {
                None => warn!("Couldn't get top clients for {}", s.base_url),
                Some(v) => println!("{}",
                    serde_json::to_string_pretty(&v).ok().unwrap()
                ),
            }
        }
    } else if let Some(_) = args.subcommand_matches("upstreams") {
        for s in &servers {
            println!("Forward destinations for {}", s.base_url);
            match s.get_upstreams() {
                None => warn!("Couldn't get forward destinations for {}",
                    s.base_url),
                Some(v) => println!("{}",
                    serde_json::to_string_pretty(&v).ok().unwrap()
                ),
            }
            println!();
        }
    } else if let Some(_) = args.subcommand_matches("query-types") {
        for s in &servers {
            println!("Query types for {}", s.base_url);
            match s.get_query_types() {
                None => warn!("Couldn't get query types for {}", s.base_url),
                Some(v) => println!("{}",
                    serde_json::to_string_pretty(&v).ok().unwrap()
                ),
            }
            println!();
        }
    } else if let Some(matches) = args.subcommand_matches("recent-blocked") {
        for s in &servers {
            let num = value_t!(matches, "num", usize).ok().unwrap();
            match s.recent_blocked(num) {
                None => warn!("Couldn't get most recent blocked for {}",
                    s.base_url),
                Some(v) => {
                    println!("Most recent blocked for {}", s.base_url);
                    for dom in v["blocked"].as_array().unwrap() {
                        println!("{}", dom.as_str().unwrap());
                    }
                }
            }
            println!();
        }
    } else if let Some(_) = args.subcommand_matches("status") {
        for s in &servers {
            match s.status() {
                None => warn!("Couldn't get status for {}", s.base_url),
                Some(v) => println!("{}: {}", s.base_url, v),
            }
        }
    }
}
