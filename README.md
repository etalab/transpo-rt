# transpo-rt

Simple API for public transport realtime data.

This API reads a public transport base schedule (a [GTFS](http://gtfs.org/)) and some realtime data (for the moment in [GTFS_RT](https://developers.google.com/transit/gtfs-realtime/) to provide realtime feeds in [siri lite](http://www.normes-donnees-tc.org/format-dechange/donnees-temps-reel/).

### Using

A [hosted version](https://app-be8e53a7-9b77-4f95-bea0-681b97077017.cleverapps.io/) of this API, with some french datasets can be freely used.

The API provides several routes:

* `GET` `/`: list the available datasets - [example call](https://app-be8e53a7-9b77-4f95-bea0-681b97077017.cleverapps.io/)
* `GET` `/{id}/gtfs_rt`: get the gtfs-rt as binary - [example call](https://app-be8e53a7-9b77-4f95-bea0-681b97077017.cleverapps.io/metromobilite/gtfs_rt)
* `GET` `/{id}/gtfs_rt.json`: get the gtfs-rt as json - [example call](https://app-be8e53a7-9b77-4f95-bea0-681b97077017.cleverapps.io/metromobilite/gtfs_rt.json)
* `GET` `/{id}/siri-lite/stop_monitoring.json`: get a siri-lite stop monitoring response - [example call](https://app-be8e53a7-9b77-4f95-bea0-681b97077017.cleverapps.io/metromobilite/siri-lite/stoppoints_discovery.json?q=mairie)
* `GET` `/{id}/siri-lite/stoppoints_discovery.json`: get a siri-lite stoppoint discovery response - [example call](https://app-be8e53a7-9b77-4f95-bea0-681b97077017.cleverapps.io/metromobilite/siri-lite/stop_monitoring.json?MonitoringRef=4235)
* `GET` `/{id}/`: simple status on the dataset - [example call](https://app-be8e53a7-9b77-4f95-bea0-681b97077017.cleverapps.io/metromobilite/)

#### API details

##### /siri/stop_monitoring.json

The API follow the [Siri-lite specification](http://www.chouette.mobi/irys/wp-content/uploads/20151023-Siri-Lite-Sp%C3%A9cification-Interfaces-V1.4.pdf) (documentation in french).

TODO document supported parameters

##### /siri/stoppoints_discovery.json

The API follow the [Siri-lite specification](http://www.chouette.mobi/irys/wp-content/uploads/20151023-Siri-Lite-Sp%C3%A9cification-Interfaces-V1.4.pdf) (documentation in french).
TODO document supported parameters

## Developping

### Building

To build the api, you need an up to date Rust version:

If you don't already have it, install Rust:
```
curl https://sh.rustup.rs -sSf | sh
```

Or update it:
```
rustup update
```

Then you can build it:
```
cargo build
```

### Running

You can check the needed cli parameters with the `-h` option:
```
cargo run --release -- -h
```

### Testing

You can run all the tests (unit test, integration, clippy and fmt) with:
```
make check
```

It will save you some time for the code review and continous integration ;)

## Architecture

The API has been made with [actix-web](https://github.com/actix/actix-web). This makes it possible to have a multithreaded API with data reloading and realtime updates without dataraces (thanks to [rust](https://www.rust-lang.org/)), nor mutexes (thanks to the [actix](https://github.com/actix/actix) [actor model](https://en.wikipedia.org/wiki/Actor_model)).

To achieve this there is an `Actor` in charge of the data (`DatasetActor`). The api workers query this actor to get the latest data. The `DatasetActor` does not return directly the data, but a `Arc` to them (a rust shared pointer).

In the background, 2 actors are in charge of periodic data reloading:
* `BaseScheduleReloader` reloads once in a while the baseschedule dataset
* `RealTimeReloader` reloads frequently the realtime dataset

Once the data (baseschedule or realtime) has been reloaded, it is send to the `DatasetActor` via a message. When the `DatasetActor` processes this message, it replaces it's `Arc` to this data, dropping the references. The API workers that have aquired an `Arc` to those data can continue their work on those data. The old data will be deleted when all workers have finished their work on them (thus noboby owns an `Arc` to those data anymore).
