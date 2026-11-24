# The server cuts the response by the outlet the request names

One URL must show the same thing on a click and on a refresh. Every in-app link is a plain `<a href>`; with htmx the request names the outlet it will fill (`HX-Target`), and the server answers with only what goes inside that outlet. Without that header (a full load, a refresh, back and forward, no JavaScript) it answers with the whole document. Every response carries `Vary: HX-Request, HX-Target, HX-History-Restore-Request`, because the same URL has more than one body. Only htmx 4 is supported.

The rule, for a request to a view's URL:

1. It carries `HX-History-Restore-Request`, or no `HX-Request`: the whole document.
2. `HX-Target` (`tag#id`) has the id of the outlet of a view the requested one is nested in: the views inside that outlet, the requested one last. The views around it do not render.
3. Anything else (no `HX-Target`, an element with no id, an id no view on the way has, the requested view's own outlet): the whole document.

## Considered Options

- **The browser cuts** (`hx-select` on a full document): simpler, but every click renders and sends what is thrown away.
- **The server compares the current URL with the requested one** and picks the swap itself (`HX-Current-URL`, `HX-Retarget`): links need no attributes, but the library must match two URLs per request and shared caches cannot store the answers. htmx 4's inherited attributes give attribute-free links more cheaply: the root view sets the default target once.
- **htmx 2 as well**: doubles the header logic; `HX-Target` as `tag#id` and `HX-Request-Type` exist only in 4.
- **Two outlets with one id on the way to a view, the innermost or the outermost taken**: either is a silent choice, and the document would hold the id twice. It is refused when the view is registered.
- **Every view says the title in a `<title>` of its own**, whole document or not: no view would need to know where it stands in the answer, but a whole document would hold a `<title>` in its body for every view on the way.

## Consequences

- By default a link fills the root view's outlet. A view that draws its own navigation may name a narrower outlet on it: its own, or the one it is shown in.
- An outlet needs an id, or the server cannot tell which one was named. The id is compared as htmx sends it, so it is made of letters, digits, `-` and `_`, and no two outlets on the way to a view share one; both are checked when a view is registered.
- What is sent for an outlet is exactly the representation of the views inside it, so a nested view answers a tab click with its own content, navigation included, and nothing of its ancestors.
- The one difference is the title, which the root view puts in the document's head and which a cut answer has no head for. The view that no other is around in an answer is told so, and says the title in a `<title>`: htmx takes it out of the answer and makes it the document's. The library writes no markup for it ([ADR 3](0003-the-core-emits-no-markup.md)).
- A request that restores history (back and forward) gets the whole document whatever else it carries. htmx 4.0.2 sends it with `HX-History-Restore-Request` alone, neither `HX-Request` nor `HX-Target`, which looks like an accident of its code; the rule does not rely on it, and so `Vary` names that header too.
- A whole document sent for an outlet that is not on the way is put by htmx inside the element the request named, the document's navigation with it. A link is aimed at an outlet the linked view is shown in; the library does not correct one that is not.
- An answer drawn outside a view, such as an ordinary route's, is not cut: a form sent to one is aimed at the body.
