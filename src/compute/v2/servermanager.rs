// Copyright 2017 Dmitry Tantsur <divius.inside@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Server management via Compute API.

use super::super::super::{ApiResult, Session, Sort};
use super::super::super::auth::Method as AuthMethod;
use super::super::super::service::Query;
use super::base::V2ServiceWrapper;
use super::protocol;


/// A request to list servers.
#[derive(Debug, Clone)]
pub struct ServerListRequest<'a, Auth: AuthMethod + 'a> {
    service: V2ServiceWrapper<'a, Auth>,
    /// Marker - ID of server to start listing from.
    pub marker: Option<String>,
    /// Limit on number of entities to return.
    ///
    /// If missing, the default number will be returned, which is not
    /// necessary all items.
    pub limit: Option<usize>,
    /// Sorting fields and directions.
    pub sort: Vec<Sort<String>>
}

/// Server manager: working with virtual servers.
///
/// # Examples
///
/// Listing summaries of all servers:
///
/// ```rust,no_run
/// use openstack;
///
/// let auth = openstack::auth::Identity::from_env()
///     .expect("Unable to authenticate");
/// let session = openstack::Session::new(auth);
/// let server_list = openstack::compute::v2::servers(&session).list()
///     .fetch().expect("Unable to fetch servers");
/// ```
///
/// Sorting servers by name:
///
/// ```rust,no_run
/// use openstack;
///
/// let auth = openstack::auth::Identity::from_env()
///     .expect("Unable to authenticate");
/// let session = openstack::Session::new(auth);
/// let server_list = openstack::compute::v2::servers(&session).list()
///     .sort_by(openstack::Sort::Asc("access_ip_v4")).with_limit(5)
///     .fetch().expect("Unable to fetch servers");
/// ```
///
/// Fetching server details by its UUID:
///
/// ```rust,no_run
/// use openstack;
///
/// let auth = openstack::auth::Identity::from_env()
///     .expect("Unable to authenticate");
/// let session = openstack::Session::new(auth);
/// let server = openstack::compute::v2::servers(&session)
///     .get("8a1c355b-2e1e-440a-8aa8-f272df72bc32")
///     .expect("Unable to get a server");
/// println!("Server name is {}", server.name());
/// ```
#[derive(Debug)]
pub struct ServerManager<'a, Auth: AuthMethod + 'a> {
    service: V2ServiceWrapper<'a, Auth>
}

/// Structure representing a summary of a single server.
#[derive(Debug)]
pub struct Server<'a, Auth: AuthMethod + 'a> {
    service: V2ServiceWrapper<'a, Auth>,
    inner: protocol::Server
}

/// Structure representing a summary of a single server.
#[derive(Debug)]
pub struct ServerSummary<'a, Auth: AuthMethod + 'a> {
    service: V2ServiceWrapper<'a, Auth>,
    inner: protocol::ServerSummary
}

/// List of servers.
pub type ServerList<'a, Auth> = Vec<ServerSummary<'a, Auth>>;


impl<'a, Auth: AuthMethod + 'a> Server<'a, Auth> {
    /// Get a reference to IPv4 address.
    pub fn access_ipv4(&self) -> &String {
        &self.inner.accessIPv4
    }

    /// Get a reference to IPv6 address.
    pub fn access_ipv6(&self) -> &String {
        &self.inner.accessIPv6
    }

    /// Get a reference to server unique ID.
    pub fn id(&self) -> &String {
        &self.inner.id
    }

    /// Get a reference to server name.
    pub fn name(&self) -> &String {
        &self.inner.name
    }

    /// Get server status.
    pub fn status(&self) -> &String {
        &self.inner.status
    }
}

impl<'a, Auth: AuthMethod + 'a> ServerSummary<'a, Auth> {
    /// Get a reference to server unique ID.
    pub fn id(&self) -> &String {
        &self.inner.id
    }

    /// Get a reference to server name.
    pub fn name(&self) -> &String {
        &self.inner.name
    }

    /// Get details.
    pub fn details(self) -> ApiResult<Server<'a, Auth>> {
        ServerManager::get_server(self.service.clone(), &self.inner.id)
    }
}

impl<'a, Auth: AuthMethod + 'a> ServerListRequest<'a, Auth> {
    fn new(service: V2ServiceWrapper<'a, Auth>)
            -> ServerListRequest<'a, Auth> {
        ServerListRequest {
            service: service,
            marker: None,
            limit: None,
            sort: Vec::new()
        }
    }

    /// Add marker to the request.
    pub fn with_marker<T: Into<String>>(self, marker: T) -> Self {
        ServerListRequest {
            marker: Some(marker.into()),
            .. self
        }
    }

    /// Add limit to the request.
    pub fn with_limit(self, limit: usize) -> Self {
        ServerListRequest {
            limit: Some(limit),
            .. self
        }
    }

    /// Add sorting to the request.
    pub fn sort_by<T: Into<String>>(mut self, sort: Sort<T>) -> Self {
        self.sort.push(match sort {
            Sort::Asc(v) => Sort::Asc(v.into()),
            Sort::Desc(v) => Sort::Desc(v.into())
        });
        self
    }

    /// Execute this request and return its result.
    #[allow(unused_results)]
    pub fn fetch(self) -> ApiResult<ServerList<'a, Auth>> {
        let service = self.service;
        let mut query = Query::new();
        if let Some(marker) = self.marker {
            query.push("marker", marker);
        }
        if let Some(limit) = self.limit {
            query.push("limit", limit);
        }
        for sort in self.sort {
            let (field, direction) = sort.into();
            query.push("sort_key", field);
            query.push("sort_dir", direction);
        }

        trace!("Listing all compute servers");
        let inner: protocol::ServersRoot = try!(
            service.http_get(&["servers"], query)
        );
        debug!("Received {} compute servers", inner.servers.len());
        trace!("Received servers: {:?}", inner.servers);
        Ok(inner.servers.into_iter().map(|x| ServerSummary {
            service: service.clone(),
            inner: x
        }).collect())
    }
}

impl<'a, Auth: AuthMethod + 'a> ServerManager<'a, Auth> {
    /// Constructor for server manager.
    pub fn new(session: &'a Session<Auth>) -> ServerManager<'a, Auth> {
        ServerManager {
            service: V2ServiceWrapper::new(session)
        }
    }

    /// List servers.
    ///
    /// Note that this method does not return results immediately, but rather
    /// a [ServerListRequest](struct.ServerListRequest.html) object that
    /// you can futher specify with e.g. filtering or sorting.
    pub fn list(&self) -> ServerListRequest<'a, Auth> {
        ServerListRequest::new(self.service.clone())
    }

    /// Get a server.
    pub fn get<Id: AsRef<str>>(&self, id: Id) -> ApiResult<Server<'a, Auth>> {
        ServerManager::get_server(self.service.clone(), id.as_ref())
    }

    fn get_server(service: V2ServiceWrapper<'a, Auth>, id: &str)
            -> ApiResult<Server<'a, Auth>> {
        trace!("Get compute server {}", id);
        let inner: protocol::ServerRoot = try!(
            service.http_get(&["servers", id], Query::new())
        );
        trace!("Received {:?}", inner.server);
        Ok(Server {
            service: service,
            inner: inner.server
        })
    }
}

/// Create a server manager.
pub fn servers<'session, Auth>(session: &'session Session<Auth>)
        -> ServerManager<'session, Auth> where Auth: AuthMethod {
    ServerManager::new(session)
}


#[cfg(test)]
pub mod test {
    #![allow(missing_debug_implementations)]
    #![allow(unused_results)]

    use hyper;

    use super::super::super::super::auth::{NoAuth, SimpleToken};
    use super::super::super::super::session::test;
    use super::super::base::test as api_test;
    use super::ServerManager;

    const SERVERS_RESPONSE: &'static str = r#"
    {
        "servers": [
            {
                "id": "22c91117-08de-4894-9aa9-6ef382400985",
                "links": [
                    {
                        "href": "http://openstack.example.com/v2/6f70656e737461636b20342065766572/servers/22c91117-08de-4894-9aa9-6ef382400985",
                        "rel": "self"
                    },
                    {
                        "href": "http://openstack.example.com/6f70656e737461636b20342065766572/servers/22c91117-08de-4894-9aa9-6ef382400985",
                        "rel": "bookmark"
                    }
                ],
                "name": "new-server-test"
            }
        ]
    }"#;

    mock_connector_in_order!(MockServers {
        String::from("HTTP/1.1 200 OK\r\nServer: Mock.Mock\r\n\
                     \r\n") + api_test::ONE_VERSION_RESPONSE
        String::from("HTTP/1.1 200 OK\r\nServer: Mock.Mock\r\n\
                     \r\n") + SERVERS_RESPONSE
    });

    #[test]
    fn test_servers_list() {
        let auth = NoAuth::new("http://127.0.2.1/v2.1").unwrap();
        let cli = hyper::Client::with_connector(MockServers::default());
        let token = SimpleToken(String::from("abcdef"));
        let session = test::new_with_params(auth, cli, token, None);

        let mgr = ServerManager::new(&session);
        let srvs = mgr.list().fetch().unwrap();
        assert_eq!(srvs.len(), 1);
        assert_eq!(srvs[0].id(),
                   "22c91117-08de-4894-9aa9-6ef382400985");
        assert_eq!(srvs[0].name(), "new-server-test");
    }
}