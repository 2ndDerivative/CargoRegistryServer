use std::path::Path;
use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::{
    publish::PublishedPackage, 
    config::CONFIG, 
    owners::UserResult
};

use self::error::AddOwnerError;

pub mod error;

fn connect() -> Result<Connection, rusqlite::Error> {
    Connection::open(&CONFIG.database.path)
}

pub(crate) fn add_owner(crate_name: &str, owner: &str) -> Result<(), AddOwnerError> {
    let mut con = connect()?;
    let con = Transaction::new(&mut con, TransactionBehavior::Deferred)?;
    match con.execute(
        "INSERT INTO ownerships(user, crate)
        SELECT userId, crateId 
        FROM users
        INNER JOIN crates 
        WHERE crates.name = ?2 AND users.name = ?1", (owner, crate_name)) {
            Ok(1) => Ok(con.commit()?),
            Ok(0) => Err(AddOwnerError::NoSuchUser),
            Ok(_) => Err(AddOwnerError::MultipleUsers),
            Err(e) => Err(AddOwnerError::SqlError(e))
        }
}

pub(crate) fn remove_owner(crate_name: &str, owner: &str) -> Result<(), rusqlite::Error> {
    let con = connect()?;
    con.execute(
        "DELETE FROM ownerships
        WHERE user IN (
            SELECT userId FROM users WHERE users.name = ?1
        )
        AND crate IN (
            SELECT crateId FROM crates WHERE crates.name = ?2
        )", (owner, crate_name))?;
    Ok(())
}

pub(crate) fn get_owners(crate_name: &str) -> Result<Vec<UserResult>, rusqlite::Error> {
    let con = connect()?;
    let mut query = con.prepare(
        "SELECT userId, users.name 
        FROM users 
        INNER JOIN ownerships ON user = ownerships.user
        INNER JOIN crates ON crateId = ownerships.crate 
        WHERE crates.name = ?1 AND user = userId")?;
    let it = query.query_map([crate_name], |f| {
        Ok(UserResult::new(
            f.get(0)?,
            f.get(1)?
        ))
    })?;
    it.collect()
}

#[allow(dead_code)]
pub(crate) fn add_user(user_name: &str) -> Result<(), rusqlite::Error> {
    let con = connect()?;
    con.execute(
        "INSERT INTO users (name) VALUES (?1)", (user_name, ))?;
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn get_all_users() -> Result<Vec<(usize, String)>, rusqlite::Error> {
    let con = connect()?;
    let mut query = con.prepare("SELECT userId, name FROM users")?;
    let it = query.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?;
    it.collect()
}

#[allow(dead_code)]
pub(crate) fn get_all_packages() -> Result<Vec<(String, String, usize)>, rusqlite::Error> {
    println!("Counting versions...");
    let con = connect()?;
    let mut query = con.prepare("SELECT version, description, crateId FROM versions")?;
    let it = query.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;
    it.collect()
}

pub(crate) fn add_package(package: &PublishedPackage) -> Result<(), rusqlite::Error> {
    let mut con = connect()?;
    let con = Transaction::new(&mut con, TransactionBehavior::Deferred)?;
    if !crate_is_in_db(&package.name)? {
        println!("A new crate has been added!");
        con.execute("INSERT INTO crates (name) VALUES (?1)", [&package.name])?;
    }
    let number_of_rows = con.execute(
        "INSERT INTO versions (version, description, documentation, homepage,
        readme, readme_file, license, license_file, repository, crateId)
        SELECT * FROM (
            (VALUES ((?1), (?2), (?3), (?4), (?5), (?6), (?7), (?8), (?9)))
            CROSS JOIN (SELECT crateId FROM crates WHERE crates.name = (?10))
        )", (   
            &package.vers, &package.description, &package.documentation, 
            &package.homepage, &package.readme, &package.readme_file, 
            &package.license, &package.license_file, &package.repository,
            &package.name))?;
    assert_eq!(number_of_rows, 1);
    con.commit()
}

pub(crate) fn crate_is_in_db(package: &str) -> Result<bool, rusqlite::Error> {
    let con = connect()?;
    let mut check_for_package = con.prepare("SELECT crateId FROM crates WHERE name=?1")?;
    let exists = check_for_package.query_map([&package], |row| {
        row.get::<usize, usize>(0)
    })?.count() > 0;
    Ok(exists)
}

pub(crate) fn get_description(crate_name: &str, version: &str) -> Result<Option<String>, rusqlite::Error> {
    let con = connect()?;
    let mut check_for_package = con.prepare(
        "SELECT description FROM versions
        INNER JOIN crates ON crates.crateId = versions.crateId
        WHERE name=?1 AND version=?2")?;
    let result = check_for_package.query_map([crate_name, version], |row| {
        row.get::<usize, Option<String>>(0)
    })?.next();
    match result {
        Some(x) => x,
        None => Ok(None)
    }
}

pub(crate) fn init(database_path: &Path) -> Result<(), rusqlite::Error> {
    let connection = Connection::open(database_path)?;
    connection.execute(
        "CREATE TABLE crates (
            crateId INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        )", 
        ()
    )?;
    connection.execute(
        "CREATE TABLE versions (
            versionId INTEGER PRIMARY KEY,
            crateId INTEGER NOT NULL,
            version TEXT NOT NULL,
            description TEXT,
            documentation TEXT,
            homepage TEXT,
            readme TEXT,
            readme_file TEXT,
            license TEXT,
            license_file TEXT,
            repository TEXT,
            FOREIGN KEY(crateId) REFERENCES crates(crateId)
        )", ())?;
    connection.execute(
        "CREATE TABLE users (
            userId INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        )", ())?;
    connection.execute(
        "CREATE TABLE ownerships (
            ownershipId INTEGER PRIMARY KEY,
            user INTEGER,
            crate INTEGER,
            FOREIGN KEY(user) REFERENCES users(userId),
            FOREIGN KEY(crate) REFERENCES crates(crateId)
        )", ())?;
    Ok(())
}