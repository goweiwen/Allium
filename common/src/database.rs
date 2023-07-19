use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::{Context, Result};
use chrono::Duration;
use log::info;
use rusqlite::{params, Connection, OptionalExtension, Row};
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
    pub core: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewGame {
    pub name: String,
    pub path: PathBuf,
    pub image: Option<PathBuf>,
    pub core: Option<String>,
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
M::up("
CREATE TABLE IF NOT EXISTS key_value (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);"),
M::up("
ALTER TABLE games ADD COLUMN core TEXT;
"),
        ])
    }

    pub fn reset_game(&self, path: &Path) -> Result<()> {
        self.conn.as_ref().unwrap().execute(
            "UPDATE games SET play_count = 0, play_time = 0, last_played = 0 WHERE path = ?",
            params![path.display().to_string()],
        )?;
        Ok(())
    }

    pub fn update_game_path(&self, old: &Path, new: &Path) -> Result<()> {
        let mut stmt = self
            .conn
            .as_ref()
            .unwrap()
            .prepare("UPDATE games SET path = ? WHERE path = ?")?;
        stmt.execute(params![
            new.display().to_string(),
            old.display().to_string()
        ])?;
        Ok(())
    }

    pub fn update_games(&self, games: &[NewGame]) -> Result<()> {
        let mut stmt = self.conn.as_ref().unwrap().prepare(
            "
INSERT INTO games (name, path, image, play_count, play_time, last_played, core)
VALUES (?, ?, ?, 0, 0, 0, ?)
ON CONFLICT(path) DO UPDATE SET name = ?, image = ?, core = ?",
        )?;

        for game in games {
            let path = game.path.display().to_string();
            let image = game.image.as_ref().map(|p| p.display().to_string());
            stmt.execute(params![
                game.name, path, image, game.core, game.name, image, game.core
            ])?;
        }

        Ok(())
    }

    /// Selects played games sorted by most play time first.
    pub fn select_most_played(&self, limit: i64) -> Result<Vec<Game>> {
        let mut stmt = self
            .conn
            .as_ref()
            .unwrap()
            .prepare("SELECT name, path, image, play_count, play_time, last_played, core FROM games WHERE last_played > 0 ORDER BY play_time DESC LIMIT ?")?;

        let rows = stmt.query_map([limit], map_game)?;

        let mut games = Vec::new();
        for row in rows {
            games.push(row?);
        }

        Ok(games)
    }

    /// Selects played games sorted by last played first.
    pub fn select_last_played(&self, limit: i64) -> Result<Vec<Game>> {
        let mut stmt = self
            .conn
            .as_ref()
            .unwrap()
            .prepare("SELECT name, path, image, play_count, play_time, last_played, core FROM games WHERE last_played > 0 ORDER BY last_played DESC LIMIT ?")?;

        let rows = stmt.query_map([limit], map_game)?;

        let mut games = Vec::new();
        for row in rows {
            games.push(row?);
        }

        Ok(games)
    }

    /// Selects random games.
    pub fn select_random(&self, limit: i64) -> Result<Vec<Game>> {
        let mut stmt = self
            .conn
            .as_ref()
            .unwrap()
            .prepare("SELECT name, path, image, play_count, play_time, last_played, core FROM games WHERE id IN (SELECT id FROM games ORDER BY RANDOM() LIMIT ?)")?;

        let rows = stmt.query_map([limit], map_game)?;

        let mut games = Vec::new();
        for row in rows {
            games.push(row?);
        }

        Ok(games)
    }

    /// Search for games by name. The query is a prefix search on words, so "Fi" will match both "Fire Emblem" and "Pokemon Fire Red".
    pub fn search(&self, query: &str, limit: i64) -> Result<Vec<Game>> {
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.conn.as_ref().unwrap();

        let mut stmt = conn.prepare("SELECT games.name, games.path, image, play_count, play_time, last_played, core FROM games JOIN games_fts ON games.id = games_fts.rowid WHERE games_fts.name MATCH ? LIMIT ?")?;

        let rows = stmt.query_map(params![format!("{}*", query), limit], map_game)?;

        let mut games = Vec::new();
        for row in rows {
            games.push(row?);
        }

        Ok(games)
    }

    pub fn select_game(&self, path: &str) -> Result<Option<Game>> {
        let game = self
            .conn
            .as_ref()
            .unwrap()
            .query_row("SELECT name, path, image, play_count, play_time, last_played, core FROM games WHERE path = ? LIMIT 1", [path], map_game)
            .optional()?;

        Ok(game)
    }

    pub fn select_games(&self, paths: &[&Path]) -> Result<Vec<Option<Game>>> {
        let mut stmt = self
            .conn
            .as_ref()
            .unwrap()
            .prepare("SELECT name, path, image, play_count, play_time, last_played, core FROM games WHERE path = ? LIMIT 1")?;

        let mut results = vec![None; paths.len()];
        for (i, path) in paths.iter().enumerate() {
            let game = stmt
                .query_row(params![path.display().to_string()], map_game)
                .optional()?;

            results[i] = game;
        }

        Ok(results)
    }

    /// Increment the play count of a game, inserting a new row if it doesn't exist.
    pub fn increment_play_count(
        &self,
        name: &str,
        path: &Path,
        image: Option<&Path>,
    ) -> Result<()> {
        self.conn.as_ref().unwrap().execute(
            "
INSERT INTO games (name, path, image, play_count, play_time, last_played, core)
VALUES (?, ?, ?, ?, ?, ?, ?)
ON CONFLICT(path) DO UPDATE SET play_count = play_count + 1;",
            params![
                name,
                path.display().to_string(),
                image.map(|p| p.display().to_string()),
                1,
                0,
                0,
                None::<String>,
            ],
        )?;

        self.conn.as_ref().unwrap().execute(
            "UPDATE games SET last_played = (SELECT MAX(last_played) FROM games) + 1 WHERE path = ?",
        [path.display().to_string()])?;

        Ok(())
    }

    /// Increases the play time of a game. Does nothing if the game doesn't exist.
    pub fn add_play_time(&self, path: &Path, play_time: Duration) -> Result<()> {
        self.conn.as_ref().unwrap().execute(
            "UPDATE games SET play_time = play_time + ? WHERE path = ?",
            params![play_time.num_seconds(), path.display().to_string()],
        )?;

        Ok(())
    }

    pub fn get_guide_cursor(&self, path: &Path) -> Result<u64> {
        let cursor = self
            .conn
            .as_ref()
            .unwrap()
            .query_row(
                "SELECT cursor FROM guides WHERE path = ?",
                [path.display().to_string()],
                |row| row.get(0),
            )
            .optional()?;

        Ok(cursor.unwrap_or(0))
    }

    pub fn update_guide_cursor(&self, path: &Path, cursor: u64) -> Result<()> {
        self
            .conn
            .as_ref()
            .unwrap()
            .execute("INSERT INTO guides (path, cursor) VALUES (?, ?) ON CONFLICT(path) DO UPDATE SET cursor = ?", params![path.display().to_string(), cursor, cursor])?;

        Ok(())
    }

    /// Deletes all games that have no play time, play count, or core.
    pub fn clear(&self) -> Result<()> {
        self.conn.as_ref().unwrap().execute(
            "DELETE FROM games WHERE last_played = 0 AND play_time = 0 AND core = NULL",
            [],
        )?;

        Ok(())
    }

    pub fn set_has_indexed(&self, has_indexed: bool) -> Result<()> {
        self
            .conn
            .as_ref()
            .unwrap()
            .execute("INSERT INTO key_value (key, value) VALUES ('has_indexed', 1) ON CONFLICT(key) DO UPDATE SET value = ?", [if has_indexed { "1" } else {"0"}])?;

        Ok(())
    }

    pub fn has_indexed(&self) -> Result<bool> {
        let value = self
            .conn
            .as_ref()
            .unwrap()
            .query_row(
                "SELECT value FROM key_value WHERE key = 'has_indexed'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        Ok(matches!(value.as_deref(), Some("1")))
    }

    pub fn get_core(&self, path: &Path) -> Result<Option<String>> {
        let core = self
            .conn
            .as_ref()
            .unwrap()
            .query_row(
                "SELECT core FROM games WHERE path = ?",
                [path.display().to_string()],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();

        Ok(core)
    }

    pub fn set_core(&self, path: &Path, core: &str) -> Result<()> {
        self.conn.as_ref().unwrap().execute(
            "UPDATE games SET core = ? WHERE path = ?",
            params![core, path.display().to_string()],
        )?;

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
            NewGame {
                name: "Game One".to_string(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                core: None,
            },
            NewGame {
                name: "Game Two".to_string(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                core: None,
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
            NewGame {
                name: "Game One".to_string(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                core: None,
            },
            NewGame {
                name: "Game Two".to_string(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                core: None,
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
            NewGame {
                name: "Game One".to_string(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                core: None,
            },
            NewGame {
                name: "Game Two".to_string(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                core: None,
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

    #[test]
    fn test_select_games() {
        let database = Database::in_memory().unwrap();

        let games = vec![
            NewGame {
                name: "Game One".to_string(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                core: None,
            },
            NewGame {
                name: "Game Two".to_string(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                core: None,
            },
        ];

        database.update_games(&games).unwrap();

        let results = database
            .select_games(&[&games[0].path, &games[1].path])
            .unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].as_ref().map(|g| &g.path), Some(&games[0].path));
        assert_eq!(results[1].as_ref().map(|g| &g.path), Some(&games[1].path));

        let results = database.select_games(&[&games[1].path]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_ref().map(|g| &g.path), Some(&games[1].path));

        let fake_path = PathBuf::from("test_directory/Game Three.rom");
        let results = database
            .select_games(&[&games[0].path, &fake_path])
            .unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].as_ref().map(|g| &g.path), Some(&games[0].path));
        assert_eq!(results[1].as_ref().map(|g| &g.path), None);
    }

    #[test]
    fn test_set_core() -> Result<()> {
        let db = Database::in_memory().unwrap();

        let games = vec![
            NewGame {
                name: "Game One".to_string(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                core: None,
            },
            NewGame {
                name: "Game Two".to_string(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                core: None,
            },
        ];

        db.update_games(&games).unwrap();

        let core = db.get_core(&games[0].path)?;
        assert_eq!(core, None);

        db.set_core(&games[0].path, "CORE")?;

        let core = db.get_core(&games[0].path)?;
        assert_eq!(core, Some("CORE".to_string()));

        Ok(())
    }
}

fn map_game(row: &Row<'_>) -> rusqlite::Result<Game> {
    Ok(Game {
        name: row.get(0)?,
        path: PathBuf::from(row.get::<_, String>(1)?),
        image: row.get::<_, Option<String>>(2)?.map(PathBuf::from),
        play_count: row.get(3)?,
        play_time: Duration::seconds(row.get(4)?),
        last_played: row.get(5)?,
        core: row.get(6)?,
    })
}
