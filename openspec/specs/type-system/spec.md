## ADDED Requirements

### Requirement: Symbol table with scoped resolution

The checker SHALL build a symbol table with lexical scoping. Each block SHALL introduce a new scope. Name resolution SHALL walk the scope chain from innermost to outermost. Duplicate bindings in the same scope SHALL produce a diagnostic. Accessing an undefined name SHALL produce a diagnostic.

#### Scenario: Nested scope shadowing

- **WHEN** the input is `let x = 1; { let x = "hello"; x }; x`
- **THEN** inner `x` resolves to `str`, outer `x` resolves to `int`

#### Scenario: Undefined variable

- **WHEN** the input references `y` without any declaration of `y` in scope
- **THEN** checker produces diagnostic "undefined variable `y`"

#### Scenario: Duplicate binding

- **WHEN** the input is `let x = 1; let x = 2` in the same scope
- **THEN** checker produces diagnostic "duplicate binding `x`"

### Requirement: Primitive type checking

The checker SHALL recognize primitive types `str`, `num`, `int`, `bool`, `nil`, and `any`. `int` SHALL be assignable to `num` (widening). `any` SHALL be compatible with all types (suppresses checking). A value of type `T` SHALL NOT be assignable to an incompatible type `U` (e.g., `str` to `int`).

#### Scenario: Type mismatch

- **WHEN** the input is `let x: int = "hello"`
- **THEN** checker produces diagnostic "type mismatch: expected `int`, found `str`"

#### Scenario: Int-to-num widening

- **WHEN** the input is `let x: num = 42` where `42` has type `int`
- **THEN** checker accepts without diagnostic

#### Scenario: Any escapes checking

- **WHEN** the input is `let x: any = 42; let y: str = x`
- **THEN** checker accepts without diagnostic

### Requirement: Local type inference for variable declarations

The checker SHALL infer the type of `let`, `mut`, and `const` bindings from the right-hand side expression when no type annotation is provided. Function parameters and return types SHALL require explicit annotations.

#### Scenario: Infer let type

- **WHEN** the input is `let x = 42`
- **THEN** checker infers `x: int`

#### Scenario: Missing function parameter type

- **WHEN** the input is `fn foo(x) -> int { x }`
- **THEN** checker produces diagnostic "parameter `x` requires a type annotation"

### Requirement: Struct type checking

The checker SHALL resolve struct declarations and check field access. Accessing a field that does not exist on a struct SHALL produce a diagnostic. Struct construction SHALL require all non-optional fields. Structural subtyping SHALL apply: a value with shape `{a: str, b: int}` SHALL satisfy type `{a: str}`.

#### Scenario: Valid field access

- **WHEN** a value `u` has type `User { name: str, age: int }` and the input accesses `u.name`
- **THEN** checker resolves the access to type `str`

#### Scenario: Invalid field access

- **WHEN** a value `u` has type `User { name: str }` and the input accesses `u.email`
- **THEN** checker produces diagnostic "field `email` does not exist on type `User`"

#### Scenario: Structural subtyping

- **WHEN** a function expects type `{ name: str }` and receives `{ name: str, age: int }`
- **THEN** checker accepts without diagnostic

### Requirement: Enum type checking

The checker SHALL resolve enum declarations and check variant construction and pattern matching. Constructing a variant SHALL require the correct fields. Using an undefined variant SHALL produce a diagnostic.

#### Scenario: Valid enum construction

- **WHEN** the input is `Status::Active(since: "2025-01-01")` and `Active` has field `since: str`
- **THEN** checker accepts and resolves type to `Status`

#### Scenario: Unknown variant

- **WHEN** the input is `Status::Unknown`
- **THEN** checker produces diagnostic "variant `Unknown` does not exist on enum `Status`"

### Requirement: Function call type checking

The checker SHALL verify function call argument count and type compatibility. Too few or too many arguments (accounting for defaults) SHALL produce a diagnostic. Argument types SHALL be checked against parameter types.

#### Scenario: Argument count mismatch

- **WHEN** function `fn add(a: int, b: int) -> int` is called as `add(1)`
- **THEN** checker produces diagnostic "expected 2 arguments, found 1"

#### Scenario: Argument type mismatch

- **WHEN** function `fn add(a: int, b: int) -> int` is called as `add(1, "two")`
- **THEN** checker produces diagnostic "argument 2: expected `int`, found `str`"

#### Scenario: Default parameter satisfied

- **WHEN** function `fn greet(name: str, loud: bool = false)` is called as `greet("Alice")`
- **THEN** checker accepts without diagnostic

### Requirement: Union type and nullable checking

The checker SHALL support union types (`A | B`) and nullable types (`T?` as sugar for `T | nil`). Assigning a value of type `A` to `A | B` SHALL be allowed. Assigning a non-nullable value to a nullable type SHALL be allowed. Accessing members on a union type without narrowing SHALL produce a diagnostic if the member is not common to all union members.

#### Scenario: Nullable assignment

- **WHEN** the input is `let x: str? = nil`
- **THEN** checker accepts without diagnostic

#### Scenario: Union type assignment

- **WHEN** the input is `let x: str | int = 42`
- **THEN** checker accepts without diagnostic

### Requirement: Match arm type narrowing

Within a `match` expression, the checker SHALL narrow the matched value's type inside each arm based on the pattern. For enum patterns, the arm body SHALL see the matched variant's fields. For literal patterns, the matched value SHALL be narrowed to that literal's type.

#### Scenario: Enum variant narrowing

- **WHEN** the input matches `Status::Active(since)` inside a match arm
- **THEN** within that arm, `since` has type `str` (as declared on the `Active` variant)

#### Scenario: Wildcard captures full type

- **WHEN** the input uses `_ => expr` as a catch-all arm
- **THEN** the value retains its original type within the arm body

### Requirement: Return type checking

The checker SHALL verify that a function body's evaluated type matches the declared return type. For functions with explicit `ret` statements, every `ret` expression SHALL match the return type. For implicit return (last expression), its type SHALL match the return type.

#### Scenario: Return type mismatch

- **WHEN** `fn foo() -> int { "hello" }`
- **THEN** checker produces diagnostic "return type mismatch: expected `int`, found `str`"

#### Scenario: Multiple return paths

- **WHEN** `fn foo(x: bool) -> int { if x { ret "no" }; 42 }`
- **THEN** checker produces diagnostic on `ret "no"`: "return type mismatch: expected `int`, found `str`"

### Requirement: Mutability checking

The checker SHALL enforce that `let` and `const` bindings cannot be reassigned. Only `mut` bindings SHALL allow reassignment. Attempting to assign to a `let` or `const` binding SHALL produce a diagnostic.

#### Scenario: Reassign immutable

- **WHEN** the input is `let x = 1; x = 2`
- **THEN** checker produces diagnostic "cannot assign to immutable binding `x`"

#### Scenario: Reassign mutable

- **WHEN** the input is `mut x = 1; x = 2`
- **THEN** checker accepts without diagnostic
