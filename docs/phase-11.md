# Phase 11 — HTTP expansion

Implemented opt-in keep_alive and max_requests_per_connection (default false and 100). Sequential requests reuse a worker, with a fresh read deadline after each response. Connection: close and shutdown stop reuse; an already-blocked idle read can delay shutdown until its read timeout. Server counters still count connections, not individual responses. HTTP/1.1 only; no concurrent multiplexing.

Chunked requests are decoded into the bounded body buffer. Content-Length combinations fail; only one chunked coding is accepted. Chunk extensions and nonempty trailers are deliberately rejected. Chunk-size/trailer-line metadata has a separate cumulative max_header_bytes budget. Request body streaming is not implemented; decoded uploads remain buffered.

Response::stream accepts a one-shot Read + Send source. Writer emits chunked encoding with 8 KiB buffer; cloned responses share consumption and a second encoding fails. HEAD does not consume the stream or claim a known size. Bodyless statuses reject streams. Stream-source reads are application code and can block indefinitely; socket deadlines cannot interrupt them. Source failure after headers closes the connection without a terminating chunk. No stream restart, seek or retries.

Request::multipart(max_parts,max_part_headers) supports a strict buffered multipart/form-data subset, validated boundary, raw per-part Headers and owned body bytes. No filesystem writes, filename interpretation, preamble, epilogue, nested multipart or general MIME parsing. Content-Disposition is checked for form-data but name/filename parameters remain raw, for the application to interpret. Overall server body limits still apply; direct Request constructors do not impose them. This API returns MultipartError explicitly; apps choose their HTTP error response.

Added four tests for chunked limits/exact consumption, streamed output and HEAD, multipart limits, and capped keep-alive loopback. Updated prior unsupported-transfer test to gzip. Compilation and all Rust tests remain unverified; source and manifests/archive checked. Compatibility/fuzz/load testing are pending. This is an experimental supported subset, not complete HTTP conformance.

Next: Phase 12 — Performance Baseline; measurement requires executable builds.
