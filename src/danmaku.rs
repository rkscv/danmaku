use anyhow::{anyhow, Result};
use hex::encode;
use md5::{Digest, Md5};
use reqwest::Client;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::File,
    io::{copy, Read},
    path::Path,
};
use unicode_segmentation::UnicodeSegmentation;

pub struct Danmaku {
    pub message: String,
    pub count: usize,
    pub time: f64,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub x: Option<f64>,
    pub row: Option<usize>,
}

#[derive(Deserialize)]
struct MatchResponse {
    #[serde(rename = "isMatched")]
    is_matched: bool,
    matches: Vec<Match>,
}

#[derive(Deserialize)]
struct Match {
    #[serde(rename = "episodeId")]
    episode_id: usize,
}

#[derive(Deserialize)]
struct CommentResponse {
    comments: Vec<Comment>,
}

#[derive(Deserialize)]
struct Comment {
    p: String,
    m: String,
}

pub async fn get_danmaku<P: AsRef<Path>>(path: P) -> Result<Vec<Danmaku>> {
    let file = File::open(&path)?;
    let mut hasher = Md5::new();
    // https://api.dandanplay.net/swagger/ui/index
    copy(&mut file.take(16 * 1024 * 1024), &mut hasher)?;
    let hash = encode(hasher.finalize());
    let file_name = path.as_ref().file_name().unwrap().to_str().unwrap();

    let client = Client::new();
    let data = client
        .post("https://api.dandanplay.net/api/v2/match")
        .header("Content-Type", "application/json")
        .json(&HashMap::from([
            ("fileName", file_name),
            ("fileHash", &hash),
        ]))
        .send()
        .await?
        .json::<MatchResponse>()
        .await?;
    if data.matches.len() > 1 {
        return Err(anyhow!("multiple matching episodes"));
    } else if !data.is_matched {
        return Err(anyhow!("no matching episode"));
    }

    let mut danmaku = client
        .get(format!(
            "https://api.dandanplay.net/api/v2/comment/{}?withRelated=true",
            data.matches[0].episode_id
        ))
        .send()
        .await?
        .json::<CommentResponse>()
        .await?
        .comments
        .into_iter()
        .map(|comment| {
            let mut p = comment.p.splitn(4, ',');
            let t = p.next().unwrap().parse().unwrap();
            _ = p.next().unwrap();
            let c = p.next().unwrap().parse::<u32>().unwrap();
            Danmaku {
                message: comment.m.replace('\n', "\\N"),
                count: comment.m.graphemes(true).count(),
                time: t,
                r: (c / (256 * 256)).try_into().unwrap(),
                g: (c % (256 * 256) / 256).try_into().unwrap(),
                b: (c % 256).try_into().unwrap(),
                x: None,
                row: None,
            }
        })
        .collect::<Vec<_>>();
    danmaku.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    Ok(danmaku)
}
