# multi-pihole
This is just a cli for running commands against multiple pihole servers.  This
is helpful if you have a primary, secondary, and any tertiary servers, but want
to manage them all at the same time.

## Configuring `pi-ctl`
This will happen automatically on a first run.  The JSON config file will be
saved at `~/.mpihole`.  You can modify that file directly, or you can run 
`pi-ctl -r` to re-configure, including adding/modifying/removing pihole servers.

## Commands
There are a number of different commands available.  You can find them all
by running `pi-ctl -h`.  They API versions of these are also documented on
the [Pi-hole API site](https://discourse.pi-hole.net/t/pi-hole-api/1863).

### `disable`
If you run `pi-ctl disable`, it will disable all configured servers for 5
minutes.  You can optionally set the disable time with the `-t` flag:
```
# Disable for 60 seconds
pi-ctl disable -t 60
```

### `enable`
To re-enable all your servers, you just run `pi-ctl enable`.  There are no
options for this.

### help
Run `pi-ctl -h` to show all the available subcommands:
```
USAGE:
    pi-ctl [FLAGS] [OPTIONS] [SUBCOMMAND]

FLAGS:
    -D, --debug          Turn on debug output
    -h, --help           Prints help information
    -r, --reconfigure    (Re)configure your pihole servers
    -s, --show-config    Show the current config and exit
    -V, --version        Prints version information

OPTIONS:
    -c, --config <PATH>    The path to the config file [default:
                           /home/jay/.mpihole]

SUBCOMMANDS:
    10min_queries     Print the query data for the top N items
    all_queries       Print all queries
    disable           Disable the pihole servers
    enable            Enable the pihole servers
    forward_dests     Print the forward destination stats
    help              Prints this message or the help of the given
                      subcommand(s)
    query_types       Print the query type stats
    recent_blocked    Print the most recently blocked domain
    summary           Print a summary for each server
    top_clients       Print the query data for the top N clients
    top_items         Print the top N domains and advertisers
    type              Print the server type for each server
    version           Print the version for each server
```