//! An API for http://ip-api.com/ written in the Rust language.
//!
//! This library lets you request information about an IP address.
//! It uses `futures` to deliver the result.
//!
//! You will need the `tokio_core` create in order to use this library.
//! # Examples
//!
//! ```
//! # extern crate futures;
//! # extern crate tokio_core;
//! # extern crate ip_api;
//! use std::net::IpAddr;
//! use tokio_core::reactor::Core;
//! use futures::future::Future;
//! use ip_api::IpApi;
//!
//! # #[allow(unused_variables)]
//! # fn main() {
//! let mut core = Core::new().unwrap();
//! let ip_api = IpApi::new();
//! let ip: IpAddr = "8.8.8.8".parse().unwrap();
//! let future = ip_api.request(Some(ip))
//!     .map(|result| {
//!         println!("{:?}", result);
//!     });
//!
//! core.run(future).unwrap();
//! # }
//! ```

#![warn(missing_docs)]

extern crate hyper;
#[macro_use]
extern crate error_chain;
extern crate serde_json;
extern crate futures;

use std::net::IpAddr;
use futures::Future;
use futures::Stream;
use hyper::Client;
use hyper::Uri;
use hyper::client::HttpConnector;
use serde_json::Value;

/// The successful result of an `IpApi::request` call.
#[derive(Debug, PartialEq)]
pub struct Response {
    /// IP used for the query
    pub query: String,
    /// The country the IP is thought to be in
    pub country: Option<NameAndCode>,
    /// The region the IP is thought to be in
    pub region: Option<NameAndCode>,
    /// The city the IP is thought to be in
    pub city: Option<String>,
    /// The ZIP code the IP is thought to have
    pub zip: Option<String>,
    /// The predicted location of this IP
    pub location: Option<Coordinates>,
    /// City timezone
    pub timezone: Option<String>,
    /// The name of the Internet Service Provider
    pub isp: Option<String>,
    /// The organization that is thought to own this IP.
    pub organization: Option<String>,
    /// Autonomous system number (ASN) and name, separated by space
    pub autonomous_system: Option<String>,
    /// Reverse DNS of the IP
    pub reverse: Option<String>,
    /// Mobile (cellular) connection
    pub mobile: bool,
    /// Proxy (anonymous)
    pub proxy: bool,
}

/// Used to pair an abbreviation with a name
#[derive(Debug, PartialEq)]
pub struct NameAndCode {
    /// The name
    pub name: String,
    /// The abbreviation
    pub code: String,
}

/// Used to store geographic coordinates
#[derive(Debug, PartialEq)]
pub struct Coordinates {
    /// The latitude
    pub latitude: f32,
    /// The longitude
    pub longitude: f32,
}

#[allow(missing_docs)]
mod error {
    use super::*;

    error_chain! {
        foreign_links {
            HyperError(hyper::Error);
            SerdeJsonError(serde_json::Error);
            FromUtf8Error(std::string::FromUtf8Error);
        }
    }
}

pub use error::*;

/// The `struct` used to request information about IP addresses.
///
/// # Examples
///
/// ```
/// # extern crate tokio_core;
/// # extern crate ip_api;
/// use tokio_core::reactor::Core;
/// use ip_api::IpApi;
///
/// # #[allow(unused_variables)]
/// # fn main() {
/// let core = Core::new().unwrap();
/// let ip_api = IpApi::new();
/// # }
/// ```
pub struct IpApi {
    client: Client<HttpConnector>,
}

impl IpApi {
    /// Constructs a new `IpApi`.
    pub fn new() -> Self {
        IpApi {
            client: Client::new(),
        }
    }

    /// Requests information about the provided IP address.
    /// If no IP address is provided, the external IP address of the host machine is used.
    pub fn request<'a>(&'a self, ip: Option<IpAddr>) -> impl Future<Item=Response, Error=Error> + 'a {
        let ip_string = ip.map(|ip| "/".to_owned() + &ip.to_string())
            .unwrap_or("".to_owned());
        let uri = (&("http://ip-api.com/json".to_owned() + &ip_string)).parse::<Uri>()
            .expect("Could not create the ip-api request URL.
                    \nThis is an implementation error, please report it to the authors.");

        self.client.get(uri)
            .and_then(|response| {
                response.into_body()
                    .map(|chunk| chunk.to_vec())
                    .collect()
                    .map(|vec| vec.concat())
            })
            .map_err(Error::from)
            .and_then(|data| {
                String::from_utf8(data)
                    .map_err(Error::from)
            })
            .and_then(|response_string| {
                serde_json::from_str::<Value>(&response_string)
                    .map_err(Error::from)
            })
            .map(move |json| {
                Response {
                    query: get_string(&json, "query")
                        .expect("The queried IP was not in the response."),
                    country: get_name_and_code(&json, "country", "countryCode"),
                    region: get_name_and_code(&json, "regionName", "region"),
                    city: get_string(&json, "city"),
                    zip: get_string(&json, "zip"),
                    location: get_coordinates(&json, "lat", "lon"),
                    timezone: get_string(&json, "timezone"),
                    isp: get_string(&json, "isp"),
                    organization: get_string(&json, "org"),
                    autonomous_system: get_string(&json, "as"),
                    reverse: get_string(&json, "reverse"),
                    mobile: get_bool(&json, "mobile"),
                    proxy: get_bool(&json, "proxy"),
                }
            })
    }
}

fn get_coordinates(json: &Value, latitude_index: &str, longitude_index: &str) -> Option<Coordinates> {
    if let (Some(latitude), Some(longitude)) = (
        json.get(latitude_index).and_then(|arg| arg.as_f64()),
        json.get(longitude_index).and_then(|arg| arg.as_f64())
    ) {
        Some(Coordinates {
            latitude: latitude as f32,
            longitude: longitude as f32,
        })
    } else {
        None
    }
}

fn get_name_and_code(json: &Value, name_index: &str, code_index: &str) -> Option<NameAndCode> {
    if let (Some(name), Some(code)) = (
        get_string(json, name_index),
        get_string(json, code_index)
    ) {
        Some(NameAndCode {
            name: name,
            code: code,
        })
    } else {
        None
    }
}

fn get_bool(json: &Value, index: &str) -> bool {
    json.get(index).and_then(|arg| arg.as_bool()).unwrap_or(false)
}

fn get_string(json: &Value, index: &str) -> Option<String> {
    json.get(index).and_then(|arg| arg.as_str()).map(|arg| arg.to_owned())
}

#[cfg(test)]
mod tests {
    extern crate tokio_core;

    use super::*;
    use tests::tokio_core::reactor::Core;

    #[test]
    fn it_works() {
        let expected = Response {
            query: "8.8.8.8".to_owned(),
            country: Some(NameAndCode { name: "United States".to_owned(), code: "US".to_owned() }),
            region: Some(NameAndCode { name: "California".to_owned(), code: "CA".to_owned() }),
            city: Some("Mountain View".to_owned()),
            zip: Some("".to_owned()),
            location: Some(Coordinates { latitude: 37.4229, longitude: -122.085 }),
            timezone: Some("America/Los_Angeles".to_owned()),
            isp: Some("Google".to_owned()),
            organization: Some("Google".to_owned()),
            autonomous_system: Some("AS15169 Google LLC".to_owned()),
            reverse: None,
            mobile: false,
            proxy: false
        };
        let mut core = Core::new().unwrap();
        let ip_api = IpApi::new();
        let future = ip_api.request(Some("8.8.8.8".parse().unwrap()))
            .map(|result| {
                assert_eq!(result, expected);
            });

        core.run(future).unwrap();
    }
}
