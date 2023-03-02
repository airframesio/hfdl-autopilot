# hfdl-autopilot
Dynamic `dumphfdl` wrapper that changes listening frequencies based off of HFDL activity.

### TODO
- [ ] Front-end UI 
- [x] Derive bands from SPDU via provided system table config instead of current shortcut.
- [x] Allow more `dumphfdl` command line argument passthrough
- [ ] <strike>Allow more arguments to be configurable via environment variables</strike>
- [ ] Use Airframes SPDU API to immediately hone into active frequencies
- [x] Use "heard from" data from aircraft's HFNPDU messages

### Building
First, install a stable [Rust](https://www.rust-lang.org/learn/get-started) toolchain. Make sure the `cargo` command is in `PATH` environment variable after completion. 

Second, clone this repository:
```
  git clone https://github.com/airframesio/hfdl-autopilot
```

Third, compile and build `hfdl-autopilot`:
```
cargo build --release  
```

### Example
```
hfdl-autopilot --bin /usr/local/bin/dumphfdl --sys-table /usr/local/etc/systable.json -v --port 7270 --chooser tracker:target=Albrook --timeout 150 -- --soapysdr driver=airspyhf --output decoded:json:tcp:address=feed.airframes.io,port=5556
```

### Modes
#### `schedule`
Schedule when band changes should occur.
```
--chooser schedule:7:00=21,19:00=8
```

#### `single`
Only stay within a single change and never change. This is the same as running `dumphfdl` normally. The only advantage this offers is the automatic grouping of frequencies within a 256-384 KHz "bands".
```
--chooser single:band=13
```

#### `rotate`
Change bands once inactivity timeout invokes.

Valid `type`s:
* `inc` - on timeout, move on to the next highest band; if already on the highest band, move to the lowest in the list
* `dec` - on timeout, move on to the next lowest band; if already on the lowest band, move to the highest in the list
* `random` - on timeout, choose a random band that we haven't visited in the last 6 sessions
```
--chooser rotate:type=random,start=21,ignore_last=8,prefer=21@7:00/10@19:00
```
#### `tracker`
Track messages to/from a specific ground station. Move on to a new band if inactivity timeout occurs or we haven't heard a message to/from target for `timeout` seconds.
```
--chooser tracker:target=Agana,last_heard_timeout=600
```

### Web API
By default, `hfdl-autopilot` will expose a simple REST API on port 7270. This API allows users to query session state information such as flight position reports (via HFDL link layer), latest ground stations frequencies, and message statistics.
* `/api/ground-stations`
* `/api/ground-station/stats`
* `/api/freq-stats`
* `/api/flights`
* `/api/flight/{CALLSIGN}`
* `/api/session`
