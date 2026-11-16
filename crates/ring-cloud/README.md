# ring-cloud

A blocking client for Ring's cloud API (the one the Ring Android app speaks),
after the endpoints `ring-client-api` documents.

- `sign_in` with email and password; Ring answers that a code is needed
  (`SignIn::NeedsCode`, naming where it was sent) and the same call with the
  code returns a `Session`: a refresh token and the hardware id it is bound to.
- `Ring::resume(session)` uses it unattended. Ring rotates the refresh token
  on every refresh: keep `Ring::session()` after use, or the next resume may
  be refused.
- `cameras`, `events` (motion, ding, on-demand…, with person detection),
  `recordings` (each with its thumbnail URL, where the account keeps video),
  `thumbnail`, `snapshot` (the camera's latest still, JPEG) and
  `recording_url`.

The parsers are public and pure, so they are tested without the network.
