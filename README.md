# transpo-rt

Simple API for public transport realtime data.

This API reads a public transport base schedule (a [GTFS](http://gtfs.org/)) and some realtime data (for the moment in [GTFS_RT](https://developers.google.com/transit/gtfs-realtime/)) to provide realtime feeds in [siri lite](http://www.normes-donnees-tc.org/format-dechange/donnees-temps-reel/).

### Using

A [hosted version](https://tr.transport.data.gouv.fr/) of this API, with some french datasets can be freely used.

The API provides several routes:

* `GET` `/`: list the available datasets - [example call](https://tr.transport.data.gouv.fr/)
* `GET` `/spec`: [OpenApi](https://www.openapis.org/) [v3](https://github.com/OAI/OpenAPI-Specification/blob/master/versions/3.0.2.md) description of this API - [example call](https://tr.transport.data.gouv.fr/spec)
* `GET` `/{id}/gtfs-rt`: get the gtfs-rt as binary - [example call](https://tr.transport.data.gouv.fr/horaires-theoriques-du-reseau-tag/gtfs-rt)
* `GET` `/{id}/gtfs-rt.json`: get the gtfs-rt as json - [example call](https://tr.transport.data.gouv.fr/horaires-theoriques-du-reseau-tag/gtfs-rt.json)
* `GET` `/{id}/siri/2.0/`: get the list of available siri-lite links [example call](https://tr.transport.data.gouv.fr/horaires-theoriques-du-reseau-tag/siri/2.0)
* `GET` `/{id}/siri/2.0/stop-monitoring.json`: get a siri-lite stop monitoring response - [example call](https://tr.transport.data.gouv.fr/horaires-theoriques-du-reseau-tag/siri/2.0/stop-monitoring.json?MonitoringRef=4235)
* `GET` `/{id}/siri/2.0/stoppoints-discovery.json`: get a siri-lite stoppoint discovery response - [example call](https://tr.transport.data.gouv.fr/horaires-theoriques-du-reseau-tag/siri/2.0/stoppoints-discovery.json?q=mairie)
* `GET` `/{id}/siri/2.0/general-message.json`: get a siri-lite general message response - [example call](https://tr.transport.data.gouv.fr/horaires-theoriques-du-reseau-tag/siri/2.0/general-message.json)
* `GET` `/{id}/`: simple status on the dataset - [example call](https://tr.transport.data.gouv.fr/horaires-theoriques-du-reseau-tag/)

#### API details

##### /siri/2.0/stop-monitoring.json

The API follows the [Siri-lite specification](http://www.chouette.mobi/irys/wp-content/uploads/20151023-Siri-Lite-Sp%C3%A9cification-Interfaces-V1.4.pdf) (documentation in french).

A formal description of the supported parameters and of the response can be seen in the [OpenAPI endpoint](https://tr.transport.data.gouv.fr/spec/).

##### /siri/2.0/stoppoints-discovery.json

The API follow the [Siri-lite specification](http://www.chouette.mobi/irys/wp-content/uploads/20151023-Siri-Lite-Sp%C3%A9cification-Interfaces-V1.4.pdf) (documentation in french).

A formal description of the supported parameters and of the response can be seen in the [OpenAPI endpoint](https://tr.transport.data.gouv.fr/spec/).

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

### Configuring & Running

You can check the needed cli parameters with the `-h` option:
```
cargo run --release -- -h
```

In particular, the application expects a config file from you, to define which datasets (GTFS + GTFS RT) should be handled.

This can be configured via a local file, or a HTTP url (see `-h` output for exact details).

An example configuration file can be found in [here](example_configuration_file.yml), which you can use like this (in debug mode):

```
cargo run -- -c example_configuration_file.yml
```

After a bit of time preparing the datasets, an url will appear in the logs (`http://localhost:8080`), showing which datasets are served.

Alternatively, you can use:

```
cargo run -- --gtfs <path or url of gtfs> --url <url fo the gtfs rt>
```

### Testing

You can run all the tests (unit test, integration, clippy and fmt) with:
```
make check
```

It will save you some time for the code review and continous integration ;)

### Manual testing

One useful trick to experiment locally is to [serve a local folder via HTTP](https://developer.mozilla.org/en-US/docs/Learn/Common_questions/set_up_a_local_testing_server#running_a_simple_local_http_server), using:

```
mkdir data
cd data
# here download files with curl or manually, then start server
python3 -m http.server
```

You can use the corresponding urls in a custom `.yml` configuration file. This makes it easier to simulate 404 errors for instance (by simply renaming the files).

## Architecture

The API has been made with [actix-web](https://github.com/actix/actix-web). This makes it possible to have a multithreaded API with data reloading and realtime updates without dataraces (thanks to [rust](https://www.rust-lang.org/)), nor mutexes (thanks to the [actix](https://github.com/actix/actix) [actor model](https://en.wikipedia.org/wiki/Actor_model)).

To achieve this there is an `Actor` in charge of the data (`DatasetActor`). The api workers query this actor to get the latest data. The `DatasetActor` does not return directly the data, but a `Arc` to them (a rust shared pointer).

In the background, 2 actors are in charge of periodic data reloading:
* `BaseScheduleReloader` reloads once in a while the baseschedule dataset
* `RealTimeReloader` reloads frequently the realtime dataset

Once the data (baseschedule or realtime) has been reloaded, it is send to the `DatasetActor` via a message. When the `DatasetActor` processes this message, it replaces it's `Arc` to this data, dropping the references. The API workers that have aquired an `Arc` to those data can continue their work on those data. The old data will be deleted when all workers have finished their work on them (thus noboby owns an `Arc` to those data anymore).
