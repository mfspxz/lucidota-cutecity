//! Private media (city-008): S3 SigV4 presigned URLs for the private R2 bucket.
//!
//! The bucket `cute-city-production` stays private — media is served only via
//! time-boxed presigned URLs. This module implements SigV4 presigning in pure
//! Rust (no network deps). Key-safety: object keys are validated; `..` and
//! leading `/` are rejected.
//!
//! NOTE: the R2 credential is NOT available in this build sandbox, so the
//! presign functions take the key material as parameters (callers in a
//! credentialed environment supply them). Tests use fake keys.

/// Minimal SHA-256 (FIPS 180-4). Pure Rust, std-only.
pub fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (hv, v) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *hv = hv.wrapping_add(v);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut k = key.to_vec();
    if k.len() > 64 {
        k = hex_to_bytes(&sha256_hex(&k));
    }
    while k.len() < 64 {
        k.push(0);
    }
    let (ipad, opad): (Vec<u8>, Vec<u8>) =
        (k.iter().map(|b| b ^ 0x36).collect(), k.iter().map(|b| b ^ 0x5c).collect());
    let mut inner = ipad;
    inner.extend_from_slice(data);
    let inner_hash = hex_to_bytes(&sha256_hex(&inner));
    let mut outer = opad;
    outer.extend_from_slice(&inner_hash);
    let out = sha256_hex(&outer);
    let mut r = [0u8; 32];
    r.copy_from_slice(&hex_to_bytes(&out));
    r
}

fn hex_to_bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(0))
        .collect()
}

fn bytes_to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[derive(Debug, PartialEq)]
pub enum MediaError {
    UnsafeKey(String),
    BadExpiry(u64),
}

impl std::fmt::Display for MediaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MediaError::UnsafeKey(k) => write!(f, "unsafe object key: {k}"),
            MediaError::BadExpiry(e) => write!(f, "bad expiry seconds: {e}"),
        }
    }
}

impl std::error::Error for MediaError {}

/// Validate an R2 object key: no `..`, no leading `/`, no backslashes, no
/// control chars. Returns the key unchanged on success.
pub fn validate_key(key: &str) -> Result<&str, MediaError> {
    if key.is_empty() || key.starts_with('/') || key.contains("..") || key.contains('\\') {
        return Err(MediaError::UnsafeKey(key.to_string()));
    }
    if key.chars().any(|c| c.is_control()) {
        return Err(MediaError::UnsafeKey(key.to_string()));
    }
    Ok(key)
}

fn url_encode_path(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// SigV4 presigned URL for GET (download) or PUT (upload).
///
/// - `endpoint`: e.g. `https://<account>.r2.cloudflarestorage.com`
/// - `bucket`, `key`: validated
/// - `access_key`, `secret_key`: supplied by the credentialed caller
/// - `region`: R2 uses `auto`
/// - `expires_secs`: 1..=604800 (7 days, S3 max)
/// - `amz_date`: `YYYYMMDDTHHMMSSZ`; `date_stamp`: `YYYYMMDD`
pub fn presign(
    method: &str,
    endpoint: &str,
    bucket: &str,
    key: &str,
    access_key: &str,
    secret_key: &str,
    region: &str,
    amz_date: &str,
    date_stamp: &str,
    expires_secs: u64,
) -> Result<String, MediaError> {
    validate_key(key)?;
    if !(1..=604_800).contains(&expires_secs) {
        return Err(MediaError::BadExpiry(expires_secs));
    }
    let service = "s3";
    let host = endpoint.trim_start_matches("https://").trim_start_matches("http://");
    let credential_scope = format!("{date_stamp}/{region}/{service}/aws4_request");
    let credential = format!("{access_key}/{credential_scope}");
    let signed_headers = "host";
    let payload_hash = "UNSIGNED-PAYLOAD";

    // Canonical query params, sorted.
    let mut params = vec![
        ("X-Amz-Algorithm", "AWS4-HMAC-SHA256".to_string()),
        ("X-Amz-Credential", credential.clone()),
        ("X-Amz-Date", amz_date.to_string()),
        ("X-Amz-Expires", expires_secs.to_string()),
        ("X-Amz-SignedHeaders", signed_headers.to_string()),
    ];
    params.sort_by(|a, b| a.0.cmp(b.0));
    let canonical_qs: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", url_encode_path(k), url_encode_path(v)))
        .collect::<Vec<_>>()
        .join("&");

    let canonical_uri = format!("/{}/{}", bucket, url_encode_path(key));
    let canonical_headers = format!("host:{host}\n");
    let canonical_request = format!(
        "{method}\n{canonical_uri}\n{canonical_qs}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
    );
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );
    let k_date = hmac_sha256(format!("AWS4{secret_key}").as_bytes(), date_stamp.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    let k_signing = hmac_sha256(&k_service, b"aws4_request");
    let signature = bytes_to_hex(&hmac_sha256(&k_signing, string_to_sign.as_bytes()));

    Ok(format!(
        "{endpoint}{canonical_uri}?{canonical_qs}&X-Amz-Signature={signature}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_two_block_vector() {
        // 448-bit message -> two blocks; catches schedule/constant errors
        // that single-block vectors can mask.
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn hmac_known_vector() {
        // RFC 4231 test case 1
        let key = [0x0b; 20];
        let mac = hmac_sha256(&key, b"Hi There");
        assert_eq!(
            bytes_to_hex(&mac),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn key_validation() {
        assert!(validate_key("cms/wallet-qr.png").is_ok());
        assert!(validate_key("projects/a/hero.png").is_ok());
        assert!(validate_key("../etc/passwd").is_err());
        assert!(validate_key("/absolute.png").is_err());
        assert!(validate_key("a\\b.png").is_err());
        assert!(validate_key("").is_err());
        assert!(validate_key("a\u{0}b").is_err());
    }

    #[test]
    fn presign_shape_and_determinism() {
        let u1 = presign(
            "GET",
            "https://acct.r2.cloudflarestorage.com",
            "cute-city-production",
            "cms/wallet-qr.png",
            "AKIDEXAMPLE",
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            "auto",
            "20261007T000000Z",
            "20261007",
            3600,
        )
        .unwrap();
        let u2 = presign(
            "GET",
            "https://acct.r2.cloudflarestorage.com",
            "cute-city-production",
            "cms/wallet-qr.png",
            "AKIDEXAMPLE",
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            "auto",
            "20261007T000000Z",
            "20261007",
            3600,
        )
        .unwrap();
        assert_eq!(u1, u2, "presign must be deterministic");
        assert!(u1.starts_with("https://acct.r2.cloudflarestorage.com/cute-city-production/cms/wallet-qr.png?"));
        assert!(u1.contains("X-Amz-Algorithm=AWS4-HMAC-SHA256"));
        assert!(u1.contains("X-Amz-Expires=3600"));
        assert!(u1.contains("X-Amz-Signature="));
        let sig = u1.split("X-Amz-Signature=").nth(1).unwrap();
        assert_eq!(sig.len(), 64, "signature must be 64 hex chars");
    }

    #[test]
    fn presign_rejects_bad_expiry() {
        let r = presign("GET", "https://h", "b", "k.png", "a", "s", "auto", "20261007T000000Z", "20261007", 0);
        assert!(matches!(r, Err(MediaError::BadExpiry(0))));
        let r = presign("GET", "https://h", "b", "k.png", "a", "s", "auto", "20261007T000000Z", "20261007", 999_999_999);
        assert!(r.is_err());
    }

    #[test]
    fn presign_rejects_unsafe_key() {
        let r = presign("PUT", "https://h", "b", "../x.png", "a", "s", "auto", "20261007T000000Z", "20261007", 300);
        assert!(matches!(r, Err(MediaError::UnsafeKey(_))));
    }
}
