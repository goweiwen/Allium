use std::path::PathBuf;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GameList {
    #[serde(default, rename = "game")]
    pub games: Vec<Game>,
    #[serde(default, rename = "folder")]
    pub folders: Vec<Folder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Game {
    pub path: PathBuf,
    pub name: String,
    #[serde(default, deserialize_with = "optional_path_buf_deserializer")]
    pub image: Option<PathBuf>,
    #[serde(default, deserialize_with = "optional_path_buf_deserializer")]
    pub thumbnail: Option<PathBuf>,
    #[serde(default, deserialize_with = "rating_deserializer")]
    pub rating: Option<u8>,
    #[serde(
        default,
        rename = "releasedate",
        deserialize_with = "optional_naivedatetime_deserializer"
    )]
    pub release_date: Option<NaiveDateTime>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    #[serde(default, rename = "genre", deserialize_with = "genre_deserializer")]
    pub genres: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Folder {
    pub path: PathBuf,
    pub name: String,
    #[serde(default, deserialize_with = "optional_path_buf_deserializer")]
    pub image: Option<PathBuf>,
    #[serde(default, deserialize_with = "optional_path_buf_deserializer")]
    pub thumbnail: Option<PathBuf>,
}

fn optional_path_buf_deserializer<'de, D>(d: D) -> Result<Option<PathBuf>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    match &s[..] {
        "" => Ok(None),
        _ => Ok(Some(s.parse::<PathBuf>().unwrap())),
    }
}

fn genre_deserializer<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    Ok(s.split(',').map(str::trim).map(str::to_string).collect())
}

fn rating_deserializer<'de, D>(d: D) -> Result<Option<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    Ok(s.parse::<f32>().map(|rating| (rating * 10.0) as u8).ok())
}

fn optional_naivedatetime_deserializer<'de, D>(d: D) -> Result<Option<NaiveDateTime>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    Ok(NaiveDateTime::parse_from_str(&s, "%Y%m%dT%H%M%S").ok())
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn test_deserialize_games() {
        let xml = r#"
        <gameList>
            <game>
                <path>path/to/game</path>
                <name>Game One</name>
                <image />
                <thumbnail>path/to/image</thumbnail>
                <genre>Strategy, Action</genre>
                <rating>0.9</rating>
                <releasedate>20030623T010203</releasedate>
            </game>
            <game>
                <path>path/to/game</path>
                <name>Game Two</name>
                <thumbnail />
                <image>path/to/image</image>
            </game>
            <game>
                <path>path/to/game</path>
                <name>Game Three</name>
                <thumbnail>path/to/image</thumbnail>
            </game>
        </gameList>
        "#;
        let game_list: GameList = quick_xml::de::from_str(xml).unwrap();
        assert_eq!(game_list.games.len(), 3);
        assert_eq!(game_list.games[0].path, PathBuf::from("path/to/game"));
        assert_eq!(game_list.games[0].name, "Game One");
        assert_eq!(game_list.games[0].image, None);
        assert_eq!(
            game_list.games[0].thumbnail,
            Some(PathBuf::from("path/to/image"))
        );
        assert_eq!(
            game_list.games[0].genres,
            vec!["Strategy".to_string(), "Action".to_string()]
        );
        assert_eq!(game_list.games[0].rating, Some(9));
        assert_eq!(
            game_list.games[0].release_date,
            Some(
                NaiveDate::from_ymd_opt(2003, 6, 23)
                    .unwrap()
                    .and_hms_opt(1, 2, 3)
                    .unwrap()
            )
        );

        assert_eq!(game_list.games[1].name, "Game Two");
        assert_eq!(
            game_list.games[1].image,
            Some(PathBuf::from("path/to/image"))
        );
        assert_eq!(game_list.games[1].thumbnail, None);

        assert_eq!(game_list.games[2].name, "Game Three");
        assert_eq!(game_list.games[2].image, None);
        assert_eq!(
            game_list.games[2].thumbnail,
            Some(PathBuf::from("path/to/image"))
        );
    }

    #[test]
    fn test_deserialize_folder() {
        let xml = r#"
        <gameList>
            <folder>
                <path>path/to/game</path>
                <name>Folder</name>
                <image>path/to/image</image>
            </folder>
        </gameList>
        "#;
        let game_list: GameList = quick_xml::de::from_str(xml).unwrap();

        assert_eq!(game_list.folders.len(), 1);
        assert_eq!(game_list.folders[0].path, PathBuf::from("path/to/game"));
        assert_eq!(game_list.folders[0].name, "Folder");
        assert_eq!(
            game_list.folders[0].image,
            Some(PathBuf::from("path/to/image"))
        );
    }

    #[test]
    fn test_ampersand() {
        let s = include_str!("test/gamelist.xml");
        let s = &s.replace('&', "&amp;");
        let _: GameList = match quick_xml::de::from_str(s) {
            Ok(gamelist) => gamelist,
            Err(quick_xml::DeError::InvalidXml(quick_xml::Error::EscapeError(
                quick_xml::escape::EscapeError::UnterminatedEntity(..),
            ))) => {
                // Some scrapers produce ill-formed XML where ampersands (&) are not escaped,
                // so we try to failover to replacing them to &amp;
                // (https://github.com/RReverser/serde-xml-rs/issues/106)
                let s = s.replace('&', "&amp;");
                quick_xml::de::from_str(&s).unwrap()
            }
            Err(e) => panic!("{:?}", e),
        };
    }
}
