fn main() -> Result<(), Box<dyn std::error::Error>> {
    let verify = std::env::args().nth(1).as_deref() == Some("--verify");
    println!(
        "{}",
        spotify_pip_lib::media_session::diagnostic_volume(verify)?
    );
    Ok(())
}
