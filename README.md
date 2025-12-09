# RubHub

Federated forge without fancy protocols, only git!

## Why RubHub?

* **Git as the single source of truth**
  Everything is stored within git, this greatly simplifies running it and also
  allows for federation, an upstream is just another remote!

* **Fast**
  RubHub is written in Rust and compiles down to a single executable, this provides
  you with a nice web interface and also handles SSH directly, so you don't need to
  change your system ssh config (you might want to change the port, but that's for you to decide).

* **Efficient**
  We don't use more system resources than absolutely necessary, the production system running
  rubhub.net for example only uses **8 MB** even under load. CPU usage is also quite low, going
  down to 0 when idle.

## Status

Right now RubHub is still in early development, so I'd not depend on it working correcly yet, still in
that early phase before the entire design has come together, so core aspects will still change. If you're
interested in the idea though and would like to contribute then I'd love to hear from you!

## License

RubHub uses the [EUPL 1.2 License](https://eupl.eu/1.2/en)