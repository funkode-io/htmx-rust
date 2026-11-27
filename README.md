# htmx-rust

Server-rendered Rust web apps with [htmx](https://htmx.org) and [axum](https://github.com/tokio-rs/axum), built on how the web already works: every URL names a resource, the server answers with HTML, and links and forms are the only way to move.

**Status: early. Views nest to any depth and are served as whole documents, and a view's fields are parsed from its URL and printed back as one; the rest of the design is written down, not built.**

```rust
use axum::Router;
use htmx_rust::{Html, NoParent, Outlet, ViewPath, View, Views};
use maud::{DOCTYPE, html};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct App;

/// The root view: the whole document, around its outlet. It has no parent
/// and no path.
#[derive(Default, Serialize, Deserialize)]
struct Document;

impl View for Document {
    type State = App;
    type Parent = NoParent;

    async fn render(_: &ViewPath<Self>, _: &App, outlet: Outlet) -> impl Into<Html> {
        html! {
            (DOCTYPE)
            title { (outlet.title) }
            main id=(outlet.id) { (outlet.content) }
        }
    }
}

/// What `/catalogs` shows, in the root view's outlet.
#[derive(Serialize, Deserialize)]
struct Catalogs;

impl View for Catalogs {
    type State = App;
    type Parent = Document;
    const PATH: &'static str = "/catalogs";
    const TITLE: &'static str = "Catalogs";

    async fn render(_: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        let teas = Catalog { catalog_id: 7, q: Some("tea".into()) };
        html! {
            h1 { "Catalogs" }
            // A link is the path of a view value, so it cannot point at a
            // route that does not exist: this one is `/catalogs/7?q=tea`.
            a href=(teas.path()) { "The teas of catalog 7" }
        }
    }
}

/// What `/catalogs/7?q=tea` shows. A view's fields are what its own part of
/// the URL carries.
#[derive(Serialize, Deserialize)]
struct Catalog {
    /// The `{catalog_id}` of its part of the path.
    catalog_id: u32,
    /// Any other field is in the query string.
    q: Option<String>,
}

impl View for Catalog {
    type State = App;
    type Parent = Document;
    const PATH: &'static str = "/catalogs/{catalog_id}";
    const TITLE: &'static str = "Catalog";

    /// A view renders at the path it is shown at, and reads its fields from it.
    async fn render(at: &ViewPath<Self>, _: &App, outlet: Outlet) -> impl Into<Html> {
        html! {
            h1 { "Catalog " (at.catalog_id) }
            // The path of a view nested in this one: `/catalogs/7/items/3?q=tea`.
            a href=(at.to(Item { item_id: 3 })) { "Its third item" }
            section id=(outlet.id) { (outlet.content) }
        }
    }
}

/// What `/catalogs/7/items/3` shows, in the outlet of the catalog's view: a
/// view names the view it is nested in, and its part of the path follows that
/// one's. It holds nothing of its parent.
#[derive(Serialize, Deserialize)]
struct Item {
    item_id: u32,
}

impl View for Item {
    type State = App;
    type Parent = Catalog;
    const PATH: &'static str = "/items/{item_id}";
    const TITLE: &'static str = "Item";

    async fn render(at: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        // The path above this one: its parent's fields, and the views beside it.
        let catalog = at.parent();
        html! {
            h2 { "Item " (at.item_id) " of catalog " (catalog.catalog_id) }
            a href=(catalog.to(Item { item_id: at.item_id + 1 })) { "Next" }
        }
    }
}

let router: Router = Router::new()
    .view::<Catalogs>()
    .view::<Catalog>()
    .view::<Item>()
    .with_state(App);

// A path starts at a view whose ancestors carry nothing (the root view is its
// `Default`) and goes down from there. It prints as the URL.
let catalog: ViewPath<Catalog> = Catalog { catalog_id: 7, q: None }.path();
let item: ViewPath<Item> = catalog.to(Item { item_id: 3 });
assert_eq!(catalog.to_string(), "/catalogs/7");
assert_eq!(item.to_string(), "/catalogs/7/items/3");
```

A view derives serde's `Serialize` and `Deserialize`: that is how its fields are read from a URL and printed back. A URL that does not parse into the fields, such as `/catalogs/seven`, is answered with `400 Bad Request` and no view runs.

Nesting is declared, never read from the path: `/catalogs/7` extends `/catalogs`, and is still shown beside it in the root view's outlet, because `Catalog` names `Document` as its parent. An outlet's id is made of the names of the views down to it (`document-outlet`, `document-catalog-outlet`, …): a `-` joins the views and a `_` joins the words of one view's name, so no two outlets of a document share one.

A URL is never written by hand. It is a `ViewPath`: a view value and, above it, the path of its parent. What the compiler refuses:

- `Item { item_id: 3 }.path()`: its parent, `Catalog`, needs a value. Only a view whose ancestors all derive `Default` starts a path.
- `Catalogs.path().to(Item { item_id: 3 })`: `Item` is not nested in `Catalogs`.

What stops the app when a view is registered: a parameter in a view's `PATH` with no field of that view, a field named as one of an ancestor's, and a nested view with no `PATH`. Every view on the way down is read from the same URL, so a name is used once on the way: `catalog_id` and `item_id`, not `id` twice.

A view is a resource with one address, its own: a view nested in another adds a part to the path. Only the root view has none, being the document around every other view. What needs no address is not a view: a function that draws markup, or a region.

## REST, as it was meant

REST describes the web of pages, links and forms. htmx-rust holds an app to its constraints, so that refresh, back and forward, sharing a link and caching work without anyone building them.

| REST constraint | In htmx-rust |
|---|---|
| **Stateless**: a request carries all that is needed to understand it | The URL is the state. Nothing about what the browser shows lives in a session or in JavaScript memory. |
| **Identification of resources** | Every view has its own URL, and a URL always shows the same view, on a click and on a refresh. |
| **Manipulation through representations** | The server sends HTML: the resource itself, ready to show, not data for the browser to assemble. |
| **Self-descriptive messages** | Real status codes. A missing resource is a 404 drawn where the resource would have been. |
| **Hypermedia as the engine of application state** | Plain `<a href>` links and plain forms. A link is the path of a typed view, never a string put together, so it cannot point at a route that does not exist. |
| **Cache** | One URL can answer with the whole document or with one part of it; every response says so (`Vary`), so caches stay correct. |
| **Code on demand** (optional) | htmx is an enhancement. Every page and action works without JavaScript. |

## Principles

- **URL as state.** A link opens a view, or a view nested inside it, and the address bar always says what is on screen.
- **Convention over configuration.** Sensible defaults first; every one of them can be overridden.
- **The server leads.** The server decides what each response contains; the browser shows it.
- **Unopinionated core.** The core writes no HTML and chooses no styling. [maud](https://maud.lambda.xyz) is supported out of the box and can be switched off.
- **Ready for agents.** Few concepts, one way to do each thing, and mistakes caught by the compiler.

## Demo

[`demo`](demo) is a small app to read and run. htmx 4 comes from the `htmx.org` package, which [pnpm](https://pnpm.io) installs:

```sh
(cd demo && pnpm install)
cargo run --package htmx-rust-demo
```

[`browser-tests`](browser-tests) is a [Playwright](https://playwright.dev) suite that starts the demo and checks it in a real browser, with JavaScript on and off:

```sh
cd browser-tests
pnpm install
pnpm exec playwright install chromium
pnpm test
```

## Documentation

- [Glossary](GLOSSARY.md): the words this project uses.
- [Decisions](docs/adr): why it is built the way it is.

## Planned

- The server sending only the outlet a request names
- Forms
- Regions: parts of a page refreshed on their own
- File-system routing
- A starter with a layout and base components

## License

Licensed under the [MIT License](LICENSE).
