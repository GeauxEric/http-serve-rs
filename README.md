# Http Server
To server static content from local files. Similar to python's [http.server](https://docs.python.org/3/library/http.server.html).
This project is mostly for educational purpose.

# Installation
```shell
cargo install http-serve-rs
```

# Usage
```shell
RUST_LOG=tower_http=debug http-serve-rs --port 3000
```
