# Performance hardening

Performance changes in Berserk must follow measured hot paths and preserve correctness contracts.

Group 12 starts with routing because the baseline exercises a 101-route dispatch on every sample and inspection showed the router used `Pattern::captures` as a match probe. That path allocated a temporary vector of path segments and cloned captured parameter names/values for every candidate route, even though those captures were discarded during candidate selection.

The hardened matcher separates two operations:

- `Pattern::matches` performs a non-allocating segment walk for candidate selection.
- `Pattern::captures` allocates owned parameter values only once, after a route has actually been selected.

This retains deterministic specificity, static-vs-parameter behavior, root/trailing-slash distinctions, parameter extraction, HEAD behavior, and 404/405 semantics while removing avoidable work from the route scan.

The benchmark harness includes both parameter-route and static-route 101-route scenarios so future routing changes can distinguish matching cost from parameter extraction cost.

No unsafe code, global route cache, hidden mutable state, or public API change is introduced. More aggressive indexing or trie routing should only be considered if retained baseline evidence shows the linear route scan itself is the next material bottleneck; complexity should not be added speculatively.

Database, JSON, TCP, load, and soak workloads remain unchanged so optimization claims can be compared against the Group 11 baseline. A performance change is not accepted merely because it appears theoretically faster: correctness tests and repeatable release measurements remain required.
