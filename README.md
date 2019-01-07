# transpo-rt
Simple API for public transport realtime data

## Building

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

## Running

You can check the needed cli parameters with the `-h` option:
```
cargo run --release -- -h
```

## Developping

You can run all the tests (unit test, integration, clippy and fmt) with:
```
make check
```

It will save you some time for the code review and continous integration ;)
