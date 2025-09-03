//! Storage subsystem
//!
//! This module provides abstractions and implementations for persisting
//! sessions, interactions, and capture artifacts.
//!
//! Components:
//! - `storage_trait`: the Storage trait defining a uniform API.
//! - `types`: shared data types used by storage backends.
//! - `database_storage`: ORM-based SQLite implementation using SeaORM.
//! - `file_storage`: filesystem-backed implementation for simple persistence and inspection.
//! - `session_filter`: helpers to build session queries.
//! - `db_entities`: SeaORM entity models for the database backend.

pub mod database_storage;
pub mod db_entities;
pub mod file_storage;
pub mod session_filter;
pub mod storage_trait;
pub mod types;
