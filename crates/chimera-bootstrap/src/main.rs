use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use tar::Archive;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

fn main() {
    if let Err(err) = run() {
        eprintln!("chimera-bootstrap error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("download") => {
            let url = take_flag_value(&mut args, "--url")?;
            let output = take_flag_value(&mut args, "--output")?;
            download_to_file(&url, Path::new(&output))
        }
        Some("verify") => {
            let file = take_flag_value(&mut args, "--file")?;
            let sha256 = take_flag_value(&mut args, "--sha256")?;
            verify_sha256(Path::new(&file), &sha256)
        }
        Some("extract") => {
            let archive = take_flag_value(&mut args, "--archive")?;
            let dest = take_flag_value(&mut args, "--dest")?;
            let strip_components =
                take_optional_usize(&mut args, "--strip-components")?.unwrap_or(0);
            extract_tar_gz(Path::new(&archive), Path::new(&dest), strip_components)
        }
        Some("install") => {
            let url = take_flag_value(&mut args, "--url")?;
            let checksum_url = take_flag_value(&mut args, "--checksum-url")?;
            let dest = take_flag_value(&mut args, "--dest")?;
            let strip_components =
                take_optional_usize(&mut args, "--strip-components")?.unwrap_or(1);
            install_bundle(&url, &checksum_url, Path::new(&dest), strip_components)
        }
        _ => {
            eprintln!("usage: chimera-bootstrap <download|verify|extract|install> --flags...");
            Err("invalid arguments".into())
        }
    }
}

fn take_flag_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    match args.next() {
        Some(actual) if actual == flag => {}
        Some(actual) => return Err(format!("expected {flag}, got {actual}").into()),
        None => return Err(format!("missing {flag}").into()),
    }
    args.next()
        .ok_or_else(|| format!("missing value for {flag}").into())
}

fn take_optional_usize(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<Option<usize>> {
    let mut collected = Vec::new();
    while let Some(item) = args.next() {
        collected.push(item);
    }
    if collected.is_empty() {
        return Ok(None);
    }
    if collected.len() != 2 || collected[0] != flag {
        return Err(format!("unexpected trailing arguments: {:?}", collected).into());
    }
    let parsed = collected[1]
        .parse::<usize>()
        .map_err(|e| format!("invalid {} value: {}", flag, e))?;
    Ok(Some(parsed))
}

fn download_to_file(url: &str, output: &Path) -> Result<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let resp = ureq::get(url).call()?;
    let mut reader = resp.into_reader();
    let mut file = File::create(output)?;
    io::copy(&mut reader, &mut file)?;
    file.flush()?;
    Ok(())
}

fn verify_sha256(file: &Path, expected_hex: &str) -> Result<()> {
    let mut hasher = Sha256::new();
    let mut reader = File::open(file)?;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual != expected_hex.trim() {
        return Err(format!(
            "sha256 mismatch expected={} actual={}",
            expected_hex.trim(),
            actual
        )
        .into());
    }
    Ok(())
}

fn extract_tar_gz(archive: &Path, dest: &Path, strip_components: usize) -> Result<()> {
    fs::create_dir_all(dest)?;
    let file = File::open(archive)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    for entry_res in archive.entries()? {
        let mut entry = entry_res?;
        let path = entry.path()?.to_path_buf();
        let stripped = strip_path_components(&path, strip_components)?;
        if stripped.as_os_str().is_empty() {
            continue;
        }
        let out_path = dest.join(stripped);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        entry.unpack(out_path)?;
    }
    Ok(())
}

fn install_bundle(
    url: &str,
    checksum_url: &str,
    dest: &Path,
    strip_components: usize,
) -> Result<()> {
    let tmp_dir = temp_dir("chimera-bootstrap")?;
    let archive = tmp_dir.join("bundle.tar.gz");
    let checksum = tmp_dir.join("bundle.tar.gz.sha256");
    download_to_file(url, &archive)?;
    download_to_file(checksum_url, &checksum)?;
    let expected = fs::read_to_string(&checksum)?
        .split_whitespace()
        .next()
        .ok_or("empty checksum file")?
        .to_string();
    verify_sha256(&archive, &expected)?;
    extract_tar_gz(&archive, dest, strip_components)?;
    Ok(())
}

fn temp_dir(prefix: &str) -> Result<PathBuf> {
    let mut base = env::temp_dir();
    let unique = format!(
        "{}-{}-{}",
        prefix,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    base.push(unique);
    fs::create_dir_all(&base)?;
    Ok(base)
}

fn strip_path_components(path: &Path, strip_components: usize) -> Result<PathBuf> {
    let mut out = PathBuf::new();
    for (idx, comp) in path.components().enumerate() {
        if idx < strip_components {
            continue;
        }
        match comp {
            std::path::Component::Normal(part) => out.push(part),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                return Err("archive entry contains parent dir path".into());
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err("archive entry contains absolute path".into());
            }
        }
    }
    Ok(out)
}
