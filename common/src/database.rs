use std::{
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};

use crate::constants::ALLIUM_DATABASE;

#[derive(Debug, Clone, Default)]
pub struct Database {
    conn: Option<Rc<Connection>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    pub id: i64,
    pub name: String,
    pub path: PathBuf,
    pub image: Option<PathBuf>,
    pub play_count: i64,
    pub play_time: Duration,
    pub last_played: i64,
}

impl Database {
    pub fn new() -> Result<Self> {
        let mut conn = Connection::open(ALLIUM_DATABASE.as_path())?;
        Self::migrations().to_latest(&mut conn)?;
        Ok(Self {
            conn: Some(Rc::new(conn)),
        })
    }

    pub fn in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        Self::migrations().to_latest(&mut conn)?;
        Ok(Self {
            conn: Some(Rc::new(conn)),
        })
    }

    pub fn migrations<'a>() -> Migrations<'a> {
        Migrations::new(vec![M::up(
            "
CREATE TABLE IF NOT EXISTS games (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    image TEXT,
    play_count INTEGER NOT NULL,
    play_time INTEGER NOT NULL,
    last_played INTEGER NOT NULL
);
            ",
        )])
    }

    pub fn select_most_played(&self, limit: i64) -> Result<Vec<Game>> {
        let mut stmt = self.conn.as_ref().unwrap().prepare("SELECT id, name, path, image, play_count, play_time, last_played FROM games ORDER BY play_count DESC LIMIT ?")?;

        let rows = stmt.query_map([limit], |row| {
            Ok(Game {
                id: row.get(0)?,
                name: row.get(1)?,
                path: PathBuf::from(row.get::<_, String>(2)?),
                image: row.get::<_, Option<String>>(3)?.map(PathBuf::from),
                play_count: row.get(4)?,
                play_time: Duration::from_secs(row.get(5)?),
                last_played: row.get(6)?,
            })
        })?;

        let mut games = Vec::new();
        for row in rows {
            games.push(row?);
        }

        Ok(games)
    }

    pub fn select_last_played(&self, limit: i64) -> Result<Vec<Game>> {
        let mut stmt = self.conn.as_ref().unwrap().prepare("SELECT id, name, path, image, play_count, play_time, last_played FROM games ORDER BY last_played DESC LIMIT ?")?;

        let rows = stmt.query_map([limit], |row| {
            Ok(Game {
                id: row.get(0)?,
                name: row.get(1)?,
                path: PathBuf::from(row.get::<_, String>(2)?),
                image: row.get::<_, Option<String>>(3)?.map(PathBuf::from),
                play_count: row.get(4)?,
                play_time: Duration::from_secs(row.get(5)?),
                last_played: row.get(6)?,
            })
        })?;

        let mut games = Vec::new();
        for row in rows {
            games.push(row?);
        }

        Ok(games)
    }

    pub fn select_game(&self, path: &str) -> Result<Option<Game>> {
        let game = self.conn.as_ref().unwrap().query_row(
            "SELECT id, name, path, image, play_count, play_time, last_played FROM games WHERE path = ? LIMIT 1",
       [path], |row| {
            Ok(Game {
                id: row.get(0)?,
                name: row.get(1)?,
                path: PathBuf::from(row.get::<_, String>(2)?),
                image: row.get::<_, Option<String>>(3)?.map(PathBuf::from),
                play_count: row.get(4)?,
                play_time: Duration::from_secs(row.get(5)?),
                last_played: row.get(6)?,
            })
        }).optional()?;

        Ok(game)
    }

    pub fn increment_play_count(
        &self,
        name: &str,
        path: &Path,
        image: Option<&Path>,
    ) -> Result<()> {
        let mut stmt = self.conn.as_ref().unwrap().prepare(
            "INSERT OR IGNORE INTO games (name, path, image, play_count, play_time, last_played) VALUES (?, ?, ?, ?, ?, ?)",)?;
        stmt.execute(params![
            name,
            path.display().to_string(),
            image.map(|p| p.display().to_string()),
            0,
            0,
            0
        ])?;

        let mut stmt = self.conn.as_ref().unwrap().prepare(
             "UPDATE games SET play_count = play_count + 1, last_played = (SELECT MAX(last_played) from games) + 1 WHERE path = ?",
        )?;
        stmt.execute([path.display().to_string()])?;

        Ok(())
    }

    pub fn add_play_time(&self, path: &str, play_time: Duration) -> Result<()> {
        let mut stmt = self
            .conn
            .as_ref()
            .unwrap()
            .prepare("UPDATE games SET play_time = play_time + ? WHERE path = ?")?;

        stmt.execute(params![play_time.as_secs(), path])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations() {
        Database::migrations().validate().unwrap();
    }

    #[test]
    fn test_most_played() {
        let database = Database::in_memory().unwrap();

        let games = vec![
            Game {
                id: 0,
                name: "Game One".to_string(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                play_count: 0,
                play_time: Duration::ZERO,
                last_played: 0,
            },
            Game {
                id: 1,
                name: "Game Two".to_string(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                play_count: 0,
                play_time: Duration::ZERO,
                last_played: 0,
            },
        ];

        database
            .increment_play_count(
                &games[1].name,
                games[1].path.as_path(),
                games[1].image.as_deref(),
            )
            .unwrap();
        let most_played = database.select_most_played(2).unwrap();
        assert_eq!(most_played.len(), 1);
        assert_eq!(most_played[0].path, games[1].path);

        for _ in 0..2 {
            database
                .increment_play_count(
                    &games[0].name,
                    games[0].path.as_path(),
                    games[0].image.as_deref(),
                )
                .unwrap();
        }
        let most_played = database.select_most_played(2).unwrap();
        assert_eq!(most_played.len(), 2);
        assert_eq!(most_played[0].path, games[0].path);
        assert_eq!(most_played[1].path, games[1].path);
    }

    #[test]
    fn test_last_played() {
        let database = Database::in_memory().unwrap();

        let games = vec![
            Game {
                id: 0,
                name: "Game One".to_string(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                play_count: 0,
                play_time: Duration::ZERO,
                last_played: 0,
            },
            Game {
                id: 1,
                name: "Game Two".to_string(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                play_count: 0,
                play_time: Duration::ZERO,
                last_played: 0,
            },
        ];

        for _ in 0..2 {
            database
                .increment_play_count(
                    &games[1].name,
                    games[1].path.as_path(),
                    games[1].image.as_deref(),
                )
                .unwrap();
        }
        let last_played = database.select_last_played(2).unwrap();
        assert_eq!(last_played.len(), 1);
        assert_eq!(last_played[0].path, games[1].path);

        database
            .increment_play_count(
                &games[0].name,
                games[0].path.as_path(),
                games[0].image.as_deref(),
            )
            .unwrap();
        let last_played = database.select_last_played(2).unwrap();
        assert_eq!(last_played.len(), 2);
        assert_eq!(last_played[0].path, games[0].path);
        assert_eq!(last_played[1].path, games[1].path);
    }
}
