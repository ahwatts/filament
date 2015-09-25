use hyper::client::{Client, Response};
use hyper::header::ContentLength;
use hyper::status::StatusCode;
use mogilefs_common::{MogError, MogResult};
use mogilefs_common::requests::{FileInfoResponse, GetPathsResponse};
use super::proxy::AlternateFileFinder;
use url::Url;

pub struct KeyUrlFinder {
    base_url: Url,
}

impl KeyUrlFinder {
    pub fn new(base_url: Url) -> KeyUrlFinder {
        KeyUrlFinder {
            base_url: base_url,
        }
    }

    fn url_for_key(&self, _domain: &str, key: &str) -> Url {
        let mut key_url = self.base_url.clone();
        let mut new_path = Vec::from(key_url.path().unwrap());
        new_path.extend(key.split("/").map(|s| s.to_string()));
        new_path.push("a.jpg".to_string());
        new_path = new_path.into_iter().filter(|p| p != "").collect();
        *key_url.path_mut().unwrap() = new_path;
        key_url
    }

    fn check_key_url(&self, domain: &str, key: &str) -> MogResult<Response> {
        let client = Client::new();
        // let url_str = format!("http://www.domain.com/{}/a.jpg", key);
        let url = self.url_for_key(domain, key);

        debug!("Attempting to find alternate file at {}", url);

        let response = try!{
            client.get(url).send().map_err(|e| {
                MogError::Other("alternate file error".to_string(), Some(format!("{}", e)))
            })
        };

        debug!("Alternate file response = {:?}", response);

        match response.status {
            StatusCode::Ok => Ok(response),
            c @ _ => Err(MogError::Other("alternate file error".to_string(), Some(format!("status code = {:?}", c)))),
        }
    }

}

impl AlternateFileFinder for KeyUrlFinder {
    fn file_info(&self, domain: &str, key: &str) -> MogResult<FileInfoResponse> {
        self.check_key_url(domain, key).map(|response| {
            FileInfoResponse {
                fid: 0,
                devcount: 1,
                length: response.headers.get::<ContentLength>().map(|clh| clh.0).unwrap_or(0),
                domain: domain.to_string(),
                class: "external".to_string(),
                key: key.to_string(),
            }
        })
    }

    fn get_paths(&self, domain: &str, key: &str) -> MogResult<GetPathsResponse> {
        self.check_key_url(domain, key).map(|_| {
            GetPathsResponse(vec![ self.url_for_key(domain, key) ])
        })
    }
}
