# ip-api-rust
An API for http://ip-api.com/ written in the Rust language.

Dual-licensed under [MIT](https://opensource.org/licenses/MIT) or the [UNLICENSE](http://unlicense.org).

## Features
This library lets you request information about an IP address. It uses `futures` to deliver the result.
```rust
pub struct Response {
    pub query: String,
    pub country: Option<NameAndCode>,  // NameAndCode { name: String, code: String }
    pub region: Option<NameAndCode>,
    pub city: Option<String>,
    pub zip: Option<String>,
    pub location: Option<Coordinates>,  // Cooridnates { latitude: f32, longitude: f32 }
    pub timezone: Option<String>,
    pub isp: Option<String>,
    pub organization: Option<String>,
    pub autonomous_system: Option<String>,
    pub reverse: Option<String>,
    pub mobile: bool,
    pub proxy: bool,
}
```

## Requirements
You need `tokio_core` in order to use this library, as the construction of `IpApi` requires an instance of `Handle`.
