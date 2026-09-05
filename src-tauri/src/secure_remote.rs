use crate::phase3::RemoteEvidence;
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::{
    CONTENT_LENGTH, CONTENT_RANGE, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED,
    LOCATION, RANGE,
};
use reqwest::StatusCode;
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs},
    path::Path,
    time::Duration,
};
use url::Url;

const MAX_REDIRECTS: usize = 5;

#[derive(Debug)]
struct RemoteTarget {
    download_id: i64,
    url: String,
    previous_etag: Option<String>,
    previous_last_modified: Option<String>,
    previous_content_length: Option<i64>,
}

#[derive(Debug)]
struct RemoteOutcome {
    method: String,
    final_url: Option<String>,
    http_status: Option<i64>,
    result_state: String,
    etag: Option<String>,
    last_modified: Option<String>,
    content_length: Option<i64>,
    evidence: String,
    error: Option<String>,
}

pub fn check_remote_freshness(path: &Path, download_id: i64) -> Result<RemoteEvidence, String> {
    crate::phase3::initialize_database(path)?;
    let target = load_remote_target(path, download_id)?;
    let outcome = perform_remote_check(&target);
    persist_remote_outcome(path, &target, &outcome)
}

fn load_remote_target(path: &Path, download_id: i64) -> Result<RemoteTarget, String> {
    let connection = Connection::open(path).map_err(|error| error.to_string())?;

    let record: Option<(Option<String>, Option<i64>, Option<i64>)> = connection
        .query_row(
            "SELECT source_identity, version_number, duplicate_of_id FROM downloads WHERE id = ?1",
            [download_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let (source_identity, version_number, duplicate_of_id) =
        record.ok_or_else(|| format!("Download record #{download_id} does not exist"))?;

    if duplicate_of_id.is_some() {
        return Err(
            "Remote freshness is checked on the primary version, not an exact duplicate".into(),
        );
    }
    let url = source_identity
        .ok_or_else(|| "This download has no canonical HTTP(S) source identity".to_string())?;
    let version_number = version_number
        .ok_or_else(|| "This download has no deterministic version number".to_string())?;

    let latest_version: Option<i64> = connection
        .query_row(
            "SELECT MAX(version_number) FROM downloads WHERE source_identity = ?1 AND duplicate_of_id IS NULL",
            [&url],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if latest_version != Some(version_number) {
        return Err(
            "Remote freshness is checked only against the latest primary version in a source family"
                .into(),
        );
    }

    let parsed = Url::parse(&url).map_err(|error| format!("Invalid source identity: {error}"))?;
    validate_public_http_url(&parsed)?;

    let previous: Option<(Option<String>, Option<String>, Option<i64>)> = connection
        .query_row(
            r#"
            SELECT etag, last_modified, content_length
            FROM remote_checks
            WHERE download_id = ?1
            ORDER BY id DESC
            LIMIT 1
            "#,
            [download_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let (previous_etag, previous_last_modified, previous_content_length) =
        previous.unwrap_or((None, None, None));

    Ok(RemoteTarget {
        download_id,
        url,
        previous_etag,
        previous_last_modified,
        previous_content_length,
    })
}

fn perform_remote_check(target: &RemoteTarget) -> RemoteOutcome {
    match send_validated_request(target, false) {
        Ok(response)
            if response.status() == StatusCode::METHOD_NOT_ALLOWED
                || response.status() == StatusCode::NOT_IMPLEMENTED =>
        {
            match send_validated_request(target, true) {
                Ok(response) => outcome_from_response(response, target, "GET_RANGE", true),
                Err(error) => network_failure(target, "GET_RANGE", error),
            }
        }
        Ok(response) => outcome_from_response(response, target, "HEAD", false),
        Err(error) => network_failure(target, "HEAD", error),
    }
}

fn send_validated_request(target: &RemoteTarget, ranged: bool) -> Result<Response, String> {
    let mut current = Url::parse(&target.url).map_err(|error| error.to_string())?;

    for hop in 0..=MAX_REDIRECTS {
        let client = client_for_public_url(&current)?;
        let builder = if ranged {
            client.get(current.clone()).header(RANGE, "bytes=0-0")
        } else {
            client.head(current.clone())
        };
        let response = conditional_request(builder, target)
            .send()
            .map_err(|error| format!("Remote request failed: {error}"))?;

        if is_followable_redirect(response.status()) {
            if hop == MAX_REDIRECTS {
                return Err(format!(
                    "Remote source exceeded the {MAX_REDIRECTS}-redirect safety limit"
                ));
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| "Redirect response did not contain a valid Location header".to_string())?;
            let next = current
                .join(location)
                .map_err(|error| format!("Invalid redirect target: {error}"))?;
            validate_public_http_url(&next)?;
            current = next;
            continue;
        }

        return Ok(response);
    }

    Err("Remote redirect processing ended unexpectedly".into())
}

fn client_for_public_url(url: &Url) -> Result<Client, String> {
    validate_public_http_url(url)?;
    let host = url
        .host_str()
        .ok_or_else(|| "Remote source has no hostname".to_string())?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| "Remote source has no usable network port".to_string())?;
    let addresses = resolve_public_addresses(host, port)?;

    let mut builder = Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("OriginKeep/0.1 (+local freshness check)");

    if host.parse::<IpAddr>().is_err() {
        builder = builder.resolve_to_addrs(host, &addresses);
    }

    builder.build().map_err(|error| error.to_string())
}

fn validate_public_http_url(url: &Url) -> Result<(), String> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err("Remote freshness supports only HTTP(S) sources".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("Remote freshness refuses URLs containing embedded credentials".into());
    }
    let host = url
        .host_str()
        .ok_or_else(|| "Remote source has no hostname".to_string())?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| "Remote source has no usable network port".to_string())?;
    resolve_public_addresses(host, port).map(|_| ())
}

fn resolve_public_addresses(host: &str, port: u16) -> Result<Vec<SocketAddr>, String> {
    let addresses = if let Ok(ip) = host.parse::<IpAddr>() {
        vec![SocketAddr::new(ip, port)]
    } else {
        (host, port)
            .to_socket_addrs()
            .map_err(|error| format!("Could not resolve remote hostname {host}: {error}"))?
            .collect::<Vec<_>>()
    };

    if addresses.is_empty() {
        return Err(format!("Remote hostname {host} did not resolve to an address"));
    }

    let mut unique = HashSet::new();
    let mut public = Vec::new();
    for address in addresses {
        if !is_public_ip(address.ip()) {
            return Err(format!(
                "Remote freshness blocked non-public destination {}",
                address.ip()
            ));
        }
        if unique.insert(address) {
            public.push(address);
        }
    }
    Ok(public)
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => is_public_ipv6(ip),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(a == 0
        || a == 10
        || a == 127
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 168)
        || (a == 192 && b == 0 && c == 0)
        || (a == 192 && b == 0 && c == 2)
        || (a == 192 && b == 88 && c == 99)
        || (a == 198 && (b == 18 || b == 19))
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113)
        || a >= 224)
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    if ip.is_unspecified() || ip.is_loopback() || ip.is_multicast() {
        return false;
    }
    let segments = ip.segments();
    if segments[0] & 0xfe00 == 0xfc00
        || segments[0] & 0xffc0 == 0xfe80
        || segments[0] & 0xffc0 == 0xfec0
        || (segments[0] == 0x2001 && segments[1] == 0x0db8)
    {
        return false;
    }

    let embedded_ipv4 = segments[0..5].iter().all(|segment| *segment == 0)
        && (segments[5] == 0 || segments[5] == 0xffff);
    !embedded_ipv4
}

fn is_followable_redirect(status: StatusCode) -> bool {
    matches!(status.as_u16(), 301 | 302 | 303 | 307 | 308)
}

fn conditional_request(builder: RequestBuilder, target: &RemoteTarget) -> RequestBuilder {
    let builder = if let Some(etag) = target.previous_etag.as_deref() {
        builder.header(IF_NONE_MATCH, etag)
    } else {
        builder
    };
    if let Some(last_modified) = target.previous_last_modified.as_deref() {
        builder.header(IF_MODIFIED_SINCE, last_modified)
    } else {
        builder
    }
}

fn network_failure(target: &RemoteTarget, method: &str, error: String) -> RemoteOutcome {
    RemoteOutcome {
        method: method.into(),
        final_url: None,
        http_status: None,
        result_state: "CHECK_FAILED".into(),
        etag: target.previous_etag.clone(),
        last_modified: target.previous_last_modified.clone(),
        content_length: target.previous_content_length,
        evidence: "The remote request was blocked or failed before OriginKeep received usable public HTTP evidence.".into(),
        error: Some(error),
    }
}

fn outcome_from_response(
    response: Response,
    target: &RemoteTarget,
    method: &str,
    ranged: bool,
) -> RemoteOutcome {
    let status = response.status().as_u16() as i64;
    let final_url = Some(response.url().to_string());
    let response_etag = header_string(&response, ETAG);
    let response_last_modified = header_string(&response, LAST_MODIFIED);
    let response_length = response_content_length(&response, ranged);
    let effective_etag = response_etag
        .clone()
        .or_else(|| target.previous_etag.clone());
    let effective_last_modified = response_last_modified
        .clone()
        .or_else(|| target.previous_last_modified.clone());
    let effective_length = response_length.or(target.previous_content_length);
    let (result_state, evidence) = classify_remote_state(
        status,
        target.previous_etag.as_deref(),
        target.previous_last_modified.as_deref(),
        target.previous_content_length,
        response_etag.as_deref(),
        response_last_modified.as_deref(),
        response_length,
    );

    RemoteOutcome {
        method: method.into(),
        final_url,
        http_status: Some(status),
        result_state,
        etag: effective_etag,
        last_modified: effective_last_modified,
        content_length: effective_length,
        evidence,
        error: None,
    }
}

fn header_string(response: &Response, name: reqwest::header::HeaderName) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

fn response_content_length(response: &Response, ranged: bool) -> Option<i64> {
    if ranged {
        if let Some(content_range) = response
            .headers()
            .get(CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
        {
            if let Some(total) = content_range.rsplit('/').next() {
                if total != "*" {
                    if let Ok(total) = total.parse::<i64>() {
                        return Some(total);
                    }
                }
            }
        }
    }
    response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<i64>().ok())
}

fn classify_remote_state(
    status: i64,
    previous_etag: Option<&str>,
    previous_last_modified: Option<&str>,
    previous_content_length: Option<i64>,
    current_etag: Option<&str>,
    current_last_modified: Option<&str>,
    current_content_length: Option<i64>,
) -> (String, String) {
    match status {
        304 => (
            "CURRENT".into(),
            "HTTP 304 matched the stored conditional validator; the remote source is unchanged."
                .into(),
        ),
        401 | 403 => (
            "AUTH_REQUIRED".into(),
            format!("HTTP {status} denied an anonymous freshness check; authentication or access may be required."),
        ),
        404 | 410 => (
            "SOURCE_MISSING".into(),
            format!("HTTP {status} indicates that the recorded remote source is no longer available at this URL."),
        ),
        200..=299 => {
            if let (Some(previous), Some(current)) = (previous_etag, current_etag) {
                if previous == current {
                    return (
                        "CURRENT".into(),
                        "The server returned the same ETag as the prior check.".into(),
                    );
                }
                return (
                    "CHANGED".into(),
                    "The server returned a different ETag from the prior check.".into(),
                );
            }
            if let (Some(previous), Some(current)) =
                (previous_last_modified, current_last_modified)
            {
                if previous == current {
                    return (
                        "CURRENT".into(),
                        "The server returned the same Last-Modified validator as the prior check."
                            .into(),
                    );
                }
                return (
                    "CHANGED".into(),
                    "The server returned a different Last-Modified validator from the prior check."
                        .into(),
                );
            }
            if let (Some(previous), Some(current)) =
                (previous_content_length, current_content_length)
            {
                if previous != current {
                    return (
                        "CHANGED".into(),
                        "The remote Content-Length changed since the prior check.".into(),
                    );
                }
            }
            (
                "SOURCE_UNKNOWN".into(),
                "Remote metadata was captured, but no prior validator proves that the local download is still current. This check establishes or refreshes the baseline without making a freshness claim.".into(),
            )
        }
        _ => (
            "CHECK_FAILED".into(),
            format!("HTTP {status} did not provide usable freshness evidence."),
        ),
    }
}

fn persist_remote_outcome(
    path: &Path,
    target: &RemoteTarget,
    outcome: &RemoteOutcome,
) -> Result<RemoteEvidence, String> {
    let mut connection = Connection::open(path).map_err(|error| error.to_string())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            r#"
            INSERT INTO remote_checks (
                download_id, request_method, request_url, final_url, http_status,
                result_state, etag, last_modified, content_length, evidence, error
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                target.download_id,
                outcome.method,
                target.url,
                outcome.final_url,
                outcome.http_status,
                outcome.result_state,
                outcome.etag,
                outcome.last_modified,
                outcome.content_length,
                outcome.evidence,
                outcome.error,
            ],
        )
        .map_err(|error| error.to_string())?;
    let check_id = transaction.last_insert_rowid();
    transaction
        .execute(
            "UPDATE downloads SET status = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![outcome.result_state, target.download_id],
        )
        .map_err(|error| error.to_string())?;
    let checked_at: String = transaction
        .query_row(
            "SELECT checked_at FROM remote_checks WHERE id = ?1",
            [check_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;

    Ok(RemoteEvidence {
        download_id: target.download_id,
        checked_at,
        request_method: outcome.method.clone(),
        request_url: target.url.clone(),
        final_url: outcome.final_url.clone(),
        http_status: outcome.http_status,
        result_state: outcome.result_state.clone(),
        etag: outcome.etag.clone(),
        last_modified: outcome.last_modified.clone(),
        content_length: outcome.content_length,
        evidence: outcome.evidence.clone(),
        error: outcome.error.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_private_and_loopback_destinations() {
        for value in [
            "http://127.0.0.1/file",
            "http://10.0.0.1/file",
            "http://169.254.169.254/latest/meta-data",
            "http://192.168.1.1/file",
            "http://[::1]/file",
            "http://[fc00::1]/file",
            "http://[fe80::1]/file",
        ] {
            let url = Url::parse(value).unwrap();
            assert!(validate_public_http_url(&url).is_err(), "{value}");
        }
    }

    #[test]
    fn accepts_public_ip_literals() {
        for value in ["https://8.8.8.8/file", "https://1.1.1.1/file"] {
            let url = Url::parse(value).unwrap();
            assert!(validate_public_http_url(&url).is_ok(), "{value}");
        }
    }

    #[test]
    fn rejects_embedded_credentials() {
        let url = Url::parse("https://user:password@example.com/file").unwrap();
        assert!(validate_public_http_url(&url).is_err());
    }
}
