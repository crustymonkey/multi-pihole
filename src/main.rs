extern crate chrono;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;

mod pihole;

use clap::{ArgMatches, App};

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
fn get_args() -> ArgMatches<'static> {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author("Jay Deiman")
        .about(crate_description!())
        .set_term_width(80)
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

fn main() {
    let args = get_args();
    setup_logging(&args);

    let api_key = "b5ca56146091ff594743d9108e5637672aed99160be6b2a1d68eeaca86fff8f0";

    let p = pihole::Pihole::new(
        "http://pihole.splitstreams.com",
        "b5ca56146091ff594743d9108e5637672aed99160be6b2a1d68eeaca86fff8f0",
    );

    debug!("{:?}", p.disable(3));
}
