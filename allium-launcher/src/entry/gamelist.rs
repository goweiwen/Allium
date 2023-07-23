use std::path::PathBuf;

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

#[cfg(test)]
mod tests {
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
        let game_list: GameList = serde_xml_rs::from_str(xml).unwrap();
        assert_eq!(game_list.games.len(), 3);
        assert_eq!(game_list.games[0].path, PathBuf::from("path/to/game"));
        assert_eq!(game_list.games[0].name, "Game One");
        assert_eq!(game_list.games[0].image, None);
        assert_eq!(
            game_list.games[0].thumbnail,
            Some(PathBuf::from("path/to/image"))
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
        let game_list: GameList = serde_xml_rs::from_str(xml).unwrap();

        assert_eq!(game_list.folders.len(), 1);
        assert_eq!(game_list.folders[0].path, PathBuf::from("path/to/game"));
        assert_eq!(game_list.folders[0].name, "Folder");
        assert_eq!(
            game_list.folders[0].image,
            Some(PathBuf::from("path/to/image"))
        );
    }
}
