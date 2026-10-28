use sha2::{Digest, Sha256};

pub(crate) fn digest(bytes: &[u8]) -> String {
    let result = Sha256::digest(bytes);
    result.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn id(namespace: &str, parts: &[&str]) -> String {
    let encoded = serde_json::to_vec(parts).expect("string arrays always serialize");
    format!("{namespace}:{}", digest(&encoded))
}
