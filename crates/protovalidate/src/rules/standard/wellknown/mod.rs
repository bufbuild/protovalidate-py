// Copyright (c) 2023-2026 Buf Technologies, Inc.
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

//! The well-known string formats: what protovalidate's CEL functions
//! (`isEmail`, `isHostname`, `isIp`, `isIpPrefix`, `isUri`, `isUriRef`,
//! `isHostAndPort`) and the regex-defined formats (UUID, ULID, Protobuf
//! names, HTTP header names and values) accept.

mod host;
mod ip;
mod text;
mod uri;

pub(crate) use host::{is_email, is_host_and_port, is_hostname};
pub(crate) use ip::{is_ip, is_ip_prefix, is_ipv4, is_ipv4_prefix, is_ipv6, is_ipv6_prefix};
pub(crate) use text::{
    is_header_name, is_header_value, is_protobuf_dot_fqn, is_protobuf_fqn, is_tuuid, is_ulid,
    is_uuid,
};
pub(crate) use uri::{is_uri, is_uri_ref};
