# Spacegate: SpacetimeDB UDP Proxy

## What is Spacegate:

Spacegate is a 2 part proxy that enables UDP data transfer via the [QUIC](https://en.wikipedia.org/wiki/QUIC) protocol to the SpacetimeDB maincloud servers.

## How does Spacegate work:

![proxy diagram](images/diagram.webp "Spacegate flow Diagram")

Spacegate itself is a local application meant to run alongside your SpacetimeDB client. You connect to it instead of the maincloud URI and it will proxy your connection using UDP to a forward proxy hosted physically close to the SpacetimeDB servers. From there, the forward proxy server routes your data to and from maincloud.

I personally run the forward proxy as a shared resource for those that want to use it.

## How to use Spacegate:

### Building:

To build the clientside proxy, with `rust` installed:

```bash
cargo build -r -p spacegate
```

### Releases:

Alternatively to building yourself, you can simply download a prebuild binary from the [releases](https://github.com/suirad/spacegate/releases) page of this repository.

### Usage:

1. Run the `spacegate` application. Once you see a printout that the proxy connection is established it is ready to use.
  * Can be run within or outside of a terminal; though within a terminal will show any errors that occur.
2. In your StDB client, change your maincloud connection uri from `wss://maincloud.spacetimedb.com` to `ws://localhost:3001`.
  * If your application stores maincloud tokens, clear them to be replaced by new ones from the proxy.
    * Unity: `edit -> clear player prefs`
    * Typescript: clear local storage
3. Enjoy!

### Additional notes:

* Using `spacetime cli` with `spacegate`:
  ```bash
  spacetime server add --url http://localhost:3001 spacegate
  spacetime server set-default spacegate
  ```
