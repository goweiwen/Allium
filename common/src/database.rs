use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::{Context, Result};
use chrono::Duration;
use log::info;
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};

use crate::constants::{ALLIUM_BASE_DIR, ALLIUM_DATABASE};

#[derive(Debug, Clone, Default)]
pub struct Database {
    conn: Option<Rc<Connection>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    pub name: String,
    pub path: PathBuf,
    pub image: Option<PathBuf>,
    pub play_count: i64,
    pub play_time: Duration,
    pub last_played: i64,
}

impl Database {
    pub fn new() -> Result<Self> {
        if !ALLIUM_DATABASE.exists() {
            let path = ALLIUM_BASE_DIR.join("state/allium.db");
            if path.exists() {
                info!("migrating database to new location");
                std::fs::copy(path, ALLIUM_DATABASE.as_path())?;
            }
        }

        let mut conn = Connection::open(ALLIUM_DATABASE.as_path())
            .with_context(|| format!("{}", ALLIUM_DATABASE.display()))?;
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
        Migrations::new(vec![
M::up("
CREATE TABLE IF NOT EXISTS games (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    image TEXT,
    play_count INTEGER NOT NULL,
    play_time INTEGER NOT NULL,
    last_played INTEGER NOT NULL
);"),
M::up("
CREATE VIRTUAL TABLE games_fts USING fts5(name, path, content='games', content_rowid='id');

CREATE TRIGGER games_fts_ai AFTER INSERT ON games BEGIN
    INSERT INTO games_fts(rowid, name, path) VALUES (new.id, new.name, new.path);
END;
CREATE TRIGGER games_fts_ad AFTER DELETE ON games BEGIN
    INSERT INTO games_fts(games_fts, rowid, name, path) VALUES ('delete', old.id, old.name, old.path);
END;
CREATE TRIGGER games_fts_au AFTER UPDATE ON games BEGIN
    INSERT INTO games_fts(games_fts, rowid, name, path) VALUES ('delete', old.id, old.name, old.path);
    INSERT INTO games_fts(rowid, name, path) VALUES (new.id, new.name, new.path);
END;"),
M::up("
CREATE TABLE IF NOT EXISTS guides (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    cursor INTEGER NOT NULL
);"),
        ])
    }

    pub fn delete_game(&self, path: &Path) -> Result<()> {
        let conn = self.conn.as_ref().unwrap();
        conn.execute(
            "DELETE FROM games WHERE path = ?; DELETE FROM games_search WHERE path = ?;",
            params![path.display().to_string(), path.display().to_string()],
        )?;
        Ok(())
    }

    pub fn update_games(&self, games: &[Game]) -> Result<()> {
        let conn = self.conn.as_ref().unwrap();
        let mut stmt = conn.prepare("
INSERT INTO games (name, path, image, play_count, play_time, last_played)
VALUES (?, ?, ?, ?, ?, ?)
ON CONFLICT(path) DO UPDATE SET name = ?, image = ?, play_count = ?, play_time = ?, last_played = ?")?;

        for game in games {
            let path = game.path.display().to_string();
            let image = game.image.as_ref().map(|p| p.display().to_string());
            stmt.execute(params![
                game.name,
                path,
                image,
                game.play_count,
                game.play_time.num_seconds(),
                game.last_played,
                game.name,
                image,
                game.play_count,
                game.play_time.num_seconds(),
                game.last_played
            ])?;
        }

        Ok(())
    }

    /// Selects played games sorted by most play time first.
    pub fn select_most_played(&self, limit: i64) -> Result<Vec<Game>> {
        let mut stmt = self.conn.as_ref().unwrap().prepare("SELECT name, path, image, play_count, play_time, last_played FROM games WHERE play_time > 0 ORDER BY play_time DESC LIMIT ?")?;

        let rows = stmt.query_map([limit], |row| {
            Ok(Game {
                name: row.get(0)?,
                path: PathBuf::from(row.get::<_, String>(1)?),
                image: row.get::<_, Option<String>>(2)?.map(PathBuf::from),
                play_count: row.get(3)?,
                play_time: Duration::seconds(row.get(4)?),
                last_played: row.get(5)?,
            })
        })?;

        let mut games = Vec::new();
        for row in rows {
            games.push(row?);
        }

        Ok(games)
    }

    /// Selects played games sorted by last played first.
    pub fn select_last_played(&self, limit: i64) -> Result<Vec<Game>> {
        let mut stmt = self.conn.as_ref().unwrap().prepare("SELECT name, path, image, play_count, play_time, last_played FROM games WHERE last_played > 0 ORDER BY last_played DESC LIMIT ?")?;

        let rows = stmt.query_map([limit], |row| {
            Ok(Game {
                name: row.get(0)?,
                path: PathBuf::from(row.get::<_, String>(1)?),
                image: row.get::<_, Option<String>>(2)?.map(PathBuf::from),
                play_count: row.get(3)?,
                play_time: Duration::seconds(row.get(4)?),
                last_played: row.get(5)?,
            })
        })?;

        let mut games = Vec::new();
        for row in rows {
            games.push(row?);
        }

        Ok(games)
    }

    /// Search for games by name. The query is a prefix search on words, so "Fi" will match both "Fire Emblem" and "Pokemon Fire Red".
    pub fn search(&self, query: &str, limit: i64) -> Result<Vec<Game>> {
        let mut stmt = self.conn.as_ref().unwrap().prepare("SELECT games.name, games.path, image, play_count, play_time, last_played FROM games JOIN games_fts ON games.id = games_fts.rowid WHERE games_fts.name MATCH ? LIMIT ?")?;

        let rows = stmt.query_map(params![format!("{}*", query), limit], |row| {
            Ok(Game {
                name: row.get(0)?,
                path: PathBuf::from(row.get::<_, String>(1)?),
                image: row.get::<_, Option<String>>(2)?.map(PathBuf::from),
                play_count: row.get(3)?,
                play_time: Duration::seconds(row.get(4)?),
                last_played: row.get(5)?,
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
            "SELECT name, path, image, play_count, play_time, last_played FROM games WHERE path = ? LIMIT 1",
       [path], |row| {
            Ok(Game {
                name: row.get(0)?,
                path: PathBuf::from(row.get::<_, String>(1)?),
                image: row.get::<_, Option<String>>(2)?.map(PathBuf::from),
                play_count: row.get(3)?,
                play_time: Duration::seconds(row.get(4)?),
                last_played: row.get(5)?,
            })
        }).optional()?;

        Ok(game)
    }

    /// Increment the play count of a game, inserting a new row if it doesn't exist.
    pub fn increment_play_count(
        &self,
        name: &str,
        path: &Path,
        image: Option<&Path>,
    ) -> Result<()> {
        let mut stmt = self.conn.as_ref().unwrap().prepare(
            "
INSERT INTO games (name, path, image, play_count, play_time, last_played)
VALUES (?, ?, ?, ?, ?, ?)
ON CONFLICT(path) DO UPDATE SET play_count = play_count + 1;",
        )?;
        stmt.execute(params![
            name,
            path.display().to_string(),
            image.map(|p| p.display().to_string()),
            1,
            0,
            0
        ])?;

        let mut stmt = self.conn.as_ref().unwrap().prepare(
            "UPDATE games SET last_played = (SELECT MAX(last_played) FROM games) + 1 WHERE path = ?",
        )?;
        stmt.execute([path.display().to_string()])?;

        Ok(())
    }

    /// Increases the play time of a game. Does nothing if the game doesn't exist.
    pub fn add_play_time(&self, path: &Path, play_time: Duration) -> Result<()> {
        let mut stmt = self
            .conn
            .as_ref()
            .unwrap()
            .prepare("UPDATE games SET play_time = play_time + ? WHERE path = ?")?;

        stmt.execute(params![play_time.num_seconds(), path.display().to_string()])?;

        Ok(())
    }

    pub fn get_guide_cursor(&self, path: &Path) -> Result<u64> {
        let mut stmt = self
            .conn
            .as_ref()
            .unwrap()
            .prepare("SELECT cursor FROM guides WHERE path = ?")?;

        let cursor = stmt
            .query_row([path.display().to_string()], |row| row.get(0))
            .optional()?;

        Ok(cursor.unwrap_or(0))
    }

    pub fn update_guide_cursor(&self, path: &Path, cursor: u64) -> Result<()> {
        let mut stmt = self
            .conn
            .as_ref()
            .unwrap()
            .prepare("INSERT INTO guides (path, cursor) VALUES (?, ?) ON CONFLICT(path) DO UPDATE SET cursor = ?")?;

        stmt.execute(params![path.display().to_string(), cursor, cursor])?;

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
                name: "Game One".to_string(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                play_count: 0,
                play_time: Duration::zero(),
                last_played: 0,
            },
            Game {
                name: "Game Two".to_string(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                play_count: 0,
                play_time: Duration::zero(),
                last_played: 0,
            },
        ];

        database.update_games(&games).unwrap();

        database
            .increment_play_count(
                &games[1].name,
                games[1].path.as_path(),
                games[1].image.as_deref(),
            )
            .unwrap();
        database
            .add_play_time(games[1].path.as_path(), Duration::seconds(1))
            .unwrap();
        let most_played = database.select_most_played(2).unwrap();
        assert_eq!(most_played.len(), 1);
        assert_eq!(most_played[0].path, games[1].path);

        database
            .increment_play_count(
                &games[0].name,
                games[0].path.as_path(),
                games[0].image.as_deref(),
            )
            .unwrap();
        database
            .add_play_time(games[0].path.as_path(), Duration::seconds(2))
            .unwrap();
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
                name: "Game One".to_string(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                play_count: 0,
                play_time: Duration::zero(),
                last_played: 0,
            },
            Game {
                name: "Game Two".to_string(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                play_count: 0,
                play_time: Duration::zero(),
                last_played: 0,
            },
        ];

        database.update_games(&games).unwrap();

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

    #[test]
    fn test_search() {
        let database = Database::in_memory().unwrap();

        let games = vec![
            Game {
                name: "Game One".to_string(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                play_count: 0,
                play_time: Duration::zero(),
                last_played: 0,
            },
            Game {
                name: "Game Two".to_string(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                play_count: 0,
                play_time: Duration::zero(),
                last_played: 0,
            },
        ];

        database.update_games(&games).unwrap();

        let results = database.search("Game", 100).unwrap();
        assert_eq!(results.len(), 2);

        let results = database.search("One", 100).unwrap();
        assert_eq!(results[0].path, games[0].path);

        let results = database.search("Game One", 100).unwrap();
        assert_eq!(results[0].path, games[0].path);

        let results = database.search("Ga", 100).unwrap();
        assert_eq!(results[0].path, games[0].path);
    }
}
