extern crate chrono;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate iron;
extern crate router;
extern crate configparser;

mod plib;

use clap::{ArgMatches, Arg, App};

use std::{
    path::Path,
    sync::Arc,
    io::prelude::*,
    fs::File,
};
use iron::{
    prelude::*,
    status,
    modifiers::Header,
    headers::ContentType,
    mime::{Mime, TopLevel, SubLevel},
};
use router::Router;
use plib::{
    config::{self, PiConfig},
    pihole::Pihole,
    web_config::get_config,
};
use configparser::ini::Ini;

struct ReqContext {
    pub web_conf: Ini,
    pub servers: Vec<Pihole>,
}

static LOGGER: GlobalLogger = GlobalLogger;
struct GlobalLogger;

/// This implements the logging to stderr from the \`log\` crate
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

/// Create a set of CLI args via the \`clap\` crate and return the matches
fn get_args() -> ArgMatches<'static> {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author("Jay Deiman")
        .about(crate_description!())
        .set_term_width(80)
        .arg_from_usage("-D, --debug 'Turn on debug output'")
        .arg(Arg::with_name("pi_list_config")
            .short("-l")
            .long("--pi-list-config")
            .default_value("/etc/.mpihole")
            .help("The path to the JSON config created by running `pi-ctl -r` \
                This is the list of servers to act upon")
        )
        .arg(Arg::with_name("web_config")
            .short("-c")
            .long("--web-config")
            .default_value("/etc/mpihole-web.ini")
            .help("The path to the web server config file.  Note this is \
                different, and separate, from the server list file")
        )
        .get_matches();

    return matches;
}

/// Set the global logger from the \`log\` crate
fn setup_logging(args: &ArgMatches) {
    let l = if args.is_present("debug") {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(l);
}

fn disable(req: &mut Request, ctx: Arc<ReqContext>) -> IronResult<Response> {
    let query = req.extensions.get::<Router>().unwrap();
    let secs_str = match query.find("secs") {
        Some(s) => s,
        None => {
            return Ok(Response::with((status::BadRequest, "Missin seconds")));
        },
    };
    let secs = match secs_str.parse::<usize>() {
        Ok(s) => s,
        Err(_) => {
            return Ok(Response::with(
                (status::BadRequest, "Invalid int value")
            ));
        },
    };

    for s in &ctx.servers {
        info!("Disabling pihole on {} for {} secs", s.base_url, secs);
        s.disable(secs);
    }

    return Ok(Response::with((status::Ok, "OK")));
}

fn enable(_: &mut Request, ctx: Arc<ReqContext>) -> IronResult<Response> {
    for s in &ctx.servers {
        info!("Enabling pihole for {}", s.base_url);
        s.enable();
    }

    return Ok(Response::with((status::Ok, "OK")));
}

fn index(_: &mut Request, ctx: Arc<ReqContext>) -> IronResult<Response> {
    let static_dir = ctx.web_conf.get("main", "static_dir").unwrap();
    let fname = Path::new(&static_dir).join("index.html");

    let mut file = File::open(&fname).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    let content_type = Header(
        ContentType(Mime(TopLevel::Text, SubLevel::Html, vec![]))
    );

    return Ok(Response::with((status::Ok, content, content_type)));
}

fn static_f(req: &mut Request, ctx: Arc<ReqContext>) -> IronResult<Response> {
    let static_dir = ctx.web_conf.get("main", "static_dir").unwrap();
    // Strip off the first part of the path ("static") and replace it with the
    // static dir to get a proper path to a static file
    let path = &req.url.path()[1..];
    let fname = Path::new(&static_dir).join(path.join("/"));
    info!("serving file: {:?}", fname);
    if !fname.exists() {
        return Ok(Response::with((status::NotFound, "Not found\n")));
    }

    let mut content = String::new();
    File::open(&fname).unwrap().read_to_string(&mut content).unwrap();

    return Ok(Response::with((status::Ok, content)));
}

fn create_routes(router: &mut Router, context: Arc<ReqContext>) {
    // TODO: There's very likely a better way to do this and make the compiler happy
    let dis_ctx = context.clone();
    let en_ctx = context.clone();
    let idx_ctx = context.clone();
    let static_ctx = context.clone();
    router.get(
        "/disable/:secs",
        move |r: &mut Request| disable(r, dis_ctx.clone()),
        "disable",
    );
    router.get(
        "/enable",
        move |r: &mut Request| enable(r, en_ctx.clone()),
        "enable",
    );
    router.get(
        "/",
        move |r: &mut Request| index(r, idx_ctx.clone()),
        "index",
    );
    router.get(
        "/static/*",
        move |r: &mut Request| static_f(r, static_ctx.clone()),
        "static",
    );
}

fn main() {
    let args = get_args();
    setup_logging(&args);

    let conf_path = Path::new(args.value_of("pi_list_config").unwrap());
    let server_conf = match PiConfig::from_path(conf_path) {
        Ok(c) => c,
        Err(config::FromPath::SerError(e))=> {
            error!("Failed to deserialize config: {}", e);
            std::process::exit(1);
        },
        _ => {
            error!("Unknown config error");
            std::process::exit(1);
        }
    };

    let web_conf = get_config(args.value_of("web_config").unwrap());

    let context = Arc::new(ReqContext {
        web_conf: web_conf,
        servers: server_conf.servers
            .iter()
            .map(|x| Pihole::from_cfg(x))
            .collect(),
    });

    let mut router = Router::new();
    create_routes(&mut router, context.clone());

    debug!("Creating web server bound to {}",
        context.web_conf.get("main", "bind_to").unwrap());
    Iron::new(router).http(context.web_conf.get("main", "bind_to").unwrap())
        .unwrap();
}