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
                // sends them itself, naming this view's outlet, and the answer
                // is what goes inside it, which htmx puts there.
                body hx-boost:inherited="true" hx-target:inherited=(target) {
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

/// The title of an answer that is not the whole document: the view no other
/// is around says it, for htmx to take it out and make it the document's.
fn title(outlet: &Outlet) -> Markup {
    html! {
        @if outlet.outermost { title { (outlet.title) } }
    }
}

#[derive(Serialize, Deserialize)]
struct Home;

impl View for Home {
    type State = App;
    type Parent = Document;
    const PATH: &'static str = "/";
    const TITLE: &'static str = "Home";

    async fn render(_: &ViewPath<Self>, _: &App, outlet: Outlet) -> impl Into<Html> {
        html! {
            (title(&outlet))
            h1 { "Home" }
            p { "Every address here shows the same thing on a click and on a refresh." }
            p { a href=(Catalogs::default().path()) { "See the catalogs" } }
            // The answer to this form is drawn by an ordinary route, as the
            // whole document, so it goes in the whole body.
            form method="post" action=(Catalogs::default().path()) hx-target="body" {
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

    async fn render(at: &ViewPath<Self>, app: &App, outlet: Outlet) -> impl Into<Html> {
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
            (title(&outlet))
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
                        a href=(Catalog { catalog_id: *id }.path().to(Tabs)) { (name) }
                        " "
                        form method="post" action={ "/catalogs/" (id) "/remove" } {
                            button { "Remove " (name) }
                        }
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
            (title(&outlet))
            @match store.catalog(at.catalog_id) {
                Some(catalog) => {
                    h1 { (catalog.name) }
                    div id=(outlet.id) { (outlet.content) }
                }
                // A missing resource is not yet a failure (ADR 4).
                None => p { "There is no catalog " (at.catalog_id) "." },
            }
            p { a href=(Catalogs::default().path()) { "All catalogs" } }
        }
    }
}

/// The tabs of a catalog: a view with no part of the path of its own, there
/// only to draw the links to the views nested in it, around its outlet. It is
/// the view at a catalog's address, which is its own: `/catalogs/{catalog_id}`.
///
/// A link to a tab is the path of that tab below the one this view is shown
/// at, so it is in the catalog the address names. It is aimed at the outlet
/// this view is shown in, the catalog's: the answer is the tabs again, with
/// the one that is open marked, and the catalog around them stays as it is.
#[derive(Serialize, Deserialize)]
struct Tabs;

impl View for Tabs {
    type State = App;
    type Parent = Catalog;
    const TITLE: &'static str = "Catalog";

    async fn render(at: &ViewPath<Self>, _: &App, outlet: Outlet) -> impl Into<Html> {
        // The tab that is open is the one whose path is the view in the outlet.
        let (items, tags) = (at.to(Items), at.to(Tags));
        html! {
            (title(&outlet))
            nav hx-target:inherited={ "#" (at.parent().outlet()) } {
                a href=(items) aria-current=[outlet.shows(&items).then_some("page")] { "Items" }
                " "
                a href=(tags) aria-current=[outlet.shows(&tags).then_some("page")] { "Tags" }
            }
            section id=(outlet.id) { (outlet.content) }
        }
    }
}

/// A catalog's tags, a tab: `/catalogs/{catalog_id}/tags`. It holds nothing:
/// the catalog it is in is two views above it, in the path it is shown at.
#[derive(Serialize, Deserialize)]
struct Tags;

impl View for Tags {
    type State = App;
    type Parent = Tabs;
    const PATH: &'static str = "/tags";
    const TITLE: &'static str = "Tags";

    async fn render(at: &ViewPath<Self>, app: &App, outlet: Outlet) -> impl Into<Html> {
        let store = app.lock().unwrap();
        let catalog = store.catalog(at.parent().parent().catalog_id);
        html! {
            (title(&outlet))
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
/// the URL names is shown in its outlet, whose id it chooses itself.
#[derive(Serialize, Deserialize)]
struct Items;

impl View for Items {
    type State = App;
    type Parent = Tabs;
    const PATH: &'static str = "/items";
    const TITLE: &'static str = "Items";
    const OUTLET: Option<&'static str> = Some("item");

    async fn render(at: &ViewPath<Self>, app: &App, outlet: Outlet) -> impl Into<Html> {
        let store = app.lock().unwrap();
        let catalog = store.catalog(at.parent().parent().catalog_id);
        html! {
            (title(&outlet))
            h2 { "Items" }
            ul {
                @for &item in catalog.into_iter().flat_map(|catalog| &catalog.items) {
                    @let item = at.to(Item { item: item.into() });
                    li {
                        a href=(item) aria-current=[outlet.shows(&item).then_some("page")] {
                            (item.item)
                        }
                    }
                }
            }
            div id=(outlet.id) { (outlet.content) }
        }
    }
}

/// One item of a catalog, nested in the catalog's items, four views below the
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

    async fn render(at: &ViewPath<Self>, app: &App, outlet: Outlet) -> impl Into<Html> {
        let store = app.lock().unwrap();
        let catalog = store
            .catalog(at.parent().parent().parent().catalog_id)
            .filter(|catalog| catalog.items.contains(&at.item.as_str()));
        html! {
            (title(&outlet))
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
        (Catalog { catalog_id: id }.path().to(Tabs), name)
    };
    let outlet = Outlet::new(
        "outlet-0",
        html! {
            h1 { "Catalog created" }
            p { strong { (name) } " is among the catalogs now." }
            p { a href=(catalog) { "View " (name) } }
            p { a href=(Catalogs::default().path()) { "See the catalogs" } }
        }
        .into(),
        "Catalog created",
    );
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
        // A catalog's address shows its tabs, with none of them open.
        .view::<Tabs>()
        .view::<Tags>()
        .view::<Items>()
        .view::<Item>()
        // Ordinary axum routes, beside the views.
        .route("/catalogs", post(start_catalog))
        .route("/catalogs/{id}/remove", post(remove_catalog))
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
