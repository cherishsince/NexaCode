//! State management system
//!
//! This module provides a Redux-like state management architecture:
//! - `Action`: Enumerates all possible state mutations
//! - `State`: The central application state
//! - `Reducer`: Pure functions that handle state transitions
//! - `Store`: Combines state, reducer, and notification system

pub mod actions;
pub mod reducers;
pub mod history;
pub mod store;

pub use actions::{Action, InputAction, MessageAction, NavigationAction, UiAction};
pub use history::History;
pub use reducers::reduce;
pub use store::{Store, Subscriber};
