// The README's example is written with maud, so it is the crate's documentation
// only where it compiles.
#![cfg_attr(feature = "maud", doc = include_str!("../README.md"))]
#![cfg_attr(
    not(feature = "maud"),
    doc = "Server-rendered web apps with htmx and axum, where the URL alone decides what the browser shows."
)]

mod html;
mod url;
mod view;

pub use html::Html;
pub use view::{NoParent, Outlet, Parent, View, ViewPath, Views};
