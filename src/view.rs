use std::{
    any::type_name,
    fmt::{self, Display, Formatter},
    future::Future,
    ops::Deref,
    sync::Arc,
};

use axum::{
    Router,
    extract::{RawPathParams, RawQuery, State},
    http::StatusCode,
    routing::get,
};
use form_urlencoded::Serializer;
use serde::{Serialize, de::DeserializeOwned};

use crate::{Html, url};
use chain::Chain;

/// What one URL shows, nested in its [`Parent`](View::Parent).
///
/// A view's fields are what its own part of the URL carries: the parameters
/// its [`PATH`](View::PATH) names (`{catalog_id}` fills the field
/// `catalog_id`), and the query string for every other field. It holds nothing
/// of the views it is nested in: their values are in the [`ViewPath`] it is shown
/// at. Each field holds a plain value, such as a number, a string or an
/// `Option` of one, or a `Vec` of them, which the query string carries by
/// repeating the field's name (`?tag=tea&tag=green`).
///
/// Every view on the way down is read from the same fields of the URL, so a
/// field's name is used once on the way: a view with a field named as one of an
/// ancestor's is an error when it is registered.
pub trait View: Serialize + DeserializeOwned + Send + Sync + 'static {
    /// The application state this view renders with: the router's own.
    type State: Sync;

    /// The view this one is shown in, or [`NoParent`] for the root view.
    type Parent: Parent<Self::State>;

    /// This view's part of the path, which follows its ancestors' parts. A
    /// parameter is a field's name in braces, as in `"/catalogs/{catalog_id}"`,
    /// and every parameter is a field of this view.
    ///
    /// The parts are joined as they are, except that a part's last `/` gives
    /// way to a next part that starts with one: a view at `"/activity"` nested
    /// in one at `"/"` is at `/activity`. A view whose whole path is empty is
    /// at `/`.
    ///
    /// Only the root view has no part: it is the document around every other
    /// view, and no address is its own. A view nested in another adds a part,
    /// so its address is its own; one that adds none is an error when it is
    /// registered.
    const PATH: &'static str = "";

    /// The title of this view, which every view around it receives when it is
    /// shown.
    const TITLE: &'static str = "";

    /// Renders the view shown `at` a path: `at.catalog_id` is a field of the
    /// view, [`at.parent()`](ViewPath::parent) the path of the view it is nested
    /// in, and [`at.to(..)`](ViewPath::to) the path of a view nested in it.
    ///
    /// Declared as `async fn render(..) -> impl Into<Html>`, it returns an
    /// engine's output, such as maud markup, as it is.
    fn render(
        at: &ViewPath<Self>,
        state: &Self::State,
        outlet: Outlet,
    ) -> impl Future<Output = impl Into<Html>> + Send;

    /// The path of this view where its ancestors carry nothing: each is its
    /// `Default`, so a link to a view at the top does not name the root view.
    /// A view below it is reached with [`ViewPath::to`].
    ///
    /// A view whose parent needs a value has no path of its own:
    ///
    /// ```compile_fail,E0277
    #[doc = include_str!("doctest_views.txt")]
    /// Item { item_id: 3 }.path();
    /// ```
    ///
    /// It is not for a view to define.
    fn path(self) -> ViewPath<Self>
    where
        Above<Self>: Default,
    {
        ViewPath::new(self, Default::default())
    }
}

/// Where a view is shown: the view and, above it, the path of its parent, up to
/// the root view. It prints as the URL that shows the view there, and a request
/// to that URL is parsed back into the same path.
///
/// A path starts from a view value ([`View::path`]) and goes down with
/// [`to`](ViewPath::to); a view is handed the one it is shown at when it renders.
/// There is no other way to a URL, so a link cannot point at a route that does
/// not exist. The view's fields are read through the path: `path.catalog_id`.
///
/// ```
#[doc = include_str!("doctest_views.txt")]
/// // `Catalog` is at "/catalogs/{catalog_id}" in the root view, which carries
/// // nothing, and `Item` at "/items/{item_id}" in `Catalog`.
/// let catalog = Catalog { catalog_id: 7, page: Some(2) }.path();
/// let item = catalog.to(Item { item_id: 3 });
///
/// // Each view fills its own part; every other field is in the query string.
/// assert_eq!(item.to_string(), "/catalogs/7/items/3?page=2");
/// assert_eq!(item.item_id, 3);
/// assert_eq!(item.parent().catalog_id, 7);
/// ```
pub struct ViewPath<V: View>(Arc<(V, Above<V>)>);

/// The path of a view's parent: a [`ViewPath`], or [`NoParent`] above the root view.
type Above<V> = <<V as View>::Parent as Chain<<V as View>::State>>::ViewPath;

impl<V: View> ViewPath<V> {
    fn new(view: V, parent: Above<V>) -> Self {
        Self(Arc::new((view, parent)))
    }

    /// The path of `child` nested in this path's view. Only a view that names
    /// this one as its parent is taken:
    ///
    /// ```compile_fail,E0271
    #[doc = include_str!("doctest_views.txt")]
    /// Document.path().to(Item { item_id: 3 });
    /// ```
    pub fn to<C>(&self, child: C) -> ViewPath<C>
    where
        C: View<Parent = V, State = V::State>,
    {
        ViewPath::new(child, self.clone())
    }

    /// The path of the view this one is nested in. From it a view reaches its
    /// ancestors' fields, and the views beside it: `at.parent().to(Sibling)`.
    pub fn parent(&self) -> &ViewPath<V::Parent>
    where
        V::Parent: View + Parent<V::State, ViewPath = ViewPath<V::Parent>>,
    {
        &self.0.1
    }

    /// The id of this view's outlet, which the view itself receives as
    /// [`Outlet::id`].
    pub fn outlet(&self) -> String {
        outlet::<V>()
    }
}

/// The id of a view's outlet: the names of the views from the root view down to
/// it, as their types spell them. A root view `Document` has `document-outlet`,
/// a `Catalog` nested in it `document-catalog-outlet`, and a `CatalogItem`
/// nested in the root view `document-catalog_item-outlet`: a `_` joins the
/// words of one name and a `-` joins the views, so the outlets of one document
/// never share an id.
fn outlet<V: View>() -> String {
    format!("{}-outlet", V::name())
}

/// A path is shared, not copied: views need not be `Clone`.
impl<V: View> Clone for ViewPath<V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<V: View + Default> Default for ViewPath<V>
where
    Above<V>: Default,
{
    fn default() -> Self {
        V::default().path()
    }
}

impl<V: View> Deref for ViewPath<V> {
    type Target = V;

    fn deref(&self) -> &V {
        &self.0.0
    }
}

/// The URL: the parts of the path from the root view down, each filled from its
/// own view's fields, then every view's other fields as the one query string.
impl<V: View> Display for ViewPath<V> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let (path, query) = self.print();
        formatter.write_str(&path)?;
        match query {
            query if query.is_empty() => Ok(()),
            query => write!(formatter, "?{query}"),
        }
    }
}

impl<V: View> ViewPath<V> {
    /// The path and the query string this path prints as.
    fn print(&self) -> (String, String) {
        let mut path = V::route();
        let mut query = Serializer::new(String::new());
        V::print(self, &mut path, &mut query);
        assert!(
            !path.contains('{'),
            "{} has no field for a parameter of its path: {path}",
            type_name::<V>()
        );
        (path, query.finish())
    }
}

/// The parent of the root view: none. It is not a view.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoParent;

/// What a view names as its parent: a [`View`] with the same state, or
/// [`NoParent`]. Nothing else can be one, and naming anything else does not
/// compile:
///
/// ```compile_fail,E0277
/// use htmx_rust::{Html, Outlet, ViewPath, View};
/// use serde::{Deserialize, Serialize};
///
/// struct NotAView;
///
/// #[derive(Serialize, Deserialize)]
/// struct Catalogs;
///
/// impl View for Catalogs {
///     type State = ();
///     type Parent = NotAView;
///
///     async fn render(_: &ViewPath<Self>, _: &(), _: Outlet) -> impl Into<Html> {
///         Html::default()
///     }
/// }
/// ```
pub trait Parent<S>: chain::Chain<S> {}

impl<S: Sync> Parent<S> for NoParent {}

impl<V: View> Parent<V::State> for V {}

/// Private, so that only this crate decides what a [`Parent`] is.
mod chain {
    use super::*;

    pub trait Chain<S> {
        /// How many views there are from the root view down to this one.
        const DEPTH: usize;

        /// Where this view is shown: a [`ViewPath`](super::ViewPath), or nothing above
        /// the root view.
        type ViewPath: Clone + Send + Sync;

        /// The whole path as the router matches it, parameters in braces.
        fn route() -> String;

        /// The names of the views from the root view down to this one, as an
        /// id spells them: a `-` between views and a `_` between the words of
        /// one view's name, `document-catalog_item`.
        fn name() -> String;

        /// Checks this view and its ancestors, whose fields' names are `seen`.
        fn check(seen: &mut Vec<(&'static str, &'static str)>);

        /// Reads this view and its ancestors from what a URL carries.
        fn parse(fields: &str) -> Result<Self::ViewPath, url::Error>;

        /// Fills this view's parameters in the route, and its ancestors' before.
        fn print(at: &Self::ViewPath, path: &mut String, query: &mut Serializer<'_, String>);

        /// Renders this view around `content`, and its ancestors around it.
        fn show(
            at: &Self::ViewPath,
            state: &S,
            content: Html,
            title: &'static str,
        ) -> impl Future<Output = Html> + Send;
    }

    impl<S: Sync> Chain<S> for NoParent {
        const DEPTH: usize = 0;
        type ViewPath = NoParent;

        fn route() -> String {
            String::new()
        }

        fn name() -> String {
            String::new()
        }

        fn check(_: &mut Vec<(&'static str, &'static str)>) {}

        fn parse(_: &str) -> Result<NoParent, url::Error> {
            Ok(NoParent)
        }

        fn print(_: &NoParent, _: &mut String, _: &mut Serializer<'_, String>) {}

        async fn show(_: &NoParent, _: &S, content: Html, _: &'static str) -> Html {
            content
        }
    }

    fn name_without_module_or_generics<V>() -> &'static str {
        let without_generics = type_name::<V>().split('<').next().unwrap_or_default();
        without_generics.rsplit("::").next().unwrap_or_default()
    }

    fn snake_case(camel_case: &str) -> String {
        let mut snake_case = String::new();
        for (index, letter) in camel_case.char_indices() {
            if letter.is_uppercase() && index > 0 {
                snake_case.push('_');
            }
            snake_case.extend(letter.to_lowercase());
        }
        snake_case
    }

    impl<V: View> Chain<V::State> for V {
        const DEPTH: usize = V::Parent::DEPTH + 1;
        type ViewPath = ViewPath<V>;

        fn route() -> String {
            let mut route = V::Parent::route();
            url::join(&mut route, V::PATH);
            route
        }

        fn name() -> String {
            let mut name = V::Parent::name();
            if !name.is_empty() {
                name.push('-');
            }
            name.push_str(&snake_case(name_without_module_or_generics::<V>()));
            name
        }

        fn check(seen: &mut Vec<(&'static str, &'static str)>) {
            assert!(
                V::Parent::DEPTH == 0 || !V::PATH.is_empty(),
                "{} has no part of the path: a view nested in another adds its own part, \
                 so that its address is its own",
                type_name::<V>()
            );
            V::Parent::check(seen);
            url::check::<V>(V::PATH, seen);
        }

        fn parse(fields: &str) -> Result<ViewPath<V>, url::Error> {
            Ok(ViewPath::new(
                url::parse(fields)?,
                V::Parent::parse(fields)?,
            ))
        }

        fn print(at: &ViewPath<V>, path: &mut String, query: &mut Serializer<'_, String>) {
            V::Parent::print(&at.0.1, path, query);
            url::print(&at.0.0, path, query);
        }

        async fn show(
            at: &ViewPath<V>,
            state: &V::State,
            content: Html,
            title: &'static str,
        ) -> Html {
            let outlet = Outlet {
                id: outlet::<V>(),
                content,
                title,
            };
            let content = V::render(at, state, outlet).await.into();
            V::Parent::show(&at.0.1, state, content, title).await
        }
    }
}

/// The place in a view where the view nested inside it is shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outlet {
    /// The id to give the element that holds [`content`](Outlet::content).
    pub id: String,
    /// The nested view, rendered. Empty when nothing is nested.
    pub content: Html,
    /// The title of the view the URL names: the innermost one.
    pub title: &'static str,
}

/// Registers views on an [`axum::Router`], beside its ordinary routes.
pub trait Views<S> {
    /// Answers GET requests to the view's path with the whole document: the
    /// view in its parent's outlet, and so on up to the root view.
    ///
    /// The view and its ancestors are read from the request's URL first. If
    /// one of them cannot be, because a parameter or the query string does not
    /// parse into its fields, the answer is `400 Bad Request` and no view
    /// renders.
    ///
    /// # Panics
    ///
    /// If the path already answers GET requests, or the view or one of its
    /// ancestors has a parameter in its part of the path that none of its
    /// fields fills, a field named as one of an ancestor's, or, nested in
    /// another view, no part of the path.
    fn view<V: View<State = S>>(self) -> Self;
}

impl<S: Clone + Send + Sync + 'static> Views<S> for Router<S> {
    fn view<V: View<State = S>>(self) -> Self {
        V::check(&mut Vec::new());
        self.route(
            &V::route(),
            get(
                |State(state): State<S>, parameters: RawPathParams, RawQuery(query): RawQuery| async move {
                    let at = V::parse(&url::fields(&parameters, query.as_deref()))
                        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
                    let document = V::show(&at, &state, Html::default(), V::TITLE).await;
                    Ok::<_, (StatusCode, String)>(axum::response::Html(String::from(document)))
                },
            ),
        )
    }
}
