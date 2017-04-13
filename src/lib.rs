#![feature(conservative_impl_trait)]

extern crate hyper;
#[macro_use]
extern crate error_chain;
extern crate serde_json;
extern crate futures;
extern crate tokio_core;

use tokio_core::reactor::Handle;
use std::net::IpAddr;
use futures::Future;
use futures::Stream;
use hyper::Client;
use hyper::Uri;
use hyper::client::HttpConnector;
use serde_json::Value;

#[derive(Debug, PartialEq)]
pub struct Response {
    pub query: String,
    pub country: Option<NameAndCode>,
    pub region: Option<NameAndCode>,
    pub city: Option<String>,
    pub zip: Option<String>,
    pub location: Option<Coordinates>,
    pub timezone: Option<String>,
    pub isp: Option<String>,
    pub organization: Option<String>,
    pub autonomous_system: Option<String>,
    pub reverse: Option<String>,
    pub mobile: bool,
    pub proxy: bool,
}

#[derive(Debug, PartialEq)]
pub struct NameAndCode {
    pub name: String,
    pub code: String,
}

#[derive(Debug, PartialEq)]
pub struct Coordinates {
    pub latitude: f32,
    pub longitude: f32,
}

error_chain! {
    foreign_links {
        HyperError(hyper::Error);
        SerdeJsonError(serde_json::Error);
        FromUtf8Error(std::string::FromUtf8Error);
    }
}

pub struct IpApi {
    client: Client<HttpConnector>,
}

impl IpApi {
    pub fn new(handle: Handle) -> Self {
        IpApi {
            client: Client::new(&handle),
        }
    }

    pub fn request<'a>(&'a self, ip: IpAddr) -> impl Future<Item=Response, Error=Error> + 'a {
        let ip_string = ip.to_string();
        let uri = (&("http://ip-api.com/json/".to_owned() + &ip_string)).parse::<Uri>()
            .expect("Could not create the ip-api request URL.
                    \nThis is an implementation error, please report it to the authors.");

        self.client.get(uri)
            .and_then(|response| {
                response.body()
                    .map(|chunk| chunk.to_vec())
                    .collect()
                    .map(|vec| vec.concat())
            })
            .map_err(|err| Error::from(err))
            .and_then(|data| {
                String::from_utf8(data)
                    .map_err(|err| Error::from(err))
            })
            .and_then(|response_string| {
                serde_json::from_str::<Value>(&response_string)
                    .map_err(|err| Error::from(err))
            })
            .map(move |json| {
                Response {
                    query: ip_string,
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
    use super::*;
    use std::net::Ipv4Addr;
    use tokio_core::reactor::Core;

    #[test]
    fn it_works() {
        let expected = Response {
            query: "8.8.8.8".to_owned(),
            country: Some(NameAndCode { name: "United States".to_owned(), code: "US".to_owned() }),
            region: Some(NameAndCode { name: "California".to_owned(), code: "CA".to_owned() }),
            city: Some("Mountain View".to_owned()),
            zip: Some("94035".to_owned()),
            location: Some(Coordinates { latitude: 37.386, longitude: -122.0838 }),
            timezone: Some("America/Los_Angeles".to_owned()),
            isp: Some("Google".to_owned()),
            organization: Some("Google".to_owned()),
            autonomous_system: Some("AS15169 Google Inc.".to_owned()),
            reverse: None,
            mobile: false,
            proxy: false
        };
        let mut core = Core::new().unwrap();
        let handle = core.handle();
        let ip_api = IpApi::new(handle);
        let future = ip_api.request(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)))
            .map(|result| {
                assert_eq!(result, expected);

                result
            });

        core.run(future).unwrap();
    }
}
