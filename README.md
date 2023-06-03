# ringhash

[![Crates.io][crates-badge]][crates-url]
[![License][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/ringhash.svg
[crates-url]: https://crates.io/crates/ringhash
[license-badge]: https://img.shields.io/crates/l/ringhash.svg
[license-url]: #license
[actions-badge]: https://github.com/Millione/ringhash/actions/workflows/ci.yaml/badge.svg
[actions-url]: https://github.com/Millione/ringhash/actions

Consistent hashing implementation.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
ringhash = "0.1"
```

## Example
```rust
use ringhash::Consistent;

fn main() {
    let c = Consistent::new();
    c.add("cacheA");
    c.add("cacheB");
    c.add("cacheC");
    let users = vec![
        "user_mcnulty",
        "user_bunk",
        "user_omar",
        "user_bunny",
        "user_stringer",
    ];
    println!("initial state [A, B, C]");
    for u in users.iter() {
        let server = c.get(u).unwrap();
        println!("{} => {}", u, server);
    }
    c.add("cacheD");
    c.add("cacheE");
    println!("with cacheD, cacheE added [A, B, C, D, E]");
    for u in users.iter() {
        let server = c.get(u).unwrap();
        println!("{} => {}", u, server);
    }
    c.remove("cacheC");
    println!("with cacheC removed [A, B, D, E]");
    for u in users.iter() {
        let server = c.get(u).unwrap();
        println!("{} => {}", u, server);
    }
}
```

## License

Dual-licensed under the MIT license and the Apache License (Version 2.0).

See [LICENSE-MIT](https://github.com/Millione/ringhash/blob/main/LICENSE-MIT) and [LICENSE-APACHE](https://github.com/Millione/ringhash/blob/main/LICENSE-APACHE) for details.
