# Generated warning investigation — 2026-10-03

Baseline: PolyXML 0.34.3, commit a3b0fd3. The investigation used a detached
baseline worktree; it did not infer correctness from successful compilation.

## Rust unreachable patterns

UCI's retained full Rust compile reported 110 unreachable-pattern warnings.
The generator treated every tagged-union field as an inline choice, including
named elements containing a choice. Two sibling wrappers sharing branch names
therefore produced identical match arms. The writer also omitted the wrappers.

A minimal independently validated schema has StartingPosition and StepIncrement,
both containing Percentage or NumberOfSteps. Baseline generated Rust rejects a
valid document with `Missing required field 'StartingPosition'`. Dispatch now
uses a named wrapper's XML name, then reads one alternative inside it, consumes
the closing wrapper and preserves the wrapper on writes. Inline choices retain
branch-tag dispatch. PHF dispatch uses the same distinction.

The regression runs generated consumers with unreachable patterns denied in
owned and borrowed modes, including PHF. It covers required, optional, repeated
wrappers and an empty string branch; missing/multiple/unknown alternatives fail.
These tests establish the covered behavior, not complete XSD validation.

## C# inherited property collisions

NeTEx's EntityStructure has a nameOfClass attribute, while the derived
CodespaceAssignment group has a distinct NameOfClass element. Both normalize
to NameOfClass in C#. A minimal three-level inheritance fixture reproduces the
collision. Baseline records report ignored property annotations (CS0657), an
unread positional parameter (CS8907), and an inherited helper collision
(CS0108). Deserializing an attribute and element then serializing emits only
the attribute: the child element is lost.

The generator now reserves actual generated property names on every ancestor
before naming derived members. Distinct XML members receive suffixed C# names
while their XML/JSON metadata retains the original wire names. Existing reuse
of redeclared inherited XML members is preserved.

The regression checks record and mutable-class compilation with CS0108,
CS8866, CS0657 and CS8907 treated as errors, then checks base-reference access,
XML reads/writes and JSON values across three inheritance levels. Local fixtures
target net8.0 and run on .NET 10 using DOTNET_ROLL_FORWARD=Major; minimum .NET 8
runtime execution remains a CI responsibility.

## Approach

Fix semantic warnings at the generator rather than adding blanket suppression
or C# new modifiers. Keep meaningful generated-runtime regression tests. Unused
imports and empty module reexports are lower-priority cleanup and do not by
themselves establish data loss. The other large-module compilation failures in
issue #135 remain separate from this warning fix.

Official references: [Rust unreachable patterns](https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#unreachable-patterns),
[C# CS0108](https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/compiler-messages/cs0108).

Verification: workspace format, strict Clippy, workspace tests, Python lint and
112 tests with 100% statement/branch coverage; seven-language codegen smoke;
507 UPA decisions matching Xerces; strict documentation build. Both minimal
schemas and input documents independently passed lxml XMLSchema validation.
