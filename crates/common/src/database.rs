use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::{Context, Result};
use chrono::{Duration, NaiveDate};
use log::{info, trace};
use rusqlite::{Connection, OptionalExtension, Row, params};
use rusqlite_migration::{M, Migrations};

use crate::constants::{ALLIUM_BASE_DIR, ALLIUM_DATABASE};

#[derive(Debug, Clone, Default)]
pub struct Database {
    conn: Option<Rc<Connection>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    pub name: String,
    pub path: PathBuf,
    pub image: Option<PathBuf>,
    pub play_count: i64,
    pub play_time: Duration,
    pub last_played: i64,
    pub core: Option<String>,
    pub rating: Option<u8>,
    pub release_date: Option<NaiveDate>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub genres: Vec<String>,
    pub favorite: bool,
    pub screenshot_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewGame {
    pub name: String,
    pub path: PathBuf,
    pub image: Option<PathBuf>,
    pub core: Option<String>,
    pub rating: Option<u8>,
    pub release_date: Option<NaiveDate>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub genres: Vec<String>,
    pub favorite: bool,
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
        M::up("
CREATE TABLE IF NOT EXISTS directories (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    gamelist_fingerprint INTEGER
);"),
        M::up("
ALTER TABLE games ADD COLUMN rating INTEGER;
ALTER TABLE games ADD COLUMN release_date STRING;
"),
        M::up("
ALTER TABLE games ADD COLUMN developer STRING;
ALTER TABLE games ADD COLUMN publisher STRING;
ALTER TABLE games ADD COLUMN genres STRING NOT NULL DEFAULT '[]';
"),
        M::up("
CREATE TEMP TABLE games_fts_backup AS SELECT * FROM games_fts;
DROP TABLE IF EXISTS games_fts;
CREATE VIRTUAL TABLE games_fts USING fts5(name, path, developer, publisher, content='games', content_rowid='id');
INSERT INTO games_fts (rowid, name, path) SELECT rowid, name, path FROM games_fts_backup;

DROP TRIGGER IF EXISTS games_fts_ai;
CREATE TRIGGER games_fts_ai AFTER INSERT ON games BEGIN
    INSERT INTO games_fts(rowid, name, path, developer, publisher) VALUES (new.id, new.name, new.path, new.developer, new.publisher);
END;

DROP TRIGGER IF EXISTS games_fts_ad;
CREATE TRIGGER games_fts_ad AFTER DELETE ON games BEGIN
    INSERT INTO games_fts(games_fts, rowid, name, path, developer, publisher) VALUES ('delete', old.id, old.name, old.path, old.developer, old.publisher);
END;

DROP TRIGGER IF EXISTS games_fts_au;
CREATE TRIGGER games_fts_au AFTER UPDATE ON games BEGIN
    INSERT INTO games_fts(games_fts, rowid, name, path, developer, publisher) VALUES ('delete', old.id, old.name, old.path, old.developer, old.publisher);
    INSERT INTO games_fts(rowid, name, path, developer, publisher) VALUES (new.id, new.name, new.path, new.developer, new.publisher);
END;"),
        M::up("
ALTER TABLE games ADD COLUMN favorite INTEGER NOT NULL DEFAULT 0;
"),
        M::up("
ALTER TABLE games ADD COLUMN screenshot_path TEXT;
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

    pub fn update_screenshot_path(
        &self,
        game_path: &Path,
        screenshot_path: Option<&Path>,
    ) -> Result<()> {
        self.conn.as_ref().unwrap().execute(
            "UPDATE games SET screenshot_path = ? WHERE path = ?",
            params![
                screenshot_path.map(|p| p.display().to_string()),
                game_path.display().to_string()
            ],
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
        let tx = self.conn.as_ref().unwrap().unchecked_transaction()?; // safe because single-threaded

        let mut stmt = tx.prepare(
            "
INSERT INTO games (name, path, image, play_count, play_time, last_played, core, rating, release_date, developer, publisher, genres)
VALUES (?, ?, ?, 0, 0, 0, ?, ?, ?, ?, ?, ?)
ON CONFLICT(path) DO UPDATE SET name = ?, image = ?, core = ?, rating = ?, release_date = ?, developer = ?, publisher = ?, genres = ?",
        )?;

        for game in games {
            let path = game.path.display().to_string();
            let image = game.image.as_ref().map(|p| p.display().to_string());
            let genres = serde_json::to_string(&game.genres)?;
            stmt.execute(params![
                game.name,
                path,
                image,
                game.core,
                game.rating,
                game.release_date,
                game.developer,
                game.publisher,
                genres,
                game.name,
                image,
                game.core,
                game.rating,
                game.release_date,
                game.developer,
                game.publisher,
                genres,
            ])?;
        }

        drop(stmt);

        tx.commit()?;

        Ok(())
    }

    /// Selects played games sorted by most play time first.
    pub fn select_most_played(&self, limit: i64) -> Result<Vec<Game>> {
        let mut stmt = self
            .conn
            .as_ref()
            .unwrap()
            .prepare("SELECT name, path, image, play_count, play_time, last_played, core, rating, release_date, developer, publisher, genres, favorite, screenshot_path FROM games WHERE last_played > 0 ORDER BY play_time DESC LIMIT ?")?;

        let results = stmt
            .query_map([limit], map_game)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Selects played games sorted by last played first.
    pub fn select_last_played(&self, limit: i64) -> Result<Vec<Game>> {
        let mut stmt = self
            .conn
            .as_ref()
            .unwrap()
            .prepare("SELECT name, path, image, play_count, play_time, last_played, core, rating, release_date, developer, publisher, genres, favorite, screenshot_path FROM games WHERE last_played > 0 ORDER BY last_played DESC LIMIT ?")?;

        let results = stmt
            .query_map([limit], map_game)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Selects played games sorted by highest rating first.
    pub fn select_by_rating(&self, limit: i64) -> Result<Vec<Game>> {
        let mut stmt = self
            .conn
            .as_ref()
            .unwrap()
            .prepare("SELECT name, path, image, play_count, play_time, last_played, core, rating, release_date, developer, publisher, genres, favorite, screenshot_path FROM games ORDER BY rating DESC LIMIT ?")?;

        let results = stmt
            .query_map([limit], map_game)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Selects played games sorted by release date (earliest first).
    pub fn select_by_release_date(&self, limit: i64) -> Result<Vec<Game>> {
        let mut stmt = self
            .conn
            .as_ref()
            .unwrap()
            .prepare("SELECT name, path, image, play_count, play_time, last_played, core, rating, release_date, developer, publisher, genres, favorite, screenshot_path FROM games ORDER BY release_date DESC LIMIT ?")?;

        let results = stmt
            .query_map([limit], map_game)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Selects random games.
    pub fn select_random(&self, limit: i64) -> Result<Vec<Game>> {
        let mut stmt = self
            .conn
            .as_ref()
            .unwrap()
            .prepare("SELECT name, path, image, play_count, play_time, last_played, core, rating, release_date, developer, publisher, genres, favorite, screenshot_path FROM games WHERE id IN (SELECT id FROM games ORDER BY RANDOM() LIMIT ?)")?;

        let results = stmt
            .query_map([limit], map_game)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Selects favorite games.
    pub fn select_favorites(&self, limit: i64) -> Result<Vec<Game>> {
        let mut stmt = self
            .conn
            .as_ref()
            .unwrap()
            .prepare("SELECT name, path, image, play_count, play_time, last_played, core, rating, release_date, developer, publisher, genres, favorite, screenshot_path FROM games WHERE favorite = 1 ORDER BY last_played DESC LIMIT ?")?;

        let results = stmt
            .query_map([limit], map_game)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Search for games by name. The query is a prefix search on words, so "Fi" will match both "Fire Emblem" and "Pokemon Fire Red".
    pub fn search(&self, query: &str, limit: i64) -> Result<Vec<Game>> {
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.conn.as_ref().unwrap();

        let mut stmt = conn.prepare("SELECT games.name, games.path, image, play_count, play_time, last_played, core, rating, release_date, games.developer, games.publisher, genres, favorite, screenshot_path FROM games JOIN games_fts ON games.id = games_fts.rowid WHERE games_fts MATCH ? LIMIT ?")?;

        let query =
            format!("name:\"{query}\" * OR developer:\"{query}\" * OR publisher:\"{query}\" *");
        let results = stmt
            .query_map(params![query, limit], map_game)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    pub fn select_games_in_directory(&self, path: &Path) -> Result<Vec<Game>> {
        trace!("select_games_in_directory({:?})", path);
        let conn = self.conn.as_ref().unwrap();

        let mut stmt = conn.prepare("SELECT games.name, games.path, image, play_count, play_time, last_played, core, rating, release_date, games.developer, games.publisher, genres, favorite, screenshot_path FROM games JOIN games_fts ON games.id = games_fts.rowid WHERE games_fts.path LIKE ? AND games_fts.path NOT LIKE ?")?;

        let results = stmt
            .query_map(
                params![
                    format!("{}/%", path.display().to_string()),
                    format!("{}/%/%", path.display().to_string())
                ],
                map_game,
            )?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    pub fn select_game(&self, path: &Path) -> Result<Option<Game>> {
        let game = self
            .conn
            .as_ref()
            .unwrap()
            .query_row("SELECT name, path, image, play_count, play_time, last_played, core, rating, release_date, developer, publisher, genres, favorite, screenshot_path FROM games WHERE path = ? LIMIT 1", [path.display().to_string()], map_game)
            .optional()?;

        Ok(game)
    }

    pub fn select_games(&self, paths: &[&Path]) -> Result<Vec<Option<Game>>> {
        let mut stmt = self
            .conn
            .as_ref()
            .unwrap()
            .prepare("SELECT name, path, image, play_count, play_time, last_played, core, rating, release_date, developer, publisher, genres, favorite, screenshot_path FROM games WHERE path = ? ORDER BY favorite DESC")?;

        let mut results = vec![None; paths.len()];
        for (i, path) in paths.iter().enumerate() {
            let game = stmt
                .query_row(params![path.display().to_string()], map_game)
                .optional()?;

            results[i] = game;
        }

        Ok(results)
    }

    pub fn select_all_games(&self) -> Result<Vec<Game>> {
        let mut stmt = self.conn.as_ref().unwrap().prepare(
            "SELECT name, path, image, play_count, play_time, last_played, core, rating, release_date, developer, publisher, genres, favorite, screenshot_path FROM games",
        )?;

        let results = stmt
            .query_map([], map_game)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Increment the play count of a game, inserting a new row if it doesn't exist.
    pub fn increment_play_count(&self, game: &NewGame) -> Result<()> {
        self.conn.as_ref().unwrap().execute(
            "
INSERT INTO games (name, path, image, play_count, play_time, last_played, core, rating, release_date)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
ON CONFLICT(path) DO UPDATE SET play_count = play_count + 1;",
            params![
                game.name,
                game.path.display().to_string(),
                game.image.as_ref().map(|p| p.display().to_string()),
                1,
                0,
                0,
                game.core,
                game.rating,
                game.release_date
            ],
        )?;

        self.conn.as_ref().unwrap().execute(
            "UPDATE games SET last_played = (SELECT MAX(last_played) FROM games) + 1 WHERE path = ?",
        [game.path.display().to_string()])?;

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

    /// Sets whether a game is a favorite.
    pub fn set_favorite(&self, path: &Path, favorite: bool) -> Result<()> {
        self.conn.as_ref().unwrap().execute(
            "UPDATE games SET favorite = ? WHERE path = ?",
            params![if favorite { 1 } else { 0 }, path.display().to_string()],
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

    /// Deletes a game from the database.
    pub fn delete_game(&self, path: &Path) -> Result<()> {
        self.conn.as_ref().unwrap().execute(
            "DELETE FROM games WHERE path = ?",
            [path.display().to_string()],
        )?;

        Ok(())
    }

    /// Deletes all games that have no play time, play count.
    pub fn delete_all_unplayed_games(&self) -> Result<()> {
        self.conn.as_ref().unwrap().execute(
            "DELETE FROM games WHERE last_played = 0 AND play_time = 0",
            [],
        )?;

        Ok(())
    }

    /// Deletes all directories
    pub fn delete_all_directories(&self) -> Result<()> {
        self.conn
            .as_ref()
            .unwrap()
            .execute("DELETE FROM directories", [])?;

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

    pub fn set_gamelist_fingerprint(&self, path: &Path, fingerprint: u64) -> Result<()> {
        trace!("set_gamelist_fingerprint({:?}, {})", path, fingerprint);
        self.conn.as_ref().unwrap().execute(
            "INSERT INTO directories (path, gamelist_fingerprint) VALUES (?, ?) ON CONFLICT(path) DO UPDATE SET gamelist_fingerprint = ?",
            params![path.display().to_string(), fingerprint, fingerprint],
        )?;

        Ok(())
    }

    pub fn get_gamelist_fingerprint(&self, path: &Path) -> Result<Option<u64>> {
        trace!("get_gamelist_fingerprint({:?})", path);
        let fingerprint = self
            .conn
            .as_ref()
            .unwrap()
            .query_row(
                "SELECT gamelist_fingerprint FROM directories WHERE path = ?",
                [path.display().to_string()],
                |row| row.get::<_, Option<u64>>(0),
            )
            .optional()?
            .flatten();

        Ok(fingerprint)
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

fn map_game(row: &Row<'_>) -> rusqlite::Result<Game> {
    Ok(Game {
        name: row.get(0)?,
        path: PathBuf::from(row.get::<_, String>(1)?),
        image: row.get::<_, Option<String>>(2)?.map(PathBuf::from),
        play_count: row.get(3)?,
        play_time: Duration::seconds(row.get(4)?),
        last_played: row.get(5)?,
        core: row.get(6)?,
        rating: row.get(7)?,
        release_date: row.get(8)?,
        developer: row.get(9)?,
        publisher: row.get(10)?,
        genres: serde_json::from_str(&row.get::<_, String>(11)?).unwrap(),
        favorite: row.get::<_, i64>(12)? != 0,
        screenshot_path: row.get::<_, Option<String>>(13)?.map(PathBuf::from),
    })
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
                name: "Game One".to_owned(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                core: None,
                rating: None,
                release_date: None,
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
            },
            NewGame {
                name: "Game Two".to_owned(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                core: None,
                rating: None,
                release_date: None,
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
            },
        ];

        database.update_games(&games).unwrap();

        database.increment_play_count(&games[1]).unwrap();
        database
            .add_play_time(games[1].path.as_path(), Duration::seconds(1))
            .unwrap();
        let most_played = database.select_most_played(2).unwrap();
        assert_eq!(most_played.len(), 1);
        assert_eq!(most_played[0].path, games[1].path);

        database.increment_play_count(&games[0]).unwrap();
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
                name: "Game One".to_owned(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                core: None,
                rating: None,
                release_date: None,
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
            },
            NewGame {
                name: "Game Two".to_owned(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                core: None,
                rating: None,
                release_date: None,
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
            },
        ];

        database.update_games(&games).unwrap();

        for _ in 0..2 {
            database.increment_play_count(&games[1]).unwrap();
        }
        let last_played = database.select_last_played(2).unwrap();
        assert_eq!(last_played.len(), 1);
        assert_eq!(last_played[0].path, games[1].path);

        database.increment_play_count(&games[0]).unwrap();
        let last_played = database.select_last_played(2).unwrap();
        assert_eq!(last_played.len(), 2);
        assert_eq!(last_played[0].path, games[0].path);
        assert_eq!(last_played[1].path, games[1].path);
    }

    #[test]
    fn test_by_rating() {
        let database = Database::in_memory().unwrap();

        let games = vec![
            NewGame {
                name: "Game One".to_owned(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                core: None,
                rating: Some(5),
                release_date: None,
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
            },
            NewGame {
                name: "Game Two".to_owned(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                core: None,
                rating: Some(9),
                release_date: None,
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
            },
        ];

        database.update_games(&games).unwrap();

        let by_rating = database.select_by_rating(2).unwrap();
        assert_eq!(by_rating.len(), 2);
        assert_eq!(by_rating[0].path, games[1].path);
        assert_eq!(by_rating[1].path, games[0].path);

        database
            .update_games(&[NewGame {
                name: "Game Two".to_owned(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                core: None,
                rating: Some(1),
                release_date: None,
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
            }])
            .unwrap();
        let by_rating = database.select_by_rating(2).unwrap();
        assert_eq!(by_rating.len(), 2);
        assert_eq!(by_rating[0].path, games[0].path);
        assert_eq!(by_rating[1].path, games[1].path);
    }

    #[test]
    fn test_by_release_date() {
        let database = Database::in_memory().unwrap();

        let games = vec![
            NewGame {
                name: "Game One".to_owned(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                core: None,
                rating: None,
                release_date: Some(NaiveDate::from_ymd_opt(2023, 1, 1).unwrap()),
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
            },
            NewGame {
                name: "Game Two".to_owned(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                core: None,
                rating: None,
                release_date: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
            },
        ];

        database.update_games(&games).unwrap();

        let by_release_date = database.select_by_release_date(2).unwrap();
        assert_eq!(by_release_date.len(), 2);
        assert_eq!(by_release_date[0].path, games[1].path);
        assert_eq!(by_release_date[1].path, games[0].path);

        database
            .update_games(&[NewGame {
                name: "Game Two".to_owned(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                core: None,
                rating: None,
                release_date: Some(NaiveDate::from_ymd_opt(2022, 1, 1).unwrap()),
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
            }])
            .unwrap();
        let by_release_date = database.select_by_release_date(2).unwrap();
        assert_eq!(by_release_date.len(), 2);
        assert_eq!(by_release_date[0].path, games[0].path);
        assert_eq!(by_release_date[1].path, games[1].path);
    }

    #[test]
    fn test_search() {
        let database = Database::in_memory().unwrap();

        let games = vec![
            NewGame {
                name: "Game One".to_owned(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                core: None,
                rating: None,
                release_date: None,
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
            },
            NewGame {
                name: "Game Two".to_owned(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                core: None,
                rating: None,
                release_date: None,
                developer: Some("Square Enix".to_owned()),
                publisher: Some("Nintendo".to_owned()),
                genres: Vec::new(),
                favorite: false,
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

        let results = database.search("square enix", 100).unwrap();
        assert_eq!(results[0].path, games[1].path);

        let results = database.search("nintendo", 100).unwrap();
        assert_eq!(results[0].path, games[1].path);
    }

    #[test]
    fn test_select_games() {
        let database = Database::in_memory().unwrap();

        let games = vec![
            NewGame {
                name: "Game One".to_owned(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                core: None,
                rating: None,
                release_date: None,
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
            },
            NewGame {
                name: "Game Two".to_owned(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                core: None,
                rating: None,
                release_date: None,
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
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
    fn test_select_games_in_directory() {
        let database = Database::in_memory().unwrap();

        let games = vec![
            NewGame {
                name: "Game One".to_owned(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                core: None,
                rating: None,
                release_date: None,
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
            },
            NewGame {
                name: "Game Two".to_owned(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                core: None,
                rating: None,
                release_date: None,
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
            },
            NewGame {
                name: "Game Three".to_owned(),
                path: PathBuf::from("different_directory/Game Three.rom"),
                image: Some(PathBuf::from("different_directory/Imgs/Game Three.png")),
                core: None,
                rating: None,
                release_date: None,
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
            },
        ];

        database.update_games(&games).unwrap();

        let results = database
            .select_games_in_directory(Path::new("test_directory"))
            .unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].path, games[0].path);
        assert_eq!(results[1].path, games[1].path);

        let results = database
            .select_games_in_directory(Path::new("test_directory/Imgs"))
            .unwrap();
        assert_eq!(results.len(), 0);

        let results = database
            .select_games_in_directory(Path::new("different_directory"))
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, games[2].path);

        let results = database
            .select_games_in_directory(Path::new("directory"))
            .unwrap();
        assert_eq!(results.len(), 0);

        let results = database
            .select_games_in_directory(Path::new("test"))
            .unwrap();
        assert_eq!(results.len(), 0);

        let results = database.select_games_in_directory(Path::new("")).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_set_core() -> Result<()> {
        let db = Database::in_memory().unwrap();

        let games = vec![
            NewGame {
                name: "Game One".to_owned(),
                path: PathBuf::from("test_directory/Game One.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
                core: None,
                rating: None,
                release_date: None,
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
            },
            NewGame {
                name: "Game Two".to_owned(),
                path: PathBuf::from("test_directory/Game Two.rom"),
                image: Some(PathBuf::from("test_directory/Imgs/Game Two.png")),
                core: None,
                rating: None,
                release_date: None,
                developer: None,
                publisher: None,
                genres: Vec::new(),
                favorite: false,
            },
        ];

        db.update_games(&games).unwrap();

        let core = db.get_core(&games[0].path)?;
        assert_eq!(core, None);

        db.set_core(&games[0].path, "CORE")?;

        let core = db.get_core(&games[0].path)?;
        assert_eq!(core, Some("CORE".to_owned()));

        Ok(())
    }

    #[test]
    fn test_set_genres() -> Result<()> {
        let db = Database::in_memory().unwrap();

        let mut games = vec![NewGame {
            name: "Game One".to_owned(),
            path: PathBuf::from("test_directory/Game One.rom"),
            image: Some(PathBuf::from("test_directory/Imgs/Game One.png")),
            core: None,
            rating: None,
            release_date: None,
            developer: None,
            publisher: None,
            genres: vec!["Action".to_owned(), "Adventure".to_owned()],
            favorite: false,
        }];

        db.update_games(&games).unwrap();
        let game = db.select_game(&games[0].path)?.unwrap();
        assert_eq!(
            game.genres,
            vec!["Action".to_owned(), "Adventure".to_owned()]
        );

        games[0].genres = Vec::new();
        db.update_games(&games).unwrap();
        let game = db.select_game(&games[0].path)?.unwrap();
        assert!(game.genres.is_empty());

        games[0].genres = vec!["Puzzle".to_owned()];
        db.update_games(&games).unwrap();
        let game = db.select_game(&games[0].path)?.unwrap();
        assert_eq!(game.genres, vec!["Puzzle".to_owned()]);

        Ok(())
    }
}
