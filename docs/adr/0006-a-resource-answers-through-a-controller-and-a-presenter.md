# A resource answers through a controller and a presenter, and a view only draws

The library is the presentation layer of an app whose use cases live in an application layer that knows nothing of HTTP. A request is answered by four pieces, each with one job:

- A **resource** is what a URL names: its part of the path, the parameters that part carries, and the resource it is nested in ([ADR 2](0002-resources-are-self-sufficient-types-that-name-their-parent.md)).
- A **controller** answers one method on a resource. It receives the whole request (the resource path, the body of a POST, and what any axum handler receives, such as who is signed in) and calls one use case: a read model for GET, a command for POST. It is the only piece that waits and the only one that can fail.
- A **presenter** turns the use case's result, success or error, into an outcome: a status, a `Location` when the status needs one, and a view when there is something to show. It is told whether another resource is nested in its own for this request. It cannot fail.
- A **view** is a value that holds what it draws, and a function that draws it as HTML. It does not wait, cannot fail, and knows nothing of the request, the method, the use case or the views around it.

The library does the rest. It reads the URL into a resource path and answers `400` when it cannot. It runs the controller and presenter of the resource the request names. When the outcome has a view and the request is for a whole page, it asks each resource the path goes through, from the root down, through that resource's own GET controller and presenter, and puts each view in the outlet of the one around it. When htmx names an outlet, it asks only the resources whose views go inside that outlet ([ADR 1](0001-server-cuts-the-response-by-outlet.md)).

## Considered Options

- **A view loads its own data** (`async fn render(at, state, outlet)`, what was first built): a view that calls the store cannot be drawn from any other data, cannot say which status to answer with, is tied to one app's state, and puts waiting and failure in the one piece that should have neither.
- **One handler per address loads the whole page**: every handler under a catalog loads the catalog again, and on an htmx click it loads what is not sent.
- **The presenter composes the page**: it would need the data of the views around its own.
- **A resource declares its default nested resource**, and the library redirects: a redirect is a status, and the status is the presenter's. The presenter only needs to know whether something is nested.
- **The server follows a redirect itself** and answers with the target's content: two URLs would show the same thing, and the address bar would name a different resource from the one shown.
- **On a POST, the resources around are asked before the command runs**, so that their `403` or `404` guards it: a use case that relies on the page around it is unguarded when called from anywhere else. Authorization belongs to the use case.
- **A client-side view model** (MVVM): state outside the URL.

## Consequences

- GET runs a read model and changes nothing. POST runs a command. A search is a GET; a search too large for a URL is not designed for yet.
- A GET on a resource always answers with the same resource, so its answer can be cached. How it is shown is a representation, chosen from the request's headers: one controller, and a presenter for each representation. For a browser the representation is a view, and on a `200` it is always the same one; other views appear with other statuses. Only that presenter is designed for: another client's, answering JSON, is not.
- The same view may be answered by several resources, and a view such as "not found" or a form with its values and errors is a view like any other: the presenter chooses it.
- A `Location` is a resource path, printed by the library. No URL is written by hand, in a view either: a view that draws a link holds that resource path among what it draws.
- No body means no view: `303`, `204`, and a `201` that shows nothing.
- Going down a resource path, the first outcome that is not a `200` is the response: its status, and its view in the place of the view that would have been there, with the views above it around it ([ADR 4](0004-failures-are-drawn-in-place.md)). Nothing below it is asked. So a GET controller can rely on the resources around its own having answered `200`.
- A resource with nothing nested in it either draws its own content (the root resource, at `/`, draws the home content) or answers `303` to a resource nested in it (a catalog to its first tab). Since a presenter with something nested has no reason to redirect, composing a page cannot loop.
- On a POST, the command runs first. The resources around are asked only when its outcome has a view and a whole page is needed, each through its GET side. A `303` asks none.
- A redirect is sent to the browser, never followed on the server.
- What htmx sends (`HX-Request`, `HX-Target`) is read by the library alone. A controller and a presenter answer the same for a click and for a full load.
- The mapping from the application's error type to a status and a view can be written once, as a presenter shared by controllers.
- The roles are those of clean architecture (controller, presenter, view) and of Action-Domain-Responder, where the domain is the application layer.
