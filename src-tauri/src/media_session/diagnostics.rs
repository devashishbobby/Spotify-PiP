pub fn log(message: &str) {
    if cfg!(debug_assertions) && std::env::var("SPOTIFY_PIP_DIAGNOSTICS").as_deref() == Ok("1") {
        eprintln!("[media {}] {message}", super::state::utc_ms());
    }
}
pub fn safe_id(id: &str) -> String {
    if id.len() <= 160
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-_!".contains(c))
    {
        id.into()
    } else {
        "<redacted application identifier>".into()
    }
}
