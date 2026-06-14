use zip::{ZipWriter, ZipArchive, write::SimpleFileOptions, CompressionMethod};
use std::path::Path;
use walkdir::WalkDir;

pub fn zip_folder(source: &str, target: &str) -> std::io::Result<()> {
    let target_path = Path::new(target);
    let tmp_target = format!("{}.tmp", target);
    let tmp_path = Path::new(&tmp_target);

    let file = std::fs::File::create(tmp_path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let src_path = Path::new(source);
    if src_path.is_file() {
        zip.start_file(src_path.file_name().unwrap().to_str().unwrap(), options)?;
        let mut f = std::fs::File::open(src_path)?;
        std::io::copy(&mut f, &mut zip)?;
    } else {
        for entry in WalkDir::new(source) {
            let entry = entry.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            let path = entry.path();
            let name = path.strip_prefix(src_path).unwrap();

            if name.as_os_str().is_empty() { continue; }

            if let Some(n) = path.file_name() {
                if n == ".pax_token" {
                    continue;
                }
                if let Some(tn) = target_path.file_name() {
                    if n == tn { continue; }
                }
                if let Some(tn) = tmp_path.file_name() {
                    if n == tn { continue; }
                }
            }

            let name_str = name.to_str().unwrap().replace('\\', "/");

            if path.is_file() {
                zip.start_file(&name_str, options)?;
                let mut f = std::fs::File::open(path)?;
                std::io::copy(&mut f, &mut zip)?;
            } else {
                let dir_name = format!("{}/", name_str);
                zip.add_directory(&dir_name, options)?;
            }
        }
    }
    zip.finish()?;

    if target_path.exists() {
        std::fs::remove_file(target_path)?;
    }
    std::fs::rename(tmp_path, target_path)?;
    Ok(())
}

pub fn unzip_archive(zip_file: &str, dest_dir: &str) -> std::io::Result<()> {
    let file = std::fs::File::open(zip_file)?;
    let mut archive = ZipArchive::new(file)?;
    std::fs::create_dir_all(dest_dir)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => Path::new(dest_dir).join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)?;
                }
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}
