# multi-pihole
This is just a cli for running commands against multiple pihole servers.  This
is helpful if you have a primary, secondary, and any tertiary servers, but want
to manage them all at the same time.

## Configuring `pi-ctl`
This will happen automatically on a first run.  The JSON config file will be
saved at `~/.mpihole`.  You can modify that file directly, or you can run 
`pi-ctl -r` to re-configure, including adding/modifying/removing pihole servers.

## Commands
There are 3 different commands available: `enable`, `disable`, and `summary`.

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

### `summary`
This command will print out a pretty-printed JSON summary for each server.
Run this as `pi-ctl summary`.  The output will look like this:
```
Summary for http://pihole.example.com
{

  "ads_blocked_today": 10713,
  "ads_percentage_today": 0.748202,
  "clients_ever_seen": 54,
  "dns_queries_all_types": 1431832,
  "dns_queries_today": 1431832,
  "domains_being_blocked": 92988,
  "gravity_last_updated": {
    "absolute": 1621137966,
    "file_exists": true,
    "relative": {
      "days": 5,
      "hours": 15,
      "minutes": 58
    }
  },
  "privacy_level": 0,
  "queries_cached": 1317823,
  "queries_forwarded": 98663,
  "reply_CNAME": 857,
  "reply_IP": 182729,
  "reply_NODATA": 68578,
  "reply_NXDOMAIN": 472,
  "status": "enabled",
  "unique_clients": 54,
  "unique_domains": 2383
}
```