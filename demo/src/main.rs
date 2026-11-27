//! The demo app: what htmx-rust does, in unstyled HTML. The browser suite in
//! `browser-tests` runs against it.

use std::{
    env, fs,
    sync::{Arc, Mutex},
};

use axum::{
    Router,
    extract::{self, State},
    http::{
        StatusCode,
        header::{CONTENT_TYPE, LOCATION},
    },
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use htmx_rust::{Html, NoParent, Outlet, View, ViewPath, Views};
use maud::{DOCTYPE, Markup, html};
use serde::{Deserialize, Serialize};

/// htmx 4, where `pnpm install` puts it.
const HTMX: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/node_modules/htmx.org/dist/htmx.min.js"
);

/// The tags a catalog can carry.
const TAGS: [&str; 4] = ["tea", "coffee", "hot", "iced"];

struct Record {
    id: u32,
    name: String,
    tags: Vec<&'static str>,
    items: Vec<&'static str>,
}

/// What the app keeps, in memory: the catalogs.
struct Store {
    /// How many catalogs were ever started; the next one's id follows from it.
    started: u32,
    catalogs: Vec<Record>,
}

impl Store {
    fn catalog(&self, id: u32) -> Option<&Record> {
        self.catalogs.iter().find(|catalog| catalog.id == id)
    }
}

/// The catalogs the demo starts with.
impl Default for Store {
    fn default() -> Self {
        let catalogs = [
            ("Spring teas", ["tea", "hot"], ["Sencha", "Darjeeling"]),
            (
                "Summer teas",
                ["tea", "iced"],
                ["Iced oolong", "Cold brew green"],
            ),
            (
                "Autumn coffees",
                ["coffee", "iced"],
                ["Cold brew", "Iced latte"],
            ),
            (
                "Winter coffees",
                ["coffee", "hot"],
                ["Espresso", "Flat white"],
            ),
        ];
        Self {
            started: catalogs.len() as u32,
            catalogs: (1..)
                .zip(catalogs)
                .map(|(id, (name, tags, items))| Record {
                    id,
                    name: name.into(),
                    tags: tags.into(),
                    items: items.into(),
                })
                .collect(),
        }
    }
}

type App = Arc<Mutex<Store>>;

/// The root view: the document, with the navigation around its outlet. It
/// carries nothing, so a path that starts below it does not name it.
#[derive(Default, Serialize, Deserialize)]
struct Document;

impl View for Document {
    type State = App;
    type Parent = NoParent;

    async fn render(_: &ViewPath<Self>, _: &App, outlet: Outlet) -> impl Into<Html> {
        let target = format!("#{}", outlet.id);
        html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    title { (outlet.title) }
                    // The one style of the demo: the link to what is open.
                    style { "[aria-current] { font-weight: bold; text-decoration: none }" }
                    script src="/htmx.js" defer {}
                }
                // Set once and inherited by every link and form below: htmx
                // sends them itself and shows the answer in this view's outlet.
                //
                // Until the server cuts the response by the outlet the request
                // names (ADR 1), every answer is the whole document, so the
                // browser cuts: `hx-select` and `hx-swap` take the outlet out
                // of the answer and put it in the place of this one.
                body
                    hx-boost:inherited="true"
                    hx-target:inherited=(target)
                    hx-select:inherited=(target)
                    hx-swap:inherited="outerHTML"
                {
                    nav {
                        a href=(Home.path()) { "Home" }
                        " "
                        a href=(Catalogs::default().path()) { "Catalogs" }
                    }
                    main id=(outlet.id) { (outlet.content) }
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Home;

impl View for Home {
    type State = App;
    type Parent = Document;
    const PATH: &'static str = "/";
    const TITLE: &'static str = "Home";

    async fn render(_: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        html! {
            h1 { "Home" }
            p { "Every address here shows the same thing on a click and on a refresh." }
            p { a href=(Catalogs::default().path()) { "See the catalogs" } }
            form method="post" action=(Catalogs::default().path()) {
                button { "Start a catalog" }
            }
        }
    }
}

/// The collection: every catalog, or those the search in its URL finds.
/// `/catalogs?q=tea&tag=hot&tag=iced` is a search like any other address.
#[derive(Default, Serialize, Deserialize)]
struct Catalogs {
    /// What a catalog's name must contain. `/catalogs` searches for nothing.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    q: String,
    /// The tags a catalog must carry, all of them. The query string repeats
    /// the name, once for each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tag: Vec<String>,
}

impl View for Catalogs {
    type State = App;
    type Parent = Document;
    const PATH: &'static str = "/catalogs";
    const TITLE: &'static str = "Catalogs";

    async fn render(at: &ViewPath<Self>, app: &App, _: Outlet) -> impl Into<Html> {
        let q = at.q.to_lowercase();
        let store = app.lock().unwrap();
        let found = store
            .catalogs
            .iter()
            .filter(|catalog| catalog.name.to_lowercase().contains(&q))
            .filter(|catalog| {
                at.tag
                    .iter()
                    .all(|tag| catalog.tags.contains(&tag.as_str()))
            });
        html! {
            h1 { "Catalogs" }
            // A GET form is a link whose query string comes from its fields:
            // each is named after a field of the view at its address.
            form action=(Catalogs::default().path()) {
                label { "Name " input type="search" name="q" value=(at.q); }
                " "
                @for tag in TAGS {
                    label {
                        input type="checkbox" name="tag" value=(tag)
                            checked[at.tag.iter().any(|checked| checked == tag)];
                        " " (tag)
                    }
                    " "
                }
                button { "Search" }
            }
            ul {
                @for Record { id, name, .. } in found {
                    li {
                        a href=(Catalog { catalog_id: *id }.path()) { (name) }
                        " "
                        a href=(Catalog { catalog_id: *id }.path().to(Remove)) { "Remove " (name) }
                    }
                }
            }
            p { a href=(Home.path()) { "Back home" } }
        }
    }
}

/// One catalog, the resource its path parameter names. It is nested in the
/// root view, beside the collection and not inside it, although its path
/// extends the collection's: the path says nothing about nesting.
///
/// What it shows is in the views nested in it. Its own address shows none, so
/// it is not registered: that address redirects to the first tab.
#[derive(Serialize, Deserialize)]
struct Catalog {
    catalog_id: u32,
}

impl View for Catalog {
    type State = App;
    type Parent = Document;
    const PATH: &'static str = "/catalogs/{catalog_id}";

    async fn render(at: &ViewPath<Self>, app: &App, outlet: Outlet) -> impl Into<Html> {
        let store = app.lock().unwrap();
        html! {
            @match store.catalog(at.catalog_id) {
                Some(catalog) => {
                    h1 { (catalog.name) }
                    section id=(outlet.id) { (outlet.content) }
                }
                // A missing resource is not yet a failure (ADR 4).
                None => p { "There is no catalog " (at.catalog_id) "." },
            }
            p { a href=(Catalogs::default().path()) { "All catalogs" } }
        }
    }
}

/// A tab of a catalog.
#[derive(PartialEq)]
enum Tab {
    Items,
    Tags,
}

/// The links to a catalog's tabs, with the `open` one marked. Each tab draws
/// them above what it shows, so the answer to a tab's address holds both.
fn tabs(catalog: &ViewPath<Catalog>, open: Tab) -> Markup {
    let tab = |path: String, name: &str, tab: Tab| {
        html! { a href=(path) aria-current=[(tab == open).then_some("page")] { (name) } }
    };
    html! {
        nav {
            (tab(catalog.to(Items).to_string(), "Items", Tab::Items))
            " "
            (tab(catalog.to(Tags).to_string(), "Tags", Tab::Tags))
        }
    }
}

/// A catalog's tags, a tab: `/catalogs/{catalog_id}/tags`. It holds nothing:
/// the catalog it is in is the view above it, in the path it is shown at.
#[derive(Serialize, Deserialize)]
struct Tags;

impl View for Tags {
    type State = App;
    type Parent = Catalog;
    const PATH: &'static str = "/tags";
    const TITLE: &'static str = "Tags";

    async fn render(at: &ViewPath<Self>, app: &App, _: Outlet) -> impl Into<Html> {
        let store = app.lock().unwrap();
        let catalog = store.catalog(at.parent().catalog_id);
        html! {
            (tabs(at.parent(), Tab::Tags))
            h2 { "Tags" }
            ul {
                @for tag in catalog.into_iter().flat_map(|catalog| &catalog.tags) {
                    li { (tag) }
                }
            }
        }
    }
}

/// A catalog's items, the other tab: `/catalogs/{catalog_id}/items`. The one
/// the URL names is shown in its outlet.
#[derive(Serialize, Deserialize)]
struct Items;

impl View for Items {
    type State = App;
    type Parent = Catalog;
    const PATH: &'static str = "/items";
    const TITLE: &'static str = "Items";

    async fn render(at: &ViewPath<Self>, app: &App, outlet: Outlet) -> impl Into<Html> {
        let store = app.lock().unwrap();
        let catalog = store.catalog(at.parent().catalog_id);
        html! {
            (tabs(at.parent(), Tab::Items))
            h2 { "Items" }
            ul {
                @for &item in catalog.into_iter().flat_map(|catalog| &catalog.items) {
                    @let item = at.to(Item { item: item.into() });
                    li {
                        a href=(item) { (item.item) }
                    }
                }
            }
            div id=(outlet.id) { (outlet.content) }
        }
    }
}

/// One item of a catalog, nested in the catalog's items, three views below the
/// root view: `/catalogs/{catalog_id}/items/{item}`. Its only field is its own.
#[derive(Serialize, Deserialize)]
struct Item {
    item: String,
}

impl View for Item {
    type State = App;
    type Parent = Items;
    const PATH: &'static str = "/{item}";
    const TITLE: &'static str = "Item";

    async fn render(at: &ViewPath<Self>, app: &App, _: Outlet) -> impl Into<Html> {
        let store = app.lock().unwrap();
        let catalog = store
            .catalog(at.parent().parent().catalog_id)
            .filter(|catalog| catalog.items.contains(&at.item.as_str()));
        html! {
            @match catalog {
                Some(catalog) => {
                    h3 { (at.item) }
                    p { "One of the items of " (catalog.name) "." }
                    // The view around this one, without an item.
                    p { a href=(at.parent()) { "Close" } }
                }
                None => p { "There is no item " (at.item) " here." },
            }
        }
    }
}

/// The removal of a catalog, a resource of its own: `/catalogs/{catalog_id}/remove`
/// asks, and the form it shows posts to that same address, its own.
#[derive(Serialize, Deserialize)]
struct Remove;

impl View for Remove {
    type State = App;
    type Parent = Catalog;
    const PATH: &'static str = "/remove";
    const TITLE: &'static str = "Remove catalog";

    async fn render(at: &ViewPath<Self>, app: &App, _: Outlet) -> impl Into<Html> {
        let store = app.lock().unwrap();
        let catalog = store.catalog(at.parent().catalog_id);
        html! {
            @if let Some(Record { name, .. }) = catalog {
                h2 { "Remove " (name) "?" }
                form method="post" action=(at) {
                    button { "Remove " (name) }
                }
                p { a href=(at.parent()) { "Keep it" } }
            }
        }
    }
}

/// Starts a catalog and answers `201 Created` with the whole document around
/// a message that links to it. Actions will draw their answer in place
/// themselves; until then the root view is rendered by hand.
async fn start_catalog(State(app): State<App>) -> impl IntoResponse {
    let (catalog, name) = {
        let mut store = app.lock().unwrap();
        store.started += 1;
        let id = store.started;
        let name = format!("Catalog {id}");
        store.catalogs.push(Record {
            id,
            name: name.clone(),
            tags: Vec::new(),
            items: Vec::new(),
        });
        (Catalog { catalog_id: id }.path(), name)
    };
    let outlet = Outlet {
        id: Document.path().outlet(),
        content: html! {
            h1 { "Catalog created" }
            p { strong { (name) } " is among the catalogs now." }
            p { a href=(catalog) { "View " (name) } }
            p { a href=(Catalogs::default().path()) { "See the catalogs" } }
        }
        .into(),
        title: "Catalog created",
    };
    let document: Html = Document::render(&Document.path(), &app, outlet)
        .await
        .into();
    (
        StatusCode::CREATED,
        [
            (LOCATION, catalog.to_string()),
            (CONTENT_TYPE, "text/html; charset=utf-8".into()),
        ],
        document.as_str().to_owned(),
    )
}

/// A catalog's own address shows its first tab: `307 Temporary Redirect`.
async fn open_catalog(extract::Path(catalog_id): extract::Path<u32>) -> Redirect {
    Redirect::temporary(&Catalog { catalog_id }.path().to(Items).to_string())
}

/// Removes a catalog and answers `303 See Other` to the collection.
async fn remove_catalog(State(app): State<App>, extract::Path(id): extract::Path<u32>) -> Redirect {
    app.lock()
        .unwrap()
        .catalogs
        .retain(|catalog| catalog.id != id);
    Redirect::to(&Catalogs::default().path().to_string())
}

#[tokio::main]
async fn main() {
    let htmx = fs::read_to_string(HTMX).unwrap_or_else(|error| {
        panic!("{HTMX}: {error}; run `pnpm install` in the demo directory")
    });

    let router = Router::new()
        .view::<Home>()
        .view::<Catalogs>()
        .view::<Tags>()
        .view::<Items>()
        .view::<Item>()
        .view::<Remove>()
        // Ordinary axum routes, beside the views, until a view can answer
        // with a redirect and an action shares a view's path.
        .route("/catalogs", post(start_catalog))
        .route("/catalogs/{catalog_id}", get(open_catalog))
        .route("/catalogs/{catalog_id}/remove", post(remove_catalog))
        .route(
            "/htmx.js",
            get(|| async move { ([(CONTENT_TYPE, "text/javascript")], htmx) }),
        )
        .with_state(App::default());

    let port = env::var("PORT").unwrap_or_else(|_| "3000".into());
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    println!("http://{}", listener.local_addr().unwrap());
    axum::serve(listener, router).await.unwrap();
}
