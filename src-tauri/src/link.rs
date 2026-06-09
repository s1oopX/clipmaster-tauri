use tauri::Url;

pub fn normalize_web_url(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if !is_safe_web_url(trimmed) {
        return None;
    }

    Some(trimmed.to_string())
}

pub fn is_safe_web_url(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
        || trimmed.contains('\\')
    {
        return false;
    }

    let Ok(url) = Url::parse(trimmed) else {
        return false;
    };

    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return false;
    }

    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host.trim_end_matches('.').to_ascii_lowercase();

    if host.is_empty() || host == "localhost" || !host.contains('.') {
        return false;
    }

    if let Ok(address) = host.parse::<std::net::IpAddr>() {
        return !is_private_or_local_address(address);
    }

    true
}

pub fn link_content_hash(url: &str) -> String {
    format!(
        "{:x}",
        md5::compute(format!("link:{}", url.trim()).as_bytes())
    )
}

fn is_private_or_local_address(address: std::net::IpAddr) -> bool {
    match address {
        std::net::IpAddr::V4(address) => {
            address.is_private()
                || address.is_loopback()
                || address.is_link_local()
                || address.is_broadcast()
                || address.is_documentation()
                || address.is_unspecified()
        }
        std::net::IpAddr::V6(address) => {
            address.is_loopback()
                || address.is_unspecified()
                || address.segments()[0] & 0xfe00 == 0xfc00
                || address.segments()[0] & 0xffc0 == 0xfe80
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_http_and_https_urls() {
        assert_eq!(
            normalize_web_url(" https://github.com/s1oopX ").unwrap(),
            "https://github.com/s1oopX"
        );
        assert!(is_safe_web_url("http://docs.example.com/path?q=1#install"));
    }

    #[test]
    fn rejects_unsafe_or_ambiguous_urls() {
        for value in [
            "",
            "example.com",
            "https://example",
            "https://localhost",
            "https://127.0.0.1",
            "https://10.0.0.2",
            "https://172.16.0.1",
            "https://192.168.1.1",
            "https://[::1]",
            "https://user:pass@example.com",
            "https://example.com\\@evil.test/",
            "https://example.com with words",
            "javascript:alert(1)",
            "file:///C:/temp/a.txt",
        ] {
            assert!(!is_safe_web_url(value), "{value}");
        }
    }

    #[test]
    fn hashes_links_with_a_type_prefix() {
        assert_ne!(
            link_content_hash("https://example.com"),
            format!("{:x}", md5::compute("https://example.com".as_bytes()))
        );
    }
}
