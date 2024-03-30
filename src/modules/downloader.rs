use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use std::fs::File;
use std::io::copy;

pub fn download_file(client: &Client, url: &str, filename: &str) -> Result<()> {
    let response = client
        .get(url)
        .send()
        .context(format!("Failed to download from {}", url))?;
    let total_size = response.content_length().unwrap_or(0);
    let mut file = File::create(filename).context(format!("Failed to create file {}", filename))?;

    let progress_bar = create_progress_bar(total_size)?;

    let mut stream = progress_bar.wrap_read(response);
    copy(&mut stream, &mut file).context(format!("Failed to write to file {}", filename))?;

    progress_bar.finish_and_clear();
    Ok(())
}

fn create_progress_bar(total_size: u64) -> Result<ProgressBar> {
    let progress_bar = ProgressBar::new(total_size);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
            .progress_chars("#>-")
    );
    Ok(progress_bar)
}
