//! Every test builds a router, sends one request and looks only at the
//! response's status, headers and body.

use std::sync::{Arc, Mutex};

use axum::{
    Router,
    body::Body,
    http::{
        Request, StatusCode,
        header::{CONTENT_TYPE, VARY},
    },
    response::Response,
    routing::get,
};
use htmx_rust::{Html, NoParent, Outlet, View, ViewPath, Views};
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use tower::ServiceExt;

#[derive(Clone)]
struct App {
    name: &'static str,
}

#[derive(Default, Serialize, Deserialize)]
struct Document;

impl View for Document {
    type State = App;
    type Parent = NoParent;

    async fn render(_: &ViewPath<Self>, app: &App, outlet: Outlet) -> impl Into<Html> {
        Html::trusted(format!(
            "<title>{} - {}</title><main id=\"{}\">{}</main>",
            outlet.title,
            app.name,
            outlet.id,
            outlet.content.as_str()
        ))
    }
}

#[derive(Serialize, Deserialize)]
struct Catalogs;

impl View for Catalogs {
    type State = App;
    type Parent = Document;
    const PATH: &'static str = "/catalogs";
    const TITLE: &'static str = "Catalogs";

    async fn render(_: &ViewPath<Self>, app: &App, outlet: Outlet) -> impl Into<Html> {
        Html::trusted(format!(
            "<h1>Catalogs of {}</h1><div id=\"{}\">{}</div>",
            app.name,
            outlet.id,
            outlet.content.as_str()
        ))
    }
}

/// A view nested in [`Catalogs`].
#[derive(Serialize, Deserialize)]
struct Archived;

impl View for Archived {
    type State = App;
    type Parent = Catalogs;
    const PATH: &'static str = "/archived";
    const TITLE: &'static str = "Archived catalogs";

    async fn render(_: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        Html::trusted("<p>None archived</p>")
    }
}

/// A view whose URL carries its fields: two in the path, the others in the
/// query string. It shows itself as a link to itself.
#[derive(Debug, Serialize, Deserialize)]
struct Entry {
    shelf: u32,
    name: String,
    #[serde(default)]
    q: String,
    page: Option<u32>,
    #[serde(default)]
    tag: Vec<String>,
}

impl View for Entry {
    type State = App;
    type Parent = Document;
    const PATH: &'static str = "/shelves/{shelf}/entries/{name}";
    const TITLE: &'static str = "Entry";

    async fn render(at: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        Html::trusted(format!("<a href=\"{at}\">{:?}</a>", **at))
    }
}

/// One catalog, nested in the root view beside [`Catalogs`], although its path
/// extends that one's. It names its outlet's id itself, and besides its
/// parameter it has a field in the query string.
#[derive(Serialize, Deserialize)]
struct Catalog {
    catalog: u32,
    page: Option<u32>,
}

impl View for Catalog {
    type State = App;
    type Parent = Document;
    const PATH: &'static str = "/catalogs/{catalog}";
    const TITLE: &'static str = "Catalog";
    const OUTLET: Option<&'static str> = Some("catalog");

    async fn render(at: &ViewPath<Self>, _: &App, outlet: Outlet) -> impl Into<Html> {
        Html::trusted(format!(
            "<h2>Catalog {}</h2><section id=\"{}\">{}</section>",
            at.catalog,
            outlet.id,
            outlet.content.as_str()
        ))
    }
}

/// A view nested in [`Catalog`], at depth two, which holds nothing of it: only
/// a field of its own, which a URL must carry. It shows a link to itself, one
/// to the view beside it and one to a view nested in it, all from the path it
/// is shown at.
#[derive(Serialize, Deserialize)]
struct Notes {
    sort: String,
}

impl View for Notes {
    type State = App;
    type Parent = Catalog;
    const PATH: &'static str = "/notes";
    const TITLE: &'static str = "Notes";

    async fn render(at: &ViewPath<Self>, _: &App, outlet: Outlet) -> impl Into<Html> {
        let note = Note {
            note: "first draft".into(),
            rev: None,
        };
        Html::trusted(format!(
            "<a href=\"{at}\">Notes of {} by {}</a><a href=\"{}\">Labels</a>\
             <a href=\"{}\">First draft</a><div id=\"{}\">{}</div>",
            at.parent().catalog,
            at.sort,
            at.parent().to(Labels),
            at.to(note),
            outlet.id,
            outlet.content.as_str()
        ))
    }
}

/// A view nested in [`Notes`], at depth three, with a parameter and a field of
/// the query string of its own. It shows itself as a link to itself.
#[derive(Debug, Serialize, Deserialize)]
struct Note {
    note: String,
    rev: Option<u32>,
}

impl View for Note {
    type State = App;
    type Parent = Notes;
    const PATH: &'static str = "/{note}";
    const TITLE: &'static str = "Note";

    async fn render(at: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        Html::trusted(format!(
            "<a href=\"{at}\">{:?} of catalog {}</a>",
            **at,
            at.parent().parent().catalog
        ))
    }
}

/// A view nested in [`Catalog`] beside [`Notes`], with no field at all.
#[derive(Serialize, Deserialize)]
struct Labels;

impl View for Labels {
    type State = App;
    type Parent = Catalog;
    const PATH: &'static str = "/labels";
    const TITLE: &'static str = "Labels";

    async fn render(at: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        let Catalog { catalog, page } = **at.parent();
        Html::trusted(format!("<p>Labels of {catalog}, page {page:?}</p>"))
    }
}

/// A view nested in [`Catalog`] with no part of the path of its own: it is
/// only there to draw, around its outlet, the links to the views nested in it.
#[derive(Serialize, Deserialize)]
struct Tabs;

impl View for Tabs {
    type State = App;
    type Parent = Catalog;
    const TITLE: &'static str = "Tabs";

    async fn render(at: &ViewPath<Self>, _: &App, outlet: Outlet) -> impl Into<Html> {
        Html::trusted(format!(
            "<nav><a href=\"{}\">Summary</a></nav><div id=\"{}\">{}</div>",
            at.to(Summary),
            outlet.id,
            outlet.content.as_str()
        ))
    }
}

/// A view nested in [`Tabs`], and so, at depth three, in [`Catalog`].
#[derive(Serialize, Deserialize)]
struct Summary;

impl View for Summary {
    type State = App;
    type Parent = Tabs;
    const PATH: &'static str = "/summary";
    const TITLE: &'static str = "Summary";

    async fn render(at: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        let catalog = at.parent().parent().catalog;
        Html::trusted(format!("<p>Summary of {catalog}</p>"))
    }
}

/// One catalog again, at the path of [`Catalog`], but nested in [`Catalogs`].
#[derive(Serialize, Deserialize)]
struct Listed {
    catalog: u32,
}

impl View for Listed {
    type State = App;
    type Parent = Catalogs;
    const PATH: &'static str = "/{catalog}";
    const TITLE: &'static str = "Catalog";

    async fn render(at: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        Html::trusted(format!("<h2>Catalog {}</h2>", at.catalog))
    }
}

/// A view whose path is `/` alone.
#[derive(Default, Serialize, Deserialize)]
struct Home;

impl View for Home {
    type State = App;
    type Parent = Document;
    const PATH: &'static str = "/";
    const TITLE: &'static str = "Home";

    async fn render(_: &ViewPath<Self>, _: &App, outlet: Outlet) -> impl Into<Html> {
        Html::trusted(format!("<h1>Home</h1>{}", outlet.content.as_str()))
    }
}

/// A view nested in [`Home`], which shows itself as a link to itself.
#[derive(Serialize, Deserialize)]
struct Activity;

impl View for Activity {
    type State = App;
    type Parent = Home;
    const PATH: &'static str = "/activity";
    const TITLE: &'static str = "Activity";

    async fn render(at: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        Html::trusted(format!("<a href=\"{at}\">Activity</a>"))
    }
}

/// A view nested in [`Home`] whose part of the path starts with no `/`.
#[derive(Serialize, Deserialize)]
struct Recent;

impl View for Recent {
    type State = App;
    type Parent = Home;
    const PATH: &'static str = "recent";

    async fn render(at: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        Html::trusted(format!("<a href=\"{at}\">Recent</a>"))
    }
}

/// A view that links to others, each from a value: one nested in the root
/// view, which the link does not name, and one further down, reached from its
/// parent's path.
#[derive(Serialize, Deserialize)]
struct Featured;

impl View for Featured {
    type State = App;
    type Parent = Document;
    const PATH: &'static str = "/featured";

    async fn render(_: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        let entry = Entry {
            shelf: 7,
            name: "red & white/rosé".into(),
            q: "a+b c?".into(),
            page: Some(2),
            tag: vec!["red".into(), "dry".into()],
        };
        let catalog = Catalog {
            catalog: 7,
            page: Some(2),
        };
        let note = Note {
            note: "a/b c".into(),
            rev: Some(3),
        };
        let notes = catalog.path().to(Notes { sort: "new".into() });
        Html::trusted(format!(
            "<a href=\"{}\">Featured</a><a href=\"{}\">Its note</a>",
            entry.path(),
            notes.to(note)
        ))
    }
}

/// A view whose path has a parameter that no field fills.
#[derive(Serialize, Deserialize)]
struct Unfilled;

impl View for Unfilled {
    type State = App;
    type Parent = Document;
    const PATH: &'static str = "/unfilled/{id}";

    async fn render(_: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        Html::default()
    }
}

/// A sound view nested in [`Unfilled`].
#[derive(Serialize, Deserialize)]
struct InUnfilled;

impl View for InUnfilled {
    type State = App;
    type Parent = Unfilled;
    const PATH: &'static str = "/inside";

    async fn render(_: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        Html::default()
    }
}

/// A view nested in [`Catalog`] whose part of the path names a parameter that
/// only its parent has a field for.
#[derive(Serialize, Deserialize)]
struct Borrowed;

impl View for Borrowed {
    type State = App;
    type Parent = Catalog;
    const PATH: &'static str = "/copies/{catalog}";

    async fn render(_: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        Html::default()
    }
}

/// A view nested in [`Catalog`] that holds its parent's parameter again.
#[derive(Serialize, Deserialize)]
struct Repeated {
    catalog: u32,
}

impl View for Repeated {
    type State = App;
    type Parent = Catalog;
    const PATH: &'static str = "/repeated";

    async fn render(_: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        Html::default()
    }
}

/// A view nested in [`Notes`] with a field of the query string named as one of
/// [`Catalog`], two views above it.
#[derive(Serialize, Deserialize)]
struct Paged {
    page: Option<u32>,
}

impl View for Paged {
    type State = App;
    type Parent = Notes;
    const PATH: &'static str = "/paged";

    async fn render(_: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        Html::default()
    }
}

/// A second view that claims the path of [`Catalogs`].
#[derive(Serialize, Deserialize)]
struct AlsoCatalogs;

impl View for AlsoCatalogs {
    type State = App;
    type Parent = Document;
    const PATH: &'static str = "/catalogs";

    async fn render(_: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        Html::default()
    }
}

fn router() -> Router {
    Router::new()
        .view::<Catalogs>()
        .view::<Archived>()
        .view::<Catalog>()
        .view::<Notes>()
        .view::<Note>()
        .view::<Labels>()
        .view::<Summary>()
        .view::<Home>()
        .view::<Activity>()
        .view::<Recent>()
        .view::<Entry>()
        .view::<Featured>()
        .route("/health", get(|| async { "ok" }))
        .with_state(App { name: "Loom" })
}

async fn get_path(router: Router, path: &str) -> Response {
    let request = Request::get(path).body(Body::empty()).unwrap();
    router.oneshot(request).await.unwrap()
}

async fn body(response: Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn a_view_answers_at_its_path_inside_the_root_view() {
    let response = get_path(router(), "/catalogs").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[CONTENT_TYPE], "text/html; charset=utf-8");
    assert_eq!(
        body(response).await,
        "<title>Catalogs - Loom</title><main id=\"outlet-0\">\
         <h1>Catalogs of Loom</h1><div id=\"outlet-1\"></div></main>"
    );
}

#[tokio::test]
async fn a_nested_view_answers_at_its_ancestors_path_and_its_own_inside_them() {
    let response = get_path(router(), "/catalogs/archived").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body(response).await,
        "<title>Archived catalogs - Loom</title><main id=\"outlet-0\">\
         <h1>Catalogs of Loom</h1><div id=\"outlet-1\"><p>None archived</p></div></main>"
    );
}

/// What [`Notes`] shows of itself in catalog 7, page 2, sorted by the newest.
const NOTES: &str = "<a href=\"/catalogs/7/notes?page=2&sort=new\">Notes of 7 by new</a>\
    <a href=\"/catalogs/7/labels?page=2\">Labels</a>\
    <a href=\"/catalogs/7/notes/first%20draft?page=2&sort=new\">First draft</a>";

#[tokio::test]
async fn a_view_nested_in_one_with_a_parameter_answers_inside_it_at_depth_two() {
    let response = get_path(router(), "/catalogs/7/notes?sort=new&page=2").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body(response).await,
        format!(
            "<title>Notes - Loom</title><main id=\"outlet-0\">\
             <h2>Catalog 7</h2><section id=\"catalog\">\
             {NOTES}<div id=\"outlet-2\"></div>\
             </section></main>"
        )
    );
}

#[tokio::test]
async fn a_view_at_depth_three_answers_inside_every_ancestor_with_their_values() {
    let url = "/catalogs/7/notes/first%20draft?rev=3&sort=new&page=2";

    let response = get_path(router(), url).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body(response).await,
        format!(
            "<title>Note - Loom</title><main id=\"outlet-0\">\
             <h2>Catalog 7</h2><section id=\"catalog\">\
             {NOTES}<div id=\"outlet-2\">\
             <a href=\"/catalogs/7/notes/first%20draft?page=2&sort=new&rev=3\">\
             Note {{ note: \"first draft\", rev: Some(3) }} of catalog 7</a>\
             </div></section></main>"
        )
    );
}

/// Every `href` in `html`, in the order they are in.
fn links(html: &str) -> Vec<&str> {
    let links = html.split("<a href=\"").skip(1);
    links.map(|rest| rest.split_once('"').unwrap().0).collect()
}

/// The `href` of the link in `html` that reads `text`.
fn link<'a>(html: &'a str, text: &str) -> &'a str {
    let (before, _) = html.split_once(&format!("\">{text}</a>")).unwrap();
    before.rsplit_once("<a href=\"").unwrap().1
}

#[tokio::test]
async fn a_nested_view_prints_the_path_it_is_shown_at_which_answers_with_the_same_view() {
    // The parents' values are in each: a parameter and a field of the query
    // string of `Catalog`, and the field `Notes` cannot be read without.
    for (url, own) in [
        ("/catalogs/7/notes?sort=new&page=2", 0),
        ("/catalogs/7/notes/a%2Fb%20c?rev=3&sort=new&page=2", 3),
    ] {
        let shown = body(get_path(router(), url).await).await;
        let printed = links(&shown)[own];
        // Not the URL as it was asked for: the one the path prints, in which
        // the fields follow the views from the root view down.
        assert_ne!(printed, url);

        let response = get_path(router(), printed).await;

        assert_eq!(response.status(), StatusCode::OK, "{printed}");
        assert_eq!(body(response).await, shown);
    }
}

#[tokio::test]
async fn a_view_links_to_the_one_beside_it_through_its_parents_path() {
    let notes = body(get_path(router(), "/catalogs/7/notes?sort=new&page=2").await).await;
    let labels = link(&notes, "Labels");
    // The parent's fields go along; those of the view that printed it do not.
    assert_eq!(labels, "/catalogs/7/labels?page=2");

    let response = get_path(router(), labels).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body(response).await,
        "<title>Labels - Loom</title><main id=\"outlet-0\">\
         <h2>Catalog 7</h2><section id=\"catalog\">\
         <p>Labels of 7, page Some(2)</p></section></main>"
    );
}

#[tokio::test]
async fn a_view_links_to_one_nested_in_it_from_its_own_path() {
    let notes = body(get_path(router(), "/catalogs/7/notes?sort=old").await).await;
    let note = link(&notes, "First draft");
    assert_eq!(note, "/catalogs/7/notes/first%20draft?sort=old");

    let response = get_path(router(), note).await;

    assert_eq!(response.status(), StatusCode::OK);
    let note = body(response).await;
    assert!(
        note.contains("Notes of 7 by old</a>")
            && note.contains("Note { note: \"first draft\", rev: None } of catalog 7</a>"),
        "{note}"
    );
}

#[tokio::test]
async fn a_path_started_from_a_value_goes_down_to_a_view_three_deep() {
    let featured = body(get_path(router(), "/featured").await).await;
    let note = link(&featured, "Its note");
    assert_eq!(note, "/catalogs/7/notes/a%2Fb%20c?page=2&sort=new&rev=3");

    let response = get_path(router(), note).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        body(response)
            .await
            .contains("Note { note: \"a/b c\", rev: Some(3) } of catalog 7</a>")
    );
}

#[tokio::test]
async fn a_view_with_no_part_of_the_path_holds_the_views_nested_in_it_at_its_parents_path() {
    let response = get_path(router(), "/catalogs/7/summary").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body(response).await,
        "<title>Summary - Loom</title><main id=\"outlet-0\">\
         <h2>Catalog 7</h2><section id=\"catalog\">\
         <nav><a href=\"/catalogs/7/summary\">Summary</a></nav><div id=\"outlet-2\">\
         <p>Summary of 7</p></div></section></main>"
    );
}

#[tokio::test]
async fn a_view_with_no_part_of_the_path_registered_answers_at_its_parents_path() {
    let router = Router::new().view::<Tabs>().view::<Summary>();

    let response = get_path(
        router.with_state(App { name: "Loom" }),
        "/catalogs/7?page=2",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body(response).await,
        "<title>Tabs - Loom</title><main id=\"outlet-0\">\
         <h2>Catalog 7</h2><section id=\"catalog\">\
         <nav><a href=\"/catalogs/7/summary?page=2\">Summary</a></nav><div id=\"outlet-2\">\
         </div></section></main>"
    );
}

#[test]
#[should_panic(expected = "/catalogs/{catalog}")]
fn a_view_with_no_part_of_the_path_and_its_parent_are_not_both_registered() {
    let _: Router<App> = Router::new().view::<Catalog>().view::<Tabs>();
}

/// The ids of the outlets in `html`, outermost first.
fn outlets(html: &str) -> Vec<&str> {
    let ids = html.split(" id=\"").skip(1);
    ids.map(|rest| rest.split_once('"').unwrap().0).collect()
}

#[tokio::test]
async fn an_outlets_id_follows_its_views_depth_unless_the_view_names_one() {
    let catalogs = body(get_path(router(), "/catalogs/archived").await).await;
    let notes = body(get_path(router(), "/catalogs/7/notes/first?sort=new").await).await;

    assert_eq!(outlets(&catalogs), ["outlet-0", "outlet-1"]);
    // The one a view names does not shift those that follow it.
    assert_eq!(outlets(&notes), ["outlet-0", "catalog", "outlet-2"]);
}

#[tokio::test]
async fn a_path_that_extends_anothers_is_nested_in_it_only_when_declared_so() {
    let beside = Router::new().view::<Catalogs>().view::<Catalog>();
    let inside = Router::new().view::<Catalogs>().view::<Listed>();
    let app = App { name: "Loom" };

    let beside = get_path(beside.with_state(app.clone()), "/catalogs/7").await;
    let inside = get_path(inside.with_state(app), "/catalogs/7").await;

    assert_eq!(
        body(beside).await,
        "<title>Catalog - Loom</title><main id=\"outlet-0\">\
         <h2>Catalog 7</h2><section id=\"catalog\"></section></main>"
    );
    assert_eq!(
        body(inside).await,
        "<title>Catalog - Loom</title><main id=\"outlet-0\">\
         <h1>Catalogs of Loom</h1><div id=\"outlet-1\"><h2>Catalog 7</h2></div></main>"
    );
}

#[tokio::test]
async fn a_parents_last_slash_is_kept_when_the_next_part_starts_with_none() {
    let response = get_path(router(), "/recent").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        body(response)
            .await
            .contains("<a href=\"/recent\">Recent</a>")
    );
}

#[tokio::test]
async fn a_view_nested_in_the_one_at_the_root_path_is_not_served_behind_two_slashes() {
    let response = get_path(router(), "/activity").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body(response).await,
        "<title>Activity - Loom</title><main id=\"outlet-0\">\
         <h1>Home</h1><a href=\"/activity\">Activity</a></main>"
    );
}

#[tokio::test]
async fn the_root_view_on_its_own_is_served_at_the_root_path() {
    let router = Router::new().view::<Document>();

    let response = get_path(router.with_state(App { name: "Loom" }), "/").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body(response).await,
        "<title> - Loom</title><main id=\"outlet-0\"></main>"
    );
}

#[tokio::test]
async fn path_parameters_and_the_query_string_are_parsed_into_the_views_fields() {
    let response = get_path(router(), "/shelves/7/entries/tea?page=2&q=green%20leaf").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body(response).await,
        "<title>Entry - Loom</title><main id=\"outlet-0\">\
         <a href=\"/shelves/7/entries/tea?q=green+leaf&page=2\">\
         Entry { shelf: 7, name: \"tea\", q: \"green leaf\", page: Some(2), tag: [] }</a></main>"
    );
}

#[tokio::test]
async fn a_field_the_url_leaves_out_takes_its_default() {
    let response = get_path(router(), "/shelves/7/entries/tea").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(body(response).await.contains(
        "<a href=\"/shelves/7/entries/tea?q=\">\
             Entry { shelf: 7, name: \"tea\", q: \"\", page: None, tag: [] }</a>"
    ));
}

#[tokio::test]
async fn a_link_rendered_from_a_view_value_requested_back_answers_with_that_view() {
    let featured = body(get_path(router(), "/featured").await).await;
    let url = link(&featured, "Featured");
    assert_eq!(
        url,
        "/shelves/7/entries/red%20%26%20white%2Fros%C3%A9?q=a%2Bb+c%3F&page=2&tag=red&tag=dry"
    );

    let response = get_path(router(), url).await;

    assert_eq!(response.status(), StatusCode::OK);
    let entry = body(response).await;
    assert!(entry.contains(
        "Entry { shelf: 7, name: \"red & white/rosé\", q: \"a+b c?\", page: Some(2), tag: [\"red\", \"dry\"] }"
    ));
    // The view the URL was parsed into prints the URL it came from.
    assert_eq!(links(&entry), [url]);
}

#[tokio::test]
async fn a_view_with_no_fields_answers_whatever_the_query_string() {
    let response = get_path(router(), "/catalogs?utm_source=mail").await;

    assert_eq!(response.status(), StatusCode::OK);
}

/// A client error, and nothing of any view in the body.
async fn assert_rejected(path: &str) {
    let response = get_path(router(), path).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}");
    assert_eq!(
        response.headers()[CONTENT_TYPE],
        "text/plain; charset=utf-8"
    );
    let body = body(response).await;
    assert!(!body.contains("Loom") && !body.contains("<"), "{body}");
}

#[tokio::test]
async fn a_path_parameter_that_does_not_parse_is_rejected_and_no_view_runs() {
    assert_rejected("/shelves/seven/entries/tea").await;
}

#[tokio::test]
async fn a_query_string_that_does_not_parse_is_rejected_and_no_view_runs() {
    assert_rejected("/shelves/7/entries/tea?page=two").await;
}

#[tokio::test]
async fn a_url_an_ancestor_cannot_be_read_from_is_rejected_and_no_view_runs() {
    // `Notes`, two views above the one the path names, has no `sort`.
    assert_rejected("/catalogs/7/notes/first").await;
    // The parameter of `Catalog`, the parent of the view the path names.
    assert_rejected("/catalogs/seven/labels").await;
}

#[tokio::test]
async fn a_query_string_that_repeats_a_path_parameter_is_rejected() {
    assert_rejected("/shelves/7/entries/tea?shelf=8").await;
}

#[test]
#[should_panic(
    expected = "Unfilled has no field for the parameter {id} of its part of the path: /unfilled/{id}"
)]
fn a_path_parameter_without_a_field_is_an_error_at_startup() {
    let _: Router<App> = Router::new().view::<Unfilled>();
}

#[test]
#[should_panic(
    expected = "Unfilled has no field for the parameter {id} of its part of the path: /unfilled/{id}"
)]
fn an_ancestor_is_checked_when_a_view_nested_in_it_is_registered() {
    let _: Router<App> = Router::new().view::<InUnfilled>();
}

#[test]
#[should_panic(
    expected = "Borrowed has no field for the parameter {catalog} of its part of the path: /copies/{catalog}"
)]
fn a_parameter_is_filled_by_a_field_of_its_own_view_not_of_an_ancestor() {
    let _: Router<App> = Router::new().view::<Borrowed>();
}

#[test]
#[should_panic(
    expected = "Repeated and router::Catalog, which it is nested in, both have a field named catalog"
)]
fn a_field_named_as_its_parents_parameter_is_an_error_at_startup() {
    let _: Router<App> = Router::new().view::<Repeated>();
}

#[test]
#[should_panic(
    expected = "Paged and router::Catalog, which it is nested in, both have a field named page"
)]
fn a_field_named_as_one_of_an_ancestors_query_string_is_an_error_at_startup() {
    let _: Router<App> = Router::new().view::<Paged>();
}

#[tokio::test]
async fn ordinary_routes_answer_beside_views() {
    let response = get_path(router(), "/health").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await, "ok");
}

#[tokio::test]
async fn a_path_no_view_claims_is_not_found() {
    let response = get_path(router(), "/nothing").await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
#[should_panic(expected = "/catalogs")]
fn two_views_on_one_path_are_an_error_at_startup() {
    let _: Router<App> = Router::new().view::<Catalogs>().view::<AlsoCatalogs>();
}

#[cfg(feature = "maud")]
mod with_maud {
    use maud::html;

    use super::*;

    #[derive(Serialize, Deserialize)]
    struct Home;

    impl View for Home {
        type State = App;
        type Parent = MaudDocument;
        const PATH: &'static str = "/";
        const TITLE: &'static str = "Home";

        async fn render(at: &ViewPath<Self>, app: &App, _: Outlet) -> impl Into<Html> {
            html! { h1 { "Welcome to " (app.name) } a href=(at.to(Search { q: "a&b".into() })) { "Search" } }
        }
    }

    #[derive(Serialize, Deserialize)]
    struct Search {
        q: String,
    }

    impl View for Search {
        type State = App;
        type Parent = Home;
        const PATH: &'static str = "search";

        async fn render(_: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
            Html::default()
        }
    }

    #[derive(Serialize, Deserialize)]
    struct MaudDocument;

    impl View for MaudDocument {
        type State = App;
        type Parent = NoParent;

        async fn render(_: &ViewPath<Self>, _: &App, outlet: Outlet) -> impl Into<Html> {
            html! {
                title { (outlet.title) }
                main id=(outlet.id) { (outlet.content) }
            }
        }
    }

    #[tokio::test]
    async fn maud_markup_becomes_html_and_holds_an_outlet_and_a_path() {
        let router = Router::new()
            .view::<Home>()
            .with_state(App { name: "<Loom>" });

        let response = get_path(router, "/").await;

        assert_eq!(
            body(response).await,
            "<title>Home</title><main id=\"outlet-0\"><h1>Welcome to &lt;Loom&gt;</h1>\
             <a href=\"/search?q=a%26b\">Search</a></main>"
        );
    }
}

/// A view whose part of the path ends in a parameter, and one nested in it.
#[derive(Serialize, Deserialize)]
struct Shelf {
    shelf: String,
}

impl View for Shelf {
    type State = App;
    type Parent = Document;
    const PATH: &'static str = "/shelves/{shelf}";

    async fn render(at: &ViewPath<Self>, _: &App, outlet: Outlet) -> impl Into<Html> {
        Html::trusted(format!(
            "Shelf {:?} books={} papers={}[{}]",
            at.shelf,
            outlet.shows(&at.to(Books)),
            outlet.shows(&at.to(Papers { page: 1 })),
            outlet.content.as_str()
        ))
    }
}

#[derive(Serialize, Deserialize)]
struct Books;

impl View for Books {
    type State = App;
    type Parent = Shelf;
    const PATH: &'static str = "/books";

    async fn render(_: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        Html::trusted("Books")
    }
}

/// A second view nested in [`Shelf`], with a field in the query string.
#[derive(Serialize, Deserialize)]
struct Papers {
    page: u32,
}

impl View for Papers {
    type State = App;
    type Parent = Shelf;
    const PATH: &'static str = "/papers";

    async fn render(_: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        Html::trusted("Papers")
    }
}

fn shelves() -> Router {
    Router::new()
        .view::<Shelf>()
        .view::<Books>()
        .view::<Papers>()
        .with_state(App { name: "Loom" })
}

#[tokio::test]
async fn an_outlet_says_which_nested_view_is_shown_in_it() {
    let shown = |path: &'static str| async move { body(get_path(shelves(), path).await).await };

    assert!(
        shown("/shelves/top")
            .await
            .contains("books=false papers=false[]")
    );
    assert!(
        shown("/shelves/top/books")
            .await
            .contains("books=true papers=false[Books]")
    );
    // The view is the same one whatever its query string says.
    assert!(
        shown("/shelves/top/papers?page=7")
            .await
            .contains("books=false papers=true[Papers]")
    );
}

#[tokio::test]
async fn a_path_whose_parameter_is_empty_is_answered_with_the_view_it_names() {
    let router = shelves;
    let books = |shelf: &str| {
        Shelf {
            shelf: shelf.into(),
        }
        .path()
        .to(Books)
        .to_string()
    };

    let named = get_path(router(), &books("top")).await;
    assert!(
        body(named)
            .await
            .contains("Shelf \"top\" books=true papers=false[Books]")
    );

    // The route with an empty parameter, not `/shelves/books`, which is the
    // shelf named "books".
    assert_eq!(books(""), "/shelves//books");
    let empty = get_path(router(), &books("")).await;
    assert!(
        body(empty)
            .await
            .contains("Shelf \"\" books=true papers=false[Books]")
    );
}

/// A view that hides the names of its fields from the startup checks.
#[derive(Serialize, Deserialize)]
struct Flattened {
    #[serde(flatten)]
    entry: Shelf,
}

impl View for Flattened {
    type State = App;
    type Parent = Document;
    const PATH: &'static str = "/flattened";

    async fn render(_: &ViewPath<Self>, _: &App, _: Outlet) -> impl Into<Html> {
        Html::default()
    }
}

#[test]
#[should_panic(expected = "Flattened does not name its fields")]
fn a_view_with_a_flattened_field_is_an_error_at_startup() {
    let _: Router<App> = Router::new().view::<Flattened>();
}

/// The state of the views below: the names of those that rendered, in the
/// order they did, which is how a test knows the ones that did not.
type Ran = Arc<Mutex<Vec<&'static str>>>;

/// What each of the views below shows: its name around its outlet, and before
/// it the title, when no view is around it to say it.
fn draw(ran: &Ran, name: &'static str, shown: String, outlet: Outlet) -> Html {
    ran.lock().unwrap().push(name);
    let title = match outlet.outermost {
        true => format!("<title>{}</title>", outlet.title),
        false => String::new(),
    };
    Html::trusted(format!(
        "{title}<div id=\"{}\">{shown}[{}]</div>",
        outlet.id,
        outlet.content.as_str()
    ))
}

/// A root view, and three views nested one in the other below it.
#[derive(Default, Serialize, Deserialize)]
struct Outer;

impl View for Outer {
    type State = Ran;
    type Parent = NoParent;

    async fn render(_: &ViewPath<Self>, ran: &Ran, outlet: Outlet) -> impl Into<Html> {
        draw(ran, "Outer", "Outer".into(), outlet)
    }
}

/// It names its outlet's id itself.
#[derive(Serialize, Deserialize)]
struct Middle {
    middle: u32,
}

impl View for Middle {
    type State = Ran;
    type Parent = Outer;
    const PATH: &'static str = "/middles/{middle}";
    const TITLE: &'static str = "A middle";
    const OUTLET: Option<&'static str> = Some("middle");

    async fn render(at: &ViewPath<Self>, ran: &Ran, outlet: Outlet) -> impl Into<Html> {
        draw(ran, "Middle", format!("Middle {}", at.middle), outlet)
    }
}

/// It says whether the view nested in it is the one its outlet shows.
#[derive(Serialize, Deserialize)]
struct Inner;

impl View for Inner {
    type State = Ran;
    type Parent = Middle;
    const PATH: &'static str = "/inner";
    const TITLE: &'static str = "An inner";

    async fn render(at: &ViewPath<Self>, ran: &Ran, outlet: Outlet) -> impl Into<Html> {
        let open = outlet.shows(&at.to(Leaf));
        draw(ran, "Inner", format!("Inner, leaf open: {open}"), outlet)
    }
}

/// It shows a value of the view two above it, and the ids of the outlets it
/// is shown in, which are those of the views it is nested in.
#[derive(Serialize, Deserialize)]
struct Leaf;

impl View for Leaf {
    type State = Ran;
    type Parent = Inner;
    const PATH: &'static str = "/leaf";
    const TITLE: &'static str = "A leaf";

    async fn render(at: &ViewPath<Self>, ran: &Ran, outlet: Outlet) -> impl Into<Html> {
        let (inner, middle) = (at.parent(), at.parent().parent());
        let shown = format!(
            "Leaf of {} in {} in {}",
            middle.middle,
            inner.outlet(),
            middle.outlet()
        );
        draw(ran, "Leaf", shown, outlet)
    }
}

const LEAF: &str = "<div id=\"outlet-3\">Leaf of 7 in outlet-2 in middle[]</div>";

/// Sends one GET request for `path` with these `headers`, and gives back the
/// answer with the names of the views that rendered for it.
async fn cut(path: &str, headers: &[(&str, &str)]) -> (Response, Vec<&'static str>) {
    let ran = Ran::default();
    let router = Router::new()
        .view::<Middle>()
        .view::<Inner>()
        .view::<Leaf>()
        .with_state(ran.clone());
    let mut request = Request::get(path);
    for &(name, value) in headers {
        request = request.header(name, value);
    }
    let response = router
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let ran = ran.lock().unwrap().clone();
    (response, ran)
}

/// What htmx sends with a click on a link whose answer goes in `target`.
fn click(target: &str) -> [(&str, &str); 2] {
    [("HX-Request", "true"), ("HX-Target", target)]
}

/// The whole document for `/middles/7/inner/leaf`, from every view on the way.
async fn assert_whole_document(headers: &[(&str, &str)]) {
    let (response, ran) = cut("/middles/7/inner/leaf", headers).await;

    assert_eq!(response.status(), StatusCode::OK, "{headers:?}");
    assert_eq!(
        body(response).await,
        format!(
            "<title>A leaf</title><div id=\"outlet-0\">Outer[<div id=\"middle\">Middle 7[\
             <div id=\"outlet-2\">Inner, leaf open: true[{LEAF}]</div>]</div>]</div>"
        ),
        "{headers:?}"
    );
    assert_eq!(ran, ["Leaf", "Inner", "Middle", "Outer"], "{headers:?}");
}

#[tokio::test]
async fn a_request_that_htmx_did_not_send_gets_the_whole_document() {
    assert_whole_document(&[]).await;
    // An outlet is named, but not by htmx.
    assert_whole_document(&[("HX-Target", "div#outlet-0")]).await;
}

#[tokio::test]
async fn a_request_naming_the_root_views_outlet_gets_what_is_inside_it_and_the_root_view_does_not_run()
 {
    let (response, ran) = cut("/middles/7/inner/leaf", &click("main#outlet-0")).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[CONTENT_TYPE], "text/html; charset=utf-8");
    // The first view inside the outlet says the title; those in it do not.
    assert_eq!(
        body(response).await,
        format!(
            "<title>A leaf</title><div id=\"middle\">Middle 7[\
             <div id=\"outlet-2\">Inner, leaf open: true[{LEAF}]</div>]</div>"
        )
    );
    assert_eq!(ran, ["Leaf", "Inner", "Middle"]);
}

#[tokio::test]
async fn a_request_naming_a_nested_views_outlet_gets_only_what_is_inside_it_and_no_view_around_it_runs()
 {
    // The outlet whose id its view names, and below it one whose id follows
    // its view's depth.
    let (inner, ran_inner) = cut("/middles/7/inner/leaf", &click("section#middle")).await;
    let (leaf, ran_leaf) = cut("/middles/7/inner/leaf", &click("div#outlet-2")).await;

    assert_eq!(
        body(inner).await,
        format!("<title>A leaf</title><div id=\"outlet-2\">Inner, leaf open: true[{LEAF}]</div>")
    );
    // The first view of the answer is told what its outlet shows as any other.
    assert_eq!(ran_inner, ["Leaf", "Inner"]);
    // The views around the one sent did not run, and it still shows the value
    // of the one two above it: they are read from the URL all the same.
    assert_eq!(body(leaf).await, format!("<title>A leaf</title>{LEAF}"));
    assert_eq!(ran_leaf, ["Leaf"]);
}

#[tokio::test]
async fn the_same_outlet_named_for_a_view_higher_up_gets_that_view() {
    let (response, ran) = cut("/middles/7", &click("main#outlet-0")).await;

    assert_eq!(
        body(response).await,
        "<title>A middle</title><div id=\"middle\">Middle 7[]</div>"
    );
    assert_eq!(ran, ["Middle"]);
}

#[tokio::test]
async fn a_request_naming_an_outlet_that_is_not_on_the_way_gets_the_whole_document() {
    // An id no view has.
    assert_whole_document(&click("div#elsewhere")).await;
    // The outlet of the requested view itself: no view is nested in it here.
    assert_whole_document(&click("div#outlet-3")).await;
    // An element with no id, as the body is.
    assert_whole_document(&click("body")).await;
    // An id that only starts as an outlet's.
    assert_whole_document(&click("div#outlet-22")).await;
}

#[tokio::test]
async fn a_request_that_restores_history_gets_the_whole_document() {
    // As htmx 4.0.2 sends it, with no outlet named.
    assert_whole_document(&[("HX-History-Restore-Request", "true")]).await;
    // And if it ever names the outlet a click would.
    let [request, target] = click("main#outlet-0");
    assert_whole_document(&[request, target, ("HX-History-Restore-Request", "true")]).await;
}

#[tokio::test]
async fn every_answer_says_it_varies_by_the_htmx_headers() {
    let leaf = "/middles/7/inner/leaf";
    for (path, headers, status) in [
        (leaf, &[][..], StatusCode::OK),
        (leaf, &click("main#outlet-0")[..], StatusCode::OK),
        (leaf, &click("div#elsewhere")[..], StatusCode::OK),
        (
            leaf,
            &[("HX-History-Restore-Request", "true")][..],
            StatusCode::OK,
        ),
        // A URL that no view is read from, whole and cut.
        ("/middles/seven/inner", &[][..], StatusCode::BAD_REQUEST),
        (
            "/middles/seven/inner",
            &click("section#middle")[..],
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let (response, _) = cut(path, headers).await;

        assert_eq!(response.status(), status, "{path} {headers:?}");
        assert_eq!(
            response.headers()[VARY],
            "HX-Request, HX-Target, HX-History-Restore-Request",
            "{path} {headers:?}"
        );
    }
}

#[tokio::test]
async fn a_cut_answer_is_rejected_when_a_view_around_it_cannot_be_read_from_the_url() {
    // `Middle` would not render, and its parameter still does not parse.
    let (response, ran) = cut("/middles/seven/inner/leaf", &click("div#outlet-2")).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(ran.is_empty(), "{ran:?}");
}

/// A view nested in [`Middle`] that names its outlet as [`Middle`] does.
#[derive(Serialize, Deserialize)]
struct AlsoMiddle;

impl View for AlsoMiddle {
    type State = Ran;
    type Parent = Middle;
    const PATH: &'static str = "/also";
    const OUTLET: Option<&'static str> = Some("middle");

    async fn render(_: &ViewPath<Self>, _: &Ran, _: Outlet) -> impl Into<Html> {
        Html::default()
    }
}

/// A view nested in [`AlsoMiddle`], and sound itself.
#[derive(Serialize, Deserialize)]
struct InAlsoMiddle;

impl View for InAlsoMiddle {
    type State = Ran;
    type Parent = AlsoMiddle;
    const PATH: &'static str = "/inside";

    async fn render(_: &ViewPath<Self>, _: &Ran, _: Outlet) -> impl Into<Html> {
        Html::default()
    }
}

/// A view nested in [`Middle`] that names its outlet with the id the root
/// view's has by default.
#[derive(Serialize, Deserialize)]
struct AsTheRootViews;

impl View for AsTheRootViews {
    type State = Ran;
    type Parent = Middle;
    const PATH: &'static str = "/root";
    const OUTLET: Option<&'static str> = Some("outlet-0");

    async fn render(_: &ViewPath<Self>, _: &Ran, _: Outlet) -> impl Into<Html> {
        Html::default()
    }
}

/// A view whose outlet's id htmx would not send as it is written.
#[derive(Serialize, Deserialize)]
struct Spaced;

impl View for Spaced {
    type State = Ran;
    type Parent = Outer;
    const PATH: &'static str = "/spaced";
    const OUTLET: Option<&'static str> = Some("the outlet");

    async fn render(_: &ViewPath<Self>, _: &Ran, _: Outlet) -> impl Into<Html> {
        Html::default()
    }
}

#[test]
#[should_panic(expected = "AlsoMiddle is shown through two outlets with the id middle")]
fn an_outlet_with_the_id_of_an_ancestors_is_an_error_at_startup() {
    let _: Router<Ran> = Router::new().view::<AlsoMiddle>();
}

#[test]
#[should_panic(expected = "InAlsoMiddle is shown through two outlets with the id middle")]
fn two_ancestors_whose_outlets_share_an_id_are_an_error_at_startup() {
    let _: Router<Ran> = Router::new().view::<InAlsoMiddle>();
}

#[test]
#[should_panic(expected = "AsTheRootViews is shown through two outlets with the id outlet-0")]
fn an_outlet_named_as_an_ancestors_default_id_is_an_error_at_startup() {
    let _: Router<Ran> = Router::new().view::<AsTheRootViews>();
}

#[test]
#[should_panic(expected = "Spaced is shown through an outlet with the id \"the outlet\"")]
fn an_outlet_whose_id_is_not_letters_digits_hyphens_and_underscores_is_an_error_at_startup() {
    let _: Router<Ran> = Router::new().view::<Spaced>();
}
