# Phase 3 — HTTP data types

## Changes

Added `crates/framework/src/http/{mod,error,headers,method,request,response,status}.rs`, public re-exports, HTTP error conversion, and HTTP contract tests. Updated the existing consumer to exercise the new types.

## API decisions

- Method is a private validated string wrapper, constructed with Method::new or FromStr. Methods are case-sensitive; extension tokens are accepted. Accepting a token does not imply protocol support for CONNECT or other special targets.
- Headers stores lowercase names, preserves value order and repetitions, and strips surrounding spaces/tabs. Values support ASCII visible characters and horizontal tab only. Other bytes are rejected. Invalid replacement leaves existing data intact. No automatic comma joining.
- Request::new supports origin-form targets with valid percent escapes and visible URI characters. It rejects spaces, raw Unicode, fragments, absolute-form and asterisk-form. Path/query remain encoded. Empty versus absent query strings remain distinct.
- Request::new is for constructing data, not parsing network messages. No Host or framing validation is claimed. There is no public parameter mutator; the router will add internal parameter assignment in Phase 4.
- Response owns its body. Borrowed request text is copied. Header access is immutable so callers cannot bypass reserved-header checks.
- StatusCode validates final numeric codes 200..=599, including unassigned codes in that range; it does not claim all codes are registered. Response::status keeps deferred validation for the agreed ergonomic builder.
- Response::validate rejects nonempty bodies for 204, 205 and 304. HEAD suppression and wire Content-Length rules are future encoder responsibilities. Response headers cannot set Content-Length, Transfer-Encoding or Connection.
- IntoResponse converts Response or framework Result<Response> to a validated Result<Response>; it never silently turns errors into success or sends data. Server-side rendering and diagnostic handling come later.

## Test source coverage

Method tokens and casing; header repetition, replacement atomicity and invalid values; encoded URI preservation and malformed targets; invalid UTF-8; owned response lifetime and byte length; reserved headers; invalid statuses; bodyless statuses; repeat Set-Cookie; error propagation.

## Validation status

Source and module declarations reviewed; all four Cargo manifests parsed and workspace paths resolved; archive integrity verified. Compilation, Rust formatting, test execution and runtime behavior remain unverified because Rust tooling is unavailable. Phase 2 tests are retained. No external dependencies added.
