# Phase 5 — HTTP parsing and encoding

## Implemented interfaces

`server::read_request(&mut impl Read, &ServerConfig) -> Result<Request, ProtocolError>` reads one request. `server::write_response(&mut impl Write, &Response, &Method) -> Result<(), ProtocolError>` writes one final response. Method must be preserved from the parsed request, especially for HEAD.

## Parsing policy

HTTP/1.1 only, origin-form targets, strict CRLF request headers, one required Host, ASCII header subset, configurable header byte/count and body limits. Host accepts a practical ASCII hostname subset or bracketed IPv6 with an optional numeric u16 port; broader URI reg-name and IPvFuture forms are intentionally unsupported. Strict duplicate Content-Length rejection includes identical duplicates and comma lists. Transfer-Encoding plus Content-Length is malformed; any other transfer encoding is unsupported. All Expect headers are rejected before body reads. No implicit request body without Content-Length.

Header bytes include the request line and terminating CRLF pair. Header count excludes request line. Reads consume exactly the declared body; extra bytes remain unread and the future server closes instead of processing another request. Body data grows in bounded chunks only after length validation. Numeric length overflow is rejected as oversized. Truncated reads preserve an I/O error. Bare LF, folded fields, whitespace before colon and invalid target escapes are rejected.

## Encoding policy

Validate response before writing any bytes. Compute length from body bytes or HEAD representation metadata; always emit Connection: close. HEAD sends no body. 204/304 omit Content-Length; 205 emits zero length. A 304 representation length is unknown and is not fabricated. Repeated headers remain separate. Unlisted status reason phrases are empty; numeric status remains authoritative. Date generation is deferred: this codec is not a claim of complete HTTP server conformance. No compression, TLS, streaming, upgrades or keep-alive.

Transport write errors propagate and can leave partial bytes; never retry a response on the same connection. ProtocolError::status_code offers request-error categorization; a caller must distinguish actual transport failures and response-write failures before deciding whether to send an error. Error rendering, shutdown, worker execution and panic containment remain later phases.

## Test sources and review

Six codec tests cover many cases: fragmented reads, exact body consumption, framing ambiguity, expectations, Host/header grammar, limits, truncation, version errors, byte lengths, HEAD via routing, bodyless statuses, invalid response atomicity and failed writes. All earlier tests retained. Source/manifests reviewed and package checked; Rust compilation and tests not executed.

The byte-at-a-time header reader avoids over-read and keeps parsing simple but should be wrapped with bounded buffering for socket use. Total deadlines cannot be guaranteed by a generic blocking Read; Phase 6/7 must enforce them. No network server is claimed in this phase.
