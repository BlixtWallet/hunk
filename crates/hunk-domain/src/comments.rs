#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentLineSide {
    Left,
    Right,
    Meta,
}

#[cfg(feature = "database")]
impl CommentLineSide {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Meta => "meta",
        }
    }

    pub(crate) fn from_db(value: &str) -> Option<Self> {
        match value {
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "meta" => Some(Self::Meta),
            _ => None,
        }
    }
}

pub fn compute_comment_anchor_hash(
    file_path: &str,
    hunk_header: Option<&str>,
    line_text: &str,
    context_before: &str,
    context_after: &str,
) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    hash = fnv1a64_update(hash, b"file:");
    hash = fnv1a64_update(hash, file_path.as_bytes());
    hash = fnv1a64_update(hash, b"\nheader:");
    hash = fnv1a64_update(hash, hunk_header.unwrap_or("").as_bytes());
    hash = fnv1a64_update(hash, b"\nline:");
    hash = fnv1a64_update(hash, line_text.as_bytes());
    hash = fnv1a64_update(hash, b"\nbefore:");
    hash = fnv1a64_update(hash, context_before.as_bytes());
    hash = fnv1a64_update(hash, b"\nafter:");
    hash = fnv1a64_update(hash, context_after.as_bytes());
    format!("{hash:016x}")
}

fn fnv1a64_update(mut hash: u64, bytes: &[u8]) -> u64 {
    const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
