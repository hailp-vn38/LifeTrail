//! Server boundary for LifeTrail Phase 1.
//!
//! HTTP, ingestion, persistence, and Daily View behavior are introduced by
//! their respective tickets. This crate owns the shared server foundation.

pub mod app;
pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod ingestion;
