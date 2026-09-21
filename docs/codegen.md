# Berserk macros and code generation

Berserk separates procedural-macro entry points from reusable code generation.

```text
application Rust
      |
      v
berserk-macros       proc-macro boundary
      |
      v
berserk-codegen      parse -> validate -> emit Rust tokens
      |
      v
generated Rust
      |
      v
rustc
```

## berserk-macros

`berserk-macros` is intentionally thin. It owns Rust procedural-macro entry
points and converts compiler token streams to and from `berserk-codegen`.

Application-facing syntax such as `#[derive(Model)]`, `#[table(...)]`,
`#[fillable]`, and relationship attributes remains exposed through this crate.

It should not accumulate ORM logic, runtime framework state, database behavior,
or a second copy of parsing rules.

## berserk-codegen

`berserk-codegen` is a normal Rust library. It owns reusable parsing,
validation, and token emission. The initial implementation contains the model
and relationship expansion that previously lived directly in
`berserk-macros`.

This split gives Berserk one source of truth for generated Rust. A future
build-time source layer can call `berserk-codegen` without pretending to be a
procedural macro or duplicating model semantics.

The package is not a general-purpose transpiler yet. It does not replace Rust,
Cargo, rustc, Claw ORM, routing, or Axe. Any future Berserk source syntax should
compile down to the same stable Rust-facing framework contracts.

## Dependency rule

The intended direction is:

```text
berserk-macros -> berserk-codegen
berserk        -> berserk-macros   (optional claw feature)
```

`berserk-codegen` must remain independent of Berserk runtime crates. Generated
tokens may reference public Berserk paths, but codegen itself must not link the
framework, Claw, HTTP, database drivers, or Axe.

That boundary keeps compiler tooling reusable and prevents a circular dependency
between generated application code and the runtime framework.
