# OXIDICE

> Note: This README is translated from `README_zh.md` by ChatGPT.

A fast, extensible dice expression engine for tabletop role-playing games, written in Rust and able to compile to WASM.

## Overview

Oxidice is a dice expression parser and execution engine designed for tabletop RPG systems. It supports most common dice syntaxes, adds some custom syntax, and provides both a Rust API and a WebAssembly interface for frontend applications. You can use it directly in web apps, and it also works well with animation libraries such as [@3d-dice/dice-box](https://github.com/3d-dice/dice-box).

## Motivation

This project started for two main reasons:

1. Dissatisfaction with existing dice parsers and engines, mainly because:

   - Missing or incomplete support for nested rolls (e.g. `(1d6)d6`). For example, the engine used by Roll20 does not support nested rolls, and while FVTT's engine supports them, the accompanying roll animations and result rendering are not solid.
   - Lack of native list support. Most engines do not support lists; list behavior in FVTT is odd and hard to use.
   - Inconsistent modifier behavior. When multiple modifiers are applied to a die, users have a hard time predicting the final result.
   - Function support (e.g. `max`, `min`). FVTT relies on JavaScript functions; beyond security concerns, this approach prevents functions from being used anywhere in an expression.
   - You cannot know if an expression is valid before rolling, and invalid expressions do not provide helpful errors.

2. Poor developer experience when using existing dice expression parsers and engines. Most are based on JavaScript and lack complete type hints, which makes development and debugging painful.

## Features

- Comprehensive modifier support covering most commonly used modifiers
- Native function support via keywords instead of JavaScript
- Native list support (lists are first-class with helper functions)
- Complete result recording: almost all roll data and intermediate node values are recorded
- Full nested support with sequential animation playback
- Constant folding and limited expression merging, e.g. `1 + max(1,2,3)` folds to `4`, and `2d6 + 2d6` is optimized into `4d6`. This simplifies expressions and makes results easier to read.
- Type checking to validate expressions before rolling

## Quick Start

### Quick testing on the command line

```
cargo run --bin oxidice
```

### Using in a web application

First compile to wasm (depending on your setup):

```
wasm-pack build --target bundler
```

or

```
wasm-pack build --target web
```

For usage details, see the [example project](https://github.com/xuyisen-x/oxidice-example).

## Detailed Guide

### Type System

This engine has a complete type system:

- List type: a list made of simple numeric values
- Numeric types:
  - Simple number: a single number
  - Dice pool: a collection of roll results from one or more rolls. Later operations may add or remove results. When a simple number is required, the pool collapses to the sum of its values.
    > Example: `3d20dl + 1`. Suppose `3d20` yields `[5, 19, 7]` (a dice pool). After `dl` (modifiers work like a pipeline here), the pool becomes `[19, 7]`. When added to `1`, the pool collapses to `26`, and the final result is `27`.
  - Success pool: a collection of roll results, each tagged as success (S), failure (F), or normal (N). Later operations may change tags. When a simple number is required, success counts as `1`, failure as `-1`, and normal as `0`, then sums to a simple number.
    > Example: `4d20cs>=5df>19 + 1`. Suppose `4d20` yields `[5, 20, 7, 1]`. After `cs>=5`, the type becomes a success pool tagged as `[5:S, 20:S, 7:S, 1:N]`. After `df>19`, it becomes `[5:S, 20:F, 7:S, 1:N]`. When adding `1`, the pool collapses to `1 - 1 + 1 + 0 = 1`, and the final result is `2`.

### Tolerant Evaluation

In some invalid cases, the engine does **not** throw an error:

- For `xdy`, `xdF`, or `xdC` when `x <= 0` or `y <= 0`, it returns an empty dice pool, treated as `0` in numeric operations.
- When a float appears where an integer is expected, it is truncated (matching Rust's `f64` to `i32` behavior). Values beyond the `i32` range are clamped to `i32::MAX`/`i32::MIN`. In this application domain, users are unlikely to exceed `i32` limits.

However, the engine **does** throw errors in these cases:

- Any form of division by zero
- `max` or `min` when asked to return a number from an empty list

### Roll Order and Rounds

Roll order matters with nesting. Example: `1d4 + (1d6)d8!`. The engine greedily traverses the computation graph (tree) from the root and collects all dice ready to roll. The order is:

- Round 1: `1d4` and `1d6`
- Round 2: `1d6` instances of `d8`
- Round 3: if the previous round rolls `n` eights, roll `n d8`
- Round 4: if the previous round rolls `m` eights, roll `m d8`
- ...

### Recursion Limits and Dice Count Limits

When calling the API, you must provide `recursion_limit` and `dice_count_limit`. They prevent expressions from consuming unlimited resources. If the limit is reached before rolling completes, an exception is thrown.

- `recursion_limit` caps the number of roll rounds. Even expressions like `1d6r<8` (which would reroll forever) stop after the limit.
- `dice_count_limit` caps the total number of dice rolled throughout the process.

### Syntax and Precedence

The parser uses recursive descent. Syntax below uses `[]` for optional and `{}` for repetition.

> Note: `**` here is only for list repetition, not exponentiation.

```
expr            = term { ("+" | "-") term } ;
term            = unary { ("*" | "/" | "//" | "%" | "**") unary } ;
unary           = ("+" | "-") unary | dice_with_modifiers ;
dice_with_modifiers
                = dice_expr { modifier } ;

// Dice expressions (count is optional; defaults to 1)
dice_expr       = atom | [atom] ("d" atom | "dc" | "df") ;

// Atomic expressions
atom            = number
                | list
                | function_call
                | "(" expr ")"
                | "{" expr "}" ;

list            = "[" [expr { "," expr }] "]" ;

function_call   = func_name "(" [expr { "," expr }] ")"
                | "filter" mod_param "(" [expr { "," expr }] ")" ;

func_name       = "floor" | "ceil" | "round" | "abs"
                | "max" | "min" | "sum" | "avg"
                | "len" | "rpdice" | "sortd" | "sort" | "tolist" ;

// Modifiers (postfix)
modifier        = type1_modifier
                | type2_modifier
                | type3_modifier ;

type1_modifier  = ("kh" | "kl" | "dh" | "dl") [atom]
                | ("min" | "max") atom ;

type2_modifier  = ("r" | "!" | "!!") [mod_param] [limit] ;

type3_modifier  = ("cs" | "df" | "sf") mod_param ;

mod_param       = [compare_op] atom ;
compare_op      = "<" | "<=" | ">" | ">=" | "=" | "<>" ;

limit           = ("lt" atom ["lc" atom])
                | ("lc" atom ["lt" atom]) ;
```

From this grammar, precedence (high to low) is:

- Atomic expressions (numbers, lists, function calls, parentheses)
- Dice expressions
- Dice modifiers
- Unary `+` and `-`
- Multiplication, division, integer division, modulo, list repetition
- Addition and subtraction

### Usage Guide

In the descriptions below, `x` and `y` are expressions that return numbers (including dice pools and success pools), `lst` returns a list, `dp` returns a dice pool, and `sp` returns a success pool.

Parameters in `[]` are optional. Parameters in `{}` are required.

`mod_param` is a comparison expression like `=3`, `>8`, `<= (1d6)`, etc. The `=` can usually be omitted (`2d8r=1` and `2d8r1` are equivalent). Supported comparison operators:

- `=`: equal
- `<>`: not equal
- `<=`: less than or equal
- `<`: less than
- `>=`: greater than or equal
- `>`: greater than

`limit` restricts rerolls and explosions. It looks like `lt{x}lc{y}`, meaning the total number of reroll/explosion rounds does not exceed `x`, and the total number of dice rolled by reroll/explosion does not exceed `y`. Each can be used alone or together, in any order. Examples: `lt3`, `lc5`, `lt2lc4`, `lc4lt2`.

#### Basic Elements

- Numbers: any numeric literal is a valid expression, e.g. `1`, `2.5`.
- Explicit lists: comma-separated numeric expressions in brackets, e.g. `[]`, `[1]`, `[1d6, 1d8]`.

#### Basic Dice Pools

- `[x]d{y}`: roll `y` dice with `x` faces, returns a dice pool. `x` defaults to 1. Examples: `8d6`, `d20`, `(1d4 + 2)d10`.
- `[x]dF`: roll `x` Fate dice (return -1, 0, or 1), returns a dice pool. `x` defaults to 1. Examples: `4dF`, `(1d6)dF`.
- `[x]dC`: roll `x` coins (return 0 or 1), returns a dice pool. `x` defaults to 1. Examples: `3dC`, `(2d4)dC`.

#### Modifiers

- `{dp}kh[x]`: keep the highest `x` dice, returns a dice pool. `x` defaults to 1. Examples: `4d6kh3`, `2d20kh`.
- `{dp}kl[x]`: keep the lowest `x` dice, returns a dice pool. `x` defaults to 1. Examples: `4d6kl2`, `2d20kl`.
- `{dp}dh[x]`: drop the highest `x` dice, returns a dice pool. `x` defaults to 1. Examples: `4d6dh2`, `2d20dh`.
- `{dp}dl[x]`: drop the lowest `x` dice, returns a dice pool. `x` defaults to 1. Examples: `4d6dl1`, `2d20dl`.
- `{dp}min{x}`: set all values below `x` to `x`, returns a dice pool. Examples: `4d6min3`, `2d20min(1d4 + 2)`.
- `{dp}max{x}`: set all values above `x` to `x`, returns a dice pool. Examples: `4d6max4`, `2d20max15`.
- `{dp}r{mod_param}[limit]`: reroll dice that match `mod_param`. If the new roll still matches, reroll recursively. Returns a dice pool. `limit` restricts rerolling (default: no limit). Examples: `4d6r<3`, `(2d10)d20r=1lt2lc5`. Note: There is no `ro` modifier; use `lt1` to emulate it.
- `{dp}![mod_param][limit]`: for each die matching `mod_param`, roll an extra die; if that roll also matches, keep exploding recursively. Returns a dice pool. If `mod_param` is omitted, it defaults to the die's maximum value (e.g. `1d6!` is `1d6!>5`). Examples: `1d6!>5`, `2d10!lt3`.
- `{dp}!![mod_param][limit]`: similar to the above, but the new roll is added to the triggering die rather than added as a separate die.
- `{dp}sf{mod_param}`: remove dice matching `mod_param`, returns a dice pool. Example: `4d6sf<3`.
- `{dp|sp}df{mod_param}`: mark dice in a dice pool or success pool as failures, returns a success pool. Examples: `4d6df>5`, `4d20cs>=15df=1`.
- `{dp|sp}cs{mod_param}`: mark dice in a success pool as successes, returns a success pool. Example: `4d20cs>=15`.

#### Functions

- `floor`: For a number, returns the floor. For a list, floors each element and returns a list. If multiple parameters are provided, they are treated as a list. Examples: `floor(3.7)`, `floor([1.2, 2.5, 3.8])`, `floor(1.5, 2.8, 3.3)`.
- `ceil`: Same as above, but ceiling.
- `round`: Same as above, but round to nearest integer.
- `abs`: Same as above, but absolute value.
- `max`: If one parameter and it's a list, returns the max. If two parameters and the first is a list while the second is a number `n`, returns the largest `n` values (preserving order). Otherwise, treats all parameters as a list and returns the max. Examples: `max([1, 5, 3, 9, 2])`, `max([1d6, 2d6, 3d6], 2)`, `max(1, 5, 3, 9, 2)`.
- `min`: Same as above, but returns the minimum or smallest `n` values.
- `sum`: For a list, returns the sum; otherwise treats all parameters as a list. For empty lists, returns 0. Examples: `sum([1, 2, 3, 4])`, `sum(1, 2, 3, 4)`.
- `avg`: Same as above, but returns the average; empty lists return 0.
- `len`: For a list, returns element count; otherwise treats all parameters as a list. Examples: `len([1, 2, 3, 4])`, `len(1, 2, 3, 4)`.
- `sort`: For a list, returns a new ascending list; otherwise treats all parameters as a list. Examples: `sort([3, 1, 4, 2])`, `sort(3, 1, 4, 2)`.
- `sortd`: Same as above, but descending.
- `tolist`: Accepts one dice pool or success pool and returns a list. Examples: `tolist(4d6dl1)`, `tolist(4d20cs>=15df=1)`.
- `filter{mod_param}`: For a list, returns a new list with elements that satisfy `mod_param`. Otherwise treats all parameters as a list. Examples: `filter>=3([1,2,3,4,5])`, `filter<=(1d6)(1,2,3,4,5,6)`.

#### Binary Operators

- Addition `+`: for two numbers, returns their sum; for two lists, concatenates them; for a number and a list, performs broadcast addition.
- Subtraction `-`: for two numbers, returns their difference; for a number and a list, performs broadcast subtraction.
- Multiplication `*`: for two numbers, returns their product; for a number and a list, performs broadcast multiplication.
- Division `/`: for two numbers, returns their quotient; for a number and a list, performs broadcast division.
- Integer division `//`: for two numbers, returns integer quotient; for a number and a list, performs broadcast division and then floors.
- Modulo `%`: for two numbers, returns modulo; for a number and a list, performs broadcast modulo.

#### Special

The following are special operations that disappear when converting the AST to higher-level IR (a kind of syntactic sugar):

- List repetition `lst ** x`: repeats the list `x` times. Example: `[1,2] ** 3` becomes `[1,2,1,2,1,2]`. `lst` must be an explicit list or foldable into one, and `x` must be a positive constant or foldable expression. This supports expressions like `[4d6kh3] ** 6`, a concise way to model classic DND 5e ability score rolls.

- Dice repetition `rpdice`: takes one parameter and doubles all dice counts in it (used to model critical hits). Example: `rpdice(1d8 + 2d6)` becomes `2d8 + 4d6`, and `(1d6)d10` becomes `(2d6*2)d10`. Note that `rpdice` is not evaluation; it directly manipulates the AST.

## Project Structure

- `types/`: structures
- `optimizer/`: constant folding and expression merging (HIR -> HIR)
- `bin/`: other executables for performance tests
- `grammar.rs`: parse string to AST (String -> Expr)
- `lower.rs`: lower AST to typed high-level IR (Expr -> HIR)
- `compiler.rs`: compile HIR to EvalGraph (HIR -> EvalGraph)
- `runtime_engine.rs`: EvalGraph execution and external interaction
- `runtime.rs`: wraps runtime_engine as a wasm interface
- `render_result.rs`: extract EvalGraph topology and values into a frontend-friendly format
- `lib.rs`
- `main.rs`

## License

This project is licensed under the MIT License.
